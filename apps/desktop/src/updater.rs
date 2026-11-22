//! Automatic updates for RealAmadeus V2 desktop (`dsuv-sg/RealAmadeus`).
//!
//! On startup a background task checks for a newer release in `dsuv-sg/RealAmadeus`:
//! 1. When built with `--features updater` and a configured `plugins.updater` endpoint
//!    (`latest.json`), it first tries `tauri-plugin-updater`.
//! 2. Otherwise (or if `latest.json` is not present on the release, as in standard GitHub
//!    Releases), it queries `https://api.github.com/repos/dsuv-sg/RealAmadeus/releases/latest`
//!    (like `RealAmadeusQT`'s `UpdateManager`), compares the release tag against
//!    [`amadeus_core::version::VERSION`], prompts the user, downloads and verifies the matching
//!    platform installer/bundle against `SHA256SUMS.txt` (when attached), installs the update,
//!    and restarts the application.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use amadeus_core::release::{CHECKSUMS_FILE, RELEASES_PAGE, REPO_SLUG};
use amadeus_core::version::{format_display_version, PRODUCT_NAME, VERSION};
use anyhow::{anyhow, bail, Context};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;

use crate::shell::{self, MAIN_WINDOW};

/// Let the first frame render before doing network work.
const STARTUP_DELAY: Duration = Duration::from_secs(3);

/// Timeout for the GitHub Releases metadata check.
const API_TIMEOUT: Duration = Duration::from_secs(15);

/// Timeout for downloading an installer or bundle archive.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
}

/// Starts the background update check.
pub fn spawn_check(app: &AppHandle) {
    #[cfg(feature = "updater")]
    let tauri_updater_ready = {
        if let Err(err) = app.plugin(tauri_plugin_updater::Builder::new().build()) {
            log::debug!("tauri-plugin-updater not configured ({err}); using GitHub Releases check");
            false
        } else {
            true
        }
    };

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(STARTUP_DELAY).await;
        #[cfg(feature = "updater")]
        if tauri_updater_ready {
            match check_and_install_tauri(&app).await {
                Ok(true) => return,
                Ok(false) => {}
                Err(err) => {
                    log::debug!(
                        "tauri updater check did not complete ({err:#}); falling back to GitHub Releases API"
                    );
                }
            }
        }
        if let Err(err) = check_and_install_github(&app).await {
            log::warn!("update check failed: {err:#}");
        }
    });
}

#[cfg(feature = "updater")]
async fn check_and_install_tauri(app: &AppHandle) -> anyhow::Result<bool> {
    use tauri_plugin_updater::UpdaterExt;

    let Some(update) = app.updater()?.check().await? else {
        log::info!("no update available (latest.json)");
        return Ok(true);
    };
    log::info!(
        "update available (latest.json): {} -> {}",
        update.current_version,
        update.version
    );
    if !ask_to_install(app, &update.version, &update.current_version).await {
        log::info!("update {} postponed by the user", update.version);
        return Ok(true);
    }

    let mut received: u64 = 0;
    let bytes = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                log::debug!(
                    "update download: {received} / {} bytes",
                    total.map_or_else(|| "?".to_owned(), |t| t.to_string())
                );
            },
            || log::info!("update downloaded"),
        )
        .await?;

    let was_fullscreen = shell::prepare_exit(app).await;
    let installer = update.clone();
    let installed = tauri::async_runtime::spawn_blocking(move || installer.install(&bytes))
        .await
        .map_err(anyhow::Error::from)
        .and_then(|result| result.map_err(anyhow::Error::from));
    if let Err(err) = installed {
        shell::resume_after_aborted_exit(app, was_fullscreen);
        return Err(err);
    }
    log::info!("update {} installed; restarting", update.version);
    app.restart()
}

