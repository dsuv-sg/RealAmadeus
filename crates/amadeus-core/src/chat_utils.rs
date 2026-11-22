//! Port of V1 `lib/amadeus/chat-utils.ts` plus the pure chat-text rules that live in
//! `RealAmadeusApp.tsx` (`splitMessage`, TTS chunking, the streaming emotion detector, backlog
//! cleaning, the API-error message table and the `/dev` commands).
//!
//! JS semantics reproduced exactly: `trim()`/`\s` use the ECMAScript whitespace set
//! ([`crate::jscompat::js_is_whitespace`]); the emotion-tag and `<think>` regexes are
//! case-insensitive for ASCII only (JS `/i` without `u`), so `[ſmile]` is not a tag;
//! lengths that V1 measures in UTF-16 units (`length > 2`, the 120-unit TTS cap, stream
//! offsets) are measured in UTF-16 units here too.
//!
//! Indices: V1 works with UTF-16 offsets; every delimiter searched here is ASCII or BMP, so
//! byte offsets give identical results. [`find_memory_command_end`] takes and returns byte
//! offsets.

use serde_json::Value;

use crate::jscompat::{js_is_whitespace, js_trim, js_trim_start, utf16_len};
use crate::settings::jsnum::js_value_to_string_checked;
use crate::types::{ChatMemoryExtraction, Emotion, MemoryEvent};

/// The 11 emotion tag names in canonical V1 order (`[NORMAL] [SMILE] …`).
pub const EMOTION_TAGS: [&str; 11] = [
    "NORMAL",
    "SMILE",
    "ANGRY",
    "SAD",
    "SURPRISED",
    "BLUSH",
    "WINK",
    "DISGUST",
    "SMUG",
    "THINKING",
    "PANIC",
];

/// Text shown when a reply is empty after cleaning (two U+2026).
pub const EMPTY_REPLY_TEXT: &str = "……";

// ---------------------------------------------------------------------------------------------
// Emotion tags and <think> blocks
// ---------------------------------------------------------------------------------------------

/// Finds the next `[TAG]` (ASCII-case-insensitive, no inner whitespace) at or after byte `from`.
/// Returns `(start, end_exclusive, emotion)`.
fn find_emotion_tag(text: &str, from: usize) -> Option<(usize, usize, Emotion)> {
    let bytes = text.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        let start = i + bytes[i..].iter().position(|&b| b == b'[')?;
        let name_start = start + 1;
        let mut j = name_start;
        while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b']' {
            if let Some(emotion) = Emotion::from_name_ignore_ascii_case(&text[name_start..j]) {
                return Some((start, j + 1, emotion));
            }
        }
        i = start + 1;
    }
    None
}

/// `text.replace(EMOTION_TAG_PATTERN, "")` — removes every `[TAG]` (global, ASCII
/// case-insensitive) WITHOUT trimming or collapsing the surrounding spaces.
pub fn strip_emotion_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut from = 0;
    while let Some((start, end, _)) = find_emotion_tag(text, from) {
        out.push_str(&text[last..start]);
        last = end;
        from = end;
    }
    out.push_str(&text[last..]);
    out
}

/// Byte offset of the first ASCII-case-insensitive occurrence of `needle` (ASCII) at or after
/// `from`.
fn find_ascii_ci(haystack: &str, needle: &[u8], from: usize) -> Option<usize> {
    let hay = haystack.as_bytes();
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    (from..=hay.len() - needle.len())
        .find(|&i| hay[i..i + needle.len()].eq_ignore_ascii_case(needle))
}

/// `text.replace(/<think>[\s\S]*?<\/think>/gi, "")` — removes reasoning blocks (non-greedy,
/// spanning newlines, ASCII case-insensitive). An unclosed `<think>` is kept. No trimming.
pub fn strip_think_blocks(text: &str) -> String {
    const OPEN: &[u8] = b"<think>";
    const CLOSE: &[u8] = b"</think>";
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    while let Some(open) = find_ascii_ci(text, OPEN, last) {
        let Some(close) = find_ascii_ci(text, CLOSE, open + OPEN.len()) else {
            break;
        };
        out.push_str(&text[last..open]);
        last = close + CLOSE.len();
    }
    out.push_str(&text[last..]);
    out
}

/// V1 `stripEmotionTag(rawText)`: removes `<think>` blocks, trims, removes **every** emotion
/// tag anywhere, trims again. Inner spaces stay (`"前置き [WINK] 後ろ"` → `"前置き  後ろ"`).
pub fn strip_emotion_tag(raw_text: &str) -> String {
    let without_think = strip_think_blocks(raw_text);
    js_trim(&strip_emotion_tags(js_trim(&without_think))).to_owned()
}

/// V1 `parseEmotionTag(rawText)` as an [`Emotion`]: the FIRST tag anywhere (case-insensitive,
/// `<think>` blocks are NOT skipped), [`Emotion::Normal`] when there is none.
pub fn parse_emotion(raw_text: &str) -> Emotion {
    find_emotion_tag(raw_text, 0)
        .map(|(_, _, e)| e)
        .unwrap_or_default()
}

/// V1 `parseEmotionTag(rawText)`: the first tag's upper-case name, or `"NORMAL"`.
pub fn parse_emotion_tag(raw_text: &str) -> &'static str {
    parse_emotion(raw_text).as_str()
}

// ---------------------------------------------------------------------------------------------
// User name from a fact
// ---------------------------------------------------------------------------------------------

fn is_name_terminator(c: char) -> bool {
    matches!(
        c,
        '」' | '』' | '"' | '\'' | '。' | '、' | '.' | '!' | '?' | '！' | '？'
    ) || js_is_whitespace(c)
}

/// Matches `\s*[「『"']?([^」』"'。、.!?！？\s]+)` (with JS backtracking) at the start of `rest`.
fn match_name_after_marker(rest: &str) -> Option<&str> {
    let rest = js_trim_start(rest);
    let first = rest.chars().next()?;
    let run_len = |s: &str| {
        s.char_indices()
            .find(|&(_, c)| is_name_terminator(c))
            .map_or(s.len(), |(i, _)| i)
    };
    if matches!(first, '「' | '『' | '"' | '\'') {
        let after_quote = &rest[first.len_utf8()..];
        let n = run_len(after_quote);
        if n > 0 {
            return Some(&after_quote[..n]);
        }
        // Backtrack: the optional quote matches nothing; an opening bracket is itself a valid
        // name character (followed by a terminator here, so the capture is just the bracket).
        if matches!(first, '「' | '『') {
            return Some(&rest[..first.len_utf8()]);
        }
        return None;
    }
    let n = run_len(rest);
    (n > 0).then(|| &rest[..n])
}

/// V1 `extractUserNameFromFact(fact)`: the first match of
/// `(?:ユーザー\s*の\s*)?名前は\s*[「『"']?([^」』"'。、.!?！？\s]+)` (not anchored), trimmed,
/// or `""`. Known V1 false positives are kept (`"猫の名前はタマ"` → `"タマ"`).
pub fn extract_user_name_from_fact(fact: &str) -> String {
    const MARKER: &str = "名前は";
    let value = js_trim(fact);
    if value.is_empty() {
        return String::new();
    }
    let mut from = 0;
    while let Some(offset) = value[from..].find(MARKER) {
        let after = from + offset + MARKER.len();
        if let Some(name) = match_name_after_marker(&value[after..]) {
            return js_trim(name).to_owned();
        }
        from = after;
    }
    String::new()
}

// ---------------------------------------------------------------------------------------------
// Memory commands
// ---------------------------------------------------------------------------------------------

