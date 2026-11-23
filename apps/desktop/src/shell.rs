//! Main window lifecycle: creation, navigation policy, fullscreen tracking and graceful quit.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, PoisonError,
    },
    time::Duration,
};

use anyhow::Context as _;
use serde::Serialize;
use tauri::{
    webview::NewWindowResponse, AppHandle, Emitter, Manager, PhysicalSize, Url, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_window_state::{AppHandleExt as _, StateFlags};

/// Label of the only window (declared in `tauri.conf.json`, created in [`create_main_window`]).
pub const MAIN_WINDOW: &str = "main";

/// Emitted with [`FullscreenChanged`] whenever the main window enters or leaves fullscreen.
pub const FULLSCREEN_CHANGED_EVENT: &str = "amadeus://fullscreen-changed";

/// Quiet period after the last resize before the fullscreen state is read once more (macOS
/// animates the transition).
const FULLSCREEN_SETTLE: Duration = Duration::from_millis(500);

/// How long [`prepare_exit`] waits for the window to leave fullscreen and get its windowed size
/// back before exiting anyway.
const LEAVE_FULLSCREEN_TIMEOUT: Duration = Duration::from_millis(2000);

/// Extra time after the windowed size arrived, for the position, before the geometry is saved.
const GEOMETRY_SETTLE: Duration = Duration::from_millis(250);

/// Set once by [`quit`]: concurrent quits collapse into one and close requests are ignored.
static QUITTING: AtomicBool = AtomicBool::new(false);

/// Set while the app prepares to exit (see [`prepare_exit`]). The window then leaves fullscreen
/// only because the app is closing, so [`FULLSCREEN_CHANGED_EVENT`] is not emitted: the UI must
/// keep the user's `Config_ScreenMode`.
static EXITING: AtomicBool = AtomicBool::new(false);

/// Payload of [`FULLSCREEN_CHANGED_EVENT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FullscreenChanged {
    pub fullscreen: bool,
}

/// What tauri-plugin-window-state saves and restores: size, position and maximized state.
///
/// * `FULLSCREEN` is excluded because fullscreen belongs to the UI setting `Config_ScreenMode`
///   (0 = fullscreen, 1 = windowed, kept in the webview's localStorage). If the plugin restored
///   fullscreen natively as well, the two sources of truth could disagree at launch. The UI applies
///   its setting through `window_set_fullscreen` and follows changes through
///   [`FULLSCREEN_CHANGED_EVENT`].
/// * `VISIBLE` and `DECORATIONS` are excluded because the window is created hidden and shown once
///   the saved geometry has been applied (no flash at the default size), and decorations never
///   change.
///
/// The plugin records whatever geometry the window has, including the screen size while
/// fullscreen, so every exit this shell controls goes through [`prepare_exit`] (leave fullscreen
/// first). macOS Cmd+Q / Dock "Quit" terminate the process without a preventable exit request, so
/// quitting that way from fullscreen still stores the screen-sized frame; the next launch is then
/// windowed at that size (or fullscreen again once the UI applies `Config_ScreenMode`).
pub fn window_state_flags() -> StateFlags {
    StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED
}

/// Managed state of the main window.
struct MainWindowState {
    fullscreen: Arc<FullscreenTracker>,
}

