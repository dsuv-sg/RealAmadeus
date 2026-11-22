//! Browser-console backend of the `log` facade (web build and desktop webview).
//!
//! V1 started the Cubism framework with `option.loggingLevel = LogLevel_Off` and no log
//! function, so every `CubismLog*` line was dropped (spec-live2d-pipeline §4.4, §11;
//! checklist L2D-015). `amadeus-live2d` sends those messages through `log` with the target
//! [`CUBISM_TARGET`], together with the Core's own log lines (which reach its registered log
//! handler) and its V2-only render notes for cases where V1 printed nothing. That target
//! therefore gets its own level, [`DEFAULT_CUBISM_LEVEL`] = `Off`, exactly like V1. Everything
//! else passes at [`APP_LEVEL`] as before.
//!
//! The console lines V1 does print are unaffected: the player's `[ReamaCubism] …` lines and the
//! other §11 strings are written with `console.*` directly, the Core prints its
//! `Live2D Cubism SDK Core Version …` banner itself, and the framework's V1 `console.warn`
//! sites (`CubismMath.mod`, the expression fade-weight accessors) log under their module path.
//!
//! For debugging, the Cubism level can be raised at build time:
//! `AMADEUS_CUBISM_LOG=warn trunk serve` (`off`, `error`, `warn`, `info`, `debug`, `trace`;
//! anything else keeps `off`).

use std::sync::OnceLock;

use log::{LevelFilter, Log, Metadata, Record};

/// `log` target of the Cubism framework channel (V1 `CubismLog*`) used by `amadeus-live2d`.
pub const CUBISM_TARGET: &str = "cubism";

/// Level of every record outside the Cubism channel (app warnings, …).
pub const APP_LEVEL: LevelFilter = LevelFilter::Info;

/// Level of the Cubism channel unless overridden at build time (V1 `LogLevel_Off`).
pub const DEFAULT_CUBISM_LEVEL: LevelFilter = LevelFilter::Off;

/// Build-time override of the Cubism level (`AMADEUS_CUBISM_LOG`).
const CUBISM_LEVEL_OVERRIDE: Option<&str> = option_env!("AMADEUS_CUBISM_LOG");

/// True for records of the Cubism channel: the target `cubism` or a `cubism::…` sub-target.
pub fn is_cubism_target(target: &str) -> bool {
    target
        .strip_prefix(CUBISM_TARGET)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with("::"))
}

/// The Cubism level for an optional build-time override value; [`DEFAULT_CUBISM_LEVEL`] when it
/// is absent or not a level name (`off`, `error`, `warn`, `info`, `debug`, `trace`, any case).
pub fn cubism_level(value: Option<&str>) -> LevelFilter {
    value
        .and_then(|v| v.trim().parse::<LevelFilter>().ok())
        .unwrap_or(DEFAULT_CUBISM_LEVEL)
}

/// A `log` backend with one level for the Cubism channel and one for everything else; records
/// that pass go to `sink` (the browser console in the app).
pub struct ConsoleLogger {
    app: LevelFilter,
    cubism: LevelFilter,
    sink: fn(&Record),
}

impl ConsoleLogger {
    pub const fn new(app: LevelFilter, cubism: LevelFilter, sink: fn(&Record)) -> Self {
        Self { app, cubism, sink }
    }

    /// The level that applies to `target`.
    pub fn level_for(&self, target: &str) -> LevelFilter {
        if is_cubism_target(target) {
            self.cubism
        } else {
            self.app
        }
    }

    /// The global `log::max_level` to install (the facade drops anything above it early).
    pub fn max_level(&self) -> LevelFilter {
        self.app.max(self.cubism)
    }
}

impl Log for ConsoleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level_for(metadata.target())
    }

    fn log(&self, record: &Record) {
        // The `log!` macros only check the global max level, so filter here too.
        if self.enabled(record.metadata()) {
            (self.sink)(record);
        }
    }

    fn flush(&self) {}
}

