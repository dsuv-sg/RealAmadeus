//! Google credentials for Vertex AI (`spec-server-chat.md` §8.2-§8.6, §9): service-account
//! parsing (the strict `route.ts` and the loose `vertex-helpers.ts` variants), RS256 JWT
//! assertions, OAuth token exchange (cached, V2 improvement Q14) and the gcloud CLI token
//! (one shared cache, Q15).
//!
//! V1 exchanged a new service-account token for every request. The V2 cache must stay
//! invisible: a token is reused only within the lifetime announced by `expires_in` (minus a
//! margin), and callers redo a request with a freshly exchanged token when Vertex rejects a
//! cached one ([`SaToken::cached`]).

use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use amadeus_core::jscompat::{js_trim, js_truthy};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use jsonwebtoken::{Algorithm, EncodingKey};
use serde::Serialize;
use serde_json::Value;

use super::net;
use super::payload::{non_nullish, prop, value_text, Thrown};
use super::texts::{ERR_KEY_EMPTY, ERR_KEY_UNSUPPORTED};

/// Default OAuth token endpoint.
pub(crate) const DEFAULT_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
/// OAuth scope requested for Vertex AI.
const CLOUD_PLATFORM_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";
/// gcloud tokens are cached for 50 minutes from fetch time.
const GCLOUD_TOKEN_TTL: Duration = Duration::from_secs(50 * 60);
/// Service-account access tokens are cached for at most 55 minutes (V1 did not cache).
const SA_TOKEN_TTL: Duration = Duration::from_secs(55 * 60);
/// A cached service-account token is dropped this long before its announced expiry.
const SA_TOKEN_EXPIRY_MARGIN: Duration = Duration::from_secs(60);
/// At most this many service-account tokens are cached: `token_uri` comes from the client's
/// credential JSON, so a client could otherwise grow the cache without bound.
const SA_TOKEN_CACHE_MAX_ENTRIES: usize = 64;
/// Longer access tokens are not cached (Google's are well below this size).
const SA_TOKEN_CACHE_MAX_LEN: usize = 8 * 1024;
/// `execFile(..., { timeout: 10000 })`.
const GCLOUD_TIMEOUT: Duration = Duration::from_secs(10);
/// `execFile`'s default `maxBuffer` (1 MiB).
const GCLOUD_MAX_STDOUT: usize = 1024 * 1024;

/// The fields of a Google service-account JSON key that V1 used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServiceAccount {
    pub(crate) client_email: String,
    pub(crate) private_key: String,
    pub(crate) token_uri: String,
    pub(crate) project_id: String,
}

/// `parseServiceAccountCredential` (route.ts, strict): trims the input and the fields.
pub(crate) fn parse_service_account_strict(raw: &str) -> Option<ServiceAccount> {
    let trimmed = js_trim(raw);
    if !trimmed.starts_with('{') {
        return None;
    }
    let v: Value = serde_json::from_str(trimmed).ok()?;
    let (Some(Value::String(email)), Some(Value::String(key))) = (
        prop(Some(&v), "client_email"),
        prop(Some(&v), "private_key"),
    ) else {
        return None;
    };
    let token_uri = match prop(Some(&v), "token_uri") {
        Some(Value::String(u)) if !js_trim(u).is_empty() => js_trim(u).to_owned(),
        _ => DEFAULT_TOKEN_URI.to_owned(),
    };
    let project_id = match prop(Some(&v), "project_id") {
        Some(Value::String(p)) => js_trim(p).to_owned(),
        _ => String::new(),
    };
    Some(ServiceAccount {
        client_email: js_trim(email).to_owned(),
        private_key: key.clone(),
        token_uri,
        project_id,
    })
}

/// `parseServiceAccount` (vertex-helpers, loose): parses the untrimmed input, keeps fields as is.
pub(crate) fn parse_service_account_loose(raw: &str) -> Option<ServiceAccount> {
    if !js_trim(raw).starts_with('{') {
        return None;
    }
    let v: Value = serde_json::from_str(raw).ok()?;
    let (Some(Value::String(email)), Some(Value::String(key))) = (
        prop(Some(&v), "client_email"),
        prop(Some(&v), "private_key"),
    ) else {
        return None;
    };
    let token_uri = match prop(Some(&v), "token_uri") {
        Some(u) if js_truthy(u) => value_text(u),
        _ => DEFAULT_TOKEN_URI.to_owned(),
    };
    let project_id = match prop(Some(&v), "project_id") {
        Some(Value::String(p)) => p.clone(),
        _ => String::new(),
    };
    Some(ServiceAccount {
        client_email: email.clone(),
        private_key: key.clone(),
        token_uri,
        project_id,
    })
}

