//! The V1 **client** SSE parser of `ai-service.ts` `sendChatStreaming`, quirks included.
//!
//! Pure (no web-sys): the web transport feeds it the `TextDecoder` output of the response body,
//! the desktop transport feeds it `"data: " + JSON.stringify(event) + "\n\n"` for every Channel
//! message, so both report exactly the same events.
//!
//! Rules (spec-lib-media §5.4):
//! 1. Lines are split on `\n` only and JS-trimmed (`\r` and surrounding blanks disappear).
//! 2. Only lines starting with exactly `data: ` count; `data:x`, `event:`, `id:`, `: ping` are
//!    ignored and multi-line `data:` fields are not joined.
//! 3. The payload after `data: ` is JS-trimmed; an empty payload is ignored.
//! 4. A payload that is not JSON — or is JSON `null` (`null.type` throws in V1) — is a raw token:
//!    `data: [DONE]` yields the token `"[DONE]"`.
//! 5. Other non-object JSON (numbers, strings, booleans, arrays) is ignored.
//! 6. `{"type":"token"}` needs a truthy `token` (`""` is ignored); token text is not trimmed.
//! 7. `{"type":"complete"}` reports `fullResponse ?? <accumulated tokens>` and ends the stream.
//! 8. `{"type":"error"}` reports `error ?? "Streaming error"` and ends the stream.
//! 9. Unknown `type`s are ignored.
//! 10. End of stream without a terminal event reports `Complete(<accumulated tokens>)`; an
//!     unterminated trailing line is discarded.

use amadeus_core::jscompat::{js_string, js_trim, js_truthy};
use serde_json::Value;

/// V1 `"Streaming error"` (an `error` event without an `error` field).
pub const STREAMING_ERROR: &str = "Streaming error";

/// One client-side stream event (V1 `streamToken` / `streamComplete` / `errorOccurred`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseEvent {
    /// `streamToken(token)`.
    Token(String),
    /// `streamComplete(fullResponse)`; the stream is over.
    Complete(String),
    /// `errorOccurred(error)`; the stream is over.
    Error(String),
}

impl SseEvent {
    /// `Complete` and `Error` end the stream (V1 `return`s after emitting them).
    pub fn is_terminal(&self) -> bool {
        !matches!(self, SseEvent::Token(_))
    }
}

/// Incremental V1 line parser. Feed decoded text with [`push`](Self::push); call
/// [`finish`](Self::finish) at end of stream.
#[derive(Debug, Default, Clone)]
pub struct SseParser {
    buffer: String,
    full: String,
    finished: bool,
}

impl SseParser {
    /// Empty parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends decoded text and returns the events of every complete line, in order. Stops at
    /// the first terminal event (later data is ignored, like V1's `return`).
    pub fn push(&mut self, text: &str) -> Vec<SseEvent> {
        let mut events = Vec::new();
        if self.finished {
            return events;
        }
        self.buffer.push_str(text);
        while let Some(index) = self.buffer.find('\n') {
            let line = js_trim(&self.buffer[..index]).to_owned();
            self.buffer.drain(..=index);
            if let Some(event) = self.process_line(&line) {
                let terminal = event.is_terminal();
                events.push(event);
                if terminal {
                    self.finished = true;
                    self.buffer.clear();
                    break;
                }
            }
        }
        events
    }

    /// End of stream: `Complete(<accumulated tokens>)` unless a terminal event was already
    /// produced (then `None`). The unterminated tail of the buffer is discarded (V1 never
    /// processes it and never flushes its decoder).
    pub fn finish(&mut self) -> Option<SseEvent> {
        if self.finished {
            return None;
        }
        self.finished = true;
        self.buffer.clear();
        Some(SseEvent::Complete(self.full.clone()))
    }

    /// True after a terminal event or [`finish`](Self::finish).
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Text of every token so far (V1 `full`).
    pub fn full(&self) -> &str {
        &self.full
    }