/// Creates the main window from its `tauri.conf.json` entry (`"create": false` there), adding a
/// navigation policy that keeps every other page out of the app webview. Runs on the main thread
/// (from `setup`).
pub fn create_main_window(app: &AppHandle) -> anyhow::Result<WebviewWindow> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == MAIN_WINDOW)
        .cloned()
        .with_context(|| format!("tauri.conf.json declares no window labelled `{MAIN_WINDOW}`"))?;
    let app_origin = AppOrigin::current(config.use_https_scheme);
    let dev_url = if tauri::is_dev() {
        app.config().build.dev_url.clone()
    } else {
        None
    };

    let nav_app = app.clone();
    let popup_app = app.clone();
    let builder = WebviewWindowBuilder::from_config(app, &config)
        .context("invalid main window configuration")?;
    // WebView2: render on the discrete GPU (see `gpu`).
    #[cfg(windows)]
    let builder = builder.additional_browser_args(&crate::gpu::windows_browser_args());
    let window = builder
        .on_navigation(move |url| {
            if is_app_url(url, &app_origin, dev_url.as_ref()) {
                return true;
            }
            redirect_to_browser(&nav_app, url, "navigation");
            false
        })
        .on_new_window(move |url, _features| {
            // `window.open()` and `target="_blank"` links: never a second webview.
            redirect_to_browser(&popup_app, &url, "new window");
            NewWindowResponse::Deny
        })
        .build()
        .context("cannot create the main window")?;

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    crate::gpu::enable_webkit_acceleration(&window);

    // tauri-plugin-window-state restored the saved geometry while the window was built.
    let fullscreen = install_window_events(&window);
    app.manage(MainWindowState { fullscreen });
    if let Err(err) = window.show() {
        log::error!("cannot show the main window: {err}");
    }
    if let Err(err) = window.set_focus() {
        log::warn!("cannot focus the main window: {err}");
    }
    Ok(window)
}

/// Where Tauri serves the app's own pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AppOrigin {
    scheme: &'static str,
    host: &'static str,
}

impl AppOrigin {
    /// `tauri://localhost` (macOS, Linux) or `http://tauri.localhost` (Windows; `https` with the
    /// window option `useHttpsScheme`), as Tauri's `tauri_protocol_url`.
    fn current(use_https_scheme: bool) -> Self {
        if cfg!(any(windows, target_os = "android")) {
            Self::WINDOWS[usize::from(use_https_scheme)]
        } else {
            Self::CUSTOM_SCHEME
        }
    }

    const CUSTOM_SCHEME: Self = Self {
        scheme: "tauri",
        host: "localhost",
    };

    const WINDOWS: [Self; 2] = [
        Self {
            scheme: "http",
            host: "tauri.localhost",
        },
        Self {
            scheme: "https",
            host: "tauri.localhost",
        },
    ];
}

/// True for the app's own pages: exactly the [`AppOrigin`] (no other host, port or user info),
/// `about:blank` / `about:srcdoc` and, in development builds, the `devUrl` origin.
///
/// Exactness matters: WebKitGTK and WebView2 resolve every `*.localhost` name to the loopback
/// address, so `http://tauri.localhost:<port>` would show a local web server inside the app
/// window, and Tauri's own local-origin check ignores the port (on Windows such a page would get
/// the app's IPC).
fn is_app_url(url: &Url, app: &AppOrigin, dev_url: Option<&Url>) -> bool {
    if url.scheme() == "about" {
        return matches!(url.path(), "blank" | "srcdoc");
    }
    let app_origin = url.scheme() == app.scheme
        && url.host_str() == Some(app.host)
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none();
    app_origin
        || dev_url.is_some_and(|dev| {
            // Only tuple origins (http/https) compare equal; never an opaque one.
            matches!(url.scheme(), "http" | "https") && dev.origin() == url.origin()
        })
}

/// Opens http(s) URLs in the default browser instead of inside the app; ignores other schemes.
fn redirect_to_browser(app: &AppHandle, url: &Url, why: &str) {
    match external_http_url(url.as_str()) {
        Ok(url) => {
            log::info!(
                "{why} to {} redirected to the default browser",
                url.origin().ascii_serialization()
            );
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = open_in_browser(&app, url).await {
                    log::warn!("{err}");
                }
            });
        }
        Err(_) => log::warn!("blocked {why} to a `{}:` URL", url.scheme()),
    }
}

/// Validates a URL for the default browser: http or https with a host.
pub fn external_http_url(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw.trim()).map_err(|err| format!("invalid URL `{raw}`: {err}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!(
            "only http and https URLs can be opened, not `{}:`",
            url.scheme()
        ));
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(format!("URL `{raw}` has no host"));
    }
    Ok(url)
}

