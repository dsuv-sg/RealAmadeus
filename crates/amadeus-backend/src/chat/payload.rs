//! Lenient chat payload handling with the JavaScript coercion semantics V1 relied on
//! (`spec-server-chat.md` §1.2, §2.4). The payload is kept as a raw JSON object (insertion
//! order preserved) because V1 never validated field types and forwards it verbatim (provider 7).

use amadeus_core::jscompat::{js_string, js_trim, js_truthy};
use serde_json::{Map, Value};

/// A JavaScript exception as V1 surfaced it through `error.message` (`TypeError` from
/// `x?.trim()`, undici's `fetch failed`, …).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Thrown(pub(crate) String);

impl Thrown {
    /// Exception with the given message.
    pub(crate) fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }

    /// `TypeError: <expr> is not a function`, thrown by `x?.trim()` when `x` is not a string.
    pub(crate) fn not_a_function(expr: &str) -> Self {
        Self(format!("{expr} is not a function"))
    }
}

/// V8's message for `m.role` on a `null` message in the question lookup.
const NULL_MESSAGE_TYPE_ERROR: &str = "Cannot read properties of null (reading 'role')";

/// `payload.provider` normalized for V1's strict `===` comparisons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ProviderVal {
    /// Key absent (`undefined`).
    Missing,
    /// JSON `null`.
    Null,
    /// Any JSON number (`8.0 === 8`).
    Num(f64),
    /// Strings, booleans, arrays, objects (`"8" !== 8`).
    Other,
}

impl ProviderVal {
    /// Normalize a raw JSON value.
    pub(crate) fn from_value(v: Option<&Value>) -> Self {
        match v {
            None => ProviderVal::Missing,
            Some(Value::Null) => ProviderVal::Null,
            Some(Value::Number(n)) => n.as_f64().map_or(ProviderVal::Other, ProviderVal::Num),
            Some(_) => ProviderVal::Other,
        }
    }

    /// `payload.provider === n`.
    pub(crate) fn is(self, n: u8) -> bool {
        matches!(self, ProviderVal::Num(x) if x == f64::from(n))
    }

    /// `provider === 8 || provider === undefined`: the global FreeAPI counter applies.
    pub(crate) fn counts_global(self) -> bool {
        self.is(8) || matches!(self, ProviderVal::Missing)
    }

    /// Branch selected by `/chat`'s `switch (payload.provider ?? 0)`; `None` = "Unknown provider.".
    pub(crate) fn chat_branch(self) -> Option<u8> {
        match self {
            ProviderVal::Missing | ProviderVal::Null => Some(0),
            ProviderVal::Num(n) => (0u8..=8).find(|&p| n == f64::from(p)),
            ProviderVal::Other => None,
        }
    }

    /// Value written to the chat log (`payload.provider ?? 0`). Only providers 0..=8 ever reach
    /// the log, so the integer form is exact.
    pub(crate) fn log_value(self) -> i64 {
        match self {
            ProviderVal::Num(n) if n.is_finite() => n as i64,
            _ => 0,
        }
    }
}

/// Parse a request body exactly like `await request.json().catch(() => null)`: UTF-8 decoding
/// with replacement characters, a leading BOM stripped, `Content-Type` ignored.
pub(crate) fn parse_json_body(body: &[u8]) -> Option<Value> {
    let text = String::from_utf8_lossy(body);
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(&text);
    serde_json::from_str(text).ok()
}

/// The parsed request payload (a JSON object whose `messages` is an array).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Payload {
    map: Map<String, Value>,
}

impl Payload {
    /// `!payload || !Array.isArray(payload.messages)` → `None` (400 "Invalid request payload").
    pub(crate) fn parse(body: &[u8]) -> Option<Self> {
        match parse_json_body(body)? {
            Value::Object(map) if map.get("messages").is_some_and(Value::is_array) => {
                Some(Self { map })
            }
            _ => None,
        }
    }

    /// Wrap an already parsed object (must contain a `messages` array).
    #[cfg(test)]
    pub(crate) fn from_value(v: Value) -> Option<Self> {
        match v {
            Value::Object(map) if map.get("messages").is_some_and(Value::is_array) => {
                Some(Self { map })
            }
            _ => None,
        }
    }