    fn process_line(&mut self, line: &str) -> Option<SseEvent> {
        let raw = js_trim(line.strip_prefix("data: ")?);
        if raw.is_empty() {
            return None;
        }
        match serde_json::from_str::<Value>(raw) {
            // `JSON.parse` failed, or returned `null` (whose `.type` throws): raw token.
            Err(_) | Ok(Value::Null) => {
                self.full.push_str(raw);
                Some(SseEvent::Token(raw.to_owned()))
            }
            Ok(Value::Object(event)) => match event.get("type").and_then(Value::as_str) {
                Some("token") => {
                    let token = event.get("token").filter(|t| js_truthy(t))?;
                    let token = js_string(token);
                    self.full.push_str(&token);
                    Some(SseEvent::Token(token))
                }
                Some("complete") => Some(SseEvent::Complete(
                    match nullish(event.get("fullResponse")) {
                        Some(full) => js_string(full),
                        None => self.full.clone(),
                    },
                )),
                Some("error") => Some(SseEvent::Error(match nullish(event.get("error")) {
                    Some(error) => js_string(error),
                    None => STREAMING_ERROR.to_owned(),
                })),
                _ => None,
            },
            // Numbers, strings, booleans, arrays: `.type` is undefined.
            Ok(_) => None,
        }
    }
}

/// JS `x ?? …`: `None` for a missing field or `null`.
fn nullish(value: Option<&Value>) -> Option<&Value> {
    value.filter(|v| !v.is_null())
}

#[cfg(test)]
mod tests {
    use super::*;
    use SseEvent::{Complete, Error, Token};

    /// Feeds `chunks` and, if the stream did not end, the EOF.
    fn run(chunks: &[&str]) -> Vec<SseEvent> {
        let mut parser = SseParser::new();
        let mut events = Vec::new();
        for chunk in chunks {
            events.extend(parser.push(chunk));
        }
        events.extend(parser.finish());
        events
    }

    fn tok(s: &str) -> SseEvent {
        Token(s.to_owned())
    }

    #[test]
    fn v1_vector_1_tokens_then_complete() {
        assert_eq!(
            run(&[
                "data: {\"type\":\"token\",\"token\":\"Hel\"}\n\n",
                "data: {\"type\":\"token\",\"token\":\"lo\"}\n\ndata: {\"type\":\"complete\",\"fullResponse\":\"Hello\"}\n\n",
            ]),
            vec![tok("Hel"), tok("lo"), Complete("Hello".into())]
        );
    }

    #[test]
    fn v1_vector_2_split_json_across_chunks() {
        assert_eq!(
            run(&["data: {\"type\":\"tok", "en\",\"token\":\"A\"}\n", "\n"]),
            vec![tok("A"), Complete("A".into())]
        );
    }

    #[test]
    fn v1_vector_3_plain_text_is_a_token() {
        assert_eq!(
            run(&["data: plain text\n"]),
            vec![tok("plain text"), Complete("plain text".into())]
        );
    }

    #[test]
    fn v1_vectors_4_5_error_events() {
        assert_eq!(
            run(&["data: {\"type\":\"error\",\"error\":\"boom\"}\n"]),
            vec![Error("boom".into())]
        );
        assert_eq!(
            run(&["data: {\"type\":\"error\"}\n"]),
            vec![Error(STREAMING_ERROR.into())]
        );
        assert_eq!(
            run(&["data: {\"type\":\"error\",\"error\":null}\n"]),
            vec![Error(STREAMING_ERROR.into())]
        );
    }

    #[test]
    fn v1_vectors_6_7_complete_fallbacks() {
        let tokens = "data: {\"type\":\"token\",\"token\":\"a\"}\ndata: {\"type\":\"token\",\"token\":\"b\"}\n";
        assert_eq!(
            run(&[tokens, "data: {\"type\":\"complete\"}\n"]),
            vec![tok("a"), tok("b"), Complete("ab".into())]
        );
        assert_eq!(
            run(&[
                "data: {\"type\":\"token\",\"token\":\"a\"}\n",
                "data: {\"type\":\"complete\",\"fullResponse\":\"\"}\n"
            ]),
            vec![tok("a"), Complete(String::new())]
        );
    }

    #[test]
    fn v1_vector_8_non_data_lines_are_ignored() {
        assert_eq!(
            run(&[
                ": ping\n",
                "event: x\n",
                "data:{\"type\":\"token\",\"token\":\"x\"}\n"
            ]),
            vec![Complete(String::new())]
        );
    }

    #[test]
    fn v1_vector_9_empty_token_is_ignored() {
        assert_eq!(
            run(&["data: {\"type\":\"token\",\"token\":\"\"}\n"]),
            vec![Complete(String::new())]
        );
    }

    #[test]
    fn v1_vector_10_done_marker_is_a_token() {
        assert_eq!(
            run(&["data: [DONE]\n"]),
            vec![tok("[DONE]"), Complete("[DONE]".into())]
        );
    }

    #[test]
    fn v1_vector_11_unterminated_line_is_dropped() {
        assert_eq!(
            run(&["data: {\"type\":\"token\",\"token\":\"x\"}"]),
            vec![Complete(String::new())]
        );
    }

