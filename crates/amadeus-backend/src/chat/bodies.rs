//! Upstream request body builders (V1 `shared.ts` §3). Objects are built in V1's key insertion
//! order (serde_json `preserve_order`), and `messages` are forwarded verbatim where V1 did so.

use amadeus_core::jscompat::js_truthy;
use serde_json::{json, Map, Value};

/// `isValidGroqModel(model, allowCompound)`.
pub(crate) fn is_valid_groq_model(model: &str, allow_compound: bool) -> bool {
    let lowered = model.to_lowercase();
    if ["llama", "mixtral", "gemma", "qwen", "deepseek"]
        .iter()
        .any(|k| lowered.contains(k))
    {
        return true;
    }
    allow_compound && lowered.contains("compound")
}

/// `buildOpenAIBody(messages, model, stream, isGroq)`.
pub(crate) fn build_openai_body(
    messages: &[Value],
    model: &str,
    stream: bool,
    is_groq: bool,
) -> Value {
    let mut body = Map::new();
    body.insert("model".into(), json!(model));
    body.insert("messages".into(), Value::Array(messages.to_vec()));
    body.insert("max_tokens".into(), json!(2048));
    if is_groq {
        body.insert("temperature".into(), json!(0.85));
        body.insert("top_p".into(), json!(0.9));
        if model.to_lowercase().contains("qwen") {
            body.insert("reasoning_format".into(), json!("hidden"));
        }
    }
    if stream {
        body.insert("stream".into(), json!(true));
    }
    Value::Object(body)
}

/// `buildGroqCompoundBody(messages, stream)`.
pub(crate) fn build_groq_compound_body(messages: &[Value], stream: bool) -> Value {
    let mut body = Map::new();
    body.insert("model".into(), json!("groq/compound"));
    body.insert("messages".into(), Value::Array(messages.to_vec()));
    body.insert("max_tokens".into(), json!(2048));
    body.insert("temperature".into(), json!(0.85));
    if stream {
        body.insert("stream".into(), json!(true));
    }
    Value::Object(body)
}

/// `message.role` / `message.content` of an arbitrary JSON message (`None` = `undefined`).
fn field<'a>(message: &'a Value, key: &str) -> Option<&'a Value> {
    message.as_object().and_then(|o| o.get(key))
}

fn is_role(message: &Value, role: &str) -> bool {
    field(message, "role").and_then(Value::as_str) == Some(role)
}

/// `{ text: content }`; JSON.stringify drops the key when `content` is `undefined`.
fn text_part(content: Option<&Value>) -> Value {
    let mut part = Map::new();
    if let Some(c) = content {
        part.insert("text".into(), c.clone());
    }
    Value::Object(part)
}

/// Maximum output tokens requested from Gemini / Vertex models (shared with thinking budget on Gemini 3+).
pub(crate) const GEMINI_MAX_OUTPUT_TOKENS: u32 = 8192;

/// `buildGeminiBody(messages, useGrounding)` (Gemini, Vertex and FreeAPI).
pub(crate) fn build_gemini_body(messages: &[Value], use_grounding: bool) -> Value {
    let mut contents = Vec::new();
    // `let systemInstruction = ""`: the last system message wins, even an empty one.
    let mut system_instruction: Option<Value> = Some(Value::String(String::new()));
    for message in messages {
        if is_role(message, "system") {
            system_instruction = field(message, "content").cloned();
            continue;
        }
        let role = if is_role(message, "assistant") {
            "model"
        } else {
            "user"
        };
        contents.push(json!({ "role": role, "parts": [text_part(field(message, "content"))] }));
    }
    let mut body = Map::new();
    body.insert("contents".into(), Value::Array(contents));
    body.insert(
        "generationConfig".into(),
        json!({ "maxOutputTokens": GEMINI_MAX_OUTPUT_TOKENS }),
    );
    if let Some(system) = system_instruction.filter(js_truthy) {
        body.insert(
            "system_instruction".into(),
            json!({ "parts": [{ "text": system }] }),
        );
    }
    if use_grounding {
        body.insert("tools".into(), json!([{ "googleSearch": {} }]));
    }
    Value::Object(body)
}

/// `buildClaudeBody(messages, model)`.
pub(crate) fn build_claude_body(messages: &[Value], model: &str) -> Value {
    let mut system: Option<Value> = Some(Value::String(String::new()));
    let mut filtered = Vec::new();
    for message in messages {
        if is_role(message, "system") {
            system = field(message, "content").cloned();
            continue;
        }
        let mut m = Map::new();
        m.insert(
            "role".into(),
            json!(if is_role(message, "assistant") {
                "assistant"
            } else {
                "user"
            }),
        );
        if let Some(c) = field(message, "content") {
            m.insert("content".into(), c.clone());
        }
        filtered.push(Value::Object(m));
    }
    let mut body = Map::new();
    body.insert("model".into(), json!(model));
    body.insert("max_tokens".into(), json!(2048));
    if let Some(system) = system.filter(js_truthy) {
        body.insert("system".into(), system);
    }
    body.insert("messages".into(), Value::Array(filtered));
    Value::Object(body)
}

