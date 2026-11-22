//! `amadeus-local-proxy`: Rust port of V1 `scripts/amadeus-local-proxy/server.mjs`
//! (`npm run start:amadeus-proxy`).
//!
//! Environment: `LOCAL_PROXY_HOST` (default `0.0.0.0`), `LOCAL_PROXY_PORT` (default `5103`),
//! `VERTEX_PROJECT`, `VERTEX_LOCATION` (default `us-central1`). Endpoints: `GET /health`,
//! `POST /chat`, `OPTIONS *` (CORS). Authenticates with `gcloud auth print-access-token`.
//!
//! Command line: V1's `node server.mjs` ignored its arguments and V2 keeps that, except that
//! `-h` / `--help` print the usage and `-V` / `--version` the version and exit without listening
//! (so asking for help never exposes the proxy on every interface).

use std::ffi::OsString;
use std::io::Write as _;
use std::process::ExitCode;
use std::sync::Arc;

use amadeus_backend::local_proxy::{LocalProxy, LocalProxyConfig};
use amadeus_server::{local_proxy_http, shutdown};
use tracing_subscriber::EnvFilter;

/// `amadeus-local-proxy --help`.
const USAGE: &str = "\
Rust port of RealAmadeus V1's scripts/amadeus-local-proxy (provider 7 \"local proxy\"):
relays chat requests to Vertex AI with the access token of `gcloud auth print-access-token`,
trying further regions when one fails. Endpoints: GET /health, POST /chat, OPTIONS * (CORS).

Usage: amadeus-local-proxy

Configured only through environment variables:
  LOCAL_PROXY_HOST  listen address (default 0.0.0.0 = every network interface;
                    set 127.0.0.1 to accept connections from this machine only)
  LOCAL_PROXY_PORT  listen port (default 5103)
  VERTEX_PROJECT    Google Cloud project for requests that name none
  VERTEX_LOCATION   preferred Vertex AI region (default us-central1)
  RUST_LOG          log filter (default warn)

Options:
  -h, --help        print this help and exit
  -V, --version     print the version and exit

Any other argument is ignored, like V1's `node server.mjs`.
";

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
enum CliAction {
    /// Start the proxy (no help / version flag; other arguments are ignored like in V1).
    Serve,
    /// `-h` / `--help`.
    Help,
    /// `-V` / `--version`.
    Version,
}

/// The first help or version flag decides; everything else is ignored.
fn cli_action(args: impl IntoIterator<Item = OsString>) -> CliAction {
    for arg in args {
        match arg.to_str() {
            Some("-h" | "--help") => return CliAction::Help,
            Some("-V" | "--version") => return CliAction::Version,
            _ => {}
        }
    }
    CliAction::Serve
}

/// The `--version` line (`amadeus-local-proxy 2.0.0`, like clap prints it for amadeus-server).
fn version_line() -> String {
    format!("amadeus-local-proxy {}", env!("CARGO_PKG_VERSION"))
}

/// Writes to stdout, ignoring a closed pipe (`amadeus-local-proxy --help | head -1`).
fn print_stdout(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(text.as_bytes()).and_then(|()| out.flush());
}

#[tokio::main]
async fn main() -> ExitCode {
    match cli_action(std::env::args_os().skip(1)) {
        CliAction::Help => {
            print_stdout(&format!("{}\n\n{USAGE}", version_line()));
            return ExitCode::SUCCESS;
        }
        CliAction::Version => {
            print_stdout(&format!("{}\n", version_line()));
            return ExitCode::SUCCESS;
        }
        CliAction::Serve => {}
    }

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();

    let config = match LocalProxyConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("[local-proxy] {err}");
            return ExitCode::FAILURE;
        }
    };
    let http = match amadeus_backend::http::build_client() {
        Ok(http) => http,
        Err(err) => {
            eprintln!("[local-proxy] cannot create the HTTP client: {err}");
            return ExitCode::FAILURE;
        }
    };
    let listener = match tokio::net::TcpListener::bind((config.host.as_str(), config.port)).await {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!(
                "[local-proxy] cannot listen on {}:{}: {err}",
                config.host, config.port
            );
            return ExitCode::FAILURE;
        }
    };
    println!("{}", config.listen_message());
    let proxy = Arc::new(LocalProxy::new(config, http));
    match axum::serve(listener, local_proxy_http::router(proxy))
        .with_graceful_shutdown(shutdown::signal())
        .await
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("[local-proxy] server error: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(args: &[&str]) -> CliAction {
        cli_action(args.iter().map(OsString::from))
    }

    #[test]
    fn help_and_version_flags_win_other_arguments_are_ignored() {
        assert_eq!(action(&[]), CliAction::Serve);
        assert_eq!(action(&["--port", "9000", "serve"]), CliAction::Serve);
        assert_eq!(action(&["-help", "help", "--HELP", "-v"]), CliAction::Serve);
        assert_eq!(action(&["--help"]), CliAction::Help);
        assert_eq!(action(&["-h"]), CliAction::Help);
        assert_eq!(action(&["whatever", "-h"]), CliAction::Help);
        assert_eq!(action(&["--version"]), CliAction::Version);
        assert_eq!(action(&["-V"]), CliAction::Version);
        assert_eq!(action(&["-V", "--help"]), CliAction::Version);
        assert_eq!(action(&["--help", "-V"]), CliAction::Help);
    }

    #[test]
    fn usage_names_every_setting() {
        use amadeus_backend::local_proxy::{DEFAULT_HOST, DEFAULT_LOCATION, DEFAULT_PORT};
        for needle in [
            "LOCAL_PROXY_HOST".to_owned(),
            format!("default {DEFAULT_HOST} "),
            "127.0.0.1".to_owned(),
            "LOCAL_PROXY_PORT".to_owned(),
            format!("default {DEFAULT_PORT})"),
            "VERTEX_PROJECT".to_owned(),
            "VERTEX_LOCATION".to_owned(),
            format!("default {DEFAULT_LOCATION})"),
            "RUST_LOG".to_owned(),
            "-h, --help".to_owned(),
            "-V, --version".to_owned(),
            "Any other argument is ignored".to_owned(),
        ] {
            assert!(USAGE.contains(&needle), "usage lacks {needle:?}");
        }
        assert_eq!(
            version_line(),
            format!("amadeus-local-proxy {}", env!("CARGO_PKG_VERSION"))
        );
    }
}
