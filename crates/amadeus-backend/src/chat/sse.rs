//! Line reader shared by every upstream stream parser (`spec-server-chat.md` §7.3) and a
//! streaming UTF-8 decoder equivalent to `new TextDecoder().decode(chunk, { stream: true })`.

use amadeus_core::jscompat::js_trim;

/// Byte-level line splitter. V1 decodes chunks with a streaming `TextDecoder` and splits the
/// decoded buffer on `"\n"`; splitting the raw bytes on `b'\n'` first is equivalent because
/// `0x0A` never occurs inside a multi-byte UTF-8 sequence.
#[derive(Debug, Default)]
pub(crate) struct LineReader {
    buf: Vec<u8>,
}

impl LineReader {
    /// Empty reader.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Append a network chunk and return every completed line, JS-trimmed (`line.trim()`).
    pub(crate) fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(chunk);
        let mut lines = Vec::new();
        let mut start = 0;
        while let Some(pos) = self.buf[start..].iter().position(|&b| b == b'\n') {
            let line = String::from_utf8_lossy(&self.buf[start..start + pos]);
            lines.push(js_trim(&line).to_owned());
            start += pos + 1;
        }
        self.buf.drain(..start);
        lines
    }

    /// What is left in V1's string buffer at EOF (not trimmed). V1 never flushes its decoder,
    /// so an incomplete trailing UTF-8 sequence is dropped rather than replaced.
    pub(crate) fn leftover(&self) -> String {
        let mut decoder = Utf8StreamDecoder::without_bom_handling();
        decoder.decode(&self.buf)
    }
}

/// `line.startsWith("data: ") ? line.slice(6).trim() : None` for an already trimmed line.
pub(crate) fn data_payload(line: &str) -> Option<&str> {
    line.strip_prefix("data: ").map(js_trim)
}

/// Streaming UTF-8 decoder with WHATWG semantics: invalid sequences become U+FFFD (maximal
/// subpart rule, like `String::from_utf8_lossy`), an incomplete trailing sequence is kept for
/// the next chunk, and a leading BOM of the whole stream is removed.
#[derive(Debug)]
pub(crate) struct Utf8StreamDecoder {
    pending: Vec<u8>,
    bom_seen: bool,
}

impl Default for Utf8StreamDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Utf8StreamDecoder {
    /// Decoder that strips a leading BOM (`new TextDecoder()`).
    pub(crate) fn new() -> Self {
        Self {
            pending: Vec::new(),
            bom_seen: false,
        }
    }

    fn without_bom_handling() -> Self {
        Self {
            pending: Vec::new(),
            bom_seen: true,
        }
    }

    /// `decoder.decode(chunk, { stream: true })`.
    pub(crate) fn decode(&mut self, chunk: &[u8]) -> String {
        let mut input = std::mem::take(&mut self.pending);
        input.extend_from_slice(chunk);
        let mut out = String::with_capacity(input.len());
        let mut rest: &[u8] = &input;
        loop {
            match std::str::from_utf8(rest) {
                Ok(s) => {
                    out.push_str(s);
                    break;
                }
                Err(e) => {
                    let (valid, tail) = rest.split_at(e.valid_up_to());
                    out.push_str(&String::from_utf8_lossy(valid));
                    match e.error_len() {
                        Some(len) => {
                            out.push('\u{FFFD}');
                            rest = &tail[len..];
                        }
                        None => {
                            self.pending = tail.to_vec();
                            break;
                        }
                    }
                }
            }
        }
        if !self.bom_seen && !out.is_empty() {
            self.bom_seen = true;
            if let Some(stripped) = out.strip_prefix('\u{FEFF}') {
                return stripped.to_owned();
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(chunks: &[&[u8]]) -> (Vec<String>, String) {
        let mut r = LineReader::new();
        let mut lines = Vec::new();
        for c in chunks {
            lines.extend(r.push(c));
        }
        (lines, r.leftover())
    }

    #[test]
    fn splits_lines_and_trims() {
        let (lines, left) = feed(&[b"data: a\r\n\r\ndata: b\n", b"data: c"]);
        assert_eq!(lines, vec!["data: a", "", "data: b"]);
        assert_eq!(left, "data: c");
    }

    #[test]
    fn handles_chunk_split_inside_multibyte_char_and_prefix() {
        let text = "data: {\"t\":\"あい\"}\n";
        let bytes = text.as_bytes();
        // Split inside the "data: " prefix and inside the 3-byte "あ".
        let cut1 = 3;
        let cut2 = text.find('あ').unwrap_or(0) + 1;
        let (lines, left) = feed(&[&bytes[..cut1], &bytes[cut1..cut2], &bytes[cut2..]]);
        assert_eq!(lines, vec!["data: {\"t\":\"あい\"}"]);
        assert_eq!(left, "");
        assert_eq!(data_payload(&lines[0]), Some("{\"t\":\"あい\"}"));
    }

    #[test]
    fn leftover_drops_incomplete_utf8_tail() {
        let (lines, left) = feed(&[b"data: {}\xe3\x81"]);
        assert!(lines.is_empty());
        assert_eq!(left, "data: {}");
        // An invalid byte in the middle is replaced, like TextDecoder.
        let (_, left) = feed(&[b"a\xffb"]);
        assert_eq!(left, "a\u{FFFD}b");
    }

    #[test]
    fn data_payload_requires_space() {
        assert_eq!(data_payload("data: [DONE]"), Some("[DONE]"));
        assert_eq!(data_payload("data:   x  "), Some("x"));
        assert_eq!(data_payload("data:{}"), None);
        assert_eq!(data_payload(": OPENROUTER PROCESSING"), None);
        // BOM and ideographic spaces are trimmed with JS semantics.
        let (lines, _) = feed(&["\u{FEFF}data: x\u{3000}\n".as_bytes()]);
        assert_eq!(lines, vec!["data: x"]);
    }

    #[test]
    fn stream_decoder_matches_text_decoder() {
        let mut d = Utf8StreamDecoder::new();
        let bytes = "\u{FEFF}あい".as_bytes();
        assert_eq!(d.decode(&bytes[..2]), "");
        assert_eq!(d.decode(&bytes[2..4]), "");
        assert_eq!(d.decode(&bytes[4..]), "あい");
        // The BOM is only stripped at the very start of the stream.
        assert_eq!(d.decode("\u{FEFF}x".as_bytes()), "\u{FEFF}x");
        let mut d = Utf8StreamDecoder::new();
        assert_eq!(d.decode(b"a\xff\xfeb"), "a\u{FFFD}\u{FFFD}b");
        assert_eq!(d.decode(b"\xe3"), "");
        assert_eq!(d.decode(b"x"), "\u{FFFD}x");
    }
}
