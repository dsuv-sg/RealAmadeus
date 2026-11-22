//! `/chat` provider branches (`route.ts` §6.2, §6.3).

use serde_json::Value;

use super::bodies::{
    build_claude_body, build_gemini_body, build_groq_compound_body, build_ollama_body,
    build_openai_body, is_valid_groq_model,
};
use super::extract::{extract_claude_response, extract_gemini_response, extract_openai_response};
use super::google::{
    encode_uri_component, parse_service_account_strict, ServiceAccount, TokenReuse,
};
use super::net::{self, content_json, Header, StatusLine};
use super::payload::{non_nullish, prop, Payload, ProviderVal, Thrown};
use super::texts::{
    ERR_NO_API_KEY, ERR_UNKNOWN_PROVIDER, ERR_VERTEX_GCLOUD_PROJECT_REQUIRED,
    ERR_VERTEX_NO_CREDENTIALS_FOUND, ERR_VERTEX_SA_JSON_INVALID, ERR_VERTEX_SA_PROJECT_REQUIRED,
    FREE_FALLBACK_COMM_ERROR,
};
use super::vertex::{self, VertexMethod};
use super::{free_api, state};
use crate::Backend;

/// Outcome of a `/chat` provider branch that did not throw.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Once {
    /// `sendResponse(value)`: `200 {"response": value}`, logged.
    Send(Value),
    /// `jsonError(error, status)`: not logged.
    Fail { status: u16, error: Value },
}

impl Once {
    fn text(s: impl Into<String>) -> Self {
        Once::Send(Value::String(s.into()))
    }

    fn fail(status: u16, msg: impl Into<String>) -> Self {
        Once::Fail {
            status,
            error: Value::String(msg.into()),
        }
    }
}

/// `switch (provider)` of `route.ts`. `model` is `payload.model?.trim() ?? ""`.
pub(crate) async fn chat_once(
    backend: &Backend,
    payload: &mut Payload,
    provider: ProviderVal,
    model: String,
) -> Result<Once, Thrown> {
    let web_search = payload.truthy("webSearch");
    match provider.chat_branch() {
        Some(0) => openai(backend, payload, model).await,
        Some(1) => gemini(backend, payload, model, web_search).await,
        Some(2) => claude(backend, payload, model).await,
        Some(3) => groq(backend, payload, model, web_search).await,
        Some(4) => vertex_route(backend, payload, model, web_search).await,
        Some(5) => ollama(backend, payload, model).await,
        Some(6) => openrouter(backend, payload, model).await,
        Some(7) => local_proxy(backend, payload).await,
        Some(8) => Ok(
            match free_api::fetch_free_api_response(backend, payload).await? {
                Ok(text) => Once::text(text),
                Err(_) => Once::text(FREE_FALLBACK_COMM_ERROR),
            },
        ),
        _ => Ok(Once::fail(400, ERR_UNKNOWN_PROVIDER)),
    }
}

/// OpenAI endpoint, or `<openaiBaseUrl trimmed, one trailing "/" removed>/chat/completions`.
pub(crate) fn openai_url(backend: &Backend, payload: &Payload) -> Result<String, Thrown> {
    if payload.truthy("openaiCompatible") && payload.truthy("openaiBaseUrl") {
        let Some(Value::String(base)) = payload.get("openaiBaseUrl") else {
            return Err(Thrown::not_a_function("payload.openaiBaseUrl.trim"));
        };
        let mut base = amadeus_core::jscompat::js_trim(base).to_owned();
        if base.ends_with('/') {
            base.pop();
        }
        return Ok(format!("{base}/chat/completions"));
    }
    Ok(state(backend).endpoints.openai.clone())
}

/// `payload.ollamaHost?.trim() || default`, prefixed with `http://` unless it starts with `http`.
pub(crate) fn ollama_host(payload: &Payload) -> Result<String, Thrown> {
    let host = payload.trim_or(
        "ollamaHost",
        "payload.ollamaHost?.trim",
        "http://localhost:11434",
    )?;
    Ok(if host.starts_with("http") {
        host
    } else {
        format!("http://{host}")
    })
}

