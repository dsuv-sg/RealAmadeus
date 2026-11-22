//! Server configuration: CLI flags with environment fallbacks (clap) and optional `.env`.
//!
//! | flag | env | default |
//! |---|---|---|
//! | `--bind` | `AMADEUS_BIND` | `127.0.0.1:9502` |
//! | `--base-path` | `AMADEUS_BASE_PATH` | `/amadeus` (slashes normalized) |
//! | `--dist-dir` | `AMADEUS_DIST_DIR` | `./dist` |
//! | `--data-dir` | `AMADEUS_DATA_DIR` | `./data` |
//! | `--env-file` | `AMADEUS_ENV_FILE` | `.env` (loaded if present, never required) |
//! | `--vertex-credentials` | `AMADEUS_VERTEX_CREDENTIALS` | none |
//! | `--proxy-allowlist` | `AMADEUS_PROXY_ALLOWLIST` | none (allow all, V1) |
//! | `--immutable-assets` | `AMADEUS_IMMUTABLE_ASSETS` | off (V1 served bundles no-store) |
//! | `--free-api-max` | `AMADEUS_FREEAPI_MAX` | `65000` (V1's FreeAPI cap; a positive integer) |
//! | — | `AI_STUDIO_API_KEY`, fallback `GEMINI_API_KEY` | none (env only, never a flag) |
//! | — | `RUST_LOG` | `info` |
//!
//! An **empty** value (`AMADEUS_DIST_DIR=`, as compose files often produce) means "use the
//! default" for every setting; `AMADEUS_BASE_PATH=/` serves the app at the origin root.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use amadeus_backend::chat::MAX_GLOBAL_REQUESTS;
use amadeus_backend::{BackendConfig, BackendMode};
use amadeus_core::jscompat::js_trim;
use amadeus_core::protocol::paths::DEFAULT_BASE_PATH;
use clap::builder::FalseyValueParser;
use clap::{value_parser, Parser};

/// Default listen address.
pub const DEFAULT_BIND: &str = "127.0.0.1:9502";
/// Default Trunk dist directory.
pub const DEFAULT_DIST_DIR: &str = "./dist";
/// Default data directory.
pub const DEFAULT_DATA_DIR: &str = "./data";
/// Default dotenv file.
pub const DEFAULT_ENV_FILE: &str = ".env";
/// Default FreeAPI cap (V1's `MAX_GLOBAL_REQUESTS`), as the flag's default text.
pub const DEFAULT_FREE_API_MAX: &str = "65000";
/// Largest accepted FreeAPI cap: `Number.MAX_SAFE_INTEGER`, so the quota's `max` and
/// `remaining` stay exact for JavaScript clients (V1 web, V1/V2 desktop relays).
pub const FREE_API_MAX_LIMIT: i64 = 9_007_199_254_740_991;