    /// Raw field value (`None` = `undefined`).
    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        self.map.get(key)
    }

    /// Normalized `payload.provider`.
    pub(crate) fn provider(&self) -> ProviderVal {
        ProviderVal::from_value(self.get("provider"))
    }

    /// `payload.messages` (always an array after [`Payload::parse`]).
    pub(crate) fn messages(&self) -> &[Value] {
        match self.map.get("messages") {
            Some(Value::Array(a)) => a,
            _ => &[],
        }
    }

    /// `Boolean(payload[key])`.
    pub(crate) fn truthy(&self, key: &str) -> bool {
        self.get(key).is_some_and(js_truthy)
    }

    /// `!!payload.apiKey` (`"  "` counts as a key, `0`/`false`/`""` do not).
    pub(crate) fn has_api_key(&self) -> bool {
        self.truthy("apiKey")
    }

    /// `` `${payload.apiKey}` `` as used in request headers.
    pub(crate) fn api_key_text(&self) -> String {
        self.get("apiKey")
            .map_or_else(|| "undefined".to_owned(), js_string)
    }

    /// `payload[key]?.trim()`: `Ok(None)` for missing/null, the JS-trimmed string, or the
    /// `TypeError` V1 threw for any other type (`expr` is the V1 source expression).
    pub(crate) fn opt_trim(&self, key: &str, expr: &str) -> Result<Option<String>, Thrown> {
        match self.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(js_trim(s).to_owned())),
            Some(_) => Err(Thrown::not_a_function(expr)),
        }
    }

    /// `payload[key]?.trim() ?? ""`.
    pub(crate) fn trim_or_empty(&self, key: &str, expr: &str) -> Result<String, Thrown> {
        Ok(self.opt_trim(key, expr)?.unwrap_or_default())
    }

    /// `payload[key]?.trim() || default`.
    pub(crate) fn trim_or(&self, key: &str, expr: &str, default: &str) -> Result<String, Thrown> {
        Ok(self
            .opt_trim(key, expr)?
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| default.to_owned()))
    }

    /// `payload[key] = value` (FreeAPI's Vertex fallback mutates model/project/location).
    pub(crate) fn set_str(&mut self, key: &str, value: &str) {
        self.map
            .insert(key.to_owned(), Value::String(value.to_owned()));
    }

    /// `payload.messages.filter(m => m.role === "user").pop()?.content || ""`.
    /// `Err` is the `TypeError` V1 threw for a `null` entry (outside any `try` block: HTTP 500
    /// with an empty body).
    pub(crate) fn question(&self) -> Result<Value, Thrown> {
        let mut last = None;
        for m in self.messages() {
            match m {
                Value::Null => return Err(Thrown::new(NULL_MESSAGE_TYPE_ERROR)),
                Value::Object(o) if o.get("role").and_then(Value::as_str) == Some("user") => {
                    last = Some(o)
                }
                _ => {}
            }
        }
        Ok(match last.and_then(|o| o.get("content")) {
            Some(c) if js_truthy(c) => c.clone(),
            _ => Value::String(String::new()),
        })
    }

    /// `JSON.stringify(payload)`: insertion order, except that array-index keys come first (V8),
    /// and numbers formatted like JS (`7.0` → `7`).
    pub(crate) fn to_json(&self) -> String {
        crate::aux::js_json::stringify(&Value::Object(self.map.clone()))
    }
}

/// Property access `v?.[key]` on a JSON value (only objects have named properties).
pub(crate) fn prop<'a>(v: Option<&'a Value>, key: &str) -> Option<&'a Value> {
    match v {
        Some(Value::Object(o)) => o.get(key),
        _ => None,
    }
}

/// Index access `v?.[0]` (arrays, or objects with a `"0"` key, like JS).
pub(crate) fn index0(v: Option<&Value>) -> Option<&Value> {
    match v {
        Some(Value::Array(a)) => a.first(),
        Some(Value::Object(o)) => o.get("0"),
        _ => None,
    }
}

/// `v ?? fallback` for a JSON value: `None` for `undefined` and `null`.
pub(crate) fn non_nullish(v: Option<&Value>) -> Option<&Value> {
    v.filter(|v| !v.is_null())
}