fn bearer(payload: &Payload) -> Header {
    (
        "authorization",
        format!("Bearer {}", payload.api_key_text()),
    )
}

/// POST, read `json or null`, then E_NS on failure or `extract` on success.
async fn post_ns(
    backend: &Backend,
    url: &str,
    headers: &[Header],
    body: &Value,
    extract: fn(Option<&Value>) -> String,
) -> Result<Once, Thrown> {
    let resp = net::post(backend.http(), url, headers, net::json_body(body)).await?;
    let line = StatusLine::of(&resp);
    let ok = resp.status().is_success();
    let data = net::read_json_or_null(resp).await;
    if !ok {
        return Ok(Once::fail(line.code, net::e_ns(&line, data.as_ref())));
    }
    Ok(Once::text(extract(data.as_ref())))
}

async fn openai(backend: &Backend, payload: &Payload, model: String) -> Result<Once, Thrown> {
    if !payload.has_api_key() {
        return Ok(Once::fail(400, ERR_NO_API_KEY));
    }
    let model = if model.is_empty() {
        "gpt-4o".to_owned()
    } else {
        model
    };
    let url = openai_url(backend, payload)?;
    let body = build_openai_body(payload.messages(), &model, false, false);
    post_ns(
        backend,
        &url,
        &[bearer(payload), content_json()],
        &body,
        extract_openai_response,
    )
    .await
}

async fn gemini(
    backend: &Backend,
    payload: &Payload,
    model: String,
    web_search: bool,
) -> Result<Once, Thrown> {
    if !payload.has_api_key() {
        return Ok(Once::fail(400, ERR_NO_API_KEY));
    }
    let model = if model.is_empty() || model.starts_with("gpt") {
        "gemini-2.5-flash".to_owned()
    } else {
        model
    };
    let url = state(backend).endpoints.gemini(&model, false);
    let headers = [("x-goog-api-key", payload.api_key_text()), content_json()];
    post_ns(
        backend,
        &url,
        &headers,
        &build_gemini_body(payload.messages(), web_search),
        extract_gemini_response,
    )
    .await
}

async fn claude(backend: &Backend, payload: &Payload, model: String) -> Result<Once, Thrown> {
    if !payload.has_api_key() {
        return Ok(Once::fail(400, ERR_NO_API_KEY));
    }
    let model = if model.is_empty() || model.starts_with("gpt") || model.starts_with("gemini") {
        "claude-3-7-sonnet-20250219".to_owned()
    } else {
        model
    };
    let headers = [
        ("x-api-key", payload.api_key_text()),
        ("anthropic-version", "2023-06-01".to_owned()),
        content_json(),
    ];
    let url = state(backend).endpoints.anthropic.clone();
    post_ns(
        backend,
        &url,
        &headers,
        &build_claude_body(payload.messages(), &model),
        extract_claude_response,
    )
    .await
}

async fn groq(
    backend: &Backend,
    payload: &Payload,
    model: String,
    web_search: bool,
) -> Result<Once, Thrown> {
    if !payload.has_api_key() {
        return Ok(Once::fail(400, ERR_NO_API_KEY));
    }
    let model = if model.is_empty() || !is_valid_groq_model(&model, true) {
        "qwen3-32b".to_owned()
    } else {
        model
    };
    let body = if web_search {
        build_groq_compound_body(payload.messages(), false)
    } else {
        build_openai_body(payload.messages(), &model, false, true)
    };
    let url = state(backend).endpoints.groq.clone();
    post_ns(
        backend,
        &url,
        &[bearer(payload), content_json()],
        &body,
        extract_openai_response,
    )
    .await
}

async fn openrouter(backend: &Backend, payload: &Payload, model: String) -> Result<Once, Thrown> {
    if !payload.has_api_key() {
        return Ok(Once::fail(400, ERR_NO_API_KEY));
    }
    let model = if model.is_empty() || model.to_lowercase().starts_with("gpt") {
        "google/gemini-2.5-flash".to_owned()
    } else {
        model
    };
    let headers = [
        bearer(payload),
        content_json(),
        ("http-referer", "https://dsuv.vercel.app".to_owned()),
        ("x-title", "RealAmadeus".to_owned()),
    ];
    let url = state(backend).endpoints.openrouter.clone();
    post_ns(
        backend,
        &url,
        &headers,
        &build_openai_body(payload.messages(), &model, false, false),
        extract_openai_response,
    )
    .await
}