/// Command line of `amadeus-server` (every flag falls back to an environment variable).
///
/// Values are kept raw here (paths as `OsString`, empty strings allowed) so that an empty
/// environment variable never aborts startup; [`ServerConfig::from_cli`] applies the defaults.
#[derive(Debug, Clone, Parser)]
#[command(name = "amadeus-server", version, about = "RealAmadeus V2 web server", long_about = None)]
pub struct Cli {
    /// Listen address (`host:port`).
    #[arg(long, env = "AMADEUS_BIND", default_value = DEFAULT_BIND)]
    pub bind: String,
    /// Base path of the app; must equal Trunk's `public_url` without the trailing slash
    /// (`/` = origin root).
    #[arg(long, env = "AMADEUS_BASE_PATH", default_value = DEFAULT_BASE_PATH)]
    pub base_path: String,
    /// Trunk output directory (index.html, hashed wasm/js/css, resources/, static/).
    #[arg(long, env = "AMADEUS_DIST_DIR", default_value = DEFAULT_DIST_DIR, value_parser = value_parser!(OsString))]
    pub dist_dir: OsString,
    /// Directory for the chat log, the request counter and rate-limit state.
    #[arg(long, env = "AMADEUS_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_parser = value_parser!(OsString))]
    pub data_dir: OsString,
    /// dotenv file loaded at startup when it exists (existing variables win).
    #[arg(long, env = "AMADEUS_ENV_FILE", default_value = DEFAULT_ENV_FILE, value_parser = value_parser!(OsString))]
    pub env_file: OsString,
    /// Google service-account JSON used by FreeAPI / Vertex.
    #[arg(long, env = "AMADEUS_VERTEX_CREDENTIALS", value_parser = value_parser!(OsString))]
    pub vertex_credentials: Option<OsString>,
    /// Comma-separated allowlist for the TTS/STT/RAG proxy targets (URL prefixes or
    /// `host[:port|:*]`). Unset = allow every URL (V1 behavior).
    #[arg(long, env = "AMADEUS_PROXY_ALLOWLIST")]
    pub proxy_allowlist: Option<String>,
    /// Serve Trunk's content-hashed js/wasm/css as `immutable` (V1 sent `no-store`). The env
    /// value is false for `0`, `false`, `no`, `off`, `n`, `f` or empty, true otherwise.
    #[arg(long, env = "AMADEUS_IMMUTABLE_ASSETS", value_parser = FalseyValueParser::new())]
    pub immutable_assets: bool,
    /// Global FreeAPI cap (a whole number from 1): requests for provider 8 or without a provider
    /// (as in V1) are refused once the request counter (`.amadeus_request_count`) reaches it;
    /// `/chat/quota` reports it as `max`.
    // `allow_negative_numbers`: `--free-api-max -5` is the cap's (invalid) value, so it gets the
    // same startup message as `--free-api-max=-5` and `AMADEUS_FREEAPI_MAX=-5` instead of clap's
    // "unexpected argument"; a flag after `--free-api-max` is still a missing value.
    #[arg(
        long,
        env = "AMADEUS_FREEAPI_MAX",
        default_value = DEFAULT_FREE_API_MAX,
        value_name = "N",
        allow_negative_numbers = true
    )]
    pub free_api_max: String,
}

/// `value`, or `default` when it is empty / blank.
fn non_empty_or(value: String, default: &str) -> String {
    if js_trim(&value).is_empty() {
        default.to_owned()
    } else {
        value
    }
}

/// `value` as a path, or `default` when it is empty.
fn path_or(value: OsString, default: &str) -> PathBuf {
    if value.is_empty() {
        PathBuf::from(default)
    } else {
        PathBuf::from(value)
    }
}

/// Outcome of loading the dotenv file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvFileStatus {
    /// Variables were loaded from this file.
    Loaded(PathBuf),
    /// The file does not exist (normal).
    Missing(PathBuf),
    /// The file exists but could not be read / parsed.
    Error(PathBuf, String),
}

/// Resolved server configuration.
#[derive(Clone)]
pub struct ServerConfig {
    /// Listen address.
    pub bind: String,
    /// Normalized base path: `""` for the root, otherwise `/seg[/seg…]` without a trailing
    /// slash.
    pub base_path: String,
    /// Trunk dist directory.
    pub dist_dir: PathBuf,
    /// Data directory of the backend.
    pub data_dir: PathBuf,
    /// The dotenv file path that was considered.
    pub env_file: PathBuf,
    /// Service-account JSON path.
    pub vertex_credentials: Option<PathBuf>,
    /// AI Studio key (never logged).
    pub ai_studio_api_key: Option<String>,
    /// Proxy allowlist entries.
    pub proxy_allowlist: Option<Vec<String>>,
    /// Immutable caching of hashed assets.
    pub immutable_assets: bool,
    /// Global FreeAPI cap (positive).
    pub free_api_max: i64,
}

impl fmt::Debug for ServerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerConfig")
            .field("bind", &self.bind)
            .field("base_path", &self.base_path)
            .field("dist_dir", &self.dist_dir)
            .field("data_dir", &self.data_dir)
            .field("env_file", &self.env_file)
            .field("vertex_credentials", &self.vertex_credentials)
            .field(
                "ai_studio_api_key",
                &self.ai_studio_api_key.as_ref().map(|_| "<set>"),
            )
            .field("proxy_allowlist", &self.proxy_allowlist)
            .field("immutable_assets", &self.immutable_assets)
            .field("free_api_max", &self.free_api_max)
            .finish()
    }
}

