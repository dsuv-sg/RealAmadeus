//! Asset URL construction. Model paths (the model directory, model3.json and every file it
//! references) are file paths, so each path segment is percent-encoded as UTF-8 (the Kurisu files
//! contain `紅莉栖` → `%E7%B4%85%E8%8E%89%E6%A0%96`; a `#`, `?` or stray `%` in a file name can no
//! longer truncate the URL). The resources prefix is a URL (it may carry a scheme/host) and is used
//! as given. Everything is finally resolved against `document.baseURI` (D1).

/// Percent-encodes every byte of `path` except RFC 3986 `pchar`s (`A-Z a-z 0-9 - . _ ~`,
/// `! $ & ' ( ) * + , ; =`, `:`, `@`), the `/` separators and `%` that already starts a valid
/// `%XX` escape (browsers never re-encode those either).
pub fn encode_path(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    for (i, &b) in bytes.iter().enumerate() {
        let keep = b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'-' | b'.'
                    | b'_'
                    | b'~'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
                    | b':'
                    | b'@'
                    | b'/'
            )
            || (b == b'%'
                && bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit)
                && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit));
        if keep {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A model directory: the human-readable form V1 printed in its console messages and the encoded
/// URL prefix requests are built from (both end with `/` when non-empty).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModelHome {
    /// `resources_base + model_dir` as written (V1 `_modelHomeDir`, for logs).
    pub display: String,
    /// `resources_base + encode_path(model_dir)` (unresolved).
    pub url: String,
}

impl ModelHome {
    /// Joins the resources prefix (a URL) and the model directory (a path).
    pub fn new(resources_base: &str, model_dir: &str) -> Self {
        Self {
            display: format!("{resources_base}{model_dir}"),
            url: format!("{resources_base}{}", encode_path(model_dir)),
        }
    }

    /// `_modelHomeDir + fileName` as printed in V1 messages.
    pub fn display_path(&self, file: &str) -> String {
        format!("{}{file}", self.display)
    }

    /// The (unresolved) request URL of `file` (a path relative to the model directory).
    pub fn file_url(&self, file: &str) -> String {
        format!("{}{}", self.url, encode_path(file))
    }
}

/// Resolves `url` against `document.baseURI` (absolute `href`). Falls back to `url` unchanged when
/// there is no document or the URL cannot be parsed (the browser then resolves it the same way).
#[cfg(target_arch = "wasm32")]
pub fn resolve(url: &str) -> String {
    let base = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.base_uri().ok().flatten());
    match base {
        Some(base) => web_sys::Url::new_with_base(url, &base)
            .map(|u| u.href())
            .unwrap_or_else(|_| url.to_owned()),
        None => url.to_owned(),
    }
}

/// Host builds (tests) have no document: URLs stay as given.
#[cfg(not(target_arch = "wasm32"))]
pub fn resolve(url: &str) -> String {
    url.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_utf8_segments_and_keeps_separators() {
        assert_eq!(
            encode_path("Live2D紅莉栖forSDK5.0.8192/texture_00.png"),
            "Live2D%E7%B4%85%E8%8E%89%E6%A0%96forSDK5.0.8192/texture_00.png"
        );
        assert_eq!(
            encode_path("models/AmadeusKurisu5.0/reama5.0/"),
            "models/AmadeusKurisu5.0/reama5.0/"
        );
    }

    #[test]
    fn encodes_url_delimiters_but_keeps_escapes() {
        assert_eq!(encode_path("a b#c?.png"), "a%20b%23c%3F.png");
        assert_eq!(encode_path("100%.png"), "100%25.png");
        assert_eq!(encode_path("%E7%B4%85.png"), "%E7%B4%85.png");
        assert_eq!(encode_path("x%4"), "x%254");
        assert_eq!(encode_path("back\\slash"), "back%5Cslash");
        assert_eq!(
            encode_path("(a)+b=c,d;e:f@g!$&'*~"),
            "(a)+b=c,d;e:f@g!$&'*~"
        );
    }

    #[test]
    fn model_home_paths() {
        let home = ModelHome::new("/resources/", "models/AmadeusKurisu5.0/reama5.0/");
        assert_eq!(home.display, "/resources/models/AmadeusKurisu5.0/reama5.0/");
        assert_eq!(
            home.display_path("Live2D紅莉栖forSDK5.0.model3.json"),
            "/resources/models/AmadeusKurisu5.0/reama5.0/Live2D紅莉栖forSDK5.0.model3.json"
        );
        assert_eq!(
            home.file_url("Live2D紅莉栖forSDK5.0.moc3"),
            "/resources/models/AmadeusKurisu5.0/reama5.0/Live2D%E7%B4%85%E8%8E%89%E6%A0%96forSDK5.0.moc3"
        );
        // The resources prefix is a URL and is not re-encoded.
        let home = ModelHome::new("https://cdn.example/r%20x/", "m/");
        assert_eq!(home.file_url("a b"), "https://cdn.example/r%20x/m/a%20b");
    }
}