#[derive(Serialize)]
struct JwtHeader {
    alg: &'static str,
    typ: &'static str,
}

#[derive(Serialize)]
struct JwtClaims<'a> {
    iss: &'a str,
    sub: &'a str,
    aud: &'a str,
    scope: &'static str,
    iat: i64,
    exp: i64,
}

/// `iat` is backdated like V1 (`now - 30`) to tolerate clock skew.
const JWT_IAT_BACKDATE_SECS: i64 = 30;
/// Assertion lifetime counted from `iat`. V1 used `exp = now + 3600`, i.e. 3630 s after `iat`,
/// which exceeds Google's documented maximum ("1 hour after the issued time", spec Q23); V2
/// keeps the backdating but caps the lifetime at exactly one hour.
const JWT_LIFETIME_SECS: i64 = 3600;

/// `buildServiceAccountAssertion` / `makeJwt`: V1's header and claims (same keys, order and
/// encoding; only `exp` is capped, see [`JWT_LIFETIME_SECS`]), signed with RSA-SHA256
/// (PKCS#1 v1.5). Errors carry the message Node's `crypto.sign` would throw.
pub(crate) fn build_assertion(sa: &ServiceAccount, now_secs: i64) -> Result<String, String> {
    let header = serde_json::to_string(&JwtHeader {
        alg: "RS256",
        typ: "JWT",
    })
    .map_err(|e| e.to_string())?;
    let iat = now_secs - JWT_IAT_BACKDATE_SECS;
    let claims = serde_json::to_string(&JwtClaims {
        iss: &sa.client_email,
        sub: &sa.client_email,
        aud: &sa.token_uri,
        scope: CLOUD_PLATFORM_SCOPE,
        iat,
        exp: iat + JWT_LIFETIME_SECS,
    })
    .map_err(|e| e.to_string())?;
    let unsigned = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(claims)
    );
    let pem = sa.private_key.replace("\\n", "\n");
    let pem = js_trim(&pem);
    if pem.is_empty() {
        return Err(ERR_KEY_EMPTY.to_owned());
    }
    let key =
        EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|_| ERR_KEY_UNSUPPORTED.to_owned())?;
    if jsonwebtoken::crypto::CryptoProvider::try_get_default().is_none() {
        return Err("jsonwebtoken crypto provider unavailable".to_owned());
    }
    let signature = jsonwebtoken::crypto::sign(unsigned.as_bytes(), &key, Algorithm::RS256)
        .map_err(|_| ERR_KEY_UNSUPPORTED.to_owned())?;
    Ok(format!("{unsigned}.{signature}"))
}

