//! Upstream response extractors (V1 `shared.ts` §4).

use amadeus_core::jscompat::js_is_whitespace;
use serde_json::Value;

use super::payload::{index0, prop};
use super::texts::PARSE_ERROR;

fn string_or_parse_error(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        _ => PARSE_ERROR.to_owned(),
    }
}

/// `extractOpenAIResponse`: `choices[0].message.content` or `"[Parse Error]"`.
pub(crate) fn extract_openai_response(v: Option<&Value>) -> String {
    string_or_parse_error(prop(prop(index0(prop(v, "choices")), "message"), "content"))
}

/// `extractGeminiResponse`: concatenates all non-thought `text` parts in
/// `candidates[0].content.parts`, or falls back to `[Parse Error]`.
pub(crate) fn extract_gemini_response(v: Option<&Value>) -> String {
    let parts = prop(prop(index0(prop(v, "candidates")), "content"), "parts");
    if let Some(Value::Array(arr)) = parts {
        let mut combined = String::new();
        let mut found_text = false;
        for part in arr {
            if part.get("thought") == Some(&Value::Bool(true)) {
                continue;
            }
            if let Some(Value::String(s)) = part.get("text") {
                combined.push_str(s);
                found_text = true;
            }
        }
        return if found_text {
            combined
        } else {
            PARSE_ERROR.to_owned()
        };
    }
    string_or_parse_error(prop(index0(parts), "text"))
}

/// Extracts `candidates[0].finishReason` if present and a string.
pub(crate) fn extract_gemini_finish_reason(v: Option<&Value>) -> Option<&str> {
    prop(index0(prop(v, "candidates")), "finishReason").and_then(Value::as_str)
}

/// Emits a warning log when Gemini stops with a reason other than `"STOP"` (e.g. `MAX_TOKENS` or `SAFETY`).
pub(crate) fn log_abnormal_gemini_finish_reason(
    context: &str,
    v: Option<&Value>,
    accumulated_chars: usize,
) {
    if let Some(reason) = extract_gemini_finish_reason(v) {
        if !reason.is_empty() && reason != "STOP" {
            super::console_warn(&format!(
                "[Gemini] {context} ended with finishReason={reason} (chars={accumulated_chars})"
            ));
        }
    }
}

/// `extractClaudeResponse`: `content[0].text` or `"[Parse Error]"`.
pub(crate) fn extract_claude_response(v: Option<&Value>) -> String {
    string_or_parse_error(prop(index0(prop(v, "content")), "text"))
}