/// Ollama and the local proxy report `body.error ?? "<status> <statusText>"` (not E_NS).
fn error_or_status(data: Option<&Value>, line: &StatusLine) -> Value {
    non_nullish(prop(data, "error"))
        .cloned()
        .unwrap_or_else(|| Value::String(format!("{} {}", line.code, line.text)))
}

async fn ollama(backend: &Backend, payload: &Payload, model: String) -> Result<Once, Thrown> {
    let host = ollama_host(payload)?;
    let model = if model.is_empty() || model.to_lowercase().starts_with("gpt") {
        "mistral".to_owned()
    } else {
        model
    };
    let body = build_ollama_body(&model, payload.messages(), false);
    let resp = net::post(
        backend.http(),
        &format!("{host}/api/chat"),
        &[content_json()],
        net::json_body(&body),
    )
    .await?;
    let line = StatusLine::of(&resp);
    let ok = resp.status().is_success();
    let data = net::read_json_or_null(resp).await;
    if !ok {
        return Ok(Once::Fail {
            status: line.code,
            error: error_or_status(data.as_ref(), &line),
        });
    }
    let content = non_nullish(prop(prop(data.as_ref(), "message"), "content")).cloned();
    Ok(Once::Send(
        content.unwrap_or_else(|| Value::String(String::new())),
    ))
}

async fn local_proxy(backend: &Backend, payload: &Payload) -> Result<Once, Thrown> {
    let host = payload.trim_or(
        "localProxyHost",
        "payload.localProxyHost?.trim",
        "127.0.0.1",
    )?;
    let port = payload.trim_or("localProxyPort", "payload.localProxyPort?.trim", "5103")?;
    let url = if host.starts_with("http") {
        format!("{host}:{port}/chat")
    } else {
        format!("http://{host}:{port}/chat")
    };
    let resp = net::post(backend.http(), &url, &[content_json()], payload.to_json()).await?;
    let line = StatusLine::of(&resp);
    let ok = resp.status().is_success();
    let data = net::read_json_or_null(resp).await;
    if !ok {
        return Ok(Once::Fail {
            status: line.code,
            error: error_or_status(data.as_ref(), &line),
        });
    }
    let response = non_nullish(prop(data.as_ref(), "response")).cloned();
    Ok(Once::Send(
        response.unwrap_or_else(|| Value::String(String::new())),
    ))
}

/// `route.ts` B1: exchange a service-account token, then one `generateContent` call with E_NS
/// errors. V1 exchanged a new token for every request; when Vertex rejects a token from the V2
/// cache (401/403), the token is forgotten and the call is redone once with a fresh exchange,
/// so the outcome is the one V1 would have produced. A rejected fresh token is forgotten too.
async fn vertex_service_account(
    backend: &Backend,
    sa: &ServiceAccount,
    url: &str,
    body: &str,
) -> Result<Once, Thrown> {
    let google = &state(backend).google;
    let mut token = google
        .exchange_strict(backend.http(), sa, TokenReuse::Allow)
        .await?;
    loop {
        let headers = [
            ("authorization", format!("Bearer {}", token.token)),
            content_json(),
        ];
        let resp = net::post(backend.http(), url, &headers, body.to_owned()).await?;
        let line = StatusLine::of(&resp);
        if vertex::is_auth_failure(line.code) {
            google.forget_sa_token(sa, &token.token);
            if token.cached {
                token = google
                    .exchange_strict(backend.http(), sa, TokenReuse::Fresh)
                    .await?;
                continue;
            }
        }
        let ok = resp.status().is_success();
        let data = net::read_json_or_null(resp).await;
        if !ok {
            return Ok(Once::fail(line.code, net::e_ns(&line, data.as_ref())));
        }
        return Ok(Once::text(extract_gemini_response(data.as_ref())));
    }
}