async fn check_and_install_github(app: &AppHandle) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .user_agent(format!("{PRODUCT_NAME}/{VERSION}"))
        .timeout(DOWNLOAD_TIMEOUT)
        .build()
        .context("failed to build HTTP client")?;

    let api_url = format!("https://api.github.com/repos/{REPO_SLUG}/releases/latest");
    let response = client
        .get(&api_url)
        .header("Accept", "application/vnd.github+json")
        .timeout(API_TIMEOUT)
        .send()
        .await
        .context("failed to query GitHub latest release")?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        log::info!("no published GitHub release found yet");
        return Ok(());
    }
    let response = response
        .error_for_status()
        .context("GitHub release check returned an error status")?;
    let release: GithubRelease = response
        .json()
        .await
        .context("invalid JSON from GitHub Releases API")?;

    if release.draft || release.prerelease {
        log::info!(
            "latest release {} is draft/prerelease; skipping",
            release.tag_name
        );
        return Ok(());
    }
    let latest_clean = clean_version(&release.tag_name);
    if latest_clean.is_empty() || !is_newer_version(VERSION, &latest_clean) {
        log::info!("no update available (current={VERSION}, latest={latest_clean})");
        return Ok(());
    }

    log::info!("update available on GitHub Releases: {VERSION} -> {latest_clean}");
    if !ask_to_install(app, &latest_clean, VERSION).await {
        log::info!("update {latest_clean} postponed by the user");
        return Ok(());
    }

    let release_url = if release.html_url.trim().is_empty() {
        RELEASES_PAGE.to_owned()
    } else {
        release.html_url.clone()
    };

    let is_appimage = std::env::var_os("APPIMAGE").is_some();
    let has_dpkg = Path::new("/usr/bin/dpkg").exists();
    let has_rpm = Path::new("/usr/bin/rpm").exists();
    let Some(asset) = select_release_asset(
        &release.assets,
        std::env::consts::OS,
        std::env::consts::ARCH,
        is_appimage,
        has_dpkg,
        has_rpm,
    ) else {
        log::warn!("no matching installer asset found in release; opening {release_url}");
        let _ = app.opener().open_url(&release_url, None::<&str>);
        return Ok(());
    };

    log::info!(
        "downloading update asset {} from {}",
        asset.name,
        asset.browser_download_url
    );
    let bytes = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .with_context(|| format!("failed to download {}", asset.name))?
        .bytes()
        .await
        .with_context(|| format!("failed to read body of {}", asset.name))?;

    // Verify SHA-256 against SHA256SUMS.txt or <asset>.sha256 when present in the release.
    if let Some(sums_asset) = release
        .assets
        .iter()
        .find(|a| a.name == CHECKSUMS_FILE || a.name == format!("{}.sha256", asset.name))
    {
        if let Ok(sums_resp) = client
            .get(&sums_asset.browser_download_url)
            .timeout(API_TIMEOUT)
            .send()
            .await
        {
            if let Ok(sums_text) = sums_resp.text().await {
                verify_sha256sums(&asset.name, &bytes, &sums_text)?;
                log::info!("verified SHA-256 checksum of {}", asset.name);
            }
        }
    }

    let temp_dir = std::env::temp_dir().join(format!("realamadeus-update-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)
        .with_context(|| format!("failed to create {}", temp_dir.display()))?;
    let downloaded_path = temp_dir.join(&asset.name);
    std::fs::write(&downloaded_path, &bytes)
        .with_context(|| format!("failed to write {}", downloaded_path.display()))?;

    let was_fullscreen = shell::prepare_exit(app).await;
    let app_clone = app.clone();
    let asset_name = asset.name.clone();
    let install_res = tauri::async_runtime::spawn_blocking(move || {
        install_downloaded_asset(&app_clone, &downloaded_path, &asset_name)
    })
    .await
    .map_err(anyhow::Error::from)
    .and_then(|r| r);

    if let Err(err) = install_res {
        log::warn!("automatic installation failed ({err:#}); opening release page {release_url}");
        shell::resume_after_aborted_exit(app, was_fullscreen);
        let _ = app.opener().open_url(&release_url, None::<&str>);
    }
    Ok(())
}

fn install_downloaded_asset(app: &AppHandle, path: &Path, asset_name: &str) -> anyhow::Result<()> {
    let lower = asset_name.to_ascii_lowercase();
    if cfg!(target_os = "windows") {
        if lower.ends_with(".exe") {
            // Tauri NSIS installer: /UPDATE preserves settings/shortcuts, /P shows passive
            // progress bar, /R restarts the application automatically once done.
            Command::new(path)
                .args(["/UPDATE", "/P", "/R"])
                .spawn()
                .with_context(|| format!("failed to launch NSIS installer {}", path.display()))?;
            app.exit(0);
            return Ok(());
        }
        if lower.ends_with(".msi") {
            Command::new("msiexec")
                .arg("/i")
                .arg(path)
                .arg("/passive")
                .spawn()
                .context("failed to launch msiexec")?;
            app.exit(0);
            return Ok(());
        }
        if lower.ends_with(".zip") {
            spawn_windows_zip_updater(path)?;
            app.exit(0);
            return Ok(());
        }
    } else if cfg!(target_os = "macos") {
        if lower.ends_with(".app.tar.gz") {
            install_macos_app_tarball(path)?;
            app.restart()
        }
        if lower.ends_with(".dmg") {
            Command::new("open")
                .arg(path)
                .spawn()
                .context("failed to open downloaded .dmg")?;
            return Ok(());
        }
    } else if cfg!(target_os = "linux") {
        if lower.ends_with(".appimage") {
            if let Some(target_appimage) = std::env::var_os("APPIMAGE").map(PathBuf::from) {
                install_linux_appimage(path, &target_appimage)?;
                app.restart()
            }
        }
        if lower.ends_with(".deb") {
            let status = Command::new("pkexec")
                .args(["dpkg", "-i"])
                .arg(path)
                .status()
                .context("failed to run pkexec dpkg -i")?;
            if !status.success() {
                bail!("pkexec dpkg -i exited with {status}");
            }
            app.restart()
        }
        if lower.ends_with(".rpm") {
            let status = Command::new("pkexec")
                .args(["rpm", "-U", "--replacepkgs"])
                .arg(path)
                .status()
                .context("failed to run pkexec rpm -U")?;
            if !status.success() {
                bail!("pkexec rpm -U exited with {status}");
            }
            app.restart()
        }
    }
    bail!("unsupported update asset format: {asset_name}")
}

/// Replaces the currently running macOS `.app` bundle with the `.app` inside `tarball`.
fn install_macos_app_tarball(tarball: &Path) -> anyhow::Result<()> {
    let exe = std::env::current_exe().context("cannot determine current_exe")?;
    // `<App>.app/Contents/MacOS/<bin>` -> `<App>.app`
    let app_bundle = exe
        .ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .ok_or_else(|| {
            anyhow!(
                "current executable {} is not inside a .app bundle",
                exe.display()
            )
        })?
        .to_path_buf();

    let extract_dir = tarball
        .parent()
        .unwrap_or_else(|| Path::new("/tmp"))
        .join("extracted-app");
    let _ = std::fs::remove_dir_all(&extract_dir);
    std::fs::create_dir_all(&extract_dir)?;

    let status = Command::new("tar")
        .arg("-xzf")
        .arg(tarball)
        .arg("-C")
        .arg(&extract_dir)
        .status()
        .context("failed to run tar -xzf")?;
    if !status.success() {
        bail!("tar -xzf exited with {status}");
    }

    let extracted_app = std::fs::read_dir(&extract_dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .ok_or_else(|| anyhow!("no .app bundle found inside {}", tarball.display()))?;

    let backup = app_bundle.with_extension("app.old");
    let _ = std::fs::remove_dir_all(&backup);
    std::fs::rename(&app_bundle, &backup)
        .with_context(|| format!("failed to move {} to backup", app_bundle.display()))?;
    if let Err(err) = std::fs::rename(&extracted_app, &app_bundle) {
        let _ = std::fs::rename(&backup, &app_bundle);
        return Err(err).context("failed to move updated .app into place");
    }
    let _ = std::fs::remove_dir_all(&backup);
    Ok(())
}

/// Atomically replaces the running Linux AppImage with the newly downloaded one.
fn install_linux_appimage(downloaded: &Path, target_appimage: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(downloaded, std::fs::Permissions::from_mode(0o755))?;
    }
    let staged = target_appimage.with_extension("AppImage.new");
    std::fs::copy(downloaded, &staged)
        .with_context(|| format!("failed to copy update to {}", staged.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&staged, target_appimage)
        .with_context(|| format!("failed to replace {}", target_appimage.display()))?;
    Ok(())
}

/// Windows portable `.zip` fallback (same flow as `RealAmadeusQT`'s `updater.ps1`).
fn spawn_windows_zip_updater(zip_path: &Path) -> anyhow::Result<()> {
    let exe = std::env::current_exe().context("cannot determine current_exe")?;
    let app_dir = exe
        .parent()
        .ok_or_else(|| anyhow!("current_exe has no parent directory"))?;
    let exe_name = exe
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let script_path = zip_path.with_file_name("realamadeus_updater.ps1");
    let script = format!(
        "$ErrorActionPreference = 'Stop'\n\
         $PidToWait = {pid}\n\
         $ZipPath = '{zip}'\n\
         $AppDir = '{app_dir}'\n\
         $ExeName = '{exe_name}'\n\
         try {{ Wait-Process -Id $PidToWait -Timeout 15 -ErrorAction SilentlyContinue }} catch {{}}\n\
         Start-Sleep -Seconds 1\n\
         $ExtractDir = Join-Path $env:TEMP 'RealAmadeus_Update_Extract'\n\
         if (Test-Path $ExtractDir) {{ Remove-Item $ExtractDir -Recurse -Force }}\n\
         Expand-Archive -Path $ZipPath -DestinationPath $ExtractDir -Force\n\
         $Source = $ExtractDir\n\
         $Sub = Get-ChildItem -Path $ExtractDir\n\
         if ($Sub.Count -eq 1 -and $Sub[0].PSIsContainer) {{ $Source = $Sub[0].FullName }}\n\
         Copy-Item -Path (Join-Path $Source '*') -Destination $AppDir -Recurse -Force\n\
         Remove-Item $ExtractDir -Recurse -Force -ErrorAction SilentlyContinue\n\
         Remove-Item $ZipPath -Force -ErrorAction SilentlyContinue\n\
         Start-Process -FilePath (Join-Path $AppDir $ExeName)\n",
        pid = std::process::id(),
        zip = zip_path.display().to_string().replace('\'', "''"),
        app_dir = app_dir.display().to_string().replace('\'', "''"),
        exe_name = exe_name.replace('\'', "''"),
    );
    std::fs::write(&script_path, script)?;
    Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script_path)
        .spawn()
        .context("failed to spawn PowerShell zip updater")?;
    Ok(())
}

/// Strips any leading non-digit prefix (`v2.0.1` -> `2.0.1`, `RealAmadeus_V2.1R` -> `2.1`).
pub(crate) fn clean_version(raw: &str) -> String {
    let trimmed = raw.trim();
    let Some(start) = trimmed.find(|c: char| c.is_ascii_digit()) else {
        return String::new();
    };
    let mut rest = trimmed[start..]
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches('.');
    if let Some(stripped) = rest.strip_suffix(['R', 'Q', 'U']) {
        if stripped.ends_with(|c: char| c.is_ascii_digit()) {
            rest = stripped;
        }
    }
    rest.to_owned()
}

/// True when `latest` is strictly newer than `current` (handles 2- or 3-component versions and
/// pre-release suffixes: `2.0.0` == `2.0`, `2.0.0` < `2.0.1`, `2.1.0-rc.1` < `2.1.0`).
pub(crate) fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse = |s: &str| -> (Vec<u64>, Option<String>) {
        let cleaned = clean_version(s);
        let (core, pre) = match cleaned.split_once('-') {
            Some((c, p)) => (c, Some(p.to_owned())),
            None => (cleaned.as_str(), None),
        };
        let mut nums: Vec<u64> = core
            .split('.')
            .filter(|p| !p.is_empty())
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect();
        while nums.len() < 3 {
            nums.push(0);
        }
        (nums, pre)
    };
    let (curr_nums, curr_pre) = parse(current);
    let (late_nums, late_pre) = parse(latest);
    if curr_nums != late_nums {
        return curr_nums < late_nums;
    }
    match (curr_pre, late_pre) {
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (Some(a), Some(b)) => a < b,
        (None, None) => false,
    }
}