/// `application/x-www-form-urlencoded` serialization of one component (`URLSearchParams`).
fn form_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => {
                out.push(char::from(b))
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Body of the JWT-bearer token request.
pub(crate) fn token_request_body(assertion: &str) -> String {
    format!(
        "grant_type={}&assertion={}",
        form_component("urn:ietf:params:oauth:grant-type:jwt-bearer"),
        form_component(assertion)
    )
}

/// JS `encodeURIComponent`.
pub(crate) fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => {
                out.push(char::from(b));
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Which gcloud executable to run.
#[derive(Debug, Clone)]
pub(crate) enum GcloudProgram {
    /// V1 discovery (`findGcloudPath`, extended per OS).
    Discover,
    /// Explicit executable (tests).
    #[cfg(test)]
    Path(PathBuf),
    /// Fixed token without running a process (tests).
    #[cfg(test)]
    Fixed(String),
    /// Sequence of tokens returned on successive fresh fetches (tests).
    #[cfg(test)]
    Sequence(std::sync::Arc<Mutex<std::collections::VecDeque<String>>>),
}

/// A gcloud CLI access token and whether it came from the 50-minute cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GcloudToken {
    pub(crate) token: String,
    pub(crate) cached: bool,
}

#[derive(Default)]
struct CachedToken {
    token: String,
    expires: Option<Instant>,
}

impl CachedToken {
    fn valid(&self) -> Option<String> {
        match self.expires {
            Some(exp) if !self.token.is_empty() && Instant::now() < exp => Some(self.token.clone()),
            _ => None,
        }
    }
}

/// How long a freshly exchanged service-account token may be reused: `expires_in` minus
/// [`SA_TOKEN_EXPIRY_MARGIN`], at most [`SA_TOKEN_TTL`]. `None` (not cached, exactly like V1)
/// when `expires_in` is missing, not a JSON number, or leaves no usable lifetime.
pub(crate) fn sa_token_ttl(expires_in: Option<&Value>) -> Option<Duration> {
    let usable = expires_in.and_then(Value::as_f64)? - SA_TOKEN_EXPIRY_MARGIN.as_secs_f64();
    if usable > 0.0 {
        // Clamp before converting: `Duration::from_secs_f64` panics on huge values.
        Duration::try_from_secs_f64(usable.min(SA_TOKEN_TTL.as_secs_f64())).ok()
    } else {
        None
    }
}

/// Whether a cached service-account token may be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenReuse {
    /// Use a valid cached token if there is one.
    Allow,
    /// Always exchange a new token (V1 behavior; the retry after Vertex rejected a cached one).
    Fresh,
}

/// A service-account access token and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SaToken {
    /// The OAuth access token.
    pub(crate) token: String,
    /// Served from the V2 cache instead of a fresh exchange. When Vertex answers 401/403 to a
    /// cached token, callers forget it and redo the request with [`TokenReuse::Fresh`], so the
    /// cache never fails where V1's per-request exchange would have succeeded.
    pub(crate) cached: bool,
}

/// Process-wide Google credential state.
pub(crate) struct GoogleAuth {
    /// The server-side service-account file (V1 `realamadeusmain-*.json` in the CWD).
    embedded_path: Option<PathBuf>,
    /// Memoized file content (`None` = unreadable; cached forever like V1).
    embedded_raw: tokio::sync::OnceCell<Option<String>>,
    /// Service-account tokens keyed by a hash of (client_email, token_uri, private_key).
    sa_tokens: Mutex<HashMap<u64, CachedToken>>,
    /// Randomly keyed per process, so cache keys can be neither predicted nor forced to collide.
    sa_key_hasher: RandomState,
    gcloud: Mutex<CachedToken>,
    gcloud_fetch: tokio::sync::Mutex<()>,
    pub(crate) gcloud_program: GcloudProgram,
}

impl GoogleAuth {
    /// State for the given embedded credential file.
    pub(crate) fn new(embedded_path: Option<PathBuf>) -> Self {
        Self {
            embedded_path,
            embedded_raw: tokio::sync::OnceCell::new(),
            sa_tokens: Mutex::new(HashMap::new()),
            sa_key_hasher: RandomState::new(),
            gcloud: Mutex::new(CachedToken::default()),
            gcloud_fetch: tokio::sync::Mutex::new(()),
            gcloud_program: GcloudProgram::Discover,
        }
    }

    async fn embedded_raw(&self) -> Option<&str> {
        self.embedded_raw
            .get_or_init(|| async {
                // `readFile(path, "utf8")`: lossy UTF-8, and a leading BOM is KEPT (the strict
                // parser trims it away, the loose one hands it to `JSON.parse`, which rejects it).
                match &self.embedded_path {
                    Some(path) => tokio::fs::read(path)
                        .await
                        .ok()
                        .map(|b| String::from_utf8_lossy(&b).into_owned()),
                    None => None,
                }
            })
            .await
            .as_deref()
    }

    /// `loadEmbeddedServiceAccountCredential()` (strict parser).
    pub(crate) async fn embedded_strict(&self) -> Option<ServiceAccount> {
        self.embedded_raw()
            .await
            .and_then(parse_service_account_strict)
    }

    /// `loadEmbeddedCredential()` (loose parser).
    pub(crate) async fn embedded_loose(&self) -> Option<ServiceAccount> {
        self.embedded_raw()
            .await
            .and_then(parse_service_account_loose)
    }

