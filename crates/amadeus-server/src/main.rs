//! RealAmadeus V2 web server binary (`amadeus-server`).
//!
//! Configuration: see [`amadeus_server::config`]. Serves the Trunk dist and the V1 HTTP API
//! under `AMADEUS_BASE_PATH`, shuts down gracefully on SIGINT / SIGTERM. Refuses to start when
//! the dist's `index.html` was built for another base path (its `<base href>` differs).

use std::future::IntoFuture;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use amadeus_backend::aux::allowlist;
use amadeus_backend::Backend;
use amadeus_server::config::{ai_studio_key_usable, EnvFileStatus};
use amadeus_server::{build_router, shutdown, AppState, ServerConfig};
use tracing_subscriber::EnvFilter;

/// How long in-flight requests (e.g. SSE streams) may run after a shutdown signal.
const DRAIN_TIMEOUT: Duration = Duration::from_secs(10);

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

/// Logs the effective configuration (never secrets).
fn log_startup(config: &ServerConfig, env_status: &EnvFileStatus) {
    tracing::info!(
        version = amadeus_core::version::VERSION,
        bind = %config.bind,
        base_path = if config.base_path.is_empty() { "/" } else { config.base_path.as_str() },
        dist_dir = %config.dist_dir.display(),
        data_dir = %config.data_dir.display(),
        immutable_assets = config.immutable_assets,
        free_api_max = config.free_api_max,
        "starting amadeus-server"
    );
    match env_status {
        EnvFileStatus::Loaded(path) => {
            tracing::info!(env_file = %path.display(), "loaded environment file")
        }
        EnvFileStatus::Missing(path) => {
            tracing::debug!(env_file = %path.display(), "no environment file")
        }
        EnvFileStatus::Error(path, err) => {
            tracing::warn!(env_file = %path.display(), %err, "cannot load environment file")
        }
    }
    match &config.vertex_credentials {
        Some(path) if path.is_file() => {
            tracing::info!(path = %path.display(), "Vertex service-account credentials configured")
        }
        Some(path) => {
            tracing::warn!(path = %path.display(), "AMADEUS_VERTEX_CREDENTIALS does not point to a file")
        }
        None => tracing::info!(
            "no Vertex service-account credentials (AMADEUS_VERTEX_CREDENTIALS unset)"
        ),
    }
    tracing::info!(
        configured = config
            .ai_studio_api_key
            .as_deref()
            .is_some_and(ai_studio_key_usable),
        "AI Studio API key"
    );
    match &config.proxy_allowlist {
        Some(entries) => {
            for entry in entries.iter().filter(|e| allowlist::parse_entry(e).is_none()) {
                tracing::warn!(%entry, "ignoring unusable AMADEUS_PROXY_ALLOWLIST entry");
            }
            tracing::info!(?entries, "TTS/STT/RAG proxy allowlist enabled");
        }
        None => tracing::info!("TTS/STT/RAG proxies accept any serverUrl (V1 behavior; set AMADEUS_PROXY_ALLOWLIST to restrict)"),
    }
}

/// How the dist's `index.html` relates to the base path the server was started with.
#[derive(Debug, PartialEq, Eq)]
enum IndexBase<'a> {
    /// `<base href>` is `{base}/`: the frontend was built for this base path.
    Matches,
    /// No `<base href>` (a hand-made index.html): served, with a warning.
    Missing,
    /// `<base href>` names another path: every page would load its wasm, scripts and styles from
    /// there and break, so the server refuses to start.
    Mismatch(&'a str),
}

/// Compares the `<base href>` of `html` (the dist's index.html) with `{base_path}/`, where
/// `base_path` is the normalized `AMADEUS_BASE_PATH` (`""` for the origin root).
fn index_base<'a>(html: &'a str, base_path: &str) -> IndexBase<'a> {
    match base_href(html) {
        None => IndexBase::Missing,
        Some(found) if found.strip_suffix('/') == Some(base_path) => IndexBase::Matches,
        Some(found) => IndexBase::Mismatch(found),
    }
}