/// V1 `findMemoryCommandEnd(text, payloadStart)`: byte offset of the first `]` at or after
/// `payload_start` that is outside JSON strings (ASCII `"` toggles, `\` escapes inside them)
/// and outside `{}` / `[]` nesting; `None` (V1: -1) when there is none.
pub fn find_memory_command_end(text: &str, payload_start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut in_string = false;
    let mut escaping = false;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, &ch) in bytes.iter().enumerate().skip(payload_start) {
        if escaping {
            escaping = false;
            continue;
        }
        if in_string && ch == b'\\' {
            escaping = true;
            continue;
        }
        if ch == b'"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            b'{' => brace_depth += 1,
            b'}' if brace_depth > 0 => brace_depth -= 1,
            b'[' => bracket_depth += 1,
            b']' => {
                if brace_depth == 0 && bracket_depth == 0 {
                    return Some(index);
                }
                // A `]` inside `{…}` with no open `[` is ignored.
                bracket_depth = bracket_depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    None
}

/// Result of looking up a JS property followed by `?.trim()`.
enum TrimLookup {
    /// Missing, `null`, or a string that trims to `""` (falsy).
    Falsy,
    /// A string with non-empty trim.
    Text(String),
    /// A non-string value: `x.trim` is not a function → `TypeError` in V1.
    TypeError,
}

fn trim_lookup(obj: &serde_json::Map<String, Value>, field: &str) -> TrimLookup {
    match obj.get(field) {
        None | Some(Value::Null) => TrimLookup::Falsy,
        Some(Value::String(s)) => {
            let t = js_trim(s);
            if t.is_empty() {
                TrimLookup::Falsy
            } else {
                TrimLookup::Text(t.to_owned())
            }
        }
        Some(_) => TrimLookup::TypeError,
    }
}

/// V1 `parseSaveFactPayload(payload)` (private in V1): a JSON object payload yields the first
/// non-blank string among `content`, `fact`, `text`; a non-string value in that order
/// (a `TypeError` in V1) or a parse error yields the raw trimmed payload. JSON that
/// serde_json rejects but `JSON.parse` accepts counts as a parse error (see
/// [`parse_save_event_payload`]).
pub fn parse_save_fact_payload(payload: &str) -> String {
    let trimmed = js_trim(payload);
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('{') {
        if let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(trimmed) {
            for field in ["content", "fact", "text"] {
                match trim_lookup(&obj, field) {
                    TrimLookup::Falsy => continue,
                    TrimLookup::Text(t) => return t,
                    TrimLookup::TypeError => break,
                }
            }
        }
    }
    trimmed.to_owned()
}

/// V1 `parseSaveEventPayload(payload)` (private in V1). For a JSON object: summary from
/// `summary`, else `content` (non-blank strings); if neither, the summary stays the raw JSON
/// text. Tags from an array (`String(tag).trim()`) or a string split on `,` `|` `、`;
/// exact-match de-duplication, empties dropped. A non-string `summary`/`content` (V1
/// `TypeError`) aborts everything: raw summary, no tags. A tag whose `String(tag)` throws
/// (an object with an own `toString` key) ends the tag list; the summary and the earlier
/// tags are kept, like V1's `catch`.
///
/// Numbers are parsed with correct rounding, like `JSON.parse`. Known deviation: JSON that
/// `JSON.parse` accepts but serde_json rejects counts as malformed here (raw summary, no
/// tags). That covers numbers beyond the `f64` range (`1e400` → tag `"Infinity"` in V1),
/// lone-surrogate escapes and nesting deeper than 128 levels.
pub fn parse_save_event_payload(payload: &str) -> MemoryEvent {
    let mut summary = js_trim(payload).to_owned();
    let mut tags: Vec<String> = Vec::new();
    if summary.is_empty() {
        return MemoryEvent::default();
    }
    if summary.starts_with('{') {
        if let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(&summary) {
            let chosen = match trim_lookup(&obj, "summary") {
                TrimLookup::Text(t) => Ok(Some(t)),
                TrimLookup::TypeError => Err(()),
                TrimLookup::Falsy => match trim_lookup(&obj, "content") {
                    TrimLookup::Text(t) => Ok(Some(t)),
                    TrimLookup::TypeError => Err(()),
                    TrimLookup::Falsy => Ok(None),
                },
            };
            if let Ok(chosen) = chosen {
                if let Some(text) = chosen {
                    summary = text;
                }
                let mut push_tag = |tag: &str| {
                    let cleaned = js_trim(tag);
                    if !cleaned.is_empty() && !tags.iter().any(|t| t == cleaned) {
                        tags.push(cleaned.to_owned());
                    }
                };
                match obj.get("tags") {
                    Some(Value::Array(items)) => {
                        for item in items {
                            let Some(tag) = js_value_to_string_checked(item) else {
                                break;
                            };
                            push_tag(&tag);
                        }
                    }
                    Some(Value::String(s)) => {
                        for part in s.split([',', '|', '、']) {
                            push_tag(part);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    MemoryEvent { summary, tags }
}

/// `s.replace(/\n{3,}/g, "\n\n")`.
fn collapse_blank_lines(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut run = 0usize;
    for c in s.chars() {
        if c == '\n' {
            run += 1;
            if run <= 2 {
                out.push('\n');
            }
        } else {
            run = 0;
            out.push(c);
        }
    }
    out
}

/// The literal start of a memory command (case-sensitive).
const MEMORY_COMMAND_OPEN: &str = "[SAVE_";
/// The two command names a normalised header must equal.
const MEMORY_COMMAND_NAMES: [&str; 2] = ["SAVE_FACT", "SAVE_EVENT"];

/// `extractMemoryCommands` header normalisation: the text between `[` and the first `:` with
/// every JS whitespace removed, upper-cased (`"SAVE_ fact "` → `"SAVE_FACT"`).
fn memory_command_header(text: &str) -> String {
    text.chars()
        .filter(|c| !js_is_whitespace(*c))
        .collect::<String>()
        .to_uppercase()
}

/// V1 `extractMemoryCommands(rawText)`: finds `[SAVE_FACT: …]` / `[SAVE_EVENT: …]` commands
/// (literal case-sensitive `[SAVE_`, first ASCII `:` after it, header whitespace removed and
/// upper-cased), parses the payloads, removes every recognised command (even with an empty
/// payload) and collapses runs of 3+ `\n` to two. An unterminated command stops the scan and
/// stays in the text.
pub fn extract_memory_commands(raw_text: &str) -> ChatMemoryExtraction {
    let text = js_trim(raw_text);
    let mut result = ChatMemoryExtraction {
        cleaned_text: text.to_owned(),
        ..Default::default()
    };
    if text.is_empty() {
        return result;
    }
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut index = 0;
    while index < text.len() {
        let Some(start) = text[index..].find(MEMORY_COMMAND_OPEN).map(|o| o + index) else {
            break;
        };
        let Some(colon) = text[start..].find(':').map(|o| o + start) else {
            break;
        };
        let header = memory_command_header(&text[start + 1..colon]);
        let is_fact = match header.as_str() {
            "SAVE_FACT" => true,
            "SAVE_EVENT" => false,
            _ => {
                index = start + MEMORY_COMMAND_OPEN.len();
                continue;
            }
        };
        let Some(end) = find_memory_command_end(text, colon + 1) else {
            break;
        };
        let payload = js_trim(&text[colon + 1..end]);
        if is_fact {
            let fact = parse_save_fact_payload(payload);
            if !fact.is_empty() && !result.facts.contains(&fact) {
                result.facts.push(fact);
            }
        } else {
            let event = parse_save_event_payload(payload);
            if !event.summary.is_empty() {
                result.events.push(event);
            }
        }
        ranges.push((start, end + 1));
        index = end + 1;
    }
    let mut cleaned = String::with_capacity(text.len());
    let mut last = 0;
    for (start, end) in ranges {
        cleaned.push_str(&text[last..start]);
        last = end;
    }
    cleaned.push_str(&text[last..]);
    result.cleaned_text = js_trim(&collapse_blank_lines(&cleaned)).to_owned();
    result
}

// ---------------------------------------------------------------------------------------------
// Pacing and message splitting
// ---------------------------------------------------------------------------------------------

/// V1 `isPauseCharacter(char, nextChar)` (page-break rule); `next == None` is the end of text.
/// `.` pauses only before space/newline/CR/end (never before another `.`); `。！？!?\n` pause
/// unless followed by `」 ） ) 』 ”`.
pub fn is_pause_character(c: char, next: Option<char>) -> bool {
    if c == '.' {
        return matches!(next, None | Some(' ' | '\n' | '\r'));
    }
    if !matches!(c, '。' | '！' | '？' | '!' | '?' | '\n') {
        return false;
    }
    !matches!(next, Some('」' | '）' | ')' | '』' | '”'))
}

/// V1 `charDelayForChar(char, textSpeed)` in ms (fractional): base `50 / max(0.1, speed)`,
/// ×2.5 after `、 , …`, ×1.5 after `」 ）`. A NaN speed yields NaN like `Math.max`.
pub fn char_delay_for_char(c: char, text_speed: f64) -> f64 {
    let speed = if text_speed.is_nan() {
        f64::NAN
    } else {
        text_speed.max(0.1)
    };
    let base = 1000.0 / 20.0 / speed;
    match c {
        '、' | ',' | '…' => base * 2.5,
        '」' | '）' => base * 1.5,
        _ => base,
    }
}

/// V1 `splitMessage(text)` (`RealAmadeusApp.tsx` 207–240): splits a message into backlog pages
/// at pause characters. Only `' ' '\t' '\n' '\r'` are skipped at a page start; pages keep their
/// terminating pause character and trailing text untrimmed; blank pages are dropped.
pub fn split_message(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = Vec::new();
    let (mut start, mut i) = (0usize, 0usize);
    while i < n {
        if i == start {
            while i < n && matches!(chars[i], ' ' | '\t' | '\n' | '\r') {
                i += 1;
            }
            start = i;
        }
        if i >= n {
            break;
        }
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        i += 1;
        if is_pause_character(c, next) || i == n {
            let segment: String = chars[start..i].iter().collect();
            if !js_trim(&segment).is_empty() {
                out.push(segment);
            }
            start = i;
        }
    }
    out
}

/// Characters that end a TTS chunk.
fn is_tts_boundary(c: char) -> bool {
    matches!(c, '。' | '！' | '？' | '!' | '?' | '\n')
}

/// Closing quotes/brackets the streamed TTS chunker keeps with the preceding chunk.
fn is_tts_closer(c: char) -> bool {
    matches!(c, '」' | '』' | '”' | '"' | ')' | '）')
}

/// Maximum TTS chunk length in UTF-16 units (non-streamed replies).
pub const TTS_CHUNK_MAX_UTF16: usize = 120;

/// V1 `splitTextForTTS(text)` (non-streamed replies): JS-whitespace runs → one space, trim,
/// then cut after `。！？!?` (and `\n`, impossible after normalisation) or when the chunk
/// reaches 120 UTF-16 units; chunks are trimmed, blanks dropped.
pub fn split_text_for_tts(text: &str) -> Vec<String> {
    let mut normalized = String::with_capacity(text.len());
    let mut in_space = false;
    for c in text.chars() {
        if js_is_whitespace(c) {
            if !in_space {
                normalized.push(' ');
                in_space = true;
            }
        } else {
            normalized.push(c);
            in_space = false;
        }
    }
    let normalized = js_trim(&normalized);
    let mut chunks = Vec::new();
    if normalized.is_empty() {
        return chunks;
    }
    let mut current = String::new();
    let mut current_units = 0usize;
    for c in normalized.chars() {
        current.push(c);
        current_units += c.len_utf16();
        if is_tts_boundary(c) || current_units >= TTS_CHUNK_MAX_UTF16 {
            let chunk = js_trim(&current);
            if !chunk.is_empty() {
                chunks.push(chunk.to_owned());
            }
            current.clear();
            current_units = 0;
        }
    }
    let tail = js_trim(&current);
    if !tail.is_empty() {
        chunks.push(tail.to_owned());
    }
    chunks
}

/// Byte offset of UTF-16 offset `units` in `s`, rounded UP to a char boundary (V1 would split
/// a surrogate pair; V2 drops the half, see [`TtsStreamCapture::capture`]).
fn utf16_offset_to_byte_ceil(s: &str, units: usize) -> usize {
    let mut acc = 0usize;
    for (i, c) in s.char_indices() {
        if acc >= units {
            return i;
        }
        acc += c.len_utf16();
    }
    s.len()
}

/// Streamed-reply TTS chunker: V1 `captureStreamTextForTTS` + `flushTTSChunkBuffer`
/// (`ttsProcessedIndexRef` / `ttsChunkBufferRef`). Feed it the current display text after
/// every token; it returns the chunks to enqueue. Call it only while TTS is enabled (V1 returns
/// before touching its state otherwise) and [`reset`](Self::reset) it when a new reply starts or
/// TTS is cancelled.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TtsStreamCapture {
    buffer: String,
    processed_utf16: usize,
}

impl TtsStreamCapture {
    /// Empty capture state.
    pub fn new() -> Self {
        Self::default()
    }

    /// V1 `prepareLiveTTSSession` / `cancelLiveTTS`: clears the buffer and the processed index.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.processed_utf16 = 0;
    }

    /// Text received but not yet emitted as a chunk.
    pub fn pending(&self) -> &str {
        &self.buffer
    }

    /// V1 `captureStreamTextForTTS(sourceText, complete)`: appends the part of `source_text`
    /// after the previously seen length (emotion tags removed from the delta), then flushes
    /// complete sentences (and everything when `complete`). If there is no new text NOTHING
    /// is flushed, even with `complete == true` (V1 behaviour).
    ///
    /// Lengths are UTF-16 units like V1. When the previously seen length ends inside a
    /// surrogate pair (the display text changed under it), V1's delta starts with the lone
    /// low surrogate. Rust strings cannot hold it, so that half is dropped. Only the half is
    /// dropped: the delta still counts as new text, the index still advances and the flush
    /// still runs, even if nothing else was appended.
    pub fn capture(&mut self, source_text: &str, complete: bool) -> Vec<String> {
        let total = utf16_len(source_text);
        if self.processed_utf16 > total {
            self.processed_utf16 = total;
        }
        // V1 `if (!delta) return;`, with the delta measured in UTF-16 units.
        if self.processed_utf16 == total {
            return Vec::new();
        }
        let start = utf16_offset_to_byte_ceil(source_text, self.processed_utf16);
        self.processed_utf16 = total;
        self.buffer
            .push_str(&strip_emotion_tags(&source_text[start..]));
        self.flush(complete)
    }

    /// V1 `flushTTSChunkBuffer(forceFlush)`: emits every complete sentence (boundary
    /// `。！？!?\n` plus any following `」』”")）`), trimming each chunk and the rest's start;
    /// with `force` the trimmed remainder is emitted too.
    pub fn flush(&mut self, force: bool) -> Vec<String> {
        let mut out = Vec::new();
        while let Some((i, c)) = self
            .buffer
            .char_indices()
            .find(|&(_, c)| is_tts_boundary(c))
        {
            let mut boundary = i + c.len_utf8();
            for next in self.buffer[boundary..].chars() {
                if is_tts_closer(next) {
                    boundary += next.len_utf8();
                } else {
                    break;
                }
            }
            let chunk = js_trim(&self.buffer[..boundary]).to_owned();
            self.buffer = js_trim_start(&self.buffer[boundary..]).to_owned();
            if !chunk.is_empty() {
                out.push(chunk);
            }
        }
        if force {
            let tail = js_trim(&self.buffer).to_owned();
            self.buffer.clear();
            if !tail.is_empty() {
                out.push(tail);
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------------------------
// Streaming emotion detection
// ---------------------------------------------------------------------------------------------

/// Result of [`StreamEmotionDetector::push_token`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamTokenUpdate {
    /// Leading tag detected by this token (V1 then calls `setCurrentEmotionTag` and
    /// `MemoryManager.recordEmotion`).
    pub emotion: Option<Emotion>,
    /// Text to show (`stripEmotionTag(raw)`), replacing the previous display text.
    pub display: String,
}

/// V1 `handleStreamToken` tag detection over the accumulated stream (`streamRawBufferRef` +
/// `streamEmotionParsedRef`).
///
/// On every token: `raw = trim(strip_think_blocks(buffer))`; while not yet parsed, a leading
/// `[...]` whose JS-trimmed, Unicode-upper-cased content is one of the 11 tags is consumed
/// (inner spaces tolerated, unlike the regexes) and the buffer is REPLACED by the rest
/// (think blocks already removed); otherwise detection ends once `raw` is longer than 2 UTF-16
/// units and does not start with `<`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamEmotionDetector {
    buffer: String,
    parsed: bool,
}

impl StreamEmotionDetector {
    /// Fresh detector (V1 resets both refs when a request starts and after completion).
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears the buffer and the parsed flag.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.parsed = false;
    }

    /// True once the leading tag (or its absence) has been decided.
    pub fn is_parsed(&self) -> bool {
        self.parsed
    }

    /// Accumulated raw stream text (after a detected tag: the text following it).
    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    /// Processes one streamed token.
    pub fn push_token(&mut self, token: &str) -> StreamTokenUpdate {
        self.buffer.push_str(token);
        let mut raw = js_trim(&strip_think_blocks(&self.buffer)).to_owned();
        let mut emotion = None;
        if !self.parsed {
            if raw.starts_with('[') {
                if let Some(close) = raw.find(']') {
                    let maybe_tag = js_trim(&raw[1..close]).to_uppercase();
                    if let Some(found) = Emotion::from_name(&maybe_tag) {
                        emotion = Some(found);
                        raw = js_trim_start(&raw[close + 1..]).to_owned();
                        self.buffer = raw.clone();
                        self.parsed = true;
                    } else if utf16_len(&raw) > 2 && !raw.starts_with('<') {
                        self.parsed = true;
                    }
                }
            } else if utf16_len(&raw) > 2 && !raw.starts_with('<') {
                self.parsed = true;
            }
        }
        StreamTokenUpdate {
            emotion,
            display: strip_emotion_tag(&raw),
        }
    }

    /// V1 `handleStreamComplete` raw text: `fullResponse.trim() || streamBuffer.trim()`.
    pub fn completion_raw(&self, full_response: &str) -> String {
        let raw = js_trim(full_response);
        if raw.is_empty() {
            js_trim(&self.buffer).to_owned()
        } else {
            raw.to_owned()
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Streamed reply display (DECISIONS D23, CHAT-028)
// ---------------------------------------------------------------------------------------------

/// The part of a streamed reply's display that no later token can change (D23, CHAT-028).
///
/// `raw` is the concatenation of every token received so far. When the stream completes, the
/// reply is displayed as V1's `handleStreamComplete` computes it:
/// `strip_emotion_tag(extract_memory_commands(raw).cleaned_text)` (`……` when empty). The
/// result here is a prefix of that display for **every** possible continuation of `raw`, so
/// text typed from it never has to be rewritten. It applies the same rules in the same order
/// and holds back whatever the rest of the stream could still change:
///
/// 1. Memory commands ([`extract_memory_commands`]): complete `[SAVE_FACT: …]` /
///    `[SAVE_EVENT: …]` commands are dropped and blank-line runs collapsed. Everything from a
///    `[SAVE_` that can still turn out to be a command (no closing `]` yet, or no `:` yet while
///    the header can still become `SAVE_FACT` / `SAVE_EVENT`) is held back, and so is a
///    trailing `[`, `[S` … `[SAVE`. A `[SAVE_` whose header can no longer be a command name is
///    literal text.
/// 2. `<think>` blocks ([`strip_think_blocks`]): closed blocks are dropped anywhere. Everything
///    from an unclosed `<think>` (or a trailing `<`, `<t` … `<think`) is held back.
/// 3. Emotion tags ([`strip_emotion_tags`]): complete `[TAG]`s are dropped anywhere. A trailing
///    `[` followed only by letters that can still become a tag name is held back. Other
///    brackets, such as `[ PANIC ]` (not a tag for the regex), are text.
/// 4. The result is trimmed. Trailing whitespace is therefore held back until something
///    follows it.
///
/// Unlike V1's `stripEmotionTag(streamBuffer)` (the never-rendered `handleStreamToken`
/// display), the leading tag is not consumed with the detector's looser rules, so
/// `[ PANIC ] えっ` is shown as the completion shows it.
pub fn stream_display_prefix(raw: &str) -> String {
    let memory = stream_memory_prefix(raw);
    let think = stream_think_prefix(&memory);
    js_trim(&stream_tag_prefix(&think)).to_owned()
}

/// Start of the longest suffix of `text`, beginning at or after byte `from`, that is a proper
/// non-empty prefix of the ASCII `pattern` (`ignore_case`: ASCII case-insensitive), i.e. a
/// trailing start of `pattern` that the next token may complete.
fn partial_suffix_start(
    text: &str,
    from: usize,
    pattern: &[u8],
    ignore_case: bool,
) -> Option<usize> {
    let bytes = text.as_bytes();
    let low = bytes.len().saturating_sub(pattern.len() - 1).max(from);
    (low..bytes.len()).find(|&i| {
        let suffix = &bytes[i..];
        let head = &pattern[..suffix.len()];
        if ignore_case {
            suffix.eq_ignore_ascii_case(head)
        } else {
            suffix == head
        }
    })
}

/// Stage 1 of [`stream_display_prefix`]: the [`extract_memory_commands`] scan over the decided
/// part of `raw` (commands removed, blank lines collapsed; trimming is left to the caller).
fn stream_memory_prefix(raw: &str) -> String {
    let mut cleaned = String::with_capacity(raw.len());
    let mut last = 0;
    let mut index = 0;
    let cut = loop {
        let Some(start) = raw[index..].find(MEMORY_COMMAND_OPEN).map(|o| o + index) else {
            break partial_suffix_start(raw, index, MEMORY_COMMAND_OPEN.as_bytes(), false)
                .unwrap_or(raw.len());
        };
        let colon = raw[start..].find(':').map(|o| o + start);
        let header = memory_command_header(&raw[start + 1..colon.unwrap_or(raw.len())]);
        match colon {
            // The header grows until the first `:` arrives.
            None if MEMORY_COMMAND_NAMES
                .iter()
                .any(|name| name.starts_with(header.as_str())) =>
            {
                break start;
            }
            Some(colon) if MEMORY_COMMAND_NAMES.contains(&header.as_str()) => {
                match find_memory_command_end(raw, colon + 1) {
                    // `extract_memory_commands` stops at an unterminated command.
                    None => break start,
                    Some(end) => {
                        cleaned.push_str(&raw[last..start]);
                        last = end + 1;
                        index = end + 1;
                    }
                }
            }
            // Never a command, whatever follows. Without a `:` V1 stops the scan here instead,
            // which is the same: no command can follow a `[SAVE_` that has no `:` after it.
            _ => index = start + MEMORY_COMMAND_OPEN.len(),
        }
    };
    cleaned.push_str(&raw[last..cut]);
    collapse_blank_lines(&cleaned)
}

/// Stage 2 of [`stream_display_prefix`]: [`strip_think_blocks`] over the decided part of `text`.
fn stream_think_prefix(text: &str) -> String {
    const OPEN: &[u8] = b"<think>";
    const CLOSE: &[u8] = b"</think>";
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    loop {
        let Some(open) = find_ascii_ci(text, OPEN, last) else {
            let cut = partial_suffix_start(text, last, OPEN, true).unwrap_or(text.len());
            out.push_str(&text[last..cut]);
            return out;
        };
        out.push_str(&text[last..open]);
        let Some(close) = find_ascii_ci(text, CLOSE, open + OPEN.len()) else {
            // An unclosed block: dropped if it closes later, kept (and shown) if it never does.
            return out;
        };
        last = close + CLOSE.len();
    }
}

/// Stage 3 of [`stream_display_prefix`]: [`strip_emotion_tags`] without a trailing `[` whose
/// letters can still become one of the [`EMOTION_TAGS`] (ASCII case-insensitive, like the
/// regex). Only the last `[` can be undecided: any earlier one is followed by that `[`.
fn stream_tag_prefix(text: &str) -> String {
    let undecided = text.rfind('[').filter(|&open| {
        let name = &text.as_bytes()[open + 1..];
        name.iter().all(u8::is_ascii_alphabetic)
            && EMOTION_TAGS.iter().any(|tag| {
                tag.len() >= name.len() && tag.as_bytes()[..name.len()].eq_ignore_ascii_case(name)
            })
    });
    strip_emotion_tags(&text[..undecided.unwrap_or(text.len())])
}

// ---------------------------------------------------------------------------------------------
// Backlog, API errors, dev commands
// ---------------------------------------------------------------------------------------------

/// V1 `addBackLog` text cleaning: trim; if it starts with `[` and has a `]` after it, drop
/// everything up to that `]` (ANY bracketed prefix, not only tags) and trim; remove every
/// emotion tag; trim; `None` when empty.
pub fn clean_backlog_message(raw_message: &str) -> Option<String> {
    let mut clean = js_trim(raw_message);
    if clean.starts_with('[') {
        if let Some(close) = clean.find(']') {
            if close > 0 {
                clean = js_trim(&clean[close + 1..]);
            }
        }
    }
    let stripped = strip_emotion_tags(clean);
    let clean = js_trim(&stripped);
    (!clean.is_empty()).then(|| clean.to_owned())
}

/// Default message of [`api_error_message`].
pub const API_ERROR_DEFAULT_MESSAGE: &str = "何かエラーが起きたみたい……もう一度試してくれる？";

/// V1 `handleApiError` table: (case-sensitive substrings, Kurisu's message). First match wins.
pub const API_ERROR_MESSAGES: [(&[&str], &str); 11] = [
    (
        &["API Key", "API key", "設定されていません"],
        "……APIキーが設定されてないみたいよ。CONFIGから設定してちょうだい。",
    ),
    (
        &["401", "Unauthorized"],
        "APIキーが無効みたい……もう一度確認して設定し直してくれる？",
    ),
    (
        &["429", "rate limit", "RESOURCE_EXHAUSTED"],
        "リクエストが多すぎるみたい。少し待ってからもう一度試してくれない？",
    ),
    (
        &["insufficient_quota", "quota_project"],
        "Geminiの利用枠か課金設定に問題があるみたい。APIキーとプロジェクト設定を確認して。",
    ),
    (
        &["timeout", "Timeout"],
        "応答がタイムアウトしたわ……ネットワークの状態を確認してみて。",
    ),
    (
        &["Cannot connect", "Network", "ネットワーク"],
        "ネットワークに接続できないわ。インターネット接続を確認してちょうだい。",
    ),
    (
        &["403", "Forbidden"],
        "このAPIへのアクセスが拒否されたわ。権限を確認してみて。",
    ),
    (
        &["500", "502", "503"],
        "サーバー側でエラーが起きてるみたい。しばらくしてからもう一度試して。",
    ),
    (
        &["model", "Model"],
        "指定されたモデルが見つからないみたい。CONFIGからモデル名を確認して。",
    ),
    (
        &["JSON"],
        "サービスアカウントJSONキーの形式が正しいか確認して。",
    ),
    (
        &["Vertex", "アクセストークン"],
        "Vertexの認証情報を確認して。APIキーかサービスアカウントJSONキーが必要よ。",
    ),
];

/// Emotion V1 shows together with an API error message.
pub const API_ERROR_EMOTION: Emotion = Emotion::Angry;

/// V1 `handleApiError(error)`: maps an error string to Kurisu's (hard-coded Japanese) message.
pub fn api_error_message(error: &str) -> &'static str {
    API_ERROR_MESSAGES
        .iter()
        .find(|(needles, _)| needles.iter().any(|n| error.contains(n)))
        .map_or(API_ERROR_DEFAULT_MESSAGE, |(_, message)| message)
}

/// Desktop `handleApiError(error)`: matches Qt/Unity Vertex gcloud messages on desktop,
/// and keeps V1 web messages on the web.
pub fn api_error_message_for_platform(error: &str, is_desktop: bool) -> &'static str {
    if !is_desktop {
        return api_error_message(error);
    }
    // Rules 0..10 (everything before V1's web-only Vertex API-key/SA-JSON rule).
    if let Some((_, message)) = API_ERROR_MESSAGES[..API_ERROR_MESSAGES.len() - 1]
        .iter()
        .find(|(needles, _)| needles.iter().any(|n| error.contains(n)))
    {
        return message;
    }
    if error.contains("Project ID") || error.contains("Config_VertexProject") {
        return "Vertex AIのProject IDが設定されてないみたいよ。CONFIGから設定してちょうだい。";
    }
    if error.contains("gcloud") || error.contains("アクセストークン") || error.contains("Vertex")
    {
        return "Vertex AIの認証に失敗したわ。gcloudの設定を確認してみて。";
    }
    API_ERROR_DEFAULT_MESSAGE
}

/// Help text of `/dev emotion help` (verbatim V1).
pub const DEV_EMOTION_HELP_TEXT: &str = concat!(
    "【サポートされているモーションID一覧】\n",
    "・NORMAL (通常)\n",
    "・SMILE (笑顔)\n",
    "・ANGRY (怒り)\n",
    "・SAD (悲しみ)\n",
    "・SURPRISED (驚き)\n",
    "・BLUSH (照れ)\n",
    "・WINK (ウィンク)\n",
    "・DISGUST (嫌悪)\n",
    "・SMUG (どや顔)\n",
    "・THINKING (考え中)\n",
    "・PANIC (混乱)\n",
    "・SLEEPING (居眠り)\n",
    "・SNEEZE (クシャミ)\n\n",
    "※コマンド例: /dev emotion smile で対象のモーションに変更します。",
);

/// A `/dev …` chat command (V1 `submitMessage`, checked on the JS-trimmed input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevCommand {
    /// `/dev emotion help` — show [`DEV_EMOTION_HELP_TEXT`] (no achievement).
    EmotionHelp,
    /// `/dev emotion <x>` — set the Live2D emotion to `x` upper-cased, UNVALIDATED (so
    /// `SLEEPING`/`SNEEZE` work); `None` when the target is empty. Notifies `dev_command`.
    SetEmotion(Option<String>),
    /// `/dev motion reset` — emotion NORMAL; notifies `dev_command`.
    MotionReset,
    /// `/dev achievements reset` — reset all achievements, then notify `dev_command`.
    AchievementsReset,
    /// `/dev steiner` — notifies `secret_steiner`.
    Steiner,
}

/// Parses a chat input as a dev command (V1 order: exact help, `starts_with("/dev emotion ")`,
/// then the exact commands). `None` = normal chat message.
pub fn parse_dev_command(raw_input: &str) -> Option<DevCommand> {
    let text = js_trim(raw_input);
    if text == "/dev emotion help" {
        return Some(DevCommand::EmotionHelp);
    }
    if text.starts_with("/dev emotion ") {
        let target = text
            .split(' ')
            .nth(2)
            .map(|p| js_trim(p).to_uppercase())
            .filter(|t| !t.is_empty());
        return Some(DevCommand::SetEmotion(target));
    }
    match text {
        "/dev motion reset" => Some(DevCommand::MotionReset),
        "/dev achievements reset" => Some(DevCommand::AchievementsReset),
        "/dev steiner" => Some(DevCommand::Steiner),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(summary: &str, tags: &[&str]) -> MemoryEvent {
        MemoryEvent {
            summary: summary.into(),
            tags: tags.iter().map(|t| (*t).to_owned()).collect(),
        }
    }

    fn mem(cleaned: &str, facts: &[&str], events: Vec<MemoryEvent>) -> ChatMemoryExtraction {
        ChatMemoryExtraction {
            cleaned_text: cleaned.into(),
            facts: facts.iter().map(|f| (*f).to_owned()).collect(),
            events,
        }
    }

    /// Golden: spec Appendix G + extra V1 runs (Node 22).
    #[test]
    fn strip_emotion_tag_golden() {
        let cases = [
            ("[SMILE] こんにちは", "こんにちは"),
            (
                "<think>考え中[ANGRY]</think>\n[blush] べ、別に！ [SMILE]",
                "べ、別に！",
            ),
            ("[ SMILE ] hi", "[ SMILE ] hi"),
            ("[SLEEPING] zzz", "[SLEEPING] zzz"),
            ("<THINK>a</THINK>[Smile]ok", "ok"),
            ("<think>unclosed [SAD] text", "<think>unclosed  text"),
            ("前置き [WINK] 後ろ", "前置き  後ろ"),
            ("<think>a</think>b<think>c</think>[SAD]d", "bd"),
            ("<Think>\nmulti\nline\n</tHiNk> [Panic]x", "x"),
            ("[\u{17F}mile] long-s", "[\u{17F}mile] long-s"),
            ("[SMILE][SMILE]", ""),
            ("\u{FEFF}[SMILE] bom\u{FEFF}", "bom"),
            ("<think></think>", ""),
            ("[smile ] x", "[smile ] x"),
            ("<think>a<think>b</think>c</think>d", "c</think>d"),
            ("[[SMILE]x", "[x"),
        ];
        for (input, expected) in cases {
            assert_eq!(strip_emotion_tag(input), expected, "{input:?}");
        }
    }

    #[test]
    fn parse_emotion_tag_golden() {
        let cases = [
            ("[smile] hi", "SMILE"),
            ("hi [ANGRY] there [SAD]", "ANGRY"),
            ("<think>[ANGRY]</think>[SMILE] hi", "ANGRY"),
            ("no tag", "NORMAL"),
            ("[SLEEPING]", "NORMAL"),
            ("[ PANIC ]", "NORMAL"),
            ("<think>a</think>b<think>c</think>[SAD]d", "SAD"),
            ("<Think>\nmulti\nline\n</tHiNk> [Panic]x", "PANIC"),
            ("[\u{17F}mile] long-s", "NORMAL"),
            ("[SMILE][SMILE]", "SMILE"),
            ("[smile ] x", "NORMAL"),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_emotion_tag(input), expected, "{input:?}");
        }
        assert_eq!(parse_emotion("x [Thinking]"), Emotion::Thinking);
    }

    #[test]
    fn extract_user_name_golden() {
        let cases = [
            ("名前は岡部倫太郎", "岡部倫太郎"),
            ("ユーザーの名前は「オカリン」です", "オカリン"),
            ("名前は Okabe Rintaro", "Okabe"),
            ("猫の名前はタマ", "タマ"),
            ("名前はまだない。", "まだない"),
            ("My name is Okabe", ""),
            ("  ", ""),
            ("名前は『鳳凰院凶真』", "鳳凰院凶真"),
            ("名前は'Okarin'!", "Okarin"),
            ("ユーザーの名前は「」X", "「"),
            ("名前は」A", ""),
            ("名前は\u{3000}\u{3000}山田", "山田"),
            ("名前は「「X」", "「X"),
            ("名前は\"Bob\"", "Bob"),
            ("名前は'", ""),
            ("名前は「", "「"),
            ("私の名前は佐藤です。", "佐藤です"),
            ("ユーザー の 名前は 鈴木！", "鈴木"),
            ("名前は\u{0085}Z", "\u{0085}Z"),
            ("名前は\u{FEFF}Q", "Q"),
            ("名前は」 名前はB", "B"),
        ];
        for (input, expected) in cases {
            assert_eq!(extract_user_name_from_fact(input), expected, "{input:?}");
        }
    }

    #[test]
    fn find_memory_command_end_golden() {
        let cases: [(&str, usize, Option<usize>); 12] = [
            ("abc]", 0, Some(3)),
            ("{\"a\":\"]\"}] x", 0, Some(9)),
            ("[x]]", 0, Some(3)),
            ("\"\\\"]\"]", 0, Some(5)),
            ("{]}]", 0, Some(3)),
            ("no end", 0, None),
            ("a\\]b]", 0, Some(2)),
            ("\"a\\\"]\"]", 0, Some(6)),
            ("{{]}}]", 0, Some(5)),
            ("[[]]]", 0, Some(4)),
            ("}]", 0, Some(1)),
            ("xx]", 2, Some(2)),
        ];
        for (text, start, expected) in cases {
            assert_eq!(find_memory_command_end(text, start), expected, "{text:?}");
        }
    }

    /// Golden: spec Appendix G (19 cases) + extra V1 runs.
    #[test]
    fn extract_memory_commands_golden() {
        let cases: Vec<(&str, ChatMemoryExtraction)> = vec![
            ("[SMILE] 覚えておくわ。[SAVE_FACT: ユーザーの名前は岡部]", mem("[SMILE] 覚えておくわ。", &["ユーザーの名前は岡部"], vec![])),
            ("[SAVE_FACT: {\"content\": \"Dr Pepperが好き\"}] ok", mem("ok", &["Dr Pepperが好き"], vec![])),
            (
                "[SAVE_EVENT: {\"summary\": \"一緒に実験した\", \"tags\": \"実験、ラボ|楽しい, 実験\"}]\n\n\n\nまた明日",
                mem("また明日", &[], vec![ev("一緒に実験した", &["実験", "ラボ", "楽しい"])]),
            ),
            ("[SAVE_FACT: abc", mem("[SAVE_FACT: abc", &[], vec![])),
            ("[SAVE_FACT: 彼は\"天才] 続き", mem("[SAVE_FACT: 彼は\"天才] 続き", &[], vec![])),
            ("[SAVE_MOOD: happy] [SAVE_FACT: x]", mem("[SAVE_MOOD: happy]", &["x"], vec![])),
            ("[SAVE_ fact : y]", mem("", &["y"], vec![])),
            ("[SAVE_FACT: a][SAVE_FACT: a]", mem("", &["a"], vec![])),
            (
                "[SAVE_EVENT: {\"summary\": \"配列[1]を説明\", \"tags\": [\"a]\", \"b\"]}] done",
                mem("done", &[], vec![ev("配列[1]を説明", &["a]", "b"])]),
            ),
            ("[SAVE_FACT: {\"foo\": \"bar\"}]", mem("", &["{\"foo\": \"bar\"}"], vec![])),
            ("[SAVE_FACT: {\"content\": 5}]", mem("", &["{\"content\": 5}"], vec![])),
            ("[SAVE_FACT：名前はX] 後で 12:30", mem("[SAVE_FACT：名前はX] 後で 12:30", &[], vec![])),
            ("[SAVE_FACT: 好きな数字は[7]] ok", mem("ok", &["好きな数字は[7]"], vec![])),
            (
                "[SAVE_EVENT: {\"content\": \"ゲームした\", \"tags\": [\"遊び\", \" 遊び \", 3, null, \"\"]}]",
                mem("", &[], vec![ev("ゲームした", &["遊び", "3", "null"])]),
            ),
            ("[SAVE_FACT: ] hi", mem("hi", &[], vec![])),
            ("[SAVE_EVENT: {\"summary\": \"  \"}]", mem("", &[], vec![ev("{\"summary\": \"  \"}", &[])])),
            ("[save_fact: lower] [SAVE_EVENT: 映画を見た] end", mem("[save_fact: lower]  end", &[], vec![ev("映画を見た", &[])])),
            ("Hello [SAVE_FACT: x] world", mem("Hello  world", &["x"], vec![])),
            ("[SAVE_EVENT: {broken json] tail", mem("[SAVE_EVENT: {broken json] tail", &[], vec![])),
            ("[SAVE_FACT: a]\r\n\r\n\r\n\r\nb", mem("b", &["a"], vec![])),
            ("x\n\n\n\n[SAVE_FACT: y]\n\n\n\nz", mem("x\n\nz", &["y"], vec![])),
            ("[SAVE_EVENT: {\"summary\": \"s\", \"tags\": \"a，b、c|d\"}]", mem("", &[], vec![ev("s", &["a，b", "c", "d"])])),
            (
                "[SAVE_EVENT: {\"summary\": 5, \"tags\": [\"t\"]}]",
                mem("", &[], vec![ev("{\"summary\": 5, \"tags\": [\"t\"]}", &[])]),
            ),
            ("[SAVE_EVENT: {\"content\": \"c\", \"tags\": {\"a\":1}}]", mem("", &[], vec![ev("c", &[])])),
            ("[SAVE_EVENT: {\"tags\": [\"t\"]}]", mem("", &[], vec![ev("{\"tags\": [\"t\"]}", &["t"])])),
            ("[SAVE_FACT: {\"content\": null, \"fact\": \"F\"}]", mem("", &["F"], vec![])),
            ("[SAVE_FACT: {\"content\": \"  \", \"fact\": 3}]", mem("", &["{\"content\": \"  \", \"fact\": 3}"], vec![])),
            (
                "[SAVE_EVENT: {\"summary\": \"x\", \"tags\": [[1,2],{\"k\":1},true,1.5,\"\u{3000}y\u{3000}\"]}]",
                mem("", &[], vec![ev("x", &["1,2", "[object Object]", "true", "1.5", "y"])]),
            ),
            ("[SAVE_ fact\t: z]", mem("", &["z"], vec![])),
            ("😀[SAVE_FACT: 絵文字😀]😀", mem("😀😀", &["絵文字😀"], vec![])),
            ("[SAVE_FACTS: q] [SAVE_FACT: r]", mem("[SAVE_FACTS: q]", &["r"], vec![])),
            ("[SAVE_EVENT: {\"summary\": \"a\\\"b]\"}] t", mem("t", &[], vec![ev("a\"b]", &[])])),
            ("   ", mem("", &[], vec![])),
            // `String(tag)` of numbers: correctly rounded parse + JS formatting.
            (
                "[SAVE_EVENT: {\"summary\":\"s\",\"tags\":[100499.99999999999, 0.9095702657941729, 1e21, 1e-7]}] ok",
                mem("ok", &[], vec![ev("s", &["100499.99999999999", "0.9095702657941729", "1e+21", "1e-7"])]),
            ),
            // `String(tag)` throws for an own `toString` key: the tag loop stops, the rest stays.
            (
                "[SAVE_EVENT: {\"summary\":\"s\",\"tags\":[\"a\",{\"toString\":1},\"b\"]}] ok",
                mem("ok", &[], vec![ev("s", &["a"])]),
            ),
            (
                "[SAVE_EVENT: {\"summary\":\"s\",\"tags\":[\"a\",[1,[{\"toString\":\"x\"}]],\"b\"]}] ok",
                mem("ok", &[], vec![ev("s", &["a"])]),
            ),
            (
                "[SAVE_EVENT: {\"summary\":\"s\",\"tags\":[\"a\",{\"valueOf\":1},\"b\"]}] ok",
                mem("ok", &[], vec![ev("s", &["a", "[object Object]", "b"])]),
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(extract_memory_commands(input), expected, "{input:?}");
        }
    }

    #[test]
    fn pause_character_golden() {
        let cases: [(char, Option<char>, bool); 15] = [
            ('.', Some('.'), false),
            ('.', Some(' '), true),
            ('.', None, true),
            ('.', Some('a'), false),
            ('.', Some('\n'), true),
            ('。', Some('」'), false),
            ('。', Some('a'), true),
            ('！', Some('？'), true),
            ('\n', Some('』'), false),
            ('\n', Some('a'), true),
            ('…', None, false),
            ('?', Some('”'), false),
            ('!', Some('"'), true),
            ('？', Some(')'), false),
            ('、', None, false),
        ];
        for (c, next, expected) in cases {
            assert_eq!(is_pause_character(c, next), expected, "{c:?} {next:?}");
        }
    }

    #[test]
    fn char_delay_golden() {
        let cases: [(char, f64, f64); 14] = [
            ('あ', 1.0, 50.0),
            ('、', 1.0, 125.0),
            (',', 1.0, 125.0),
            ('…', 0.5, 250.0),
            ('」', 1.0, 75.0),
            ('）', 2.0, 37.5),
            (')', 1.0, 50.0),
            ('a', 0.0, 500.0),
            ('a', 0.05, 500.0),
            ('a', 3.0, 16.666666666666668),
            ('あ', 0.1, 500.0),
            ('、', 3.0, 41.66666666666667),
            ('…', -5.0, 1250.0),
            ('」', 0.3, 250.00000000000003),
        ];
        for (c, speed, expected) in cases {
            assert_eq!(char_delay_for_char(c, speed), expected, "{c:?} {speed}");
        }
        assert!(char_delay_for_char(',', f64::NAN).is_nan());
    }

    /// Golden: spec Appendix G, app-part1 Appendix B and extra V1 runs.
    #[test]
    fn split_message_golden() {
        let cases: Vec<(&str, Vec<&str>)> = vec![
            (
                "こんにちは。今日は「いい天気ね。」そうでしょ？\n\n  ねえ!?本当... えっ.続き",
                vec![
                    "こんにちは。",
                    "今日は「いい天気ね。」そうでしょ？",
                    "ねえ!",
                    "?",
                    "本当...",
                    "えっ.続き",
                ],
            ),
            (
                "  先頭空白。\r\n次の行\n最後",
                vec!["先頭空白。", "次の行\n", "最後"],
            ),
            (
                "Dr. Pepper is great. Yes!",
                vec!["Dr.", "Pepper is great.", "Yes!"],
            ),
            ("えっ！？　何それ", vec!["えっ！", "？", "　何それ"]),
            ("", vec![]),
            ("   ", vec![]),
            ("こんにちは。元気？", vec!["こんにちは。", "元気？"]),
            ("Hello. World", vec!["Hello.", "World"]),
            ("Wait... what?", vec!["Wait...", "what?"]),
            (
                "「そうね。」と彼女は言った。",
                vec!["「そうね。」と彼女は言った。"],
            ),
            ("Line1\nLine2", vec!["Line1\n", "Line2"]),
            (
                "  leading spaces. trailing  ",
                vec!["leading spaces.", "trailing  "],
            ),
            ("3.14 is pi.", vec!["3.14 is pi."]),
            ("Hi!! Really?!", vec!["Hi!", "!", "Really?", "!"]),
            ("\n\n\n", vec![]),
            ("A。\n\nB", vec!["A。", "B"]),
            ("やった！😊すごい", vec!["やった！", "😊すごい"]),
            ("A。　B", vec!["A。", "　B"]),
            ("A。　", vec!["A。"]),
            ("A\r\nB", vec!["A\r\n", "B"]),
            ("(ok?) yes", vec!["(ok?) yes"]),
            (
                "He said \"Hi.\" Then left.",
                vec!["He said \"Hi.\" Then left."],
            ),
            ("Tab.\tNext", vec!["Tab.\tNext"]),
            ("end.", vec!["end."]),
            ("”quoted！”after", vec!["”quoted！”after"]),
            ("a?)b", vec!["a?)b"]),
            ("x！』y", vec!["x！』y"]),
            ("\t\r\n leading", vec!["leading"]),
            ("one\n\ntwo\n", vec!["one\n", "two\n"]),
            ("\u{FEFF}。\u{FEFF}", vec!["\u{FEFF}。"]),
            ("a\u{0085}。b", vec!["a\u{0085}。", "b"]),
            ("😀。😀", vec!["😀。", "😀"]),
        ];
        for (input, expected) in cases {
            assert_eq!(split_message(input), expected, "{input:?}");
        }
    }

    #[test]
    fn split_text_for_tts_golden() {
        let long = "あ".repeat(130);
        let emoji = format!("a{}", "😀".repeat(70));
        let xs = format!("{}。y", "x".repeat(119));
        let cases: Vec<(String, Vec<String>)> = vec![
            (
                "こんにちは。元気？ そう!".into(),
                vec!["こんにちは。".into(), "元気？".into(), "そう!".into()],
            ),
            (
                "  多重\n\n空白\t\tテスト。 ".into(),
                vec!["多重 空白 テスト。".into()],
            ),
            (String::new(), vec![]),
            (long, vec!["あ".repeat(120), "あ".repeat(10)]),
            (
                emoji,
                vec![format!("a{}", "😀".repeat(60)), "😀".repeat(10)],
            ),
            (
                "a\u{0085}b。c\u{FEFF}d".into(),
                vec!["a\u{0085}b。".into(), "c d".into()],
            ),
            ("Hi! . ok".into(), vec!["Hi!".into(), ". ok".into()]),
            (xs, vec![format!("{}。", "x".repeat(119)), "y".into()]),
        ];
        for (input, expected) in cases {
            assert_eq!(split_text_for_tts(&input), expected, "{input:?}");
        }
    }

    /// Golden: V1 `captureStreamTextForTTS` sequences (chunks enqueued so far after each call).
    #[test]
    fn tts_stream_capture_golden() {
        type Step<'a> = (&'a str, bool, &'a [&'a str]);
        let sequences: Vec<Vec<Step>> = vec![
            vec![
                ("こんに", false, &[]),
                ("こんにちは。元", false, &["こんにちは。"]),
                (
                    "こんにちは。元気？「そう」",
                    false,
                    &["こんにちは。", "元気？"],
                ),
                (
                    "こんにちは。元気？「そう」と言った",
                    true,
                    &["こんにちは。", "元気？", "「そう」と言った"],
                ),
            ],
            vec![
                ("「はい。」次", false, &["「はい。」"]),
                ("「はい。」次!) and", false, &["「はい。」", "次!)"]),
                (
                    "「はい。」次!) and more",
                    true,
                    &["「はい。」", "次!)", "and more"],
                ),
            ],
            vec![
                ("[SMILE] Hi", false, &[]),
                ("[SMILE] Hi there. [W", false, &[]),
                (
                    "[SMILE] Hi there. [WINK] ok",
                    true,
                    &["Hi there. [WINK] ok"],
                ),
            ],
            vec![
                ("abcdef", false, &[]),
                ("abc", false, &[]),
                ("abcXYZ!", true, &["abcdefXYZ!"]),
            ],
            vec![("", true, &[])],
            vec![
                ("  。  ", false, &["。"]),
                ("  。  x\n", true, &["。", "x"]),
            ],
            // No new text → no flush even when complete (V1 quirk).
            vec![("Hello", false, &[]), ("Hello", true, &[])],
            // The seen length ends inside a surrogate pair: V1's delta is the lone low half
            // (dropped in V2), yet it is new text, so the index advances and the flush runs.
            vec![("abc", false, &[]), ("ab😀", true, &["abc"])],
            vec![
                ("abc", false, &[]),
                ("ab😀", false, &[]),
                ("abcd", true, &[]),
            ],
            vec![
                ("abc", false, &[]),
                ("ab😀!", false, &["abc!"]),
                ("ab😀!x", true, &["abc!", "x"]),
            ],
            vec![
                ("ab", false, &[]),
                ("a😀", false, &[]),
                ("a😀。", true, &["ab。"]),
            ],
        ];
        for seq in sequences {
            let mut capture = TtsStreamCapture::new();
            let mut all: Vec<String> = Vec::new();
            for (text, complete, expected) in seq {
                all.extend(capture.capture(text, complete));
                assert_eq!(all, expected, "after {text:?}");
            }
        }
        let mut capture = TtsStreamCapture::new();
        capture.capture("abc", false);
        assert_eq!(capture.pending(), "abc");
        capture.reset();
        assert_eq!(capture.capture("abc。", false), vec!["abc。".to_owned()]);
    }

    /// Golden: V1 `handleStreamToken` detection sequences.
    #[test]
    fn stream_emotion_detector_golden() {
        type Step<'a> = (&'a str, Option<Emotion>, &'a str, bool, &'a str);
        let sequences: Vec<Vec<Step>> = vec![
            vec![
                ("[SM", None, "[SM", false, "[SM"),
                ("ILE] こん", Some(Emotion::Smile), "こん", true, "こん"),
                (
                    "にちは [WINK] ね",
                    None,
                    "こんにちは  ね",
                    true,
                    "こんにちは [WINK] ね",
                ),
            ],
            vec![
                ("<think>", None, "<think>", false, "<think>"),
                ("考え中", None, "<think>考え中", false, "<think>考え中"),
                ("</think>", None, "", false, "<think>考え中</think>"),
                ("[ angry ]", Some(Emotion::Angry), "", true, ""),
                (" 何よ", None, "何よ", true, " 何よ"),
            ],
            vec![
                ("Hi", None, "Hi", false, "Hi"),
                ("!", None, "Hi!", true, "Hi!"),
                (" there", None, "Hi! there", true, "Hi! there"),
            ],
            vec![
                ("[NOTATAG] x", None, "[NOTATAG] x", true, "[NOTATAG] x"),
                (" y", None, "[NOTATAG] x y", true, "[NOTATAG] x y"),
            ],
            vec![
                ("[", None, "[", false, "["),
                (
                    "\u{17F}mile] long s",
                    Some(Emotion::Smile),
                    "long s",
                    true,
                    "long s",
                ),
            ],
            vec![
                ("<b", None, "<b", false, "<b"),
                ("old> x", None, "<bold> x", false, "<bold> x"),
            ],
            vec![
                (
                    "  [BLUSH]   べ、別に",
                    Some(Emotion::Blush),
                    "べ、別に",
                    true,
                    "べ、別に",
                ),
                ("！", None, "べ、別に！", true, "べ、別に！"),
            ],
            vec![
                ("ab", None, "ab", false, "ab"),
                ("[SAD] c", None, "ab c", true, "ab[SAD] c"),
            ],
            vec![
                ("[]", None, "[]", false, "[]"),
                ("x", None, "[]x", true, "[]x"),
            ],
        ];
        for seq in sequences {
            let mut detector = StreamEmotionDetector::new();
            for (token, emotion, display, parsed, buffer) in seq {
                let update = detector.push_token(token);
                assert_eq!(
                    update,
                    StreamTokenUpdate {
                        emotion,
                        display: display.to_owned()
                    },
                    "token {token:?}"
                );
                assert_eq!(detector.is_parsed(), parsed, "token {token:?}");
                assert_eq!(detector.buffer(), buffer, "token {token:?}");
            }
        }
        let mut d = StreamEmotionDetector::new();
        d.push_token("  partial ");
        assert_eq!(d.completion_raw("  "), "partial");
        assert_eq!(d.completion_raw(" full "), "full");
        d.reset();
        assert!(!d.is_parsed() && d.buffer().is_empty());
    }

    /// The text `handleStreamComplete` displays for a finished stream (before the `……`
    /// fallback): `strip_emotion_tag(extract_memory_commands(raw.trim()).cleaned_text)`.
    fn completion_display(full: &str) -> String {
        strip_emotion_tag(&extract_memory_commands(js_trim(full)).cleaned_text)
    }

    /// Every char-boundary prefix of `text` (the stream so far after any token split; the
    /// display depends only on the concatenation, so this covers every way to chunk `text`).
    fn prefixes(text: &str) -> impl Iterator<Item = &str> {
        text.char_indices()
            .map(|(i, _)| &text[..i])
            .chain(std::iter::once(text))
    }

    /// Checks the D23 display invariant for one reply: every intermediate display is a prefix
    /// of the next one and of the completion display. Returns the distinct displays.
    fn check_stream(full: &str) -> Vec<String> {
        let finished = completion_display(full);
        let mut previous = String::new();
        let mut seen = Vec::new();
        for raw in prefixes(full) {
            let display = stream_display_prefix(raw);
            assert!(
                finished.starts_with(&display),
                "{full:?}: after {raw:?} the display {display:?} is not a prefix of {finished:?}"
            );
            assert!(
                display.starts_with(&previous),
                "{full:?}: after {raw:?} the display {display:?} does not extend {previous:?}"
            );
            if display != previous || seen.is_empty() {
                seen.push(display.clone());
            }
            previous = display;
        }
        seen
    }

    /// `text` cut into chunks of `n` characters (the audit mock streams 4-character chunks).
    fn chunked(text: &str, n: usize) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        chars.chunks(n).map(|c| c.iter().collect()).collect()
    }

    /// The display after each chunk.
    fn displays_by_chunk(text: &str, n: usize) -> Vec<String> {
        let mut raw = String::new();
        chunked(text, n)
            .into_iter()
            .map(|chunk| {
                raw.push_str(&chunk);
                stream_display_prefix(&raw)
            })
            .collect()
    }

    #[test]
    fn stream_display_audit_cases() {
        // CHAT-028 case 1: a <think> block after the leading tag (was typed, then the real text
        // was never shown).
        let think_after =
            "[SMILE] <think>ここは内部の思考メモで表示されてはいけない</think>本文です。";
        let shown = displays_by_chunk(think_after, 4);
        assert!(
            shown[..shown.len() - 2].iter().all(String::is_empty),
            "{shown:?}"
        );
        assert_eq!(shown[shown.len() - 2..], ["本文です", "本文です。"]);
        assert_eq!(
            check_stream(think_after),
            ["", "本", "本文", "本文で", "本文です", "本文です。"]
        );

        // Case 2: memory commands (were typed as their own pages).
        let memo = "[SMILE] 覚えておくわ。[SAVE_FACT: ユーザーの名前は岡部]\n\n\n\
                    [SAVE_EVENT: {\"summary\": \"ラボで実験した\", \"tags\": [\"ラボ\", \"実験\"]}]";
        let shown = displays_by_chunk(memo, 4);
        assert_eq!(shown[..4], ["", "", "覚えてお", "覚えておくわ。"]);
        assert!(
            shown[4..].iter().all(|d| d == "覚えておくわ。"),
            "{shown:?}"
        );
        assert_eq!(completion_display(memo), "覚えておくわ。");
        check_stream(memo);

        // Case 3: a mid-text tag (`前置き [W` was typed, then the text jumped).
        let tag_mid = "前置き [WINK] 後ろ";
        assert_eq!(
            displays_by_chunk(tag_mid, 4),
            ["前置き", "前置き", "前置き  後", "前置き  後ろ"]
        );
        assert_eq!(completion_display(tag_mid), "前置き  後ろ");

        // Case 4: `[ PANIC ]` is no tag for the completion path, so it is shown from the start
        // (`えっ、ちょっ` was typed, then the text jumped to `[ PANIC ] …`).
        let spaced = "[ PANIC ] えっ、ちょっと";
        assert_eq!(
            displays_by_chunk(spaced, 4),
            [
                "[ PA",
                "[ PANIC",
                "[ PANIC ] えっ",
                "[ PANIC ] えっ、ちょっ",
                "[ PANIC ] えっ、ちょっと"
            ]
        );
        assert_eq!(completion_display(spaced), spaced);

        // Case 5 (the partial backlog page) is a typewriter rule; the text itself only grows.
        let long = "[THINKING] そうね……少し長くなるけど説明するわ。まず、世界線の収束について。";
        check_stream(long);
        assert_eq!(stream_display_prefix(long), completion_display(long));

        // The converge TTS run: the command inside the think block is dropped with it.
        let think = "<think>internal [SAVE_FACT: 好物はプリン]</think>[BLUSH] な、何よ。";
        assert_eq!(
            check_stream(think).last().map(String::as_str),
            Some("な、何よ。")
        );
    }

    #[test]
    fn stream_display_holds_back_undecided_text() {
        let cases = [
            // Partial or unterminated memory commands.
            ("", ""),
            ("[", ""),
            ("ok [S", "ok"),
            ("ok [SA", "ok"),
            ("ok [SAV", "ok"),
            ("ok [SAVE", "ok"),
            ("ok [SAVE_", "ok"),
            ("ok [SAVE_F", "ok"),
            ("ok [SAVE_ fa", "ok"),
            ("ok [SAVE_FACT", "ok"),
            ("ok [SAVE_FACT ", "ok"),
            ("ok [SAVE_FACT: x", "ok"),
            ("ok [SAVE_FACT: x]", "ok"),
            ("ok [SAVE_FACT: x] go", "ok  go"),
            ("ok [SAVE_EVENT: {\"summary\": \"a]\"", "ok"),
            ("ok [SAVE_EVENT: {\"summary\": \"a]\"}] go", "ok  go"),
            // Headers that can no longer be a command name are text.
            ("ok [SAVE_X", "ok [SAVE_X"),
            ("ok [SAVE_FACTS", "ok [SAVE_FACTS"),
            ("ok [SAVE_FOO: x] tail", "ok [SAVE_FOO: x] tail"),
            ("ok [SAVE_X [SA", "ok [SAVE_X"),
            ("[SAVE_[SAVE_FACT: a] b", "[SAVE_ b"),
            // A command and a tag split by one another (single-pass removal, like V1).
            ("[SMI[SAVE_FACT: x]LE] hi", "hi"),
            // <think> blocks anywhere.
            ("a <", "a"),
            ("a <thi", "a"),
            ("a <THINK", "a"),
            ("a <think>", "a"),
            ("a <think>memo</thi", "a"),
            ("a <think>memo</think>", "a"),
            ("a <think>memo</think> b", "a  b"),
            ("a <Think>x</THINK>b<think>y", "a b"),
            ("<b>bold", "<b>bold"),
            ("a <x", "a <x"),
            // Emotion tags anywhere.
            ("前置き [W", "前置き"),
            ("前置き [WINK", "前置き"),
            ("前置き [WINK]", "前置き"),
            ("前置き [wink] 後", "前置き  後"),
            ("前置き [WINKS", "前置き [WINKS"),
            ("x [1", "x [1"),
            ("x []", "x []"),
            ("x [ smile", "x [ smile"),
            ("[ PANIC", "[ PANIC"),
            // Trailing whitespace and blank lines.
            ("abc  ", "abc"),
            ("abc\n\n", "abc"),
            ("x\n\n\n\ny", "x\n\ny"),
            ("x\n[SAVE_FACT: y]\n\n\nz", "x\n\nz"),
            // An unterminated command inside a think block stops V1's scan: everything after it
            // waits for the completion text.
            ("<think>[SAVE_FACT: a</think>本文。[SAVE_FACT: b]", ""),
        ];
        for (raw, expected) in cases {
            assert_eq!(stream_display_prefix(raw), expected, "{raw:?}");
        }
        assert_eq!(
            completion_display("<think>[SAVE_FACT: a</think>本文。[SAVE_FACT: b]"),
            "本文。[SAVE_FACT: b]"
        );
    }

    #[test]
    fn stream_display_every_split_of_multi_tag_replies() {
        // ASCII only, so every byte boundary is a possible token boundary.
        let ascii = "[SMILE] <think>plan [SAVE_FACT: x]</think>Hi! [WINK] ok.[SAVE_EVENT: \
                     {\"summary\": \"lab]\", \"tags\": [\"a\"]}]\n\n\n[ PANIC ] end [SAVE_X: y] \
                     <b>[sad]bye";
        let shown = check_stream(ascii);
        assert_eq!(
            completion_display(ascii),
            "Hi!  ok.\n\n[ PANIC ] end [SAVE_X: y] <b>bye"
        );
        assert_eq!(shown.last(), Some(&completion_display(ascii)));
        // Two and three tokens at every pair of byte boundaries.
        let finished = completion_display(ascii);
        for a in 0..=ascii.len() {
            let first = stream_display_prefix(&ascii[..a]);
            assert!(finished.starts_with(&first), "split at {a}");
            for b in a..=ascii.len() {
                let second = stream_display_prefix(&ascii[..b]);
                assert!(second.starts_with(&first) && finished.starts_with(&second));
            }
        }
        let japanese = "[SMILE]<think>考え中[SAVE_FACT: 好物はプリン]</think>覚えておくわ。\
                        [SAVE_FACT: ユーザーの名前は岡部]\n\n\n前置き [WINK] 後ろ「そうよ。」😀";
        let shown = check_stream(japanese);
        assert_eq!(
            shown.last().map(String::as_str),
            Some("覚えておくわ。\n\n前置き  後ろ「そうよ。」😀")
        );
    }

    /// A tiny deterministic generator for the property test (no RNG dependency).
    struct XorShift(u64);

    impl XorShift {
        fn below(&mut self, n: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 % n as u64) as usize
        }

        fn reply(&mut self, atoms: &[&str], max: usize) -> String {
            (0..self.below(max) + 1)
                .map(|_| atoms[self.below(atoms.len())])
                .collect()
        }
    }

    #[test]
    fn stream_display_is_stable_for_any_continuation() {
        let atoms = [
            "[SMILE]",
            "[smile]",
            "[ PANIC ]",
            "[WINK]",
            "[SAD]",
            "[NOTATAG]",
            "[",
            "]",
            "[S",
            "[SA",
            "[SAVE",
            "[SAVE_",
            "[SAVE_FACT: 名前は岡部]",
            "[SAVE_FACT: x",
            "[SAVE_ fact : y]",
            "[SAVE_EVENT: {\"summary\": \"a]\", \"tags\": [\"t\"]}]",
            "[SAVE_EVENT: {",
            "[SAVE_FOO: z]",
            "[SAVE_X",
            "FACT",
            ":",
            "\"",
            "{",
            "}",
            "<think>",
            "</think>",
            "<THINK>",
            "</THINK>",
            "<",
            "<th",
            "</th",
            "think>",
            "メモ",
            "本文です。",
            "そうね……",
            "Hello. ",
            "3.14",
            " ",
            "  ",
            "\n",
            "\n\n\n",
            "\t",
            "\u{3000}",
            "\u{FEFF}",
            "😀",
            "。",
            "x",
            "ab",
            "「",
            "」",
        ];
        let mut rng = XorShift(0x5DEE_CE66_D1CE_4E5B);
        for _ in 0..3000 {
            let raw = rng.reply(&atoms, 8);
            let shown = stream_display_prefix(&raw);
            for _ in 0..12 {
                let full = format!("{raw}{}", rng.reply(&atoms, 6));
                let finished = completion_display(&full);
                assert!(
                    finished.starts_with(&shown),
                    "{raw:?} shows {shown:?}, but {full:?} completes as {finished:?}"
                );
            }
        }
        for _ in 0..400 {
            check_stream(&rng.reply(&atoms, 14));
        }
    }

    #[test]
    fn api_error_messages_golden() {
        let cases = [
            (
                "API key not set",
                "……APIキーが設定されてないみたいよ。CONFIGから設定してちょうだい。",
            ),
            (
                "401 Unauthorized",
                "APIキーが無効みたい……もう一度確認して設定し直してくれる？",
            ),
            (
                "Request failed (429)",
                "リクエストが多すぎるみたい。少し待ってからもう一度試してくれない？",
            ),
            (
                "insufficient_quota",
                "Geminiの利用枠か課金設定に問題があるみたい。APIキーとプロジェクト設定を確認して。",
            ),
            (
                "Gateway Timeout 504",
                "応答がタイムアウトしたわ……ネットワークの状態を確認してみて。",
            ),
            (
                "Network error",
                "ネットワークに接続できないわ。インターネット接続を確認してちょうだい。",
            ),
            (
                "403",
                "このAPIへのアクセスが拒否されたわ。権限を確認してみて。",
            ),
            (
                "HTTP 500",
                "サーバー側でエラーが起きてるみたい。しばらくしてからもう一度試して。",
            ),
            (
                "model not found",
                "指定されたモデルが見つからないみたい。CONFIGからモデル名を確認して。",
            ),
            (
                "Invalid JSON",
                "サービスアカウントJSONキーの形式が正しいか確認して。",
            ),
            (
                "Vertex token",
                "Vertexの認証情報を確認して。APIキーかサービスアカウントJSONキーが必要よ。",
            ),
            (
                "アクセストークン取得失敗",
                "Vertexの認証情報を確認して。APIキーかサービスアカウントJSONキーが必要よ。",
            ),
            ("weird", API_ERROR_DEFAULT_MESSAGE),
            ("", API_ERROR_DEFAULT_MESSAGE),
            ("api key", API_ERROR_DEFAULT_MESSAGE),
            ("Failed to fetch", API_ERROR_DEFAULT_MESSAGE),
            ("Unknown error", API_ERROR_DEFAULT_MESSAGE),
        ];
        for (error, expected) in cases {
            assert_eq!(api_error_message(error), expected, "{error:?}");
            assert_eq!(
                api_error_message_for_platform(error, false),
                expected,
                "web:{error:?}"
            );
        }
        assert_eq!(
            api_error_message_for_platform(
                "Vertex AI: Project ID が未設定です。Config_VertexProject を設定してください。",
                true
            ),
            "Vertex AIのProject IDが設定されてないみたいよ。CONFIGから設定してちょうだい。"
        );
        assert_eq!(
            api_error_message_for_platform("Vertex AI: アクセストークンの取得に失敗しました。\ngcloud CLI がインストールされ、gcloud auth login 済みか確認してください。", true),
            "Vertex AIの認証に失敗したわ。gcloudの設定を確認してみて。"
        );
        assert_eq!(
            api_error_message_for_platform("Vertex AI: All regions failed", true),
            "Vertex AIの認証に失敗したわ。gcloudの設定を確認してみて。"
        );
        assert_eq!(
            api_error_message_for_platform("Request failed (429)", true),
            "リクエストが多すぎるみたい。少し待ってからもう一度試してくれない？"
        );
        assert_eq!(
            api_error_message_for_platform("401 Unauthorized", true),
            "APIキーが無効みたい……もう一度確認して設定し直してくれる？"
        );
    }

    #[test]
    fn dev_commands() {
        assert_eq!(
            parse_dev_command("  /dev emotion help "),
            Some(DevCommand::EmotionHelp)
        );
        assert_eq!(
            parse_dev_command("/dev emotion smile"),
            Some(DevCommand::SetEmotion(Some("SMILE".into())))
        );
        assert_eq!(
            parse_dev_command("/dev emotion sleeping now"),
            Some(DevCommand::SetEmotion(Some("SLEEPING".into())))
        );
        assert_eq!(
            parse_dev_command("/dev emotion  smile"),
            Some(DevCommand::SetEmotion(None))
        );
        assert_eq!(
            parse_dev_command("/dev emotion \u{3000}x"),
            Some(DevCommand::SetEmotion(Some("X".into())))
        );
        assert_eq!(parse_dev_command("/dev emotion"), None);
        assert_eq!(
            parse_dev_command("/dev motion reset"),
            Some(DevCommand::MotionReset)
        );
        assert_eq!(
            parse_dev_command("/dev achievements reset"),
            Some(DevCommand::AchievementsReset)
        );
        assert_eq!(parse_dev_command("/dev steiner"), Some(DevCommand::Steiner));
        assert_eq!(parse_dev_command("/dev Steiner"), None);
        assert_eq!(parse_dev_command("hello"), None);
        assert!(DEV_EMOTION_HELP_TEXT
            .starts_with("【サポートされているモーションID一覧】\n・NORMAL (通常)\n"));
        assert!(DEV_EMOTION_HELP_TEXT.ends_with("・SNEEZE (クシャミ)\n\n※コマンド例: /dev emotion smile で対象のモーションに変更します。"));
    }

    #[test]
    fn helpers() {
        assert_eq!(strip_emotion_tags("a [SMILE] b [x] [Wink]"), "a  b [x] ");
        assert_eq!(strip_think_blocks(" <think>x</think> y "), "  y ");
        assert_eq!(clean_backlog_message("  "), None);
        assert_eq!(EMOTION_TAGS, Emotion::ALL.map(Emotion::as_str));
        assert_eq!(utf16_offset_to_byte_ceil("a😀b", 2), 5);
        assert_eq!(utf16_offset_to_byte_ceil("a😀b", 3), 5);
        assert_eq!(utf16_offset_to_byte_ceil("a😀b", 9), 6);
        assert_eq!(collapse_blank_lines("a\n\n\n\nb\n\nc"), "a\n\nb\n\nc");
    }
}