impl ServerConfig {
    /// Builds the configuration from parsed flags and the (already resolved) AI Studio key.
    /// Empty values select the defaults (see the module docs).
    pub fn from_cli(cli: Cli, ai_studio_api_key: Option<String>) -> Result<Self, String> {
        Ok(Self {
            bind: non_empty_or(cli.bind, DEFAULT_BIND),
            base_path: normalize_base_path(&non_empty_or(cli.base_path, DEFAULT_BASE_PATH))?,
            dist_dir: path_or(cli.dist_dir, DEFAULT_DIST_DIR),
            data_dir: path_or(cli.data_dir, DEFAULT_DATA_DIR),
            env_file: path_or(cli.env_file, DEFAULT_ENV_FILE),
            vertex_credentials: cli
                .vertex_credentials
                .filter(|p| !p.is_empty())
                .map(PathBuf::from),
            ai_studio_api_key,
            proxy_allowlist: parse_allowlist(cli.proxy_allowlist.as_deref()),
            immutable_assets: cli.immutable_assets,
            free_api_max: parse_free_api_max(&cli.free_api_max)?,
        })
    }

    /// Defaults for a given dist / data directory (tests, embedding).
    pub fn new(dist_dir: impl Into<PathBuf>, data_dir: impl Into<PathBuf>) -> Self {
        Self {
            bind: DEFAULT_BIND.to_owned(),
            base_path: DEFAULT_BASE_PATH.to_owned(),
            dist_dir: dist_dir.into(),
            data_dir: data_dir.into(),
            env_file: PathBuf::from(DEFAULT_ENV_FILE),
            vertex_credentials: None,
            ai_studio_api_key: None,
            proxy_allowlist: None,
            immutable_assets: false,
            free_api_max: MAX_GLOBAL_REQUESTS,
        }
    }

    /// Loads `.env` (if present) and parses flags / environment. Exits the process on
    /// `--help`, `--version` and invalid flags (clap behavior).
    pub fn load() -> Result<(Self, EnvFileStatus), String> {
        // Phase 1: locate the dotenv file without failing (phase 2 reports errors / help).
        let raw_env_file = Cli::try_parse()
            .map(|c| c.env_file)
            .unwrap_or_else(|_| std::env::var_os("AMADEUS_ENV_FILE").unwrap_or_default());
        let env_file = path_or(raw_env_file, DEFAULT_ENV_FILE);
        let env_status = load_env_file(&env_file);
        // Phase 2: the dotenv values are now visible to clap's env fallbacks.
        let cli = Cli::parse();
        let key = resolve_ai_studio_key(
            std::env::var("AI_STUDIO_API_KEY").ok(),
            std::env::var("GEMINI_API_KEY").ok(),
        );
        Ok((Self::from_cli(cli, key)?, env_status))
    }

    /// The backend configuration (web server mode).
    pub fn backend_config(&self) -> BackendConfig {
        BackendConfig {
            mode: BackendMode::Server,
            data_dir: self.data_dir.clone(),
            vertex_credentials_path: self.vertex_credentials.clone(),
            ai_studio_api_key: self.ai_studio_api_key.clone(),
            proxy_allowlist: self.proxy_allowlist.clone(),
            free_api_max: self.free_api_max,
        }
    }
}

/// Loads a dotenv file without overriding variables that are already set.
pub fn load_env_file(path: &Path) -> EnvFileStatus {
    if !path.is_file() {
        return EnvFileStatus::Missing(path.to_path_buf());
    }
    match dotenvy::from_path(path) {
        Ok(()) => EnvFileStatus::Loaded(path.to_path_buf()),
        Err(err) => EnvFileStatus::Error(path.to_path_buf(), err.to_string()),
    }
}

/// Normalizes `AMADEUS_BASE_PATH`: surrounding whitespace removed, repeated / leading /
/// trailing slashes collapsed, `"/"` → `""`. Only unreserved URL characters are accepted in a
/// segment (the base is used verbatim in routes and redirects).
pub fn normalize_base_path(raw: &str) -> Result<String, String> {
    let segments: Vec<&str> = raw.trim().split('/').filter(|s| !s.is_empty()).collect();
    for segment in &segments {
        let valid = segment
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'));
        if !valid || *segment == "." || *segment == ".." {
            return Err(format!(
                "invalid AMADEUS_BASE_PATH {raw:?}: segments may only contain A-Z a-z 0-9 - . _ ~"
            ));
        }
    }
    Ok(if segments.is_empty() {
        String::new()
    } else {
        format!("/{}", segments.join("/"))
    })
}