/// Opens `url` with the system's default handler (tauri-plugin-opener, Rust API).
pub async fn open_in_browser(app: &AppHandle, url: Url) -> Result<(), String> {
    let app = app.clone();
    let target = url.to_string();
    tauri::async_runtime::spawn_blocking(move || app.opener().open_url(target, None::<&str>))
        .await
        .map_err(|err| format!("cannot open {url}: {err}"))?
        .map_err(|err| format!("cannot open {url}: {err}"))
}

/// Current native fullscreen state of `window` (the `window_is_fullscreen` command).
pub fn is_fullscreen(window: &WebviewWindow) -> tauri::Result<bool> {
    match window.try_state::<MainWindowState>() {
        Some(main) if window.label() == MAIN_WINDOW => main.fullscreen.is_fullscreen(window),
        _ => window.is_fullscreen(),
    }
}

fn install_window_events(window: &WebviewWindow) -> Arc<FullscreenTracker> {
    let tracker = Arc::new(FullscreenTracker::default());
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    if let Err(err) = gtk_fullscreen::watch(window, &tracker) {
        log::warn!("cannot follow the GTK window state ({err}); fullscreen changes made by the window manager are not reported");
    }
    tracker.reset_reported(window);

    let handle = window.clone();
    let events = Arc::clone(&tracker);
    window.on_window_event(move |event| match event {
        WindowEvent::Resized(_) | WindowEvent::Focused(true) => events.on_change_hint(&handle),
        WindowEvent::CloseRequested { api, .. } => {
            // Closing while fullscreen: leave fullscreen first so window-state saves the windowed
            // geometry instead of the screen size. A close during that sequence is ignored too.
            if QUITTING.load(Ordering::Acquire) || events.is_fullscreen(&handle).unwrap_or(false) {
                api.prevent_close();
                tauri::async_runtime::spawn(quit(handle.app_handle().clone()));
            }
        }
        _ => {}
    });
    tracker
}

/// Knows the window's real fullscreen state and emits [`FULLSCREEN_CHANGED_EVENT`] when it
/// differs from the last one reported.
///
/// tao's `is_fullscreen` is the native state on macOS and Windows (checked on every resize, and
/// once more after resizes have settled). On Linux it is only the state the app last *requested*:
/// GTK's window-state events (see `gtk_fullscreen`) feed the native state instead, so a change
/// made by the window manager is seen too.
#[derive(Default)]
struct FullscreenTracker {
    /// Native state reported by GTK; used once `native_tracked` is set.
    native: AtomicBool,
    native_tracked: AtomicBool,
    reported: Mutex<bool>,
    generation: AtomicU64,
}

impl FullscreenTracker {
    fn is_fullscreen(&self, window: &WebviewWindow) -> tauri::Result<bool> {
        if self.native_tracked.load(Ordering::Acquire) {
            Ok(self.native.load(Ordering::Acquire))
        } else {
            window.is_fullscreen()
        }
    }

    #[cfg_attr(
        not(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        )),
        allow(dead_code)
    )]
    fn set_native(&self, fullscreen: bool) {
        self.native.store(fullscreen, Ordering::Release);
        self.native_tracked.store(true, Ordering::Release);
    }

    /// Takes the current state as the reported one (at window creation: no event).
    fn reset_reported(&self, window: &WebviewWindow) {
        let current = self.is_fullscreen(window).unwrap_or(false);
        *self.reported.lock().unwrap_or_else(PoisonError::into_inner) = current;
    }

    fn on_change_hint(self: &Arc<Self>, window: &WebviewWindow) {
        self.sync(window);
        // Trailing check: only the task of the latest hint in a burst re-reads the state.
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let tracker = Arc::clone(self);
        let window = window.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(FULLSCREEN_SETTLE).await;
            if tracker.generation.load(Ordering::Acquire) == generation {
                tracker.sync(&window);
            }
        });
    }

    fn sync(&self, window: &WebviewWindow) {
        let Ok(fullscreen) = self.is_fullscreen(window) else {
            return;
        };
        {
            let mut reported = self.reported.lock().unwrap_or_else(PoisonError::into_inner);
            if *reported == fullscreen {
                return;
            }
            *reported = fullscreen;
        }
        if EXITING.load(Ordering::Acquire) {
            log::info!(
                "main window {} fullscreen while exiting; the UI is not notified",
                if fullscreen { "entered" } else { "left" }
            );
            return;
        }
        log::info!(
            "main window {} fullscreen",
            if fullscreen { "entered" } else { "left" }
        );
        if let Err(err) = window
            .app_handle()
            .emit(FULLSCREEN_CHANGED_EVENT, FullscreenChanged { fullscreen })
        {
            log::warn!("cannot emit {FULLSCREEN_CHANGED_EVENT}: {err}");
        }
    }
}