/// Text of a value interpolated into a string or event (`String(x)` for non-strings).
pub(crate) fn value_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => js_string(other),
    }
}

/// JS `parseInt(s, 10)` restricted to the integer range (`None` = `NaN`).
pub(crate) fn js_parse_int(s: &str) -> Option<i64> {
    let s = amadeus_core::jscompat::js_trim_start(s);
    let (neg, rest) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let digits: &str = &rest[..rest.bytes().take_while(u8::is_ascii_digit).count()];
    if digits.is_empty() {
        return None;
    }
    let mut value: i64 = 0;
    for d in digits.bytes() {
        value = value.saturating_mul(10).saturating_add(i64::from(d - b'0'));
    }
    Some(if neg { -value } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn payload(v: Value) -> Payload {
        Payload::from_value(v).expect("test payload")
    }

    #[test]
    fn parse_is_lenient_like_request_json() {
        assert!(Payload::parse(b"{\"messages\":[]}").is_some());
        assert!(Payload::parse("\u{FEFF}{\"messages\":[]}".as_bytes()).is_some());
        assert!(Payload::parse(b"  {\"messages\":[]}\n").is_some());
        assert!(Payload::parse(b"").is_none());
        assert!(Payload::parse(b"null").is_none());
        assert!(Payload::parse(b"[1]").is_none());
        assert!(Payload::parse(b"{\"messages\":{}}").is_none());
        assert!(Payload::parse(b"{\"messages\":null}").is_none());
        assert!(Payload::parse(b"{\"messages\":[],}").is_none());
        // Invalid UTF-8 inside a string becomes U+FFFD instead of failing.
        let p = Payload::parse(b"{\"messages\":[],\"model\":\"a\xffb\"}").expect("lossy");
        assert_eq!(p.get("model"), Some(&json!("a\u{FFFD}b")));
    }

    #[test]
    fn key_order_is_preserved_for_passthrough() {
        let raw = br#"{"provider":7,"messages":[{"role":"user","content":"U"}],"apiKey":"k","zeta":1,"alpha":2}"#;
        let p = Payload::parse(raw).expect("valid");
        assert_eq!(p.to_json(), String::from_utf8_lossy(raw));
        // JSON.stringify(JSON.parse(raw)) in V8: array-index keys move first, `7.0` prints `7`.
        let quirky = br#"{"messages":[{"1":"x","role":"user"}],"provider":7.0,"10":"b","2":"a"}"#;
        let p = Payload::parse(quirky).expect("valid");
        assert_eq!(
            p.to_json(),
            r#"{"2":"a","10":"b","messages":[{"1":"x","role":"user"}],"provider":7}"#
        );
    }

    #[test]
    fn provider_normalization() {
        let cases = [
            (None, ProviderVal::Missing),
            (Some(json!(null)), ProviderVal::Null),
            (Some(json!(8)), ProviderVal::Num(8.0)),
            (Some(json!(8.0)), ProviderVal::Num(8.0)),
            (Some(json!("8")), ProviderVal::Other),
            (Some(json!(true)), ProviderVal::Other),
            (Some(json!([8])), ProviderVal::Other),
        ];
        for (v, expected) in cases {
            assert_eq!(ProviderVal::from_value(v.as_ref()), expected, "{v:?}");
        }
        assert!(ProviderVal::Num(8.0).is(8));
        assert!(!ProviderVal::Other.is(8));
        assert!(ProviderVal::Num(8.0).counts_global());
        assert!(ProviderVal::Missing.counts_global());
        assert!(!ProviderVal::Null.counts_global());
        assert!(!ProviderVal::Num(1.0).counts_global());
        assert_eq!(ProviderVal::Missing.chat_branch(), Some(0));
        assert_eq!(ProviderVal::Null.chat_branch(), Some(0));
        assert_eq!(ProviderVal::Num(4.0).chat_branch(), Some(4));
        assert_eq!(ProviderVal::Num(-0.0).chat_branch(), Some(0));
        assert_eq!(ProviderVal::Num(8.5).chat_branch(), None);
        assert_eq!(ProviderVal::Num(9.0).chat_branch(), None);
        assert_eq!(ProviderVal::Other.chat_branch(), None);
        assert_eq!(ProviderVal::Num(6.0).log_value(), 6);
        assert_eq!(ProviderVal::Null.log_value(), 0);
    }

    #[test]
    fn trim_helpers_follow_optional_chaining() {
        let p = payload(json!({
            "messages": [], "model": "  gpt-4o\u{3000}", "empty": "   ", "num": 5, "nul": null
        }));
        assert_eq!(
            p.opt_trim("model", "payload.model?.trim"),
            Ok(Some("gpt-4o".to_owned()))
        );
        assert_eq!(p.opt_trim("missing", "x"), Ok(None));
        assert_eq!(p.opt_trim("nul", "x"), Ok(None));
        assert_eq!(p.trim_or_empty("missing", "x"), Ok(String::new()));
        assert_eq!(p.trim_or("empty", "x", "dflt"), Ok("dflt".to_owned()));
        assert_eq!(p.trim_or("model", "x", "dflt"), Ok("gpt-4o".to_owned()));
        assert_eq!(
            p.opt_trim("num", "payload.model?.trim"),
            Err(Thrown("payload.model?.trim is not a function".to_owned()))
        );
    }

    #[test]
    fn api_key_truthiness() {
        for (v, expected) in [
            (json!("k"), true),
            (json!("  "), true),
            (json!(""), false),
            (json!(0), false),
            (json!(false), false),
            (json!(null), false),
            (json!(12), true),
        ] {
            let p = payload(json!({ "messages": [], "apiKey": v }));
            assert_eq!(p.has_api_key(), expected, "{v:?}");
        }
        assert!(!payload(json!({ "messages": [] })).has_api_key());
        assert_eq!(
            payload(json!({ "messages": [], "apiKey": 12 })).api_key_text(),
            "12"
        );
    }

    #[test]
    fn question_is_last_user_message() {
        let p = payload(json!({ "messages": [
            {"role": "system", "content": "S"},
            {"role": "user", "content": "first"},
            {"role": "assistant", "content": "A"},
            {"role": "user", "content": "second"},
            "primitive",
        ]}));
        assert_eq!(p.question(), Ok(json!("second")));
        let empty = payload(json!({ "messages": [{"role": "user", "content": ""}] }));
        assert_eq!(empty.question(), Ok(json!("")));
        let none = payload(json!({ "messages": [{"role": "assistant", "content": "x"}] }));
        assert_eq!(none.question(), Ok(json!("")));
        let null_entry = payload(json!({ "messages": [{"role": "user", "content": "x"}, null] }));
        assert_eq!(
            null_entry.question(),
            Err(Thrown::new(
                "Cannot read properties of null (reading 'role')"
            ))
        );
        let numeric = payload(json!({ "messages": [{"role": "user", "content": 5}] }));
        assert_eq!(numeric.question(), Ok(json!(5)));
    }

    #[test]
    fn parse_int_matches_js() {
        assert_eq!(js_parse_int("65000"), Some(65000));
        assert_eq!(js_parse_int("65000\n"), Some(65000));
        assert_eq!(js_parse_int("  42abc"), Some(42));
        assert_eq!(js_parse_int("-5"), Some(-5));
        assert_eq!(js_parse_int("+7"), Some(7));
        assert_eq!(js_parse_int("\u{FEFF}9"), Some(9));
        assert_eq!(js_parse_int("abc"), None);
        assert_eq!(js_parse_int(""), None);
        assert_eq!(js_parse_int("-"), None);
        assert_eq!(js_parse_int("0x10"), Some(0));
        assert_eq!(js_parse_int("99999999999999999999999"), Some(i64::MAX));
    }

    #[test]
    fn json_navigation_helpers() {
        let v =
            json!({"choices": [{"message": {"content": "x"}}], "obj": {"0": {"a": 1}}, "n": null});
        assert_eq!(
            prop(index0(prop(Some(&v), "choices")), "message"),
            Some(&json!({"content": "x"}))
        );
        assert_eq!(prop(index0(prop(Some(&v), "obj")), "a"), Some(&json!(1)));
        assert_eq!(index0(Some(&json!("str"))), None);
        assert_eq!(non_nullish(prop(Some(&v), "n")), None);
        assert_eq!(value_text(&json!("s")), "s");
        assert_eq!(value_text(&json!(5)), "5");
    }
}