    fn sa_key(&self, sa: &ServiceAccount) -> u64 {
        self.sa_key_hasher
            .hash_one((&sa.client_email, &sa.token_uri, &sa.private_key))
    }

    fn cached_sa_token(&self, sa: &ServiceAccount) -> Option<String> {
        self.sa_tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&self.sa_key(sa))
            .and_then(CachedToken::valid)
    }

    /// Cache a freshly exchanged token for [`sa_token_ttl`]; expired entries are pruned first.
    fn store_sa_token(&self, sa: &ServiceAccount, token: &str, expires_in: Option<&Value>) {
        let Some(ttl) = sa_token_ttl(expires_in) else {
            return;
        };
        if token.len() > SA_TOKEN_CACHE_MAX_LEN {
            return;
        }
        let now = Instant::now();
        let Some(expires) = now.checked_add(ttl) else {
            return;
        };
        let key = self.sa_key(sa);
        let mut cache = self.sa_tokens.lock().unwrap_or_else(|e| e.into_inner());
        cache.retain(|_, entry| entry.expires.is_some_and(|exp| now < exp));
        if cache.len() >= SA_TOKEN_CACHE_MAX_ENTRIES && !cache.contains_key(&key) {
            return;
        }
        cache.insert(
            key,
            CachedToken {
                token: token.to_owned(),
                expires: Some(expires),
            },
        );
    }

    /// Forget the cached token of `sa` after Vertex rejected `token` with 401/403 (a newer
    /// token cached meanwhile by a concurrent request is kept).
    pub(crate) fn forget_sa_token(&self, sa: &ServiceAccount, token: &str) {
        let key = self.sa_key(sa);
        let mut cache = self.sa_tokens.lock().unwrap_or_else(|e| e.into_inner());
        if cache.get(&key).is_some_and(|entry| entry.token == token) {
            cache.remove(&key);
        }
    }

    /// `exchangeServiceAccountToken` (route.ts): failures throw
    /// `Vertex service account token exchange failed: <detail>`.
    pub(crate) async fn exchange_strict(
        &self,
        http: &reqwest::Client,
        sa: &ServiceAccount,
        reuse: TokenReuse,
    ) -> Result<SaToken, Thrown> {
        if reuse == TokenReuse::Allow {
            if let Some(token) = self.cached_sa_token(sa) {
                return Ok(SaToken {
                    token,
                    cached: true,
                });
            }
        }
        let assertion = build_assertion(sa, unix_now()).map_err(Thrown)?;
        let resp = post_token_request(http, sa, &assertion).await?;
        let ok = resp.status().is_success();
        let data = net::read_json_or_null(resp).await;
        let token = non_nullish(prop(data.as_ref(), "access_token")).filter(|t| js_truthy(t));
        match token {
            Some(token) if ok => {
                let token = value_text(token);
                self.store_sa_token(sa, &token, prop(data.as_ref(), "expires_in"));
                Ok(SaToken {
                    token,
                    cached: false,
                })
            }
            _ => {
                let detail = non_nullish(prop(data.as_ref(), "error_description"))
                    .or_else(|| non_nullish(prop(data.as_ref(), "error")))
                    .map_or_else(|| "Token exchange failed".to_owned(), value_text);
                Err(Thrown::new(format!(
                    "Vertex service account token exchange failed: {detail}"
                )))
            }
        }
    }

    /// `exchangeToken` (vertex-helpers): `Err` carries the thrown message, which the caller
    /// wraps as `Vertex token error: <message>`.
    pub(crate) async fn exchange_loose(
        &self,
        http: &reqwest::Client,
        sa: &ServiceAccount,
        reuse: TokenReuse,
    ) -> Result<SaToken, String> {
        if reuse == TokenReuse::Allow {
            if let Some(token) = self.cached_sa_token(sa) {
                return Ok(SaToken {
                    token,
                    cached: true,
                });
            }
        }
        let assertion = build_assertion(sa, unix_now())?;
        let resp = post_token_request(http, sa, &assertion)
            .await
            .map_err(|t| t.0)?;
        let ok = resp.status().is_success();
        let data = net::read_json_strict(resp).await.map_err(|t| t.0)?;
        if data.is_null() {
            let prop_name = if ok { "access_token" } else { "error" };
            return Err(format!(
                "Cannot read properties of null (reading '{prop_name}')"
            ));
        }
        match prop(Some(&data), "access_token").filter(|t| js_truthy(t)) {
            Some(token) if ok => {
                let token = value_text(token);
                self.store_sa_token(sa, &token, prop(Some(&data), "expires_in"));
                Ok(SaToken {
                    token,
                    cached: false,
                })
            }
            _ => {
                let detail = non_nullish(prop(Some(&data), "error"))
                    .map_or_else(|| "failed".to_owned(), value_text);
                Err(format!("Vertex token exchange: {detail}"))
            }
        }
    }

    /// `getGcloudAccessToken()`: cached for 50 minutes, failures (`""`) are not cached.
    pub(crate) async fn gcloud_token(&self) -> String {
        self.gcloud_token_with_meta(TokenReuse::Allow)
            .await
            .map_or_else(String::new, |t| t.token)
    }

    /// Like [`Self::gcloud_token`], but allows forcing a fresh lookup and reports whether the
    /// token came from the 50-minute cache.
    pub(crate) async fn gcloud_token_with_meta(&self, reuse: TokenReuse) -> Option<GcloudToken> {
        if reuse == TokenReuse::Allow {
            if let Some(token) = self
                .gcloud
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .valid()
            {
                return Some(GcloudToken {
                    token,
                    cached: true,
                });
            }
        }
        // Single flight: concurrent callers wait for one gcloud run instead of spawning many.
        let _guard = self.gcloud_fetch.lock().await;
        if reuse == TokenReuse::Allow {
            if let Some(token) = self
                .gcloud
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .valid()
            {
                return Some(GcloudToken {
                    token,
                    cached: true,
                });
            }
        }
        let token = match &self.gcloud_program {
            GcloudProgram::Discover => discover_and_run_gcloud_token().await,
            #[cfg(test)]
            GcloudProgram::Path(path) => run_gcloud_token(path).await,
            #[cfg(test)]
            GcloudProgram::Fixed(token) => token.clone(),
            #[cfg(test)]
            GcloudProgram::Sequence(seq) => seq
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .pop_front()
                .unwrap_or_default(),
        };
        if token.is_empty() {
            return None;
        }
        *self.gcloud.lock().unwrap_or_else(|e| e.into_inner()) = CachedToken {
            token: token.clone(),
            expires: Some(Instant::now() + GCLOUD_TOKEN_TTL),
        };
        Some(GcloudToken {
            token,
            cached: false,
        })
    }

    /// `clearGcloudTokenCache()`.
    pub(crate) fn clear_gcloud_token(&self) {
        *self.gcloud.lock().unwrap_or_else(|e| e.into_inner()) = CachedToken::default();
    }
}

