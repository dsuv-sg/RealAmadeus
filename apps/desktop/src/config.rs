//! Desktop runtime configuration: hosted server base URL and the backend data directory.

use std::path::PathBuf;

use tauri::{AppHandle, Manager, Url};

/// Default base URL of the hosted RealAmadeus web server. On desktop the FreeAPI provider (and
/// its quota) is forwarded there because it needs server-side credentials. This constant is the
/// single place the URL is defined for the desktop app.
pub const DEFAULT_REMOTE_API_BASE: &str = "https://dsuv.elvelt.com/amadeus";

/// Environment variable that overrides [`DEFAULT_REMOTE_API_BASE`] (e.g. a staging server).
pub const REMOTE_API_BASE_ENV: &str = "AMADEUS_REMOTE_API_BASE";

/// Name of the backend data directory inside the app data directory.
const DATA_DIR_NAME: &str = "data";

/// Resolves the remote API base from [`REMOTE_API_BASE_ENV`], falling back to the default.
pub fn remote_api_base() -> String {
    let raw = std::env::var(REMOTE_API_BASE_ENV).ok();
    match normalize_remote_api_base(raw.as_deref()) {
        Ok(Some(base)) => {
            log::info!("{REMOTE_API_BASE_ENV} is set; remote API base: {base}");
            base
        }
        Ok(None) => DEFAULT_REMOTE_API_BASE.to_owned(),
        Err(reason) => {
            log::warn!("ignoring {REMOTE_API_BASE_ENV}: {reason}; using {DEFAULT_REMOTE_API_BASE}");
            DEFAULT_REMOTE_API_BASE.to_owned()
        }
    }
}

/// `Ok(None)` when unset or blank, `Ok(Some(base))` (no trailing slash) for a valid http(s) URL.
fn normalize_remote_api_base(raw: Option<&str>) -> Result<Option<String>, String> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let base = raw.trim_end_matches('/');
    let url = Url::parse(base).map_err(|e| format!("`{raw}` is not a valid URL ({e})"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(format!("`{raw}` must be an http(s) URL with a host"));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(format!("`{raw}` must not contain a query or fragment"));
    }
    Ok(Some(base.to_owned()))
}

/// `{app_data_dir}/data`, created if missing. Falls back to the temp directory when the
/// platform data directory cannot be resolved (the desktop backend keeps no essential state).
pub fn data_dir(app: &AppHandle) -> PathBuf {
    let dir = match app.path().app_data_dir() {
        Ok(base) => base.join(DATA_DIR_NAME),
        Err(err) => {
            let fallback = std::env::temp_dir()
                .join(&app.config().identifier)
                .join(DATA_DIR_NAME);
            log::error!(
                "cannot resolve the app data directory ({err}); using {}",
                fallback.display()
            );
            fallback
        }
    };
    if let Err(err) = std::fs::create_dir_all(&dir) {
        log::error!("cannot create the data directory {}: {err}", dir.display());
    }
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_api_base_normalization() {
        assert_eq!(normalize_remote_api_base(None), Ok(None));
        assert_eq!(normalize_remote_api_base(Some("  ")), Ok(None));
        assert_eq!(
            normalize_remote_api_base(Some(" https://example.com/amadeus/ ")),
            Ok(Some("https://example.com/amadeus".to_owned()))
        );
        assert_eq!(
            normalize_remote_api_base(Some("http://127.0.0.1:9501/amadeus")),
            Ok(Some("http://127.0.0.1:9501/amadeus".to_owned()))
        );
        assert!(normalize_remote_api_base(Some("ftp://example.com")).is_err());
        assert!(normalize_remote_api_base(Some("example.com/amadeus")).is_err());
        assert!(normalize_remote_api_base(Some("https://example.com/?x=1")).is_err());
    }

    #[test]
    fn default_remote_api_base_is_valid() {
        assert_eq!(
            normalize_remote_api_base(Some(DEFAULT_REMOTE_API_BASE)),
            Ok(Some(DEFAULT_REMOTE_API_BASE.to_owned()))
        );
    }
}