/// Linux and the BSDs (GTK): follows `window-state-event`, the only place the real fullscreen
/// state shows up. tao records only what `set_fullscreen` asked for, so a window-manager shortcut,
/// a title-bar double-click or a refused request would otherwise go unnoticed.
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod gtk_fullscreen {
    use std::sync::Arc;

    use gtk::{gdk::WindowState, glib::Propagation, prelude::*};
    use tauri::WebviewWindow;

    use super::FullscreenTracker;

    /// Must run on the main thread (GTK objects are not `Send`); `setup` does.
    pub(super) fn watch(
        window: &WebviewWindow,
        tracker: &Arc<FullscreenTracker>,
    ) -> tauri::Result<()> {
        let gtk_window = window.gtk_window()?;
        let initial = gtk_window
            .window()
            .is_some_and(|gdk_window| gdk_window.state().contains(WindowState::FULLSCREEN));
        tracker.set_native(initial);
        let tracker = Arc::clone(tracker);
        let window = window.clone();
        gtk_window.connect_window_state_event(move |_, event| {
            if event.changed_mask().contains(WindowState::FULLSCREEN) {
                tracker.set_native(event.new_window_state().contains(WindowState::FULLSCREEN));
                tracker.sync(&window);
            }
            Propagation::Proceed
        });
        Ok(())
    }
}

/// Graceful exit: [`prepare_exit`], then exits the app (window-state saves again on exit).
/// Concurrent calls collapse into one.
pub async fn quit(app: AppHandle) {
    if QUITTING.swap(true, Ordering::AcqRel) {
        return;
    }
    prepare_exit(&app).await;
    log::info!("quitting");
    app.exit(0);
}

/// Prepares an exit that is about to happen (quit, or the restart after an update): stops
/// reporting fullscreen changes to the UI, leaves fullscreen (bounded wait) so the windowed
/// geometry rather than the screen size is what tauri-plugin-window-state records, and saves the
/// window state right away (the Windows updater ends the process without an exit event).
/// Returns `true` if the window was fullscreen.
pub async fn prepare_exit(app: &AppHandle) -> bool {
    EXITING.store(true, Ordering::Release);
    let window = app.get_webview_window(MAIN_WINDOW);
    let tracker = app
        .try_state::<MainWindowState>()
        .map(|main| Arc::clone(&main.fullscreen));
    let was_fullscreen = match (window, tracker) {
        (Some(window), Some(tracker)) => leave_fullscreen(&window, &tracker).await,
        _ => false,
    };
    if let Err(err) = app.save_window_state(window_state_flags()) {
        log::warn!("cannot save the window state: {err}");
    }
    was_fullscreen
}

/// Undoes [`prepare_exit`] when the exit did not happen (e.g. the update failed to install).
pub fn resume_after_aborted_exit(app: &AppHandle, was_fullscreen: bool) {
    if was_fullscreen {
        if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
            if let Err(err) = window.set_fullscreen(true) {
                log::warn!("cannot return to fullscreen: {err}");
            }
        }
    }
    EXITING.store(false, Ordering::Release);
}

