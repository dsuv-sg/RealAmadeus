//! Optional SSRF guard for the TTS/STT/RAG proxies (`AMADEUS_PROXY_ALLOWLIST`, new in V2).
//!
//! V1 let any visitor make the server request `<anything>/health`, `/synthesize`,
//! `/transcribe` and `/embed`. With an allowlist configured, the upstream URL (and every
//! redirect hop) must match one entry; otherwise the route answers 403. Without one (`None`),
//! behavior is exactly V1's.
//!
//! Entry forms (comma separated in the env var, whitespace trimmed):
//! * **URL prefix** `scheme://host[:port][/path]`: same scheme, host and effective port; the
//!   target path must equal the prefix path or continue it at a `/` boundary.
//! * **Host pattern** `host`, `host:port` or `host:*` (no scheme): `http`/`https` URLs to that
//!   host; a bare host or `*` accepts any port.
//!
//! Matching is done on parsed URLs (so `http://localhost:5100.evil.com` or
//! `http://localhost:5100@evil.com` never match `http://localhost:5100`).

use url::Url;

/// One parsed allowlist entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowEntry {
    /// URL prefix entry.
    Prefix {
        /// Lower-case scheme.
        scheme: String,
        /// Serialized host (`url::Url::host_str`).
        host: String,
        /// Effective port (explicit or the scheme default).
        port: Option<u16>,
        /// Path prefix (always starts with `/`).
        path: String,
    },
    /// Host pattern entry (`http`/`https`); `port: None` = any port.
    Host {
        /// Serialized host.
        host: String,
        /// Required effective port, if any.
        port: Option<u16>,
    },
}

/// Parses one allowlist entry; `None` for an unusable entry (callers should warn).
pub fn parse_entry(raw: &str) -> Option<AllowEntry> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.contains("://") {
        let url = Url::parse(raw).ok()?;
        let host = url.host_str()?.to_owned();
        return Some(AllowEntry::Prefix {
            scheme: url.scheme().to_owned(),
            host,
            port: url.port_or_known_default(),
            path: url.path().to_owned(),
        });
    }
    let (host_part, any_port) = match raw.strip_suffix(":*") {
        Some(h) => (h, true),
        None => (raw, false),
    };
    let url = Url::parse(&format!("http://{host_part}/")).ok()?;
    if url.path() != "/" || url.query().is_some() || !url.username().is_empty() {
        return None;
    }
    let host = url.host_str()?.to_owned();
    // `Url::port()` hides default ports (`localhost:80` → None), so detect an explicit port
    // on the raw text instead.
    let has_port = host_part
        .rsplit_once(':')
        .is_some_and(|(_, p)| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    if any_port && has_port {
        return None;
    }
    let port = if has_port {
        url.port_or_known_default()
    } else {
        None
    };
    Some(AllowEntry::Host { host, port })
}

impl AllowEntry {
    /// True when `url` is covered by this entry.
    pub fn matches(&self, url: &Url) -> bool {
        let Some(host) = url.host_str() else {
            return false;
        };
        match self {
            AllowEntry::Prefix {
                scheme,
                host: h,
                port,
                path,
            } => {
                if url.scheme() != scheme || host != h || url.port_or_known_default() != *port {
                    return false;
                }
                let target = url.path();
                if path.ends_with('/') {
                    target.starts_with(path.as_str())
                } else {
                    target == path
                        || target
                            .strip_prefix(path.as_str())
                            .is_some_and(|rest| rest.starts_with('/'))
                }
            }
            AllowEntry::Host { host: h, port } => {
                matches!(url.scheme(), "http" | "https")
                    && host == h
                    && port.is_none_or(|p| url.port_or_known_default() == Some(p))
            }
        }
    }
}

/// True when `url` matches at least one (parsable) entry of `allowlist`.
pub fn is_allowed(url: &Url, allowlist: &[String]) -> bool {
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    allowlist
        .iter()
        .filter_map(|raw| parse_entry(raw))
        .any(|entry| entry.matches(url))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(url: &str, list: &[&str]) -> bool {
        let list: Vec<String> = list.iter().map(|s| (*s).to_owned()).collect();
        Url::parse(url)
            .map(|u| is_allowed(&u, &list))
            .unwrap_or(false)
    }

    #[test]
    fn url_prefix_entries() {
        let list = ["http://localhost:5100", "https://tts.example.com/api/"];
        assert!(allowed("http://localhost:5100/health", &list));
        assert!(allowed("http://LOCALHOST:5100//synthesize", &list));
        assert!(!allowed("http://localhost:5101/health", &list));
        assert!(!allowed("https://localhost:5100/health", &list));
        assert!(!allowed("http://localhost:5100.evil.com/health", &list));
        assert!(!allowed("http://localhost:5100@evil.com/health", &list));
        assert!(!allowed("http://user:pw@localhost:5100/health", &list));
        assert!(allowed("https://tts.example.com/api/health", &list));
        assert!(allowed("https://tts.example.com:443/api/x", &list));
        assert!(!allowed("https://tts.example.com/apix/health", &list));
        assert!(!allowed("https://tts.example.com/health", &list));
        assert!(allowed("http://h/tts/health", &["http://h/tts"]));
        assert!(allowed("http://h/tts", &["http://h/tts"]));
        assert!(!allowed("http://h/ttsx", &["http://h/tts"]));
    }

    #[test]
    fn host_pattern_entries() {
        assert!(allowed("http://127.0.0.1:5102/embed", &["127.0.0.1:*"]));
        assert!(allowed("https://127.0.0.1/embed", &["127.0.0.1"]));
        assert!(allowed("http://localhost:5100/health", &["localhost:5100"]));
        assert!(!allowed(
            "http://localhost:5101/health",
            &["localhost:5100"]
        ));
        assert!(allowed("http://localhost/health", &["localhost:80"]));
        assert!(!allowed("http://localhost:8080/health", &["localhost:80"]));
        assert!(!allowed("https://localhost/health", &["localhost:80"]));
        assert!(allowed("http://localhost:8080/health", &["localhost"]));
        assert!(!allowed("ftp://localhost/health", &["localhost"]));
        assert!(allowed("http://[::1]:5100/health", &["[::1]:5100"]));
        assert!(!allowed("http://localhost:5100/health", &[]));
        assert!(!allowed(
            "http://localhost:5100/health",
            &["", "not a url://", "a/b"]
        ));
    }

    #[test]
    fn entry_parsing() {
        assert_eq!(
            parse_entry("  localhost:5100 "),
            Some(AllowEntry::Host {
                host: "localhost".into(),
                port: Some(5100)
            })
        );
        assert_eq!(
            parse_entry("example.com:*"),
            Some(AllowEntry::Host {
                host: "example.com".into(),
                port: None
            })
        );
        assert_eq!(parse_entry("example.com:80:*"), None);
        assert_eq!(parse_entry("a/b"), None);
        assert_eq!(
            parse_entry("http://localhost:5100"),
            Some(AllowEntry::Prefix {
                scheme: "http".into(),
                host: "localhost".into(),
                port: Some(5100),
                path: "/".into()
            })
        );
    }
}