async fn post_token_request(
    http: &reqwest::Client,
    sa: &ServiceAccount,
    assertion: &str,
) -> Result<reqwest::Response, Thrown> {
    let headers = [(
        "content-type",
        "application/x-www-form-urlencoded".to_owned(),
    )];
    net::post(http, &sa.token_uri, &headers, token_request_body(assertion)).await
}

fn push_unique_candidate(candidates: &mut Vec<String>, candidate: String) {
    if !candidate.is_empty() && !candidates.contains(&candidate) {
        candidates.push(candidate);
    }
}

/// gcloud executable candidates in V1 order (`findGcloudPath`), extended for macOS, Linux and Windows.
pub(crate) fn gcloud_candidates(home: Option<&str>, path_env: Option<&str>) -> Vec<String> {
    let mut candidates: Vec<String> = Vec::new();
    if cfg!(windows) {
        candidates.push("gcloud".to_owned());
        for (var, sub) in [
            ("CLOUDSDK_ROOT_DIR", r"bin\gcloud.cmd"),
            (
                "LOCALAPPDATA",
                r"Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd",
            ),
            (
                "ProgramFiles",
                r"Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd",
            ),
            (
                "ProgramFiles(x86)",
                r"Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd",
            ),
            (
                "USERPROFILE",
                r"AppData\Local\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd",
            ),
        ] {
            if let Some(base) = std::env::var_os(var).filter(|v| !v.is_empty()) {
                push_unique_candidate(
                    &mut candidates,
                    Path::new(&base).join(sub).to_string_lossy().into_owned(),
                );
            }
        }
        return candidates;
    }
    candidates.extend(
        [
            "/usr/local/bin/gcloud",
            "/usr/bin/gcloud",
            "/snap/bin/gcloud",
            "/usr/lib/google-cloud-sdk/bin/gcloud",
        ]
        .map(String::from),
    );
    if cfg!(target_os = "macos") {
        for path in [
            "/opt/homebrew/bin/gcloud",
            "/opt/homebrew/share/google-cloud-sdk/bin/gcloud",
            "/opt/homebrew/Caskroom/google-cloud-sdk/latest/google-cloud-sdk/bin/gcloud",
            "/usr/local/share/google-cloud-sdk/bin/gcloud",
            "/usr/local/Caskroom/google-cloud-sdk/latest/google-cloud-sdk/bin/gcloud",
        ] {
            push_unique_candidate(&mut candidates, path.to_owned());
        }
    }
    if let Some(root) = std::env::var_os("CLOUDSDK_ROOT_DIR").filter(|v| !v.is_empty()) {
        push_unique_candidate(
            &mut candidates,
            Path::new(&root)
                .join("bin/gcloud")
                .to_string_lossy()
                .into_owned(),
        );
    }
    if let Some(home) = home.filter(|h| !h.is_empty()) {
        push_unique_candidate(
            &mut candidates,
            format!("{home}/google-cloud-sdk/bin/gcloud"),
        );
        push_unique_candidate(&mut candidates, format!("{home}/.local/bin/gcloud"));
    }
    for dir in path_env.unwrap_or("").split(':') {
        let test = format!("{dir}/gcloud");
        if test != "/gcloud" {
            push_unique_candidate(&mut candidates, test);
        }
    }
    candidates
}