    #[test]
    fn v1_vector_12_json_null_is_a_token() {
        assert_eq!(
            run(&["data: null\n"]),
            vec![tok("null"), Complete("null".into())]
        );
    }

    #[test]
    fn v1_vector_13_trims_lines_but_not_tokens() {
        let mut parser = SseParser::new();
        assert_eq!(
            parser.push("  data: {\"type\":\"token\",\"token\":\" a \"}  \r\n"),
            vec![tok(" a ")]
        );
        assert_eq!(parser.full(), " a ");
    }

    #[test]
    fn data_after_a_terminal_event_is_ignored() {
        let mut parser = SseParser::new();
        let events = parser.push(
            "data: {\"type\":\"complete\",\"fullResponse\":\"x\"}\ndata: {\"type\":\"token\",\"token\":\"y\"}\n",
        );
        assert_eq!(events, vec![Complete("x".into())]);
        assert!(parser.is_finished());
        assert!(parser
            .push("data: {\"type\":\"token\",\"token\":\"z\"}\n")
            .is_empty());
        assert_eq!(parser.finish(), None);
    }

    #[test]
    fn other_json_shapes_are_ignored() {
        assert_eq!(
            run(&[
                "data: 42\n",
                "data: \"token\"\n",
                "data: true\n",
                "data: [1,2]\n",
                "data: {\"type\":\"unknown\",\"token\":\"x\"}\n",
                "data: {\"token\":\"x\"}\n",
                "data: {\"type\":\"TOKEN\",\"token\":\"x\"}\n",
                "data:    \n",
                "data:\n",
            ]),
            vec![Complete(String::new())]
        );
    }

    #[test]
    fn non_string_values_use_js_string_conversion() {
        assert_eq!(
            run(&[
                "data: {\"type\":\"token\",\"token\":5}\n",
                "data: {\"type\":\"complete\",\"fullResponse\":8.0}\n"
            ]),
            vec![tok("5"), Complete("8".into())]
        );
        assert_eq!(
            run(&["data: {\"type\":\"error\",\"error\":{\"a\":1}}\n"]),
            vec![Error("[object Object]".into())]
        );
        // A falsy token (0) is ignored, like V1's `event.token` truthiness check.
        assert_eq!(
            run(&["data: {\"type\":\"token\",\"token\":0}\n"]),
            vec![Complete(String::new())]
        );
    }

    #[test]
    fn crlf_blank_lines_and_unicode() {
        assert_eq!(
            run(&[
                "data: {\"type\":\"token\",\"token\":\"あ😀\"}\r\n\r\n",
                "data: {\"type\":\"token\",\"token\":\"\\n\"}\r\n",
            ]),
            vec![tok("あ😀"), tok("\n"), Complete("あ😀\n".into())]
        );
        // JS trim also removes U+3000 / U+FEFF around the line and the payload.
        assert_eq!(
            run(&["\u{feff}data: \u{3000}raw\u{3000}\n"]),
            vec![tok("raw"), Complete("raw".into())]
        );
    }

    #[test]
    fn chunk_boundaries_do_not_matter() {
        let stream = "data: {\"type\":\"token\",\"token\":\"紅莉栖\"}\n\ndata: x y\n\ndata: {\"type\":\"complete\",\"fullResponse\":\"done\"}\n\n";
        let whole = run(&[stream]);
        assert_eq!(
            whole,
            vec![tok("紅莉栖"), tok("x y"), Complete("done".into())]
        );
        let chars: Vec<String> = stream.chars().map(String::from).collect();
        let refs: Vec<&str> = chars.iter().map(String::as_str).collect();
        assert_eq!(run(&refs), whole);
    }

    #[test]
    fn desktop_channel_frames_parse_like_http() {
        // The desktop transport feeds `data: <JSON.stringify(event)>\n\n` per Channel message.
        let mut parser = SseParser::new();
        let mut events = Vec::new();
        for json in [
            "{\"type\":\"token\",\"token\":\"[SMILE]\"}",
            "{\"type\":\"token\",\"token\":\"ok\"}",
            "{\"type\":\"complete\",\"fullResponse\":\"[SMILE]ok\"}",
        ] {
            events.extend(parser.push(&format!("data: {json}\n\n")));
        }
        assert_eq!(
            events,
            vec![tok("[SMILE]"), tok("ok"), Complete("[SMILE]ok".into())]
        );
        assert_eq!(parser.finish(), None);
    }
}