/// Picks the best release asset for `(os, arch)`.
pub(crate) fn select_release_asset<'a>(
    assets: &'a [GithubAsset],
    os: &str,
    arch: &str,
    is_appimage: bool,
    has_dpkg: bool,
    has_rpm: bool,
) -> Option<&'a GithubAsset> {
    let find_matching = |pred: &dyn Fn(&str) -> bool| -> Option<&'a GithubAsset> {
        assets.iter().find(|a| {
            let n = a.name.as_str();
            !n.ends_with(".sig") && !n.ends_with(".sha256") && pred(n)
        })
    };

    match os {
        "windows" => {
            let arch_tag = if arch == "aarch64" { "arm64" } else { "x64" };
            find_matching(&|n| {
                n.contains("Windows") && n.contains(arch_tag) && n.ends_with("-setup.exe")
            })
            .or_else(|| find_matching(&|n| n.ends_with("-setup.exe")))
            .or_else(|| {
                find_matching(&|n| {
                    n.contains("Windows") && n.contains(arch_tag) && n.ends_with(".msi")
                })
            })
            .or_else(|| find_matching(&|n| n.ends_with(".msi")))
            .or_else(|| find_matching(&|n| n.contains("Win") && n.ends_with(".zip")))
        }
        "macos" => find_matching(&|n| n.contains("macOS") && n.ends_with(".app.tar.gz"))
            .or_else(|| find_matching(&|n| n.ends_with(".app.tar.gz")))
            .or_else(|| find_matching(&|n| n.contains("macOS") && n.ends_with(".dmg")))
            .or_else(|| find_matching(&|n| n.ends_with(".dmg"))),
        "linux" => {
            let is_arm = arch == "aarch64";
            let appimage_arch = if is_arm { "aarch64" } else { "amd64" };
            let deb_arch = if is_arm { "arm64" } else { "amd64" };
            let rpm_arch = if is_arm { "aarch64" } else { "x86_64" };

            let appimage = || {
                find_matching(&|n| n.contains(appimage_arch) && n.ends_with(".AppImage"))
                    .or_else(|| find_matching(&|n| n.ends_with(".AppImage")))
            };
            let deb = || {
                find_matching(&|n| n.contains(deb_arch) && n.ends_with(".deb"))
                    .or_else(|| find_matching(&|n| n.ends_with(".deb")))
            };
            let rpm = || {
                find_matching(&|n| n.contains(rpm_arch) && n.ends_with(".rpm"))
                    .or_else(|| find_matching(&|n| n.ends_with(".rpm")))
            };

            if is_appimage {
                appimage().or_else(deb).or_else(rpm)
            } else if has_dpkg {
                deb().or_else(appimage).or_else(rpm)
            } else if has_rpm {
                rpm().or_else(appimage).or_else(deb)
            } else {
                appimage().or_else(deb).or_else(rpm)
            }
        }
        _ => None,
    }
}