/// Installs the console logger (first call wins; later calls are no-ops).
pub fn init() {
    static LOGGER: OnceLock<ConsoleLogger> = OnceLock::new();
    let logger = LOGGER.get_or_init(|| {
        ConsoleLogger::new(
            APP_LEVEL,
            cubism_level(CUBISM_LEVEL_OVERRIDE),
            console_log::log,
        )
    });
    if log::set_logger(logger).is_ok() {
        log::set_max_level(logger.max_level());
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use log::{Level, LevelFilter, Log, Metadata, Record};

    use super::{cubism_level, is_cubism_target, ConsoleLogger, APP_LEVEL, DEFAULT_CUBISM_LEVEL};

    thread_local! {
        static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    /// Test sink: remembers `"<level> <target> <message>"` for this thread.
    fn capture(record: &Record) {
        let line = format!("{} {} {}", record.level(), record.target(), record.args());
        SEEN.with(|seen| seen.borrow_mut().push(line));
    }

    /// Sends `(level, target, message)` records through `logger`; returns what reached the sink.
    fn forwarded(logger: &ConsoleLogger, records: &[(Level, &str, &str)]) -> Vec<String> {
        SEEN.with(|seen| seen.borrow_mut().clear());
        for (level, target, message) in records {
            logger.log(
                &Record::builder()
                    .level(*level)
                    .target(target)
                    .args(format_args!("{message}"))
                    .build(),
            );
        }
        SEEN.with(|seen| std::mem::take(&mut *seen.borrow_mut()))
    }

    const ALL_LEVELS: [Level; 5] = [
        Level::Error,
        Level::Warn,
        Level::Info,
        Level::Debug,
        Level::Trace,
    ];

    #[test]
    fn cubism_channel_targets() {
        for target in ["cubism", "cubism::core"] {
            assert!(is_cubism_target(target), "{target}");
        }
        // Module-path targets (no explicit `target:`) are app records, including the
        // framework's V1 `console.warn` sites and V2's documented render diagnostics.
        for target in [
            "cubismx",
            "Cubism",
            "",
            "amadeus_live2d::math::cubism_math",
            "amadeus_live2d::framework::motion::expression_manager",
            "amadeus_live2d::render::renderer",
            "amadeus_ui::storage",
        ] {
            assert!(!is_cubism_target(target), "{target}");
        }
    }

    #[test]
    fn cubism_level_defaults_to_off_like_v1() {
        assert_eq!(DEFAULT_CUBISM_LEVEL, LevelFilter::Off);
        let cases = [
            (None, LevelFilter::Off),
            (Some(""), LevelFilter::Off),
            (Some("verbose"), LevelFilter::Off),
            (Some("off"), LevelFilter::Off),
            (Some("error"), LevelFilter::Error),
            (Some("warn"), LevelFilter::Warn),
            (Some(" DEBUG "), LevelFilter::Debug),
        ];
        for (value, level) in cases {
            assert_eq!(cubism_level(value), level, "{value:?}");
        }
    }

    #[test]
    fn default_logger_drops_every_cubism_record() {
        let logger = ConsoleLogger::new(APP_LEVEL, cubism_level(None), capture);
        let mut records = Vec::new();
        for level in ALL_LEVELS {
            records.push((level, "cubism", "Failed to loadPhysics()."));
            records.push((level, "cubism::core", "[CSM] [E]csmReviveMocInPlace"));
        }
        assert_eq!(forwarded(&logger, &records), Vec::<String>::new());
        let metadata = Metadata::builder()
            .level(Level::Error)
            .target("cubism")
            .build();
        assert!(!logger.enabled(&metadata));
    }

    #[test]
    fn default_logger_keeps_app_records_at_info() {
        let logger = ConsoleLogger::new(APP_LEVEL, cubism_level(None), capture);
        let records = [
            (
                Level::Warn,
                "amadeus_ui::storage",
                "localStorage is unavailable",
            ),
            (Level::Error, "amadeus_ui::app::config", "import failed"),
            (
                Level::Info,
                "amadeus_ui::app::conversation_io",
                "export cancelled",
            ),
            (Level::Debug, "amadeus_ui::storage", "below Info"),
            (Level::Trace, "amadeus_ui::storage", "below Info"),
            // V1 printed this framework warning with console.warn: it stays visible.
            (
                Level::Warn,
                "amadeus_live2d::math::cubism_math",
                "mod() returns 'NaN'.",
            ),
            // V2 diagnostic documented as logged (D4, V1_DIFFERENCES 9.1).
            (
                Level::Error,
                "amadeus_live2d::render::renderer",
                "shader unavailable",
            ),
        ];
        assert_eq!(
            forwarded(&logger, &records),
            [
                "WARN amadeus_ui::storage localStorage is unavailable",
                "ERROR amadeus_ui::app::config import failed",
                "INFO amadeus_ui::app::conversation_io export cancelled",
                "WARN amadeus_live2d::math::cubism_math mod() returns 'NaN'.",
                "ERROR amadeus_live2d::render::renderer shader unavailable",
            ]
        );
        assert_eq!(logger.max_level(), LevelFilter::Info);
    }

    #[test]
    fn raised_cubism_level_shows_the_channel() {
        let logger = ConsoleLogger::new(APP_LEVEL, cubism_level(Some("warn")), capture);
        let records = [
            (Level::Error, "cubism", "Failed to loadPhysics()."),
            (Level::Warn, "cubism", "unknown Type"),
            (Level::Info, "cubism", "below Warn"),
        ];
        assert_eq!(
            forwarded(&logger, &records),
            [
                "ERROR cubism Failed to loadPhysics().",
                "WARN cubism unknown Type"
            ]
        );
        assert_eq!(logger.max_level(), LevelFilter::Info);

        // Raising only the Cubism channel leaves app records at Info.
        let verbose = ConsoleLogger::new(APP_LEVEL, cubism_level(Some("trace")), capture);
        assert_eq!(verbose.max_level(), LevelFilter::Trace);
        let records = [
            (Level::Debug, "amadeus_ui::storage", "below Info"),
            (Level::Trace, "cubism", "kept"),
        ];
        assert_eq!(forwarded(&verbose, &records), ["TRACE cubism kept"]);
    }
}