/// `extractOpenAIStreamToken`: `choices[0].delta.content` or `""`.
pub(crate) fn extract_openai_stream_token(v: Option<&Value>) -> String {
    match prop(prop(index0(prop(v, "choices")), "delta"), "content") {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

/// JS line terminators (what `.` does not match without the `s` flag).
fn is_js_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// Match `\s*:\s*"((?:\\.|[^"\\])*)"` starting at byte `i` (just after `"text"`). Returns the
/// captured group and the byte index after the closing quote.
fn match_text_value(s: &str, i: usize) -> Option<(&str, usize)> {
    let skip_ws =
        |from: usize| from + s[from..].len() - s[from..].trim_start_matches(js_is_whitespace).len();
    let mut i = skip_ws(i);
    if !s[i..].starts_with(':') {
        return None;
    }
    i = skip_ws(i + 1);
    if !s[i..].starts_with('"') {
        return None;
    }
    let start = i + 1;
    let mut chars = s[start..].char_indices();
    while let Some((off, c)) = chars.next() {
        match c {
            '"' => return Some((&s[start..start + off], start + off + 1)),
            // `\\.`: an escape consumes the next character unless it is a line terminator. A
            // dead end here cannot be rescued by backtracking (every earlier `"` was escaped).
            '\\' => match chars.next() {
                Some((_, n)) if !is_js_line_terminator(n) => {}
                _ => return None,
            },
            _ => {}
        }
    }
    None
}

/// `extractVertexStreamTokensLikeUnity(chunk)` (vertex-proxy streaming only): every
/// `"text": "<escaped>"` occurrence in an arbitrary chunk, with V1's sequential (buggy) unescape.
pub(crate) fn extract_vertex_stream_tokens_like_unity(chunk: &str) -> Vec<String> {
    const KEY: &str = "\"text\"";
    let mut tokens = Vec::new();
    let mut from = 0;
    while let Some(rel) = chunk[from..].find(KEY) {
        let start = from + rel;
        match match_text_value(chunk, start + KEY.len()) {
            Some((escaped, end)) => {
                let unescaped = escaped
                    .replace("\\n", "\n")
                    .replace("\\r", "\r")
                    .replace("\\t", "\t")
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\");
                if !unescaped.is_empty() {
                    tokens.push(unescaped);
                }
                from = end;
            }
            None => from = start + 1,
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn openai_extractor() {
        let ok = json!({"choices": [{"message": {"content": "hi"}}]});
        assert_eq!(extract_openai_response(Some(&ok)), "hi");
        let null_content = json!({"choices": [{"message": {"content": null}}]});
        assert_eq!(extract_openai_response(Some(&null_content)), PARSE_ERROR);
        assert_eq!(extract_openai_response(None), PARSE_ERROR);
        assert_eq!(extract_openai_response(Some(&json!("x"))), PARSE_ERROR);
    }

    #[test]
    fn gemini_extractor_joins_non_thought_parts() {
        let ok = json!({"candidates": [{"content": {"parts": [{"text": "a"}, {"text": "b"}]}}]});
        assert_eq!(extract_gemini_response(Some(&ok)), "ab");
        let with_thought = json!({"candidates": [{"content": {"parts": [
            {"text": "internal", "thought": true},
            {"thoughtSignature": "sig"},
            {"text": "answer"},
            {"text": "!"}
        ]}}]});
        assert_eq!(extract_gemini_response(Some(&with_thought)), "answer!");
        let thought_only =
            json!({"candidates": [{"content": {"parts": [{"text": "only", "thought": true}]}}]});
        assert_eq!(extract_gemini_response(Some(&thought_only)), PARSE_ERROR);
        let empty = json!({"candidates": [{"content": {"parts": [{"text": ""}]}}]});
        assert_eq!(extract_gemini_response(Some(&empty)), "");
        let safety = json!({"candidates": [{"finishReason": "SAFETY"}]});
        assert_eq!(extract_gemini_response(Some(&safety)), PARSE_ERROR);
        assert_eq!(extract_gemini_finish_reason(Some(&safety)), Some("SAFETY"));
        let max_tokens = json!({"candidates": [{"finishReason": "MAX_TOKENS"}]});
        assert_eq!(
            extract_gemini_finish_reason(Some(&max_tokens)),
            Some("MAX_TOKENS")
        );
    }

    #[test]
    fn claude_and_stream_extractors() {
        assert_eq!(
            extract_claude_response(Some(&json!({"content": [{"text": "c"}]}))),
            "c"
        );
        assert_eq!(
            extract_claude_response(Some(&json!({"content": []}))),
            PARSE_ERROR
        );
        assert_eq!(
            extract_openai_stream_token(Some(&json!({"choices": [{"delta": {"content": "t"}}]}))),
            "t"
        );
        assert_eq!(
            extract_openai_stream_token(Some(&json!({"choices": [{"delta": {}}]}))),
            ""
        );
        assert_eq!(
            extract_openai_stream_token(Some(&json!({"error": {"message": "x"}}))),
            ""
        );
    }

    #[test]
    fn unity_regex_golden() {
        let input = r#"data: {"candidates":[{"content":{"parts":[{"text": "a\nb \"q\" \\n "#
            .to_owned()
            + "\\u3042"
            + r#""}]}}]}"#
            + "\r\n"
            + r#"{"text":""} {"text" : "c"}"#;
        let tokens = extract_vertex_stream_tokens_like_unity(&input);
        assert_eq!(
            serde_json::to_string(&tokens).unwrap_or_default(),
            r#"["a\nb \"q\" \\\n \\u3042","c"]"#
        );
    }

    #[test]
    fn unity_regex_edge_cases() {
        // Unterminated value: no match.
        assert!(extract_vertex_stream_tokens_like_unity(r#"{"text": "abc"#).is_empty());
        // Escaped backslash before a line break is a dead end for that occurrence only.
        let s = "{\"text\": \"a\\\nb\"} {\"text\":\"ok\"}";
        assert_eq!(
            extract_vertex_stream_tokens_like_unity(s),
            vec!["ok".to_owned()]
        );
        // Literal newlines inside the value are allowed by `[^"\\]`.
        assert_eq!(
            extract_vertex_stream_tokens_like_unity("\"text\":\"a\nb\""),
            vec!["a\nb".to_owned()]
        );
        // Ideographic space counts as `\s`.
        assert_eq!(
            extract_vertex_stream_tokens_like_unity("\"text\"\u{3000}:\"x\""),
            vec!["x".to_owned()]
        );
        // `\/` and `\u` are not decoded.
        assert_eq!(
            extract_vertex_stream_tokens_like_unity(r#""text":"a\/b""#),
            vec![r"a\/b".to_owned()]
        );
    }
}