/// Try each candidate in [`gcloud_candidates`] (plus plain `gcloud` via PATH) until one prints
/// a `ya29.` access token.
async fn discover_and_run_gcloud_token() -> String {
    let home = std::env::var("HOME").ok();
    let path_env = std::env::var("PATH").ok();
    let mut tried_plain = false;
    for candidate in gcloud_candidates(home.as_deref(), path_env.as_deref()) {
        let token = if candidate == "gcloud" {
            tried_plain = true;
            run_gcloud_token(Path::new("gcloud")).await
        } else if Path::new(&candidate).exists() {
            run_gcloud_token(Path::new(&candidate)).await
        } else {
            continue;
        };
        if !token.is_empty() {
            return token;
        }
    }
    if !tried_plain {
        let token = run_gcloud_token(Path::new("gcloud")).await;
        if !token.is_empty() {
            return token;
        }
    }
    String::new()
}

/// `runGcloudToken(path)`: `gcloud auth print-access-token` with a 10 s timeout; any failure,
/// or output not starting with `ya29.`, yields `""`.
pub(crate) async fn run_gcloud_token(path: &Path) -> String {
    let mut cmd = gcloud_command(path);
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let Ok(child) = cmd.spawn() else {
        return String::new();
    };
    let output = match tokio::time::timeout(GCLOUD_TIMEOUT, child.wait_with_output()).await {
        Ok(Ok(output)) => output,
        _ => return String::new(),
    };
    if !output.status.success() || output.stdout.len() > GCLOUD_MAX_STDOUT {
        return String::new();
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let token = js_trim(&stdout);
    if token.starts_with("ya29.") {
        token.to_owned()
    } else {
        String::new()
    }
}

#[cfg(not(windows))]
fn gcloud_command(path: &Path) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(path);
    cmd.args(["auth", "print-access-token"]);
    cmd
}

