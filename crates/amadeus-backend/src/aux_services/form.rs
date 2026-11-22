//! `await request.formData()` exactly as Node 22's undici implements it.
//!
//! V1 `stt/transcribe` parsed the incoming body with undici's own (strict) multipart parser and
//! its `application/x-www-form-urlencoded` support. This is a line-by-line port of
//! `lib/web/fetch/formdata-parser.js`, `extractMimeType` / `parseMIMEType` and the
//! `formData()` body mixin, including their quirks (bare-boundary search, `&&` CRLF check,
//! part `Content-Type` defaulting to `text/plain`, Blob type normalization), verified against
//! Node 22.22.0 golden vectors (see the tests).

use bytes::Bytes;

use super::js::{blob_type, latin1_decode, text_decoder_decode, utf8_decode_bytes};

/// One parsed form entry (undici `FormData` value).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormValue {
    /// A string field.
    Text(String),
    /// A file field (the part had a `filename` parameter).
    File(FormFile),
}

impl FormValue {
    /// `String(value)`: files stringify to `"[object File]"`.
    pub fn to_js_string(&self) -> String {
        match self {
            FormValue::Text(s) => s.clone(),
            FormValue::File(_) => "[object File]".to_owned(),
        }
    }
}

/// A file entry (`File` object).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormFile {
    /// `file.name` (may be empty).
    pub filename: String,
    /// `file.type` after Blob normalization (lower-cased, or empty when it contained characters
    /// outside U+0020..=U+007E or non-ASCII bytes).
    pub content_type: String,
    /// The file bytes.
    pub data: Bytes,
}

/// Parsed form entries in body order (duplicates kept).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormData {
    entries: Vec<(String, FormValue)>,
}

impl FormData {
    /// `form.get(name)`: the FIRST entry with that name.
    pub fn get(&self, name: &str) -> Option<&FormValue> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    /// All entries in order.
    pub fn entries(&self) -> &[(String, FormValue)] {
        &self.entries
    }
}

/// `request.formData()` rejected (wrong/missing content type or malformed body). V1 maps it to
/// 400 `{"error":"Invalid form data"}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidForm;

/// Parsed MIME type (`parseMIMEType` result).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MimeType {
    /// Lower-cased `type/subtype`.
    pub essence: String,
    /// Parameters (lower-cased names, first occurrence wins), in order.
    pub parameters: Vec<(String, String)>,
}