/// Checks the dist's index.html against the base path; `false` = refuse to start (logged as an
/// error). A missing or unreadable index.html is only a warning (the frontend may be built
/// later; the API works without it).
fn check_dist_index(config: &ServerConfig) -> bool {
    let index = config.dist_dir.join("index.html");
    let expected = format!("{}/", config.base_path);
    match std::fs::read_to_string(&index) {
        Ok(html) => match index_base(&html, &config.base_path) {
            IndexBase::Matches => true,
            IndexBase::Missing => {
                tracing::warn!(
                    index = %index.display(),
                    %expected,
                    "index.html has no <base href>; pages below the base path may load their assets from the wrong URL"
                );
                true
            }
            IndexBase::Mismatch(found) => {
                tracing::error!(
                    index = %index.display(),
                    %expected,
                    %found,
                    "index.html <base href> does not match the base path; AMADEUS_BASE_PATH must equal Trunk's public_url (refusing to start: every page would load its assets from {found})"
                );
                false
            }
        },
        Err(err) => {
            tracing::warn!(index = %index.display(), %err, "dist index.html not readable; pages will fail until the frontend is built");
            true
        }
    }
}

/// Value of the `href` attribute of the first `<base>` tag in `html`. Trunk writes
/// `<base href="/amadeus/">`, and its release minifier emits the unquoted form
/// `<base href=/amadeus/>` (an unquoted value runs until whitespace or `>`).
fn base_href(html: &str) -> Option<&str> {
    // ASCII lowercasing keeps byte offsets, so indices into `lower` are valid for `html`.
    let lower = html.to_ascii_lowercase();
    // Skip `<base` text inside HTML comments (the dev build keeps index.html's comments).
    let mut from = 0;
    let start = loop {
        let tag = from + lower[from..].find("<base")?;
        match lower[from..tag].rfind("<!--") {
            Some(open) if !lower[from + open..tag].contains("-->") => {
                from = tag + lower[tag..].find("-->")? + "-->".len();
            }
            _ => break tag,
        }
    };
    let end = start + lower[start..].find('>')?;
    let value_start = start + lower[start..end].find("href=")? + "href=".len();
    let rest = &html[value_start..end];
    match rest.chars().next()? {
        quote @ ('"' | '\'') => {
            let inner = &rest[1..];
            inner.find(quote).map(|close| &inner[..close])
        }
        _ => Some(
            rest.split(|c: char| c.is_ascii_whitespace())
                .next()
                .unwrap_or(rest),
        ),
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let (config, env_status) = match ServerConfig::load() {
        Ok(loaded) => loaded,
        Err(err) => {
            eprintln!("amadeus-server: {err}");
            return ExitCode::FAILURE;
        }
    };
    init_tracing();
    log_startup(&config, &env_status);
    if !check_dist_index(&config) {
        return ExitCode::FAILURE;
    }

    if let Err(err) = std::fs::create_dir_all(&config.data_dir) {
        tracing::warn!(data_dir = %config.data_dir.display(), %err, "cannot create the data directory");
    }
    let backend = match Backend::new(config.backend_config()) {
        Ok(backend) => backend,
        Err(err) => {
            tracing::error!(%err, "cannot initialize the backend");
            return ExitCode::FAILURE;
        }
    };
    let app = build_router(AppState::new(backend, &config));
    let listener = match tokio::net::TcpListener::bind(config.bind.as_str()).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!(bind = %config.bind, %err, "cannot listen");
            return ExitCode::FAILURE;
        }
    };
    match listener.local_addr() {
        Ok(addr) => tracing::info!(%addr, "listening on http://{addr}{}/", config.base_path),
        Err(_) => tracing::info!(bind = %config.bind, "listening"),
    }

    let stop = Arc::new(tokio::sync::Notify::new());
    let server = axum::serve(listener, app).with_graceful_shutdown({
        let stop = Arc::clone(&stop);
        async move { stop.notified().await }
    });
    let mut server = std::pin::pin!(server.into_future());
    let result = tokio::select! {
        result = &mut server => result,
        () = shutdown::signal() => {
            tracing::info!("shutdown signal received, draining connections");
            stop.notify_one();
            match tokio::time::timeout(DRAIN_TIMEOUT, &mut server).await {
                Ok(result) => result,
                Err(_) => {
                    tracing::warn!("drain timeout elapsed, closing remaining connections");
                    Ok(())
                }
            }
        }
    };
    match result {
        Ok(()) => {
            tracing::info!("stopped");
            ExitCode::SUCCESS
        }
        Err(err) => {
            tracing::error!(%err, "server error");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{base_href, index_base, IndexBase};

    #[test]
    fn index_base_decides_whether_to_serve() {
        let release = "<!doctype html><html><head><meta charset=utf-8><base href=/amadeus/>";
        let debug = r#"<!-- becomes <base href="{public_url}"> -->
            <base href="/amadeus/" />"#;
        assert_eq!(index_base(release, "/amadeus"), IndexBase::Matches);
        assert_eq!(index_base(debug, "/amadeus"), IndexBase::Matches);
        // `--base-path /zzz` with the /amadeus/ dist (API-037): refuse.
        assert_eq!(
            index_base(release, "/zzz"),
            IndexBase::Mismatch("/amadeus/")
        );
        assert_eq!(index_base(release, ""), IndexBase::Mismatch("/amadeus/"));
        assert_eq!(
            index_base(release, "/amadeus/v2"),
            IndexBase::Mismatch("/amadeus/")
        );
        // The desktop dist (public_url /) at the origin root, and at a prefix.
        assert_eq!(index_base("<base href=\"/\">", ""), IndexBase::Matches);
        assert_eq!(
            index_base("<base href=\"/\">", "/amadeus"),
            IndexBase::Mismatch("/")
        );
        // Exact path only: no missing slash, case change or absolute URL.
        assert_eq!(
            index_base("<base href=/amadeus>", "/amadeus"),
            IndexBase::Mismatch("/amadeus")
        );
        assert_eq!(
            index_base("<base href=/Amadeus/>", "/amadeus"),
            IndexBase::Mismatch("/Amadeus/")
        );
        assert_eq!(
            index_base("<base href=//amadeus/>", "/amadeus"),
            IndexBase::Mismatch("//amadeus/")
        );
        assert_eq!(
            index_base(r#"<base href="https://cdn.example/amadeus/">"#, "/amadeus"),
            IndexBase::Mismatch("https://cdn.example/amadeus/")
        );
        // No <base>: keep serving (with a warning).
        assert_eq!(
            index_base("<html><head></head></html>", "/amadeus"),
            IndexBase::Missing
        );
        assert_eq!(
            index_base("<base data-trunk-public-url />", "/amadeus"),
            IndexBase::Missing
        );
    }

    #[test]
    fn base_href_quoted_unquoted_and_missing() {
        assert_eq!(
            base_href(r#"<head><base href="/amadeus/"></head>"#),
            Some("/amadeus/")
        );
        assert_eq!(
            base_href("<head><base href='/amadeus/'></head>"),
            Some("/amadeus/")
        );
        assert_eq!(
            base_href("<head><base href=/amadeus/><meta charset=utf-8>"),
            Some("/amadeus/")
        );
        assert_eq!(base_href("<BASE HREF=/ >"), Some("/"));
        assert_eq!(
            base_href(r#"<link href="/x.css"><base data-trunk-public-url>"#),
            None
        );
        assert_eq!(base_href("<html></html>"), None);
        assert_eq!(
            base_href(r#"<!-- becomes <base href="{public_url}"> --><base href="/amadeus/">"#),
            Some("/amadeus/")
        );
        assert_eq!(
            base_href(r#"<!-- <base href="/x/"> --><!-- --><base href=/amadeus/>"#),
            Some("/amadeus/")
        );
        assert_eq!(base_href(r#"<!-- unterminated <base href="/x/">"#), None);
    }
}