/// Vertex (provider 4) decision tree of `route.ts` (§6.3), or Qt/Unity gcloud parity on desktop.
async fn vertex_route(
    backend: &Backend,
    payload: &Payload,
    model: String,
    web_search: bool,
) -> Result<Once, Thrown> {
    if backend.config().is_desktop() {
        return Ok(match vertex::desktop_vertex_once(backend, payload).await? {
            Ok(text) => Once::text(text),
            Err((error, status)) => Once::fail(status, error),
        });
    }
    // A) `useGCloudCLI` (client default true): the vertex-helpers path, which never runs gcloud.
    if payload.truthy("useGCloudCLI") {
        return Ok(
            match vertex::fetch_vertex_response(backend, payload).await? {
                Ok(text) => Once::text(text),
                Err((error, status)) => Once::fail(status, error),
            },
        );
    }
    // B) payload service account → server service account → gcloud (empty key) → raw API key.
    let cred = payload.trim_or_empty("apiKey", "payload.apiKey?.trim")?;
    let payload_sa = parse_service_account_strict(&cred);
    if cred.starts_with('{') && payload_sa.is_none() {
        return Ok(Once::fail(400, ERR_VERTEX_SA_JSON_INVALID));
    }
    let model = if model.is_empty() || model.to_lowercase().starts_with("gpt") {
        "gemini-2.5-flash".to_owned()
    } else {
        model
    };
    let location = payload.trim_or(
        "vertexLocation",
        "payload.vertexLocation?.trim",
        "us-central1",
    )?;
    let st = state(backend);
    let sa = match payload_sa {
        Some(sa) => Some(sa),
        None => st.google.embedded_strict().await,
    };
    let body = build_gemini_body(payload.messages(), web_search);

    if let Some(sa) = sa {
        let project = payload
            .opt_trim("vertexProject", "payload.vertexProject?.trim")?
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| sa.project_id.clone());
        if project.is_empty() {
            return Ok(Once::fail(400, ERR_VERTEX_SA_PROJECT_REQUIRED));
        }
        let url = st
            .endpoints
            .vertex(&project, &location, &model, VertexMethod::Generate);
        return vertex_service_account(backend, &sa, &url, &net::json_body(&body)).await;
    }

    if cred.is_empty() {
        let token = st.google.gcloud_token().await;
        if token.is_empty() {
            return Ok(Once::fail(400, ERR_VERTEX_NO_CREDENTIALS_FOUND));
        }
        let project = payload.trim_or_empty("vertexProject", "payload.vertexProject?.trim")?;
        if project.is_empty() {
            return Ok(Once::fail(400, ERR_VERTEX_GCLOUD_PROJECT_REQUIRED));
        }
        let url = st
            .endpoints
            .vertex(&project, &location, &model, VertexMethod::Generate);
        let headers = [("authorization", format!("Bearer {token}")), content_json()];
        return post_ns(backend, &url, &headers, &body, extract_gemini_response).await;
    }

    // B3: raw API key against regional Vertex (`?key=`) and then AI Studio.
    let mut candidates = Vec::new();
    let project = payload.trim_or_empty("vertexProject", "payload.vertexProject?.trim")?;
    if !project.is_empty() {
        let url = st
            .endpoints
            .vertex(&project, &location, &model, VertexMethod::Generate);
        candidates.push(format!("{url}?key={}", encode_uri_component(&cred)));
    }
    candidates.push(st.endpoints.gemini(&model, false));
    let headers = [("x-goog-api-key", cred.clone()), content_json()];
    let body = net::json_body(&body);
    let mut last = (
        StatusLine {
            code: 500,
            text: "Upstream request failed".to_owned(),
        },
        None,
    );
    for url in &candidates {
        let resp = net::post(backend.http(), url, &headers, body.clone()).await?;
        let line = StatusLine::of(&resp);
        let ok = resp.status().is_success();
        let data = net::read_json_or_null(resp).await;
        if ok {
            return Ok(Once::text(extract_gemini_response(data.as_ref())));
        }
        last = (line, data);
    }
    Ok(Once::fail(last.0.code, net::e_ns(&last.0, last.1.as_ref())))
}