/// Leaves fullscreen and waits (bounded) until the window is windowed again and has its windowed
/// size (window managers can apply the size after the state). Returns `true` if the window was
/// fullscreen.
async fn leave_fullscreen(window: &WebviewWindow, tracker: &FullscreenTracker) -> bool {
    if !tracker.is_fullscreen(window).unwrap_or(false) {
        return false;
    }
    let fullscreen_size = window.inner_size().ok();
    if let Err(err) = window.set_fullscreen(false) {
        log::warn!("cannot leave fullscreen before exiting: {err}");
        return true;
    }
    let deadline = tokio::time::Instant::now() + LEAVE_FULLSCREEN_TIMEOUT;
    let left = wait_until(deadline, || !tracker.is_fullscreen(window).unwrap_or(false)).await;
    let resized = left
        && wait_until(deadline, || {
            windowed_size_arrived(fullscreen_size, window.inner_size().ok())
        })
        .await;
    if resized {
        tokio::time::sleep(GEOMETRY_SETTLE).await;
    } else {
        log::warn!("the window did not get its windowed geometry back in time; exiting anyway");
    }
    true
}

/// True once the size differs from the one the window had while fullscreen (or either is
/// unknown).
fn windowed_size_arrived(
    fullscreen: Option<PhysicalSize<u32>>,
    current: Option<PhysicalSize<u32>>,
) -> bool {
    match (fullscreen, current) {
        (Some(fullscreen), Some(current)) => fullscreen != current,
        _ => true,
    }
}