impl MimeType {
    /// Parameter value by (lower-case) name.
    pub fn parameter(&self, name: &str) -> Option<&str> {
        self.parameters
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Parses a request body like undici `Body.formData()`.
///
/// `content_type` is the request's combined `Content-Type` header value (latin1-decoded,
/// multiple header lines joined with `", "`).
pub fn parse_form(content_type: Option<&str>, body: &[u8]) -> Result<FormData, InvalidForm> {
    let mime = content_type
        .and_then(extract_mime_type)
        .ok_or(InvalidForm)?;
    match mime.essence.as_str() {
        "multipart/form-data" => {
            let boundary = mime.parameter("boundary").ok_or(InvalidForm)?;
            parse_multipart(body, boundary)
                .map(|entries| FormData { entries })
                .ok_or(InvalidForm)
        }
        "application/x-www-form-urlencoded" => Ok(FormData {
            entries: parse_urlencoded(body),
        }),
        _ => Err(InvalidForm),
    }
}

// ---------------------------------------------------------------------------------------------
// MIME type extraction (fetch spec "extract a MIME type" + mimesniff "parse a MIME type").
// ---------------------------------------------------------------------------------------------

fn is_http_ws(c: char) -> bool {
    matches!(c, '\r' | '\n' | '\t' | ' ')
}

fn is_tab_or_space(c: char) -> bool {
    matches!(c, '\t' | ' ')
}

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "!#$%&'*+-.^_|~".contains(c)
}

fn is_token(s: &[char]) -> bool {
    !s.is_empty() && s.iter().copied().all(is_token_char)
}

fn is_quoted_string_tokens(s: &[char]) -> bool {
    !s.is_empty()
        && s.iter()
            .all(|&c| c == '\t' || (' '..='~').contains(&c) || ('\u{80}'..='\u{ff}').contains(&c))
}

/// undici `removeChars` (note: trailing removal never inspects index 0).
fn remove_chars<T: Copy>(s: &[T], leading: bool, trailing: bool, pred: impl Fn(T) -> bool) -> &[T] {
    let len = s.len();
    let mut lead = 0usize;
    let mut trail = len as isize - 1;
    if leading {
        while lead < len && pred(s[lead]) {
            lead += 1;
        }
    }
    if trailing {
        while trail > 0 && pred(s[trail as usize]) {
            trail -= 1;
        }
    }
    if lead == 0 && trail == len as isize - 1 {
        return s;
    }
    let end = usize::try_from(trail + 1).unwrap_or(0);
    if end <= lead {
        &s[..0]
    } else {
        &s[lead..end]
    }
}

fn find_char(s: &[char], c: char, from: usize) -> Option<usize> {
    s.get(from..)?
        .iter()
        .position(|&x| x == c)
        .map(|i| i + from)
}

/// `collectAnHTTPQuotedString`; `extract_value` = return the unquoted value instead of the
/// raw slice.
fn collect_http_quoted_string(input: &[char], pos: &mut usize, extract_value: bool) -> String {
    let start = *pos;
    let mut value = String::new();
    *pos += 1; // opening quote
    loop {
        while *pos < input.len() && input[*pos] != '"' && input[*pos] != '\\' {
            value.push(input[*pos]);
            *pos += 1;
        }
        if *pos >= input.len() {
            break;
        }
        let quote_or_backslash = input[*pos];
        *pos += 1;
        if quote_or_backslash == '\\' {
            if *pos >= input.len() {
                value.push('\\');
                break;
            }
            value.push(input[*pos]);
            *pos += 1;
        } else {
            break;
        }
    }
    if extract_value {
        value
    } else {
        input[start..(*pos).min(input.len())].iter().collect()
    }
}

/// `gettingDecodingSplitting`: splits a header value on commas outside quoted strings.
fn split_header_values(value: &str) -> Vec<String> {
    let input: Vec<char> = value.chars().collect();
    let mut values = Vec::new();
    let mut pos = 0usize;
    let mut temporary = String::new();
    while pos < input.len() {
        while pos < input.len() && input[pos] != '"' && input[pos] != ',' {
            temporary.push(input[pos]);
            pos += 1;
        }
        if pos < input.len() {
            if input[pos] == '"' {
                temporary.push_str(&collect_http_quoted_string(&input, &mut pos, false));
                if pos < input.len() {
                    continue;
                }
            } else {
                pos += 1; // ','
            }
        }
        let chars: Vec<char> = temporary.chars().collect();
        values.push(
            remove_chars(&chars, true, true, is_tab_or_space)
                .iter()
                .collect(),
        );
        temporary.clear();
    }
    values
}

/// `extractMimeType(headers)`: the LAST parsable value that is not `*/*` wins.
pub fn extract_mime_type(header_value: &str) -> Option<MimeType> {
    let mut mime = None;
    for value in split_header_values(header_value) {
        match parse_mime_type(&value) {
            Some(m) if m.essence != "*/*" => mime = Some(m),
            _ => {}
        }
    }
    mime
}

/// `parseMIMEType(input)`.
pub fn parse_mime_type(input: &str) -> Option<MimeType> {
    let chars: Vec<char> = input.chars().collect();
    let input = remove_chars(&chars, true, true, is_http_ws);
    let len = input.len();
    let mut pos = 0usize;

    let type_end = find_char(input, '/', pos).unwrap_or(len);
    let type_ = &input[pos..type_end];
    pos = type_end;
    if !is_token(type_) {
        return None;
    }
    pos += 1; // '/' (may move past the end, like the JS code)

    let subtype: &[char] = if pos > len {
        &[]
    } else {
        let end = find_char(input, ';', pos).unwrap_or(len);
        let s = &input[pos..end];
        pos = end;
        s
    };
    if pos > len {
        pos = len;
    }
    let subtype = remove_chars(subtype, false, true, is_http_ws);
    if !is_token(subtype) {
        return None;
    }
    let essence = format!(
        "{}/{}",
        type_.iter().collect::<String>().to_ascii_lowercase(),
        subtype.iter().collect::<String>().to_ascii_lowercase()
    );
    let mut parameters: Vec<(String, String)> = Vec::new();

    while pos < len {
        pos += 1; // ';'
        while pos < len && is_http_ws(input[pos]) {
            pos += 1;
        }
        let name_start = pos;
        while pos < len && input[pos] != ';' && input[pos] != '=' {
            pos += 1;
        }
        let name: String = input[name_start..pos]
            .iter()
            .collect::<String>()
            .to_lowercase();
        if pos < len {
            if input[pos] == ';' {
                continue;
            }
            pos += 1; // '='
        }
        if pos > len {
            break;
        }
        let value: Vec<char> = if pos < len && input[pos] == '"' {
            let v = collect_http_quoted_string(input, &mut pos, true);
            pos = find_char(input, ';', pos).unwrap_or(len);
            v.chars().collect()
        } else {
            let end = find_char(input, ';', pos).unwrap_or(len);
            let v = remove_chars(&input[pos..end], false, true, is_http_ws).to_vec();
            pos = end;
            if v.is_empty() {
                continue;
            }
            v
        };
        let name_chars: Vec<char> = name.chars().collect();
        if is_token(&name_chars)
            && (value.is_empty() || is_quoted_string_tokens(&value))
            && !parameters.iter().any(|(n, _)| *n == name)
        {
            parameters.push((name, value.iter().collect()));
        }
    }
    Some(MimeType {
        essence,
        parameters,
    })
}

// ---------------------------------------------------------------------------------------------
// multipart/form-data (undici formdata-parser.js)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Default)]
struct PartHeaders {
    name: Option<String>,
    filename: Option<String>,
    content_type: Option<String>,
    encoding: Option<String>,
}

/// `bufferStartsWith(buffer, start, position)`.
fn starts_with_at(input: &[u8], start: &[u8], pos: usize) -> bool {
    input.len() >= start.len() && input.get(pos..pos.saturating_add(start.len())) == Some(start)
}

/// `buffer.indexOf(needle, from)` (an empty needle returns `min(from, len)`).
fn index_of(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() {
        return Some(from.min(haystack.len()));
    }
    let rest = haystack.get(from..)?;
    rest.windows(needle.len())
        .position(|w| w == needle)
        .map(|i| i + from)
}

fn is_crlf_at(input: &[u8], pos: usize) -> bool {
    input.get(pos) == Some(&b'\r') && input.get(pos.saturating_add(1)) == Some(&b'\n')
}

fn collect_bytes(input: &[u8], pos: &mut usize, cond: impl Fn(u8) -> bool) -> (usize, usize) {
    let start = *pos;
    while *pos < input.len() && cond(input[*pos]) {
        *pos += 1;
    }
    (start, *pos)
}

/// `parseMultipartFormDataName`: quoted value up to `"`, then `%0A`/`%0D` (case-insensitive)
/// and `%22` are unescaped.
fn parse_part_name(input: &[u8], pos: &mut usize) -> Option<String> {
    let (start, end) = collect_bytes(input, pos, |b| b != b'\n' && b != b'\r' && b != b'"');
    if input.get(*pos) != Some(&b'"') {
        return None;
    }
    *pos += 1;
    let name = text_decoder_decode(&input[start..end]);
    Some(
        name.replace("%0A", "\n")
            .replace("%0a", "\n")
            .replace("%0D", "\r")
            .replace("%0d", "\r")
            .replace("%22", "\""),
    )
}

/// `parseMultipartFormDataHeaders`.
fn parse_part_headers(input: &[u8], pos: &mut usize) -> Option<PartHeaders> {
    let mut headers = PartHeaders::default();
    loop {
        if is_crlf_at(input, *pos) {
            headers.name.as_ref()?;
            return Some(headers);
        }
        let (start, end) = collect_bytes(input, pos, |b| b != b'\n' && b != b'\r' && b != b':');
        let header_name = remove_chars(&input[start..end], true, true, |b| b == b'\t' || b == b' ');
        if header_name.is_empty() || !header_name.iter().all(|&b| is_token_char(char::from(b))) {
            return None;
        }
        if input.get(*pos) != Some(&b':') {
            return None;
        }
        *pos += 1;
        collect_bytes(input, pos, |b| b == b' ' || b == b'\t');

        match header_name.to_ascii_lowercase().as_slice() {
            b"content-disposition" => {
                headers.name = None;
                headers.filename = None;
                if !starts_with_at(input, b"form-data; name=\"", *pos) {
                    return None;
                }
                *pos += 17;
                headers.name = Some(parse_part_name(input, pos)?);
                if starts_with_at(input, b"; filename", *pos) {
                    let mut check = *pos + 10;
                    if input.get(check) == Some(&b'*') {
                        *pos += 1;
                        check += 1;
                    }
                    if input.get(check) != Some(&b'=') || input.get(check + 1) != Some(&b'"') {
                        return None;
                    }
                    *pos += 12;
                    headers.filename = Some(parse_part_name(input, pos)?);
                }
            }
            b"content-type" | b"content-transfer-encoding" => {
                let is_type = header_name.eq_ignore_ascii_case(b"content-type");
                let (start, end) = collect_bytes(input, pos, |b| b != b'\n' && b != b'\r');
                let value =
                    remove_chars(&input[start..end], false, true, |b| b == b'\t' || b == b' ');
                let value = latin1_decode(value);
                if is_type {
                    headers.content_type = Some(value);
                } else {
                    headers.encoding = Some(value);
                }
            }
            _ => {
                collect_bytes(input, pos, |b| b != b'\n' && b != b'\r');
            }
        }
        // V1 quirk: `&&`, so only a position where NEITHER byte matches fails.
        if input.get(*pos) != Some(&b'\r') && input.get(pos.saturating_add(1)) != Some(&b'\n') {
            return None;
        }
        *pos += 2;
    }
}

/// Node's lenient `Buffer.from(str, "base64")`: both alphabets, other bytes skipped, decoding
/// stops at the first `=`.
fn node_base64_decode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for &b in input {
        let v = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            _ => continue,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

/// `multipartFormDataParser(input, mimeType)`; `None` = "failure".
fn parse_multipart(input: &[u8], boundary_string: &str) -> Option<Vec<(String, FormValue)>> {
    // `Buffer.from("--" + boundary, "utf8")`: the (latin1-decoded) parameter is UTF-8 encoded.
    let boundary = format!("--{boundary_string}").into_bytes();
    let mut entries = Vec::new();
    let mut pos = 0usize;
    while is_crlf_at(input, pos) {
        pos += 2;
    }
    let mut trailing = input.len();
    while trailing >= 2 && input[trailing - 1] == b'\n' && input[trailing - 2] == b'\r' {
        trailing -= 2;
    }
    let input = &input[..trailing];

    loop {
        if input.get(pos..pos.saturating_add(boundary.len())) == Some(boundary.as_slice()) {
            pos += boundary.len();
        } else {
            return None;
        }
        if (pos + 2 == input.len() && starts_with_at(input, b"--", pos))
            || (pos + 4 == input.len() && starts_with_at(input, b"--\r\n", pos))
        {
            return Some(entries);
        }
        if !is_crlf_at(input, pos) {
            return None;
        }
        pos += 2;
        let headers = parse_part_headers(input, &mut pos)?;
        pos += 2;

        // V1 quirk: searches the boundary WITHOUT its leading dashes and assumes "\r\n--" before.
        let boundary_index = index_of(input, &boundary[2..], pos)?;
        let body_end = boundary_index.saturating_sub(4);
        let body: &[u8] = if body_end > pos {
            &input[pos..body_end]
        } else {
            &[]
        };
        pos += body.len();
        let data: Vec<u8> = if headers.encoding.as_deref() == Some("base64") {
            node_base64_decode(body)
        } else {
            body.to_vec()
        };

        if !is_crlf_at(input, pos) {
            return None;
        }
        pos += 2;

        let name = headers.name.unwrap_or_default();
        let value = match headers.filename {
            Some(filename) => {
                let mut content_type = headers
                    .content_type
                    .unwrap_or_else(|| "text/plain".to_owned());
                if !content_type.is_ascii() {
                    content_type = String::new();
                }
                FormValue::File(FormFile {
                    filename,
                    content_type: blob_type(&content_type),
                    data: Bytes::from(data),
                })
            }
            None => FormValue::Text(utf8_decode_bytes(&data)),
        };
        entries.push((name, value));
    }
}

// ---------------------------------------------------------------------------------------------
// application/x-www-form-urlencoded
// ---------------------------------------------------------------------------------------------

/// `new URLSearchParams(body.toString())`: lossy UTF-8 (BOM kept), one leading `?` removed,
/// then the WHATWG urlencoded parser. Every value is a string.
fn parse_urlencoded(body: &[u8]) -> Vec<(String, FormValue)> {
    let text = String::from_utf8_lossy(body);
    let text = text.strip_prefix('?').unwrap_or(&text);
    url::form_urlencoded::parse(text.as_bytes())
        .map(|(k, v)| (k.into_owned(), FormValue::Text(v.into_owned())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: &str = "----WebKitFormBoundaryABC123xyz";

    fn mp(parts: &[(&[&str], &[u8])], lead: &str, tail: &str) -> Vec<u8> {
        let mut out = lead.as_bytes().to_vec();
        for (headers, body) in parts {
            out.extend_from_slice(format!("--{B}\r\n{}\r\n\r\n", headers.join("\r\n")).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\r\n");
        }
        out.extend_from_slice(format!("--{B}{tail}").as_bytes());
        out
    }

    fn ct() -> String {
        format!("multipart/form-data; boundary={B}")
    }

    fn text(s: &str) -> FormValue {
        FormValue::Text(s.to_owned())
    }

    fn file(name: &str, ct: &str, data: &[u8]) -> FormValue {
        FormValue::File(FormFile {
            filename: name.to_owned(),
            content_type: ct.to_owned(),
            data: Bytes::copy_from_slice(data),
        })
    }

    fn parse(body: &[u8], content_type: &str) -> Result<Vec<(String, FormValue)>, InvalidForm> {
        parse_form(Some(content_type), body).map(|f| f.entries().to_vec())
    }

    fn one(headers: &[&str], body: &[u8]) -> Vec<u8> {
        mp(&[(headers, body)], "", "--\r\n")
    }

    #[test]
    fn standard_browser_body() {
        let body = mp(
            &[
                (
                    &[
                        "Content-Disposition: form-data; name=\"audio\"; filename=\"x.webm\"",
                        "Content-Type: audio/webm;codecs=opus",
                    ],
                    b"AUDIO",
                ),
                (
                    &["Content-Disposition: form-data; name=\"provider\""],
                    b"openai",
                ),
                (
                    &["Content-Disposition: form-data; name=\"language\""],
                    b"en",
                ),
                (
                    &["Content-Disposition: form-data; name=\"serverUrl\""],
                    b"  http://h:1  ",
                ),
            ],
            "",
            "--\r\n",
        );
        let form = parse_form(Some(&ct()), &body).unwrap_or_default();
        assert_eq!(
            form.get("audio"),
            Some(&file("x.webm", "audio/webm;codecs=opus", b"AUDIO"))
        );
        assert_eq!(form.get("provider"), Some(&text("openai")));
        assert_eq!(form.get("language"), Some(&text("en")));
        assert_eq!(form.get("serverUrl"), Some(&text("  http://h:1  ")));
    }

    #[test]
    fn part_content_type_normalization() {
        let cd = "Content-Disposition: form-data; name=\"audio\"; filename=\"a\"";
        assert_eq!(
            parse(&one(&[cd], b"X"), &ct()),
            Ok(vec![("audio".into(), file("a", "text/plain", b"X"))])
        );
        assert_eq!(
            parse(&one(&[cd, "Content-Type: "], b"X"), &ct()),
            Ok(vec![("audio".into(), file("a", "", b"X"))])
        );
        assert_eq!(
            parse(
                &one(&[cd, "Content-Type: Audio/WebM; Codecs=OPUS  "], b"X"),
                &ct()
            ),
            Ok(vec![(
                "audio".into(),
                file("a", "audio/webm; codecs=opus", b"X")
            )])
        );
        assert_eq!(
            parse(&one(&[cd, "Content-Type: audio/webm;\tx=1"], b"X"), &ct()),
            Ok(vec![("audio".into(), file("a", "", b"X"))])
        );
        let latin1 = one(&[cd, "Content-Type: audio/\u{0}"], b"X");
        let latin1: Vec<u8> = latin1
            .iter()
            .map(|&b| if b == 0 { 0xE9 } else { b })
            .collect();
        assert_eq!(
            parse(&latin1, &ct()),
            Ok(vec![("audio".into(), file("a", "", b"X"))])
        );
        assert_eq!(
            parse(&one(&[cd, "Content-Type:    audio/mp4   "], b"x"), &ct()),
            Ok(vec![("audio".into(), file("a", "audio/mp4", b"x"))])
        );
    }

    #[test]
    fn golden_edge_cases() {
        // Expected results produced by Node 22.22.0 `new Request(...).formData()`.
        let cd = |n: &str| format!("Content-Disposition: form-data; name=\"{n}\"");
        assert_eq!(
            parse(
                &one(&[&cd("provider")], b"\xef\xbb\xbf\xef\xbb\xbfgroq"),
                &ct()
            ),
            Ok(vec![("provider".into(), text("groq"))])
        );
        assert_eq!(
            parse(
                &one(
                    &["Content-Disposition: form-data; name=\"audio\"; filename=\"\""],
                    b"X"
                ),
                &ct()
            ),
            Ok(vec![("audio".into(), file("", "text/plain", b"X"))])
        );
        assert_eq!(
            parse(
                &one(
                    &[
                        "content-disposition: form-data; name=\"audio\"; filename=\"a\"",
                        "content-type: audio/ogg"
                    ],
                    b"X"
                ),
                &ct()
            ),
            Ok(vec![("audio".into(), file("a", "audio/ogg", b"X"))])
        );
        assert_eq!(
            parse(
                &one(&["Content-Disposition: form-data; name=audio"], b"X"),
                &ct()
            ),
            Err(InvalidForm)
        );
        assert_eq!(
            parse(
                &one(&["Content-Disposition: Form-Data; name=\"a\""], b"X"),
                &ct()
            ),
            Err(InvalidForm)
        );
        assert_eq!(
            parse(&mp(&[(&[&cd("a")], b"X")], "\r\n\r\n", "--"), &ct()),
            Ok(vec![("a".into(), text("X"))])
        );
        assert_eq!(
            parse(&mp(&[(&[&cd("a")], b"X")], "junk\r\n", "--\r\n"), &ct()),
            Err(InvalidForm)
        );
        assert_eq!(
            parse(
                &one(&[&cd("a")], b"X"),
                &format!("Multipart/Form-Data; boundary=\"{B}\"")
            ),
            Ok(vec![("a".into(), text("X"))])
        );
        assert_eq!(
            parse(&one(&[&cd("a")], b"X"), "multipart/form-data"),
            Err(InvalidForm)
        );
        let dup = mp(
            &[
                (&[&cd("provider")], b"first"),
                (&[&cd("provider")], b"second"),
            ],
            "",
            "--\r\n",
        );
        let form = parse_form(Some(&ct()), &dup).unwrap_or_default();
        assert_eq!(form.get("provider"), Some(&text("first")));
        assert_eq!(form.entries().len(), 2);
        assert_eq!(
            parse(&one(&[&cd("a%22b%0aC%0D")], b"X"), &ct()),
            Ok(vec![("a\"b\nC\r".into(), text("X"))])
        );
        assert_eq!(
            parse(
                &one(
                    &[
                        "Content-Disposition: form-data; name=\"audio\"; filename=\"a\"",
                        "Content-Transfer-Encoding: base64"
                    ],
                    b"aGVs bG8*\r\nd29y=bGQ"
                ),
                &ct()
            ),
            Ok(vec![(
                "audio".into(),
                file("a", "text/plain", b"hello\x1d\xdb\xdc")
            )])
        );
        assert_eq!(
            parse(
                &one(
                    &[&cd("t"), "Content-Transfer-Encoding: base64"],
                    b"aGVsbG8gd29ybGQ"
                ),
                &ct()
            ),
            Ok(vec![("t".into(), text("hello world"))])
        );
        assert_eq!(
            parse(
                &one(&[&cd("a")], b"X"),
                &format!("text/plain, multipart/form-data; boundary={B}")
            ),
            Ok(vec![("a".into(), text("X"))])
        );
        assert_eq!(
            parse(
                &one(&[&cd("a")], b"X"),
                &format!("multipart/form-data; boundary={B}, */*")
            ),
            Ok(vec![("a".into(), text("X"))])
        );
        let inner = format!("AB{B}CD");
        assert_eq!(
            parse(
                &one(
                    &["Content-Disposition: form-data; name=\"audio\"; filename=\"a\""],
                    inner.as_bytes()
                ),
                &ct()
            ),
            Err(InvalidForm)
        );
        assert_eq!(
            parse(
                &one(
                    &["Content-Disposition: form-data; name=\"a\"; filename*=\"x\""],
                    b"X"
                ),
                &ct()
            ),
            Ok(vec![("a".into(), file("x", "text/plain", b"X"))])
        );
        assert_eq!(
            parse(
                &one(
                    &["Content-Disposition: form-data; name=\"a\"; filename = \"x\""],
                    b"X"
                ),
                &ct()
            ),
            Err(InvalidForm)
        );
        assert_eq!(
            parse(
                &one(
                    &["Content-Disposition: form-data; name=\"a\"; foo=\"x\""],
                    b"X"
                ),
                &ct()
            ),
            Err(InvalidForm)
        );
        assert_eq!(
            parse(&one(&[&cd("a"), "X-Foo: bar"], b"X"), &ct()),
            Ok(vec![("a".into(), text("X"))])
        );
        assert_eq!(
            parse(&one(&[&cd("a"), "X Foo: bar"], b"X"), &ct()),
            Err(InvalidForm)
        );
        assert_eq!(parse(&one(&[], b"X"), &ct()), Err(InvalidForm));
        assert_eq!(parse(b"", &ct()), Err(InvalidForm));
        assert_eq!(parse(format!("--{B}--").as_bytes(), &ct()), Ok(vec![]));
        assert_eq!(
            parse(format!("--{B}--\r\n\r\n").as_bytes(), &ct()),
            Ok(vec![])
        );
        assert_eq!(
            parse(
                &one(&[&cd("a")], b"X"),
                &format!("multipart/form-data;charset=utf-8; BOUNDARY={B}; boundary=other")
            ),
            Ok(vec![("a".into(), text("X"))])
        );
        assert_eq!(
            parse(&one(&[&cd("provider")], b"a\xff\xfeb"), &ct()),
            Ok(vec![("provider".into(), text("a\u{fffd}\u{fffd}b"))])
        );
        assert_eq!(
            parse(
                &one(&["Content-Disposition: form-data; name=\"\u{3042}\""], b"X"),
                &ct()
            ),
            Ok(vec![("\u{3042}".into(), text("X"))])
        );
        assert_eq!(
            parse(
                &one(&[" Content-Disposition : form-data; name=\"a\""], b"x"),
                &ct()
            ),
            Ok(vec![("a".into(), text("x"))])
        );
        assert_eq!(
            parse(&one(&[&cd("a")], b"x\r\ny\r\n"), &ct()),
            Ok(vec![("a".into(), text("x\r\ny\r\n"))])
        );
        assert_eq!(
            parse(
                &one(&[&cd("a")], b"x"),
                &format!("  multipart/form-data ;  boundary={B}  ")
            ),
            Ok(vec![("a".into(), text("x"))])
        );
        assert_eq!(
            parse(&one(&[&cd("a")], b"x"), "multipart"),
            Err(InvalidForm)
        );
    }

    #[test]
    fn header_line_quirks() {
        let lf_only = format!("--{B}\nContent-Disposition: form-data; name=\"a\"\n\nx\n--{B}--");
        assert_eq!(parse(lf_only.as_bytes(), &ct()), Err(InvalidForm));
        let bare_cr =
            format!("--{B}\r\nContent-Disposition: form-data; name=\"a\"\rX\r\n\r\nx\r\n--{B}--");
        assert_eq!(
            parse(bare_cr.as_bytes(), &ct()),
            Ok(vec![("a".into(), text("\r\nx"))])
        );
        let lf_after =
            format!("--{B}\r\nContent-Disposition: form-data; name=\"a\"Z\n\r\n\r\nx\r\n--{B}--");
        assert_eq!(
            parse(lf_after.as_bytes(), &ct()),
            Ok(vec![("a".into(), text("\r\nx"))])
        );
        let no_dash =
            format!("--{B}\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\nx\r\n--{B}");
        assert_eq!(parse(no_dash.as_bytes(), &ct()), Err(InvalidForm));
        let junk =
            format!("--{B}\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\nx\r\n--{B}--junk");
        assert_eq!(parse(junk.as_bytes(), &ct()), Err(InvalidForm));
        let twice = format!("--{B}\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"f\"\r\nContent-Disposition: form-data; name=\"b\"\r\n\r\nx\r\n--{B}--");
        assert_eq!(
            parse(twice.as_bytes(), &ct()),
            Ok(vec![("b".into(), text("x"))])
        );
        let ct_first = format!("--{B}\r\nContent-Type: audio/ogg\r\nContent-Disposition: form-data; name=\"a\"; filename=\"f\"\r\n\r\nx\r\n--{B}--");
        assert_eq!(
            parse(ct_first.as_bytes(), &ct()),
            Ok(vec![("a".into(), file("f", "audio/ogg", b"x"))])
        );
        let upper_b64 = one(
            &[
                "Content-Disposition: form-data; name=\"a\"; filename=\"f\"",
                "Content-Transfer-Encoding: BASE64",
            ],
            b"aGk=",
        );
        assert_eq!(
            parse(&upper_b64, &ct()),
            Ok(vec![("a".into(), file("f", "text/plain", b"aGk="))])
        );
        let trail_b64 = one(
            &[
                "Content-Disposition: form-data; name=\"a\"; filename=\"f\"",
                "Content-Transfer-Encoding: base64 \t",
            ],
            b"aGk=",
        );
        assert_eq!(
            parse(&trail_b64, &ct()),
            Ok(vec![("a".into(), file("f", "text/plain", b"hi"))])
        );
        let empty_name = one(&["Content-Disposition: form-data; name=\"\""], b"x");
        assert_eq!(
            parse(&empty_name, &ct()),
            Ok(vec![(String::new(), text("x"))])
        );
    }

    #[test]
    fn boundary_is_utf8_encoded_from_latin1_header() {
        let latin1_body =
            b"--b\xe9\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\nx\r\n--b\xe9--";
        let utf8_body =
            b"--b\xc3\xa9\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\nx\r\n--b\xc3\xa9--";
        let header = "multipart/form-data; boundary=b\u{e9}";
        assert_eq!(parse(latin1_body, header), Err(InvalidForm));
        assert_eq!(parse(utf8_body, header), Ok(vec![("a".into(), text("x"))]));
    }

    #[test]
    fn urlencoded_bodies() {
        assert_eq!(
            parse(
                b"?a=b&serverUrl=http%3A%2F%2Fx+y&&c&audio=hello&a=dup",
                "application/x-www-form-urlencoded; charset=UTF-8"
            ),
            Ok(vec![
                ("a".into(), text("b")),
                ("serverUrl".into(), text("http://x y")),
                ("c".into(), text("")),
                ("audio".into(), text("hello")),
                ("a".into(), text("dup")),
            ])
        );
        assert_eq!(
            parse(b"\xef\xbb\xbfa=1", "application/x-www-form-urlencoded"),
            Ok(vec![("\u{feff}a".into(), text("1"))])
        );
        assert_eq!(
            parse(b"A=1&serverUrl=%zz%41", "Application/X-WWW-Form-Urlencoded"),
            Ok(vec![
                ("A".into(), text("1")),
                ("serverUrl".into(), text("%zzA"))
            ])
        );
    }

    #[test]
    fn rejected_content_types() {
        assert_eq!(parse_form(Some("text/plain"), b"a=b"), Err(InvalidForm));
        assert_eq!(parse_form(None, b"a=b"), Err(InvalidForm));
        assert_eq!(
            parse_form(Some("application/json"), b"{}"),
            Err(InvalidForm)
        );
        assert_eq!(parse_form(Some(""), b"a=b"), Err(InvalidForm));
    }

    #[test]
    fn mime_parsing() {
        let m = parse_mime_type("Text/HTML; Charset=\"ut\\f-8\"; x=; y=\"\"; charset=other")
            .unwrap_or(MimeType {
                essence: String::new(),
                parameters: vec![],
            });
        assert_eq!(m.essence, "text/html");
        assert_eq!(m.parameter("charset"), Some("utf-8"));
        assert_eq!(m.parameter("x"), None);
        assert_eq!(m.parameter("y"), Some(""));
        assert_eq!(parse_mime_type("text"), None);
        assert_eq!(parse_mime_type("/html"), None);
        assert_eq!(parse_mime_type("te xt/html"), None);
        assert_eq!(
            split_header_values("a, \"b,c\" ,d"),
            vec!["a".to_owned(), "\"b,c\"".to_owned(), "d".to_owned()]
        );
        assert_eq!(node_base64_decode(b"ab==cd"), b"i".to_vec());
        assert_eq!(node_base64_decode(b"-_-_"), vec![0xfb, 0xff, 0xbf]);
        assert_eq!(node_base64_decode(b"a"), Vec::<u8>::new());
    }

    #[test]
    fn form_value_string_conversion() {
        assert_eq!(file("a", "", b"").to_js_string(), "[object File]");
        assert_eq!(text("x").to_js_string(), "x");
    }
}
