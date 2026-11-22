//! `.amadeus_chat_log.jsonl` writer (`logInteraction`, spec §10.6 / §13.2). Web server only.

use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

/// File name of the interaction log.
pub const CHAT_LOG_FILE: &str = ".amadeus_chat_log.jsonl";

/// `new Date().toISOString()`: `YYYY-MM-DDTHH:MM:SS.mmmZ`.
pub(crate) fn iso_timestamp(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// The serialized line (without the trailing LF):
/// `JSON.stringify({ timestamp, ip, provider, count, requestText, replyText })`.
pub(crate) fn format_entry(
    timestamp: String,
    ip: &str,
    provider: i64,
    request_text: &Value,
    reply_text: &Value,
    count: i64,
) -> String {
    let mut entry = Map::new();
    entry.insert("timestamp".into(), Value::String(timestamp));
    entry.insert("ip".into(), Value::String(ip.to_owned()));
    entry.insert("provider".into(), Value::from(provider));
    entry.insert("count".into(), Value::from(count));
    entry.insert("requestText".into(), request_text.clone());
    entry.insert("replyText".into(), reply_text.clone());
    crate::aux::js_json::stringify(&Value::Object(entry))
}

/// `s.substring(0, n)` counted in UTF-16 code units (a split surrogate pair is left out).
pub(crate) fn utf16_prefix(s: &str, n: usize) -> &str {
    let mut units = 0;
    for (i, c) in s.char_indices() {
        units += c.len_utf16();
        if units > n {
            return &s[..i];
        }
    }
    s
}

/// Append-only interaction log.
pub(crate) struct ChatLog {
    path: PathBuf,
    lock: Mutex<()>,
}

impl ChatLog {
    /// Log writing to `path`.
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    /// `logInteraction(ip, provider, requestText, replyText, count)`.
    pub(crate) fn log_interaction(
        &self,
        ip: &str,
        provider: i64,
        request_text: &Value,
        reply_text: &Value,
        count: i64,
    ) {
        let line = format_entry(
            iso_timestamp(Utc::now()),
            ip,
            provider,
            request_text,
            reply_text,
            count,
        ) + "\n";
        let written = {
            let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .and_then(|mut f| f.write_all(line.as_bytes()))
        };
        if let Err(e) = written {
            super::console_error(&format!("Failed to log interaction {e}"));
            return;
        }
        // V1 called `.substring` on both texts after the append; non-strings threw there.
        match (request_text, reply_text) {
            (Value::String(req), Value::String(rep)) => super::console_log(&format!(
                "[Interaction Log] Count: {count} | Provider: {provider} | IP: {ip} | Req: {}... | Rep: {}...",
                utf16_prefix(req, 30),
                utf16_prefix(rep, 30)
            )),
            (Value::String(_), _) => {
                super::console_error("Failed to log interaction TypeError: replyText.substring is not a function");
            }
            _ => super::console_error("Failed to log interaction TypeError: requestText.substring is not a function"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    #[test]
    fn iso_timestamp_golden() {
        let t = Utc
            .with_ymd_and_hms(2026, 11, 16, 1, 2, 3)
            .single()
            .map(|t| t + chrono::Duration::milliseconds(4));
        assert_eq!(
            t.map(iso_timestamp).as_deref(),
            Some("2026-11-16T01:02:03.004Z")
        );
    }

    #[test]
    fn entry_key_order_and_escaping() {
        let line = format_entry(
            "2026-11-16T01:02:03.004Z".into(),
            "::1",
            8,
            &json!("Q\"あ"),
            &json!("A\n"),
            65000,
        );
        assert_eq!(
            line,
            r#"{"timestamp":"2026-11-16T01:02:03.004Z","ip":"::1","provider":8,"count":65000,"requestText":"Q\"あ","replyText":"A\n"}"#
        );
    }

    #[test]
    fn utf16_prefix_counts_code_units() {
        assert_eq!(utf16_prefix("abc", 2), "ab");
        assert_eq!(utf16_prefix("あいう", 30), "あいう");
        assert_eq!(utf16_prefix("a😀b", 2), "a");
        assert_eq!(utf16_prefix("a😀b", 3), "a😀");
    }

    #[test]
    fn appends_lines() {
        let dir = std::env::temp_dir().join(format!("amadeus-log-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(CHAT_LOG_FILE);
        let _ = std::fs::remove_file(&path);
        let log = ChatLog::new(path.clone());
        log.log_interaction("1.1.1.1", 1, &json!("q1"), &json!("a1"), 3);
        log.log_interaction("2.2.2.2", 0, &json!("q2"), &json!(5), 3);
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with(
            r#""ip":"1.1.1.1","provider":1,"count":3,"requestText":"q1","replyText":"a1"}"#
        ));
        assert!(lines[1].ends_with(r#""requestText":"q2","replyText":5}"#));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
