//! Logging: tauri-plugin-log writes to the platform log directory, and panics are logged too.
//!
//! | OS      | log file                                                          |
//! |---------|-------------------------------------------------------------------|
//! | Linux   | `$XDG_DATA_HOME/com.dsuv.realamadeus/logs/RealAmadeus.log`        |
//! | macOS   | `~/Library/Logs/com.dsuv.realamadeus/RealAmadeus.log`             |
//! | Windows | `%LOCALAPPDATA%\com.dsuv.realamadeus\logs\RealAmadeus.log`        |
//!
//! `amadeus-backend` logs through `tracing`; the `log` feature of `tracing` (enabled in this
//! crate's `Cargo.toml`) turns those events into `log` records because no tracing subscriber is
//! installed here, so they land in the same file. That bridge also turns the `info` spans of
//! libraries into records (zbus logs every D-Bus call it serves, e.g. the single-instance handover
//! of a second launch), so only the app's own crates are recorded from `info` up and every other
//! target from `warn` up.

use std::any::Any;

use log::LevelFilter;
use tauri::{plugin::TauriPlugin, Runtime};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

/// Rotate the log file at this size…
const MAX_LOG_FILE_BYTES: u128 = 2 * 1024 * 1024;
/// …and keep this many rotated files.
const ROTATED_LOG_FILES: usize = 3;

/// Level of the app's own records ([`APP_LOG_TARGETS`]).
const APP_LOG_LEVEL: LevelFilter = LevelFilter::Info;
/// Level of every other target: the libraries (zbus, tao, wry, reqwest, …) and `tracing`'s span
/// lifecycle records (`tracing::span`).
const LIBRARY_LOG_LEVEL: LevelFilter = LevelFilter::Warn;
/// Log targets of the app's own crates: their module paths, which also cover every submodule
/// (`amadeus_desktop_lib::shell`) and the backend's explicit `amadeus_backend::chat` target.
/// Panics are logged at `error` under the target `panic`, which every level lets through.
const APP_LOG_TARGETS: [&str; 4] = [
    "amadeus_desktop",
    "amadeus_desktop_lib",
    "amadeus_backend",
    "amadeus_core",
];

/// Per-target level overrides.
type TargetLevels = [(&'static str, LevelFilter); APP_LOG_TARGETS.len()];

/// The level table of [`plugin`]: the default level and the per-target overrides.
fn log_levels() -> (LevelFilter, TargetLevels) {
    (
        LIBRARY_LOG_LEVEL,
        APP_LOG_TARGETS.map(|target| (target, APP_LOG_LEVEL)),
    )
}

/// tauri-plugin-log writing to the app log directory (and stdout in debug builds): the app's own
/// records from `info` up, everything else from `warn` up ([`log_levels`]).
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    let (default_level, app_levels) = log_levels();
    let mut builder = tauri_plugin_log::Builder::new()
        .clear_targets()
        .target(Target::new(TargetKind::LogDir { file_name: None }))
        .level(default_level)
        .max_file_size(MAX_LOG_FILE_BYTES)
        .rotation_strategy(RotationStrategy::KeepSome(ROTATED_LOG_FILES))
        .timezone_strategy(TimezoneStrategy::UseLocal);
    for (target, level) in app_levels {
        builder = builder.level_for(target, level);
    }
    if cfg!(debug_assertions) {
        builder = builder.target(Target::new(TargetKind::Stdout));
    }
    builder.build()
}

/// Logs every panic (thread, location, message, backtrace) before the default hook runs.
/// Records emitted before the log plugin is set up are dropped by the `log` facade.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "an unknown location".to_owned());
        log::error!(
            target: "panic",
            "thread '{}' panicked at {location}: {}\n{}",
            thread.name().unwrap_or("<unnamed>"),
            panic_message(info.payload()),
            std::backtrace::Backtrace::force_capture()
        );
        previous(info);
    }));
}

/// Text of a panic payload (`panic!` with a literal or a formatted message).
pub fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "(non-string panic payload)".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use log::{Level, LevelFilter, Log, Metadata};
    use tauri_plugin_log::fern;

    use super::{log_levels, panic_message};

    #[test]
    fn library_info_records_are_dropped_and_the_app_keeps_its_own() {
        // The same table on a plain fern dispatch (tauri-plugin-log is a fern dispatch too).
        let (default_level, app_levels) = log_levels();
        let mut dispatch = fern::Dispatch::new().level(default_level);
        for (target, level) in app_levels {
            dispatch = dispatch.level_for(target, level);
        }
        let (max_level, logger) = dispatch.chain(fern::Output::call(|_| {})).into_log();
        assert_eq!(max_level, LevelFilter::Info);
        let enabled = |target: &str, level: Level| {
            logger.enabled(&Metadata::builder().target(target).level(level).build())
        };

        // The second-launch line of the platform audit, other library `info` records and span
        // lifecycle records without fields.
        for target in [
            "zbus::object_server",
            "zbus::connection",
            "zbus",
            "tracing::span",
            "tao::platform_impl::platform::device",
            "amadeus_desktop_library",
        ] {
            assert!(!enabled(target, Level::Info), "{target}");
        }
        assert!(enabled("zbus::object_server", Level::Warn));
        assert!(enabled("tauri::manager", Level::Error));

        for target in [
            "amadeus_desktop",
            "amadeus_desktop_lib",
            "amadeus_desktop_lib::shell",
            "amadeus_desktop_lib::platform",
            "amadeus_backend::chat",
            "amadeus_core::providers",
        ] {
            assert!(enabled(target, Level::Info), "{target}");
            assert!(!enabled(target, Level::Debug), "{target}");
        }
        assert!(enabled("panic", Level::Error));
    }

    #[test]
    fn panic_payloads() {
        let literal = std::panic::catch_unwind(|| panic!("literal")).unwrap_err();
        assert_eq!(panic_message(literal.as_ref()), "literal");
        let formatted = std::panic::catch_unwind(|| panic!("value {}", 42)).unwrap_err();
        assert_eq!(panic_message(formatted.as_ref()), "value 42");
        let other = std::panic::catch_unwind(|| std::panic::panic_any(7_u8)).unwrap_err();
        assert_eq!(panic_message(other.as_ref()), "(non-string panic payload)");
    }
}