/// Verifies `bytes` against a `SHA256SUMS.txt` (or `<file>.sha256`) text if `asset_name` is
/// listed in it.
pub(crate) fn verify_sha256sums(
    asset_name: &str,
    bytes: &[u8],
    sums_text: &str,
) -> anyhow::Result<()> {
    let expected = sums_text.lines().find_map(|line| {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let file = parts.next().map(|f| f.trim_start_matches('*'))?;
        (file == asset_name).then_some(hash)
    });
    let Some(expected) = expected else {
        return Ok(());
    };
    let actual: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if !actual.eq_ignore_ascii_case(expected) {
        bail!("SHA-256 mismatch for {asset_name}: expected {expected}, got {actual}");
    }
    Ok(())
}

/// Native yes/no dialog; `false` when declined or closed.
async fn ask_to_install(app: &AppHandle, new_version: &str, current_version: &str) -> bool {
    let new = display(new_version);
    let current = display(current_version);
    let message = format!(
        "RealAmadeus の新しいバージョン {new} が利用可能です（現在: {current}）。\n\
         今すぐダウンロードしてインストールし、アプリを再起動しますか？\n\n\
         A new version of RealAmadeus ({new}) is available (current: {current}).\n\
         Download and install it now, then restart the app?"
    );
    let mut dialog = app
        .dialog()
        .message(message)
        .title("アップデート / Update")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "インストール / Install".to_owned(),
            "後で / Later".to_owned(),
        ));
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        dialog = dialog.parent(&window);
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    dialog.show(move |accepted| {
        let _ = tx.send(accepted);
    });
    rx.await.unwrap_or(false)
}