/// Ollama `/api/chat` body (`{ model, messages, stream }`).
pub(crate) fn build_ollama_body(model: &str, messages: &[Value], stream: bool) -> Value {
    json!({ "model": model, "messages": messages, "stream": stream })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msgs(v: Value) -> Vec<Value> {
        v.as_array().cloned().unwrap_or_default()
    }

    fn s(v: &Value) -> String {
        serde_json::to_string(v).unwrap_or_default()
    }

    #[test]
    fn groq_model_filter_golden() {
        let cases = [
            ("llama-3.1-8b-instant", true, true),
            ("groq/compound", true, false),
            ("compound-beta", true, false),
            ("openai/gpt-oss-120b", false, false),
            ("Qwen/Qwen3-32B", true, true),
            ("moonshotai/kimi-k2-instruct", false, false),
            ("DeepSeek-r1", true, true),
        ];
        for (model, with_compound, without) in cases {
            assert_eq!(is_valid_groq_model(model, true), with_compound, "{model}");
            assert_eq!(is_valid_groq_model(model, false), without, "{model}");
        }
    }

    #[test]
    fn openai_body_golden() {
        let m = msgs(json!([{"role": "user", "content": "U"}]));
        assert_eq!(
            s(&build_openai_body(&m, "gpt-4o", false, false)),
            r#"{"model":"gpt-4o","messages":[{"role":"user","content":"U"}],"max_tokens":2048}"#
        );
        assert_eq!(
            s(&build_openai_body(&m, "qwen3-32b", true, true)),
            r#"{"model":"qwen3-32b","messages":[{"role":"user","content":"U"}],"max_tokens":2048,"temperature":0.85,"top_p":0.9,"reasoning_format":"hidden","stream":true}"#
        );
        assert_eq!(
            s(&build_openai_body(
                &m,
                "llama-3.3-70b-versatile",
                false,
                true
            )),
            r#"{"model":"llama-3.3-70b-versatile","messages":[{"role":"user","content":"U"}],"max_tokens":2048,"temperature":0.85,"top_p":0.9}"#
        );
    }

    #[test]
    fn messages_are_forwarded_verbatim() {
        let m = msgs(json!([{"content": "C", "role": "tool", "extra": [1, 2]}, "raw"]));
        assert_eq!(
            s(&build_openai_body(&m, "x", false, false)),
            r#"{"model":"x","messages":[{"content":"C","role":"tool","extra":[1,2]},"raw"],"max_tokens":2048}"#
        );
    }

    #[test]
    fn groq_compound_body_golden() {
        let m = msgs(json!([{"role": "user", "content": "U"}]));
        assert_eq!(
            s(&build_groq_compound_body(&m, true)),
            r#"{"model":"groq/compound","messages":[{"role":"user","content":"U"}],"max_tokens":2048,"temperature":0.85,"stream":true}"#
        );
        assert_eq!(
            s(&build_groq_compound_body(&m, false)),
            r#"{"model":"groq/compound","messages":[{"role":"user","content":"U"}],"max_tokens":2048,"temperature":0.85}"#
        );
    }

    #[test]
    fn gemini_body_golden() {
        let m = msgs(json!([
            {"role": "system", "content": "S"},
            {"role": "user", "content": "U"},
            {"role": "assistant", "content": "A"},
            {"role": "system", "content": "S2"},
            {"role": "user", "content": "U2"},
        ]));
        assert_eq!(
            s(&build_gemini_body(&m, true)),
            r#"{"contents":[{"role":"user","parts":[{"text":"U"}]},{"role":"model","parts":[{"text":"A"}]},{"role":"user","parts":[{"text":"U2"}]}],"generationConfig":{"maxOutputTokens":8192},"system_instruction":{"parts":[{"text":"S2"}]},"tools":[{"googleSearch":{}}]}"#
        );
        let m = msgs(json!([{"role": "user", "content": "U"}, {"role": "tool", "content": "T"}]));
        assert_eq!(
            s(&build_gemini_body(&m, false)),
            r#"{"contents":[{"role":"user","parts":[{"text":"U"}]},{"role":"user","parts":[{"text":"T"}]}],"generationConfig":{"maxOutputTokens":8192}}"#
        );
    }

    #[test]
    fn gemini_body_js_edge_cases() {
        // The last system message wins even when empty; missing content drops the `text` key.
        let m = msgs(
            json!([{"role": "system", "content": "S"}, {"role": "system", "content": ""}, {"role": "user"}, "x"]),
        );
        assert_eq!(
            s(&build_gemini_body(&m, false)),
            r#"{"contents":[{"role":"user","parts":[{}]},{"role":"user","parts":[{}]}],"generationConfig":{"maxOutputTokens":8192}}"#
        );
    }

    #[test]
    fn claude_body_golden() {
        let m = msgs(json!([
            {"role": "system", "content": "S"},
            {"role": "user", "content": "U"},
            {"role": "assistant", "content": "A"},
            {"role": "system", "content": "S2"},
            {"role": "user", "content": "U2"},
        ]));
        assert_eq!(
            s(&build_claude_body(&m, "m")),
            r#"{"model":"m","max_tokens":2048,"system":"S2","messages":[{"role":"user","content":"U"},{"role":"assistant","content":"A"},{"role":"user","content":"U2"}]}"#
        );
        let m = msgs(json!([{"role": "user", "content": "U"}]));
        assert_eq!(
            s(&build_claude_body(&m, "m")),
            r#"{"model":"m","max_tokens":2048,"messages":[{"role":"user","content":"U"}]}"#
        );
    }

    #[test]
    fn ollama_body_golden() {
        let m = msgs(json!([{"role": "user", "content": "U"}]));
        assert_eq!(
            s(&build_ollama_body("mistral", &m, true)),
            r#"{"model":"mistral","messages":[{"role":"user","content":"U"}],"stream":true}"#
        );
        assert_eq!(
            s(&build_ollama_body("mistral", &m, false)),
            r#"{"model":"mistral","messages":[{"role":"user","content":"U"}],"stream":false}"#
        );
    }
}