/// Polls `condition` every 50 ms until it holds (`true`) or `deadline` passes (`false`).
async fn wait_until(deadline: tokio::time::Instant, mut condition: impl FnMut() -> bool) -> bool {
    loop {
        if condition() {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Second launch (tauri-plugin-single-instance): bring the running window to the front.
pub fn focus_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        log::warn!("second instance: no main window to focus");
        return;
    };
    for result in [window.unminimize(), window.show(), window.set_focus()] {
        if let Err(err) = result {
            log::warn!("second instance: cannot focus the main window: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn app_urls_on_macos_and_linux() {
        let app = AppOrigin::CUSTOM_SCHEME;
        let is_app = |s: &str| is_app_url(&url(s), &app, None);
        assert!(is_app("tauri://localhost/app"));
        assert!(is_app("tauri://localhost/"));
        assert!(is_app("tauri://localhost/docs/x?y=1#z"));
        assert!(is_app("about:blank"));
        assert!(is_app("about:srcdoc"));
        assert!(!is_app("about:config"));
        assert!(
            !is_app("tauri://evil/app"),
            "another host of the app scheme"
        );
        assert!(!is_app("tauri://localhost:8080/app"));
        assert!(!is_app("tauri://user@localhost/app"));
        assert!(!is_app("http://tauri.localhost/app"), "the Windows origin");
        assert!(!is_app("https://tauri.localhost/app"));
        assert!(!is_app("http://tauri.localhost:9701/state"));
        assert!(!is_app("http://localhost/"));
        assert!(!is_app("https://dsuv.elvelt.com/amadeus"));
        assert!(!is_app("file:///etc/passwd"));
        assert!(!is_app("blob:tauri://localhost/123"));
        assert!(!is_app("data:text/html,x"));
    }

    #[test]
    fn app_urls_on_windows() {
        let http = AppOrigin::WINDOWS[0];
        let is_app = |s: &str| is_app_url(&url(s), &http, None);
        assert!(is_app("http://tauri.localhost/app"));
        assert!(is_app("http://tauri.localhost:80/app"), "the default port");
        assert!(is_app("about:blank"));
        assert!(
            !is_app("http://tauri.localhost:9701/state"),
            "a local server on another port"
        );
        assert!(
            !is_app("https://tauri.localhost/app"),
            "https is a different origin"
        );
        assert!(!is_app("http://tauri.localhost.evil.com/"));
        assert!(!is_app("http://evil.tauri.localhost/"));
        assert!(!is_app("http://user:pw@tauri.localhost/"));
        assert!(!is_app("tauri://localhost/app"));

        let https = AppOrigin::WINDOWS[1];
        assert!(is_app_url(
            &url("https://tauri.localhost/app"),
            &https,
            None
        ));
        assert!(!is_app_url(
            &url("http://tauri.localhost/app"),
            &https,
            None
        ));
        assert!(!is_app_url(
            &url("https://tauri.localhost:8443/"),
            &https,
            None
        ));
    }

    #[test]
    fn app_origin_of_this_platform() {
        let origin = AppOrigin::current(false);
        if cfg!(windows) {
            assert_eq!(origin, AppOrigin::WINDOWS[0]);
            assert_eq!(AppOrigin::current(true), AppOrigin::WINDOWS[1]);
        } else {
            assert_eq!(origin, AppOrigin::CUSTOM_SCHEME);
            assert_eq!(AppOrigin::current(true), AppOrigin::CUSTOM_SCHEME);
        }
    }

    #[test]
    fn dev_url_origin() {
        let app = AppOrigin::CUSTOM_SCHEME;
        let dev = url("http://127.0.0.1:1420");
        assert!(is_app_url(
            &url("http://127.0.0.1:1420/app"),
            &app,
            Some(&dev)
        ));
        assert!(is_app_url(&url("tauri://localhost/app"), &app, Some(&dev)));
        assert!(!is_app_url(
            &url("http://127.0.0.1:9999/app"),
            &app,
            Some(&dev)
        ));
        assert!(!is_app_url(
            &url("https://127.0.0.1:1420/app"),
            &app,
            Some(&dev)
        ));
        assert!(!is_app_url(&url("http://127.0.0.1:1420/app"), &app, None));
    }

    #[test]
    fn external_urls() {
        assert_eq!(
            external_http_url(" https://example.com/a?b=c ")
                .unwrap()
                .as_str(),
            "https://example.com/a?b=c"
        );
        assert!(external_http_url("http://localhost:5100").is_ok());
        assert!(external_http_url("file:///etc/passwd").is_err());
        assert!(external_http_url("javascript:alert(1)").is_err());
        assert!(external_http_url("mailto:a@example.com").is_err());
        assert!(external_http_url("not a url").is_err());
    }

    #[test]
    fn fullscreen_event_payload() {
        assert_eq!(
            serde_json::to_string(&FullscreenChanged { fullscreen: true }).unwrap(),
            r#"{"fullscreen":true}"#
        );
    }

    #[test]
    fn window_state_never_touches_fullscreen() {
        let flags = window_state_flags();
        assert!(!flags.contains(StateFlags::FULLSCREEN));
        assert!(!flags.contains(StateFlags::VISIBLE));
        assert!(flags.contains(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED));
    }

    #[test]
    fn windowed_size_detection() {
        let screen = Some(PhysicalSize::new(1920, 1080));
        assert!(!windowed_size_arrived(screen, screen));
        assert!(windowed_size_arrived(
            screen,
            Some(PhysicalSize::new(1280, 720))
        ));
        assert!(windowed_size_arrived(None, screen));
        assert!(windowed_size_arrived(screen, None));
    }

    #[tokio::test]
    async fn wait_until_is_bounded() {
        let started = tokio::time::Instant::now();
        let deadline = started + Duration::from_millis(200);
        assert!(!wait_until(deadline, || false).await);
        assert!(started.elapsed() < Duration::from_secs(2));
        let mut calls = 0;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        assert!(
            wait_until(deadline, || {
                calls += 1;
                calls == 3
            })
            .await
        );
    }

    #[test]
    fn native_state_overrides_the_requested_one() {
        let tracker = FullscreenTracker::default();
        assert!(!tracker.native_tracked.load(Ordering::Acquire));
        tracker.set_native(true);
        assert!(tracker.native_tracked.load(Ordering::Acquire));
        assert!(tracker.native.load(Ordering::Acquire));
    }
}