/// Windows: `gcloud` is a `.cmd` script, run through `cmd.exe /d /s /c` without a console window
/// (same invocation as the V1 local proxy).
#[cfg(windows)]
fn gcloud_command(path: &Path) -> tokio::process::Command {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut cmd = tokio::process::Command::new("cmd.exe");
    cmd.raw_arg(format!(
        "/d /s /c \"\"{}\" auth print-access-token\"",
        path.display()
    ));
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strict_and_loose_parsers_differ_like_v1() {
        let raw = "  {\"client_email\":\" a@b \",\"private_key\":\"K\",\"token_uri\":\" \",\"project_id\":\" p \"} ";
        let strict = parse_service_account_strict(raw).expect("strict");
        assert_eq!(strict.client_email, "a@b");
        assert_eq!(strict.token_uri, DEFAULT_TOKEN_URI);
        assert_eq!(strict.project_id, "p");
        let loose = parse_service_account_loose(raw).expect("loose");
        assert_eq!(loose.client_email, " a@b ");
        assert_eq!(loose.token_uri, " ");
        assert_eq!(loose.project_id, " p ");
        assert!(parse_service_account_strict("AIzaKey").is_none());
        assert!(
            parse_service_account_strict("{\"client_email\":1,\"private_key\":\"k\"}").is_none()
        );
        assert!(parse_service_account_loose("{broken").is_none());
        // JS trim removes U+3000 but JSON.parse does not accept it: loose fails, strict succeeds.
        let ideographic = "\u{3000}{\"client_email\":\"e\",\"private_key\":\"k\"}";
        assert!(parse_service_account_loose(ideographic).is_none());
        assert!(parse_service_account_strict(ideographic).is_some());
        // Same for a credential file saved with a BOM (`readFile(.., "utf8")` keeps it).
        let bom = "\u{FEFF}{\"client_email\":\"e\",\"private_key\":\"k\"}";
        assert!(parse_service_account_loose(bom).is_none());
        assert!(parse_service_account_strict(bom).is_some());
    }

    #[test]
    fn sa_token_ttl_respects_expires_in_and_never_panics() {
        let ttl = |v: Value| sa_token_ttl(Some(&v));
        // Google's usual answer: 3599 s minus the margin, capped at 55 minutes.
        assert_eq!(ttl(json!(3599)), Some(SA_TOKEN_TTL));
        assert_eq!(ttl(json!(1000)), Some(Duration::from_secs(940)));
        assert_eq!(ttl(json!(61)), Some(Duration::from_secs(1)));
        assert_eq!(ttl(json!(90.5)), Some(Duration::from_millis(30_500)));
        // Short-lived, expired or nonsensical lifetimes are not cached (V1 never cached).
        assert_eq!(ttl(json!(60)), None);
        assert_eq!(ttl(json!(30)), None);
        assert_eq!(ttl(json!(0)), None);
        assert_eq!(ttl(json!(-5)), None);
        assert_eq!(ttl(json!("3599")), None);
        assert_eq!(ttl(Value::Null), None);
        assert_eq!(sa_token_ttl(None), None);
        // Huge values used to panic in `Duration::from_secs_f64`; they are clamped first.
        assert_eq!(ttl(json!(1e20)), Some(SA_TOKEN_TTL));
        assert_eq!(ttl(json!(u64::MAX)), Some(SA_TOKEN_TTL));
        assert_eq!(ttl(json!(f64::MAX)), Some(SA_TOKEN_TTL));
    }

    fn sa(email: &str) -> ServiceAccount {
        ServiceAccount {
            client_email: email.to_owned(),
            private_key: "K".to_owned(),
            token_uri: DEFAULT_TOKEN_URI.to_owned(),
            project_id: String::new(),
        }
    }

    #[test]
    fn sa_token_cache_is_bounded_and_forgets_rejected_tokens() {
        let auth = GoogleAuth::new(None);
        let a = sa("a@x");
        let long_lived = json!(3599);
        auth.store_sa_token(&a, "ya29.short", Some(&json!(30)));
        assert_eq!(auth.cached_sa_token(&a), None, "too short-lived to cache");
        auth.store_sa_token(&a, "ya29.none", None);
        assert_eq!(auth.cached_sa_token(&a), None, "unknown lifetime");
        auth.store_sa_token(
            &a,
            &"t".repeat(SA_TOKEN_CACHE_MAX_LEN + 1),
            Some(&long_lived),
        );
        assert_eq!(auth.cached_sa_token(&a), None, "oversized token");
        auth.store_sa_token(&a, "ya29.a1", Some(&long_lived));
        assert_eq!(auth.cached_sa_token(&a).as_deref(), Some("ya29.a1"));
        // Same email, different key or token endpoint: a different cache entry.
        let other_key = ServiceAccount {
            private_key: "K2".to_owned(),
            ..a.clone()
        };
        assert_eq!(auth.cached_sa_token(&other_key), None);
        let other_uri = ServiceAccount {
            token_uri: "https://evil.example/token".to_owned(),
            ..a.clone()
        };
        assert_eq!(auth.cached_sa_token(&other_uri), None);
        // A rejected token is forgotten, a newer one is kept.
        auth.forget_sa_token(&a, "ya29.stale");
        assert_eq!(auth.cached_sa_token(&a).as_deref(), Some("ya29.a1"));
        auth.forget_sa_token(&a, "ya29.a1");
        assert_eq!(auth.cached_sa_token(&a), None);
        // The number of entries is capped.
        for i in 0..SA_TOKEN_CACHE_MAX_ENTRIES + 10 {
            auth.store_sa_token(&sa(&format!("{i}@x")), "ya29.n", Some(&long_lived));
        }
        let len = auth.sa_tokens.lock().map(|m| m.len()).unwrap_or_default();
        assert_eq!(len, SA_TOKEN_CACHE_MAX_ENTRIES);
        assert_eq!(auth.cached_sa_token(&sa("0@x")).as_deref(), Some("ya29.n"));
        assert_eq!(auth.cached_sa_token(&sa("70@x")), None);
    }

    #[test]
    fn encode_uri_component_golden() {
        assert_eq!(
            encode_uri_component("AIza-_.!~*'() /+?=&:"),
            "AIza-_.!~*'()%20%2F%2B%3F%3D%26%3A"
        );
        assert_eq!(encode_uri_component("あ"), "%E3%81%82");
    }

    #[test]
    fn token_request_body_matches_url_search_params() {
        assert_eq!(
            token_request_body("a.b-c_d"),
            "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=a.b-c_d"
        );
    }

    #[test]
    fn assertion_errors_match_node() {
        let mut sa = ServiceAccount {
            client_email: "e".into(),
            private_key: "  ".into(),
            token_uri: DEFAULT_TOKEN_URI.into(),
            project_id: String::new(),
        };
        assert_eq!(build_assertion(&sa, 0), Err(ERR_KEY_EMPTY.to_owned()));
        sa.private_key = "-----BEGIN PRIVATE KEY-----\\nAAAA\\n-----END PRIVATE KEY-----".into();
        assert_eq!(build_assertion(&sa, 0), Err(ERR_KEY_UNSUPPORTED.to_owned()));
    }

    #[test]
    fn gcloud_candidates_follow_v1_order() {
        if cfg!(windows) {
            return;
        }
        let c = gcloud_candidates(Some("/home/u"), Some("/usr/bin::/opt/x"));
        assert_eq!(c[0], "/usr/local/bin/gcloud");
        assert_eq!(c[3], "/usr/lib/google-cloud-sdk/bin/gcloud");
        assert!(c.contains(&"/home/u/google-cloud-sdk/bin/gcloud".to_owned()));
        // `/usr/bin/gcloud` is not duplicated, the empty PATH entry is skipped.
        assert_eq!(c.iter().filter(|p| *p == "/usr/bin/gcloud").count(), 1);
        assert!(!c.contains(&"/gcloud".to_owned()));
        assert_eq!(c.last().map(String::as_str), Some("/opt/x/gcloud"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn gcloud_runner_filters_output() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("amadeus-gcloud-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let script = |name: &str, body: &str| {
            let p = dir.join(name);
            let _ = std::fs::write(&p, format!("#!/bin/sh\n{body}\n"));
            let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755));
            p
        };
        let good = script("good", "echo '  ya29.token  '");
        let bad = script("bad", "echo 'not-a-token'");
        let fail = script("fail", "echo ya29.x; exit 1");
        assert_eq!(run_gcloud_token(&good).await, "ya29.token");
        assert_eq!(run_gcloud_token(&bad).await, "");
        assert_eq!(run_gcloud_token(&fail).await, "");
        assert_eq!(run_gcloud_token(&dir.join("missing")).await, "");
        let auth = GoogleAuth {
            gcloud_program: GcloudProgram::Path(good),
            ..GoogleAuth::new(None)
        };
        assert_eq!(auth.gcloud_token().await, "ya29.token");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