/// Parses `AMADEUS_FREEAPI_MAX` / `--free-api-max`: blank selects the default (65000, V1's cap),
/// otherwise a whole number from 1 to [`FREE_API_MAX_LIMIT`] (surrounding whitespace ignored).
/// Anything else (`0`, `-5`, `1e5`, `65,000`, `abc`) stops the server with a message.
pub fn parse_free_api_max(raw: &str) -> Result<i64, String> {
    let value = js_trim(raw);
    if value.is_empty() {
        return Ok(MAX_GLOBAL_REQUESTS);
    }
    // Digits only: `str::parse` alone would also take a leading `+`.
    let parsed = if value.bytes().all(|b| b.is_ascii_digit()) {
        value.parse::<i64>().ok()
    } else {
        None
    };
    parsed
        .filter(|n| (1..=FREE_API_MAX_LIMIT).contains(n))
        .ok_or_else(|| {
            format!(
                "invalid AMADEUS_FREEAPI_MAX / --free-api-max {raw:?}: expected a whole number of \
                 requests from 1 to {FREE_API_MAX_LIMIT} (default {DEFAULT_FREE_API_MAX})"
            )
        })
}

/// Parses `AMADEUS_PROXY_ALLOWLIST` (comma separated, trimmed, empty entries dropped);
/// `None` when unset or empty.
pub fn parse_allowlist(raw: Option<&str>) -> Option<Vec<String>> {
    let entries: Vec<String> = raw?
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    if entries.is_empty() {
        None
    } else {
        Some(entries)
    }
}

/// `process.env.AI_STUDIO_API_KEY || process.env.GEMINI_API_KEY` (JS `||`: an empty value
/// falls back). The value is handed to the backend raw: the chat module applies the rest of V1
/// `getAiStudioApiKey()` (`"undefined"` / blank rejected, trimmed), so those rules run exactly
/// once. Unlike V1 there is no hard-coded fallback key.
pub fn resolve_ai_studio_key(ai_studio: Option<String>, gemini: Option<String>) -> Option<String> {
    ai_studio
        .filter(|k| !k.is_empty())
        .or_else(|| gemini.filter(|k| !k.is_empty()))
}