/// `2.1.0` → `V2.1`, the style of `amadeus_core::version::display_version`.
fn display(version: &str) -> String {
    format_display_version(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_and_compares_versions() {
        assert_eq!(clean_version("v2.0.0"), "2.0.0");
        assert_eq!(clean_version("RealAmadeus_V2.1R"), "2.1");
        assert_eq!(clean_version("V2.1.0-rc.1"), "2.1.0-rc.1");
        assert!(!is_newer_version("2.0.0", "2.0.0"));
        assert!(!is_newer_version("2.0.0", "v2.0"));
        assert!(!is_newer_version("2.1.0", "2.0.9"));
        assert!(is_newer_version("2.0.0", "2.0.1"));
        assert!(is_newer_version("2.0.0", "v2.1"));
        assert!(is_newer_version("2.0.0", "2.1.0"));
        assert!(is_newer_version("2.1.0-rc.1", "2.1.0"));
        assert!(!is_newer_version("2.1.0", "2.1.0-rc.1"));
    }

    #[test]
    fn selects_platform_assets() {
        let assets = vec![
            GithubAsset {
                name: "RealAmadeus_V2.0_Windows_x64-setup.exe".into(),
                browser_download_url: "https://example.com/win.exe".into(),
            },
            GithubAsset {
                name: "RealAmadeus_V2.0_Windows_x64-setup.exe.sig".into(),
                browser_download_url: "https://example.com/win.exe.sig".into(),
            },
            GithubAsset {
                name: "RealAmadeus_V2.0_Windows_x64.msi".into(),
                browser_download_url: "https://example.com/win.msi".into(),
            },
            GithubAsset {
                name: "RealAmadeus_V2.0_macOS_universal.dmg".into(),
                browser_download_url: "https://example.com/mac.dmg".into(),
            },
            GithubAsset {
                name: "RealAmadeus_V2.0_macOS_universal.app.tar.gz".into(),
                browser_download_url: "https://example.com/mac.app.tar.gz".into(),
            },
            GithubAsset {
                name: "RealAmadeus_V2.0_Linux_amd64.AppImage".into(),
                browser_download_url: "https://example.com/linux.AppImage".into(),
            },
            GithubAsset {
                name: "RealAmadeus_V2.0_Linux_amd64.deb".into(),
                browser_download_url: "https://example.com/linux.deb".into(),
            },
            GithubAsset {
                name: "RealAmadeus_V2.0_Linux_x86_64.rpm".into(),
                browser_download_url: "https://example.com/linux.rpm".into(),
            },
        ];

        assert_eq!(
            select_release_asset(&assets, "windows", "x86_64", false, false, false)
                .map(|a| a.name.as_str()),
            Some("RealAmadeus_V2.0_Windows_x64-setup.exe")
        );
        assert_eq!(
            select_release_asset(&assets, "macos", "aarch64", false, false, false)
                .map(|a| a.name.as_str()),
            Some("RealAmadeus_V2.0_macOS_universal.app.tar.gz")
        );
        assert_eq!(
            select_release_asset(&assets, "linux", "x86_64", true, true, false)
                .map(|a| a.name.as_str()),
            Some("RealAmadeus_V2.0_Linux_amd64.AppImage")
        );
        assert_eq!(
            select_release_asset(&assets, "linux", "x86_64", false, true, false)
                .map(|a| a.name.as_str()),
            Some("RealAmadeus_V2.0_Linux_amd64.deb")
        );
        assert_eq!(
            select_release_asset(&assets, "linux", "x86_64", false, false, true)
                .map(|a| a.name.as_str()),
            Some("RealAmadeus_V2.0_Linux_x86_64.rpm")
        );
    }

    #[test]
    fn verifies_sha256sums() {
        let data = b"hello world";
        let good_hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        let sums = format!("{good_hash}  RealAmadeus_V2.0_Windows_x64-setup.exe\n");
        assert!(verify_sha256sums("RealAmadeus_V2.0_Windows_x64-setup.exe", data, &sums).is_ok());
        assert!(
            verify_sha256sums("RealAmadeus_V2.0_Windows_x64-setup.exe", b"tampered", &sums)
                .is_err()
        );
    }
}