/// Whether V1 `getAiStudioApiKey()` would use this raw key (not `"undefined"`, not blank).
/// Only used for the startup log.
pub fn ai_studio_key_usable(raw: &str) -> bool {
    raw != "undefined" && !js_trim(raw).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_path_normalization() {
        assert_eq!(normalize_base_path("/amadeus"), Ok("/amadeus".to_owned()));
        assert_eq!(normalize_base_path(" amadeus/ "), Ok("/amadeus".to_owned()));
        assert_eq!(
            normalize_base_path("//amadeus//v2//"),
            Ok("/amadeus/v2".to_owned())
        );
        assert_eq!(normalize_base_path("/"), Ok(String::new()));
        assert_eq!(normalize_base_path(""), Ok(String::new()));
        assert!(normalize_base_path("/{x}").is_err());
        assert!(normalize_base_path("/a b").is_err());
        assert!(normalize_base_path("/../x").is_err());
        assert!(normalize_base_path("/:id").is_err());
    }

    #[test]
    fn allowlist_parsing() {
        assert_eq!(parse_allowlist(None), None);
        assert_eq!(parse_allowlist(Some(" , ")), None);
        assert_eq!(
            parse_allowlist(Some("http://localhost:5100, 127.0.0.1:*,")),
            Some(vec![
                "http://localhost:5100".to_owned(),
                "127.0.0.1:*".to_owned()
            ])
        );
    }

    #[test]
    fn ai_studio_key_resolution() {
        let s = |v: &str| Some(v.to_owned());
        // JS `||`: only an empty (or unset) AI_STUDIO_API_KEY falls back; values stay raw.
        assert_eq!(resolve_ai_studio_key(s(" k1 "), s("k2")), s(" k1 "));
        assert_eq!(resolve_ai_studio_key(s(""), s("k2")), s("k2"));
        assert_eq!(resolve_ai_studio_key(None, s("k2")), s("k2"));
        assert_eq!(
            resolve_ai_studio_key(s("undefined"), s("k2")),
            s("undefined")
        );
        assert_eq!(resolve_ai_studio_key(s("   "), s("k2")), s("   "));
        assert_eq!(resolve_ai_studio_key(s(""), s("")), None);
        assert_eq!(resolve_ai_studio_key(None, None), None);
        // getAiStudioApiKey(): "undefined" and blank keys are not used, " undefined " is.
        assert!(ai_studio_key_usable(" k1 "));
        assert!(ai_studio_key_usable(" undefined "));
        assert!(!ai_studio_key_usable("undefined"));
        assert!(!ai_studio_key_usable(" \u{3000} "));
    }

    #[test]
    fn cli_defaults_and_env_names() {
        // Inspect clap's definitions instead of parsing, so ambient AMADEUS_* variables (CI,
        // developer shells) cannot change the outcome.
        use clap::CommandFactory;
        let command = Cli::command();
        let arg = |id: &str| {
            command
                .get_arguments()
                .find(|a| a.get_id() == id)
                .unwrap_or_else(|| panic!("missing --{id}"))
        };
        for (id, env, default) in [
            ("bind", "AMADEUS_BIND", Some("127.0.0.1:9502")),
            ("base_path", "AMADEUS_BASE_PATH", Some("/amadeus")),
            ("dist_dir", "AMADEUS_DIST_DIR", Some("./dist")),
            ("data_dir", "AMADEUS_DATA_DIR", Some("./data")),
            ("env_file", "AMADEUS_ENV_FILE", Some(".env")),
            ("vertex_credentials", "AMADEUS_VERTEX_CREDENTIALS", None),
            ("proxy_allowlist", "AMADEUS_PROXY_ALLOWLIST", None),
            ("immutable_assets", "AMADEUS_IMMUTABLE_ASSETS", None),
            ("free_api_max", "AMADEUS_FREEAPI_MAX", Some("65000")),
        ] {
            let a = arg(id);
            assert_eq!(a.get_env().and_then(|e| e.to_str()), Some(env), "{id}");
            let defaults: Vec<String> = a
                .get_default_values()
                .iter()
                .map(|v| v.to_string_lossy().into_owned())
                .collect();
            assert_eq!(defaults.first().map(String::as_str), default, "{id}");
        }
    }

    fn raw_cli() -> Cli {
        Cli {
            bind: DEFAULT_BIND.to_owned(),
            base_path: DEFAULT_BASE_PATH.to_owned(),
            dist_dir: OsString::from(DEFAULT_DIST_DIR),
            data_dir: OsString::from(DEFAULT_DATA_DIR),
            env_file: OsString::from(DEFAULT_ENV_FILE),
            vertex_credentials: None,
            proxy_allowlist: None,
            immutable_assets: false,
            free_api_max: DEFAULT_FREE_API_MAX.to_owned(),
        }
    }

    #[test]
    fn empty_values_select_defaults() {
        let cli = Cli {
            bind: " ".to_owned(),
            base_path: String::new(),
            dist_dir: OsString::new(),
            data_dir: OsString::new(),
            env_file: OsString::new(),
            vertex_credentials: Some(OsString::new()),
            proxy_allowlist: Some(String::new()),
            immutable_assets: false,
            free_api_max: " ".to_owned(),
        };
        let cfg = ServerConfig::from_cli(cli, None).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(cfg.free_api_max, 65_000);
        assert_eq!(cfg.backend_config().free_api_max, 65_000);
        assert_eq!(cfg.bind, DEFAULT_BIND);
        assert_eq!(cfg.base_path, "/amadeus");
        assert_eq!(cfg.dist_dir, PathBuf::from(DEFAULT_DIST_DIR));
        assert_eq!(cfg.data_dir, PathBuf::from(DEFAULT_DATA_DIR));
        assert_eq!(cfg.env_file, PathBuf::from(DEFAULT_ENV_FILE));
        assert_eq!((cfg.vertex_credentials, cfg.proxy_allowlist), (None, None));
        // "/" is the explicit way to serve at the origin root.
        let root = Cli {
            base_path: "/".to_owned(),
            ..raw_cli()
        };
        let cfg = ServerConfig::from_cli(root, None).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(cfg.base_path, "");
        let bad = Cli {
            base_path: "/a b".to_owned(),
            ..raw_cli()
        };
        assert!(ServerConfig::from_cli(bad, None).is_err());
        assert_eq!(ServerConfig::new("d", "e").free_api_max, 65_000);
    }

    #[test]
    fn free_api_max_parsing() {
        assert_eq!(
            parse_free_api_max(DEFAULT_FREE_API_MAX),
            Ok(MAX_GLOBAL_REQUESTS)
        );
        assert_eq!(parse_free_api_max(""), Ok(65_000));
        assert_eq!(parse_free_api_max(" \t"), Ok(65_000));
        assert_eq!(parse_free_api_max("65000"), Ok(65_000));
        assert_eq!(parse_free_api_max(" 100000 "), Ok(100_000));
        assert_eq!(parse_free_api_max("1"), Ok(1));
        assert_eq!(parse_free_api_max("007"), Ok(7));
        assert_eq!(
            parse_free_api_max("9007199254740991"),
            Ok(FREE_API_MAX_LIMIT)
        );
        for bad in [
            "0",
            "-5",
            "+5",
            "1e5",
            "65,000",
            "65_000",
            "6.5",
            "abc",
            "12abc",
            "9007199254740992",
            "99999999999999999999",
        ] {
            let err = parse_free_api_max(bad).expect_err(bad);
            assert!(
                err.starts_with(&format!(
                    "invalid AMADEUS_FREEAPI_MAX / --free-api-max {bad:?}: "
                )),
                "{err}"
            );
            assert!(
                err.contains("from 1 to 9007199254740991 (default 65000)"),
                "{err}"
            );
        }
        let zero = Cli {
            free_api_max: "0".to_owned(),
            ..raw_cli()
        };
        assert!(ServerConfig::from_cli(zero, None)
            .err()
            .is_some_and(|e| e.contains("AMADEUS_FREEAPI_MAX")));
    }

    /// `--free-api-max -5` is the cap's value (rejected with the startup message), exactly like
    /// `--free-api-max=-5`; only numbers pass as values, so a flag after it is a missing value.
    #[test]
    fn negative_cap_gets_the_startup_message() {
        for (args, value) in [
            (&["--free-api-max", "-5"][..], "-5"),
            (&["--free-api-max=-5"][..], "-5"),
            (&["--free-api-max", "-1e5"][..], "-1e5"),
            (
                &["--free-api-max", "-0.5", "--bind", "0.0.0.0:1"][..],
                "-0.5",
            ),
        ] {
            let cli = Cli::try_parse_from(std::iter::once("amadeus-server").chain(args.to_vec()))
                .unwrap_or_else(|e| panic!("{args:?}: {e}"));
            assert_eq!(cli.free_api_max, value, "{args:?}");
            let err = ServerConfig::from_cli(cli, None).expect_err(value);
            assert!(
                err.starts_with(&format!(
                    "invalid AMADEUS_FREEAPI_MAX / --free-api-max {value:?}: "
                )),
                "{err}"
            );
        }
        for args in [
            &["--free-api-max", "--bind", "0.0.0.0:1"][..],
            &["--free-api-max", "-x"][..],
            &["--free-api-max"][..],
        ] {
            assert!(
                Cli::try_parse_from(std::iter::once("amadeus-server").chain(args.to_vec()))
                    .is_err(),
                "{args:?}"
            );
        }
    }

    #[test]
    fn flags_parse_and_debug_redacts_the_key() {
        let cli = Cli::try_parse_from([
            "amadeus-server",
            "--bind",
            "0.0.0.0:1",
            "--base-path",
            "amadeus/",
            "--dist-dir",
            "/srv/dist",
            "--proxy-allowlist",
            "a",
            "--vertex-credentials",
            "/srv/sa.json",
            "--immutable-assets",
            "--free-api-max",
            "70000",
        ])
        .unwrap_or_else(|e| panic!("{e}"));
        let cfg = ServerConfig::from_cli(cli, Some("secret-key".to_owned()))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            (cfg.bind.as_str(), cfg.base_path.as_str()),
            ("0.0.0.0:1", "/amadeus")
        );
        assert_eq!(cfg.dist_dir, PathBuf::from("/srv/dist"));
        assert_eq!(cfg.vertex_credentials, Some(PathBuf::from("/srv/sa.json")));
        assert_eq!(cfg.proxy_allowlist, Some(vec!["a".to_owned()]));
        assert!(cfg.immutable_assets);
        let debug = format!("{cfg:?}");
        assert!(!debug.contains("secret-key"));
        assert!(debug.contains("<set>"));
        let backend = cfg.backend_config();
        assert!(backend.is_server());
        assert_eq!(backend.proxy_allowlist, Some(vec!["a".to_owned()]));
        assert_eq!(backend.ai_studio_api_key.as_deref(), Some("secret-key"));
        assert_eq!((cfg.free_api_max, backend.free_api_max), (70_000, 70_000));
        assert!(debug.contains("free_api_max: 70000"));
        assert_eq!(
            backend.vertex_credentials_path,
            Some(PathBuf::from("/srv/sa.json"))
        );
    }
}
