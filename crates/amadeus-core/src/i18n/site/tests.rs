//! System B tests: exact V1 data, typed/raw consistency and `useLanguage` semantics.

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;

use super::*;

fn raw(locale: SiteLocale) -> Value {
    match serde_json::from_str(SITE_LOCALE_JSON[locale.slot()]) {
        Ok(value) => value,
        Err(err) => panic!("site/{}.json does not parse: {err}", locale.code()),
    }
}

/// Order-preserving tree of a site locale file. Order-sensitive checks cannot use
/// `serde_json::Value`: unless serde_json's `preserve_order` feature is enabled (Cargo feature
/// unification decides that, not this crate) its objects are key-sorted maps.
#[derive(Debug)]
enum Node {
    Str(String),
    Arr(Vec<Node>),
    Obj(Vec<(String, Node)>),
}

impl Node {
    /// The segments if this node is a rich-text leaf (`{"$rich": [segment, …]}`).
    fn rich_segments(&self) -> Option<&[Node]> {
        match self {
            Node::Obj(entries) => match entries.as_slice() {
                [(key, Node::Arr(segments))] if key == "$rich" => Some(segments),
                _ => None,
            },
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct NodeVisitor;

        impl<'de> Visitor<'de> for NodeVisitor {
            type Value = Node;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a string, an array or an object")
            }

            fn visit_str<E: de::Error>(self, text: &str) -> Result<Node, E> {
                Ok(Node::Str(text.to_owned()))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Node, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Node::Arr(items))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Node, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    entries.push(entry);
                }
                Ok(Node::Obj(entries))
            }
        }

        deserializer.deserialize_any(NodeVisitor)
    }
}

/// `json` as an order-preserving tree (`what` names it in the panic message).
fn parse_ordered(json: &str, what: &str) -> Node {
    match serde_json::from_str(json) {
        Ok(node) => node,
        Err(err) => panic!("{what} does not parse: {err}"),
    }
}

/// The embedded file of `locale` as an order-preserving tree.
fn ordered(locale: SiteLocale) -> Node {
    parse_ordered(
        SITE_LOCALE_JSON[locale.slot()],
        &format!("site/{}.json", locale.code()),
    )
}

fn is_rich(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|map| map.len() == 1 && map.contains_key("$rich"))
}

/// Exact structural comparison; returns the paths where `a` and `b` differ.
fn value_diffs(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for key in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) {
                let child = format!("{path}.{key}");
                match (x.get(key), y.get(key)) {
                    (Some(x), Some(y)) => value_diffs(x, y, &child, out),
                    (Some(_), None) => out.push(format!("{child}: only in typed")),
                    (None, _) => out.push(format!("{child}: only in JSON file")),
                }
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (x, y)) in x.iter().zip(y).enumerate() {
                value_diffs(x, y, &format!("{path}.{i}"), out);
            }
        }
        _ if a == b => {}
        _ => out.push(format!("{path}: {a} != {b}")),
    }
}

#[test]
fn both_locales_parse_into_the_typed_dict() {
    for locale in SiteLocale::ALL {
        let parsed = parse_site_dict(SITE_LOCALE_JSON[locale.slot()]);
        assert!(parsed.is_ok(), "{}: {parsed:?}", locale.code());
        assert_ne!(site_dict(locale), &SiteDict::default());
    }
}

#[test]
fn typed_dict_round_trips_to_the_raw_json() {
    // The typed structs are lenient at runtime; this round trip is what guarantees that they
    // carry every V1 leaf (no unknown key dropped, no expected key missing).
    for locale in SiteLocale::ALL {
        let typed = serde_json::to_value(site_dict(locale));
        assert!(typed.is_ok(), "{typed:?}");
        if let Ok(typed) = typed {
            let mut diffs = Vec::new();
            value_diffs(&typed, &raw(locale), "", &mut diffs);
            assert!(diffs.is_empty(), "{}: {diffs:#?}", locale.code());
        }
    }
}

#[test]
fn lenient_parsing_never_blanks_the_whole_dictionary() {
    let mut value = raw(SiteLocale::En);
    if let Some(docs) = value
        .pointer_mut("/amadeus/docs")
        .and_then(Value::as_object_mut)
    {
        docs.insert("futureKey".to_owned(), Value::from("x"));
        docs.remove("back");
    }
    let parsed = parse_site_dict(&value.to_string());
    assert!(parsed.is_ok(), "{parsed:?}");
    if let Ok(parsed) = parsed {
        assert_eq!(parsed.amadeus.docs.back, "");
        assert_eq!(parsed.amadeus.docs.sidebar.download, "Download");
        // ...while the round-trip guard pinpoints both schema slips.
        let mut diffs = Vec::new();
        if let Ok(typed) = serde_json::to_value(&parsed) {
            value_diffs(&typed, &value, "", &mut diffs);
        }
        assert_eq!(
            diffs,
            [
                ".amadeus.docs.back: only in typed",
                ".amadeus.docs.futureKey: only in JSON file"
            ]
        );
    }
}

#[test]
fn ja_and_en_have_identical_shapes() {
    // Same keys in the same order, same array lengths, rich text at the same positions.
    fn walk(a: &Node, b: &Node, path: &str, diffs: &mut Vec<String>) {
        match (a, b) {
            (Node::Str(_), Node::Str(_)) => {}
            _ if a.rich_segments().is_some() && b.rich_segments().is_some() => {}
            (Node::Arr(x), Node::Arr(y)) => {
                if x.len() != y.len() {
                    diffs.push(format!("{path}: length {} vs {}", x.len(), y.len()));
                }
                for (i, (x, y)) in x.iter().zip(y).enumerate() {
                    walk(x, y, &format!("{path}.{i}"), diffs);
                }
            }
            (Node::Obj(x), Node::Obj(y))
                if a.rich_segments().is_none() && b.rich_segments().is_none() =>
            {
                let xk: Vec<&str> = x.iter().map(|(k, _)| k.as_str()).collect();
                let yk: Vec<&str> = y.iter().map(|(k, _)| k.as_str()).collect();
                if xk != yk {
                    diffs.push(format!("{path}: keys {xk:?} vs {yk:?}"));
                }
                for ((k, x), (_, y)) in x.iter().zip(y) {
                    walk(x, y, &format!("{path}.{k}"), diffs);
                }
            }
            _ => diffs.push(format!("{path}: different kinds")),
        }
    }
    let mut diffs = Vec::new();
    walk(
        &ordered(SiteLocale::Ja),
        &ordered(SiteLocale::En),
        "",
        &mut diffs,
    );
    assert!(diffs.is_empty(), "shape differences: {diffs:#?}");
}

#[test]
fn locale_state_semantics_match_use_language() {
    assert_eq!(SiteLocale::default(), SiteLocale::Ja);
    assert_eq!(SiteLocale::INITIAL, SiteLocale::Ja);
    assert_eq!(SiteLocale::Ja.toggled(), SiteLocale::En);
    assert_eq!(SiteLocale::En.toggled(), SiteLocale::Ja);
    assert_eq!(SiteLocale::Ja.toggled().toggled(), SiteLocale::Ja);
    assert_eq!(SiteLocale::Ja.toggle_label(), "EN");
    assert_eq!(SiteLocale::En.toggle_label(), "JP");
    assert_eq!(SiteLocale::Ja.pick("戻る", "Return"), "戻る");
    assert_eq!(
        SiteLocale::En.pick("ダウンロードへ", "BACK TO DOWNLOAD"),
        "BACK TO DOWNLOAD"
    );
    for locale in SiteLocale::ALL {
        assert_eq!(SiteLocale::from_code(locale.code()), Some(locale));
    }
    assert_eq!(SiteLocale::from_code("JA"), None);
    assert_eq!(SiteLocale::from_code("zh"), None);
}

#[test]
fn golden_values() {
    let ja = site_dict(SiteLocale::Ja);
    let en = site_dict(SiteLocale::En);

    assert_eq!(ja.amadeus.docs.sidebar.download, "ダウンロード");
    assert_eq!(en.amadeus.docs.sidebar.download, "Download");
    assert_eq!(ja.amadeus.docs.sidebar.api_openai, "OpenAI API");
    assert_eq!(ja.amadeus.docs.back, "システムへ戻る");
    assert_eq!(en.amadeus.docs.back, "RETURN TO SYSTEM");
    assert_eq!(ja.amadeus.release.title, "SYSTEM V2.0 RELEASED");
    assert_eq!(
        ja.amadeus.release.message,
        "Real Amadeus V2.0が正式にリリースされました。"
    );
    assert_eq!(
        en.amadeus.release.message,
        "REAL AMADEUS V2.0 HAS BEEN OFFICIALLY RELEASED."
    );
    assert_eq!(
        ja.amadeus.release.lightweight_windows_download,
        "軽量化版Windows"
    );
    assert_eq!(ja.home.role, "Lead Architect at ELVELT");
    assert_eq!(en.home.tagline, "Architecting the Unknown.");
    assert_eq!(ja.common.system_online, "SYSTEM ONLINE");
    assert_eq!(
        ja.amadeus.docs.download.file_structure.title,
        "ファイル構成について"
    );
    assert!(en
        .amadeus
        .docs
        .download
        .file_structure
        .windows_qt
        .starts_with("After extraction"));
    assert_eq!(
        ja.amadeus.docs.download.notice,
        "※ Windows以外のOSでは動作未確認の場合があります。問題が発生した場合は、GitHub Issuesにて報告をお願いします。"
    );

    // Changelog: 6 versions, newest first; working-tree V1.4 product is "RealAmadeus".
    for dict in [ja, en] {
        let versions = &dict.amadeus.docs.changelog.versions;
        let names: Vec<&str> = versions.iter().map(|v| v.version.as_str()).collect();
        assert_eq!(names, ["V1.4", "V1.3", "V1.2", "V1.1", "V1.0.1", "V1.0"]);
        let counts: Vec<usize> = versions.iter().map(|v| v.items.len()).collect();
        assert_eq!(counts, [9, 14, 9, 6, 4, 2]);
        let products: Vec<&str> = versions.iter().map(|v| v.product.as_str()).collect();
        assert_eq!(
            products,
            [
                "RealAmadeus",
                "RealAmadeus",
                "RealAmadeusQT",
                "RealAmadeus",
                "RealAmadeus",
                "RealAmadeus"
            ]
        );
    }
    assert_eq!(ja.amadeus.docs.changelog.title, "変更履歴");
    assert_eq!(en.amadeus.docs.changelog.title, "CHANGELOG");
    assert_eq!(
        ja.amadeus.docs.changelog.versions[0].items[0],
        "起動メッセージを実装"
    );
    assert_eq!(
        en.amadeus.docs.changelog.versions[5].items[1],
        "Implemented basic conversation features"
    );

    // Controls: 5 items (the working tree added "Alt + Enter / F11").
    let keys = |d: &SiteDict| -> Vec<String> {
        d.amadeus
            .docs
            .controls
            .items
            .iter()
            .map(|i| i.key.clone())
            .collect()
    };
    assert_eq!(
        keys(ja),
        [
            "Enter",
            "Tab / 右クリック",
            "W A S D / 矢印",
            "Backspace",
            "Alt + Enter / F11"
        ]
    );
    assert_eq!(
        keys(en),
        [
            "Enter",
            "Tab / Right-click",
            "W A S D / Arrows",
            "Backspace",
            "Alt + Enter / F11"
        ]
    );
    assert_eq!(
        ja.amadeus.docs.controls.items[4].desc,
        "フルスクリーン / ウィンドウモード切替"
    );
    assert_eq!(
        en.amadeus.docs.controls.items[4].desc,
        "Toggle Fullscreen / Windowed Mode"
    );

    assert_eq!(ja.amadeus.docs.login_doc.id_value, "Salieri");
    assert_eq!(en.amadeus.docs.login_doc.pass_value, "MakiseKurisu");
    assert_eq!(ja.amadeus.docs.api.steps.len(), 4);
    assert_eq!(ja.amadeus.docs.api.model_labels.title, "利用可能なモデル");
    assert_eq!(en.amadeus.docs.api.model_labels.recommended, "Recommended");

    // Model lists are identical in both locales.
    let gemini = [
        "gemini-4-carbon",
        "gemini-4-argon",
        "gemini-4-neon",
        "gemini-4-hydrogen",
        "gemini-3.8-flash",
        "gemini-3.6-flash",
        "gemini-3.5-flash-lite",
    ];
    for dict in [ja, en] {
        let docs = &dict.amadeus.docs;
        assert_eq!(
            docs.api_openai.model_list.available,
            [
                "gpt-6.2-astra",
                "gpt-6.2-sol",
                "gpt-6.1-terra",
                "gpt-6-luna",
                "gpt-5.6-sol",
                "gpt-5.6-terra",
                "gpt-5.6-luna"
            ]
        );
        assert_eq!(docs.api_openai.model_list.recommended, "gpt-6.2-sol");
        assert_eq!(docs.api_gemini.model_list.available, gemini);
        assert_eq!(docs.api_vertex.model_list.available, gemini);
        assert_eq!(docs.api_gemini.model_list.recommended, "gemini-3.8-flash");
        assert_eq!(docs.api_vertex.model_list.recommended, "gemini-3.8-flash");
        assert_eq!(
            docs.api_claude.model_list.available,
            [
                "claude-fable-5-2",
                "claude-opus-5-5",
                "claude-sonnet-5-5",
                "claude-haiku-5-5",
                "claude-opus-4-6",
                "claude-sonnet-4-6"
            ]
        );
        assert_eq!(docs.api_claude.model_list.recommended, "claude-sonnet-5-5");
        assert_eq!(
            docs.api_groq.model_list.available,
            [
                "qwen/qwen3.8-27b",
                "qwen/qwen3.6-27b",
                "openai/gpt-oss-120b",
                "openai/gpt-oss-20b",
                "groq/compound",
                "llama-3.3-70b-versatile",
                "llama-3.1-8b-instant"
            ]
        );
        assert_eq!(docs.api_groq.model_list.recommended, "qwen/qwen3.8-27b");

        // Step counts and which contents are rich (JSX) in V1.
        let rich_positions = |p: &ProviderDoc| -> Vec<bool> {
            p.steps
                .iter()
                .map(|s| s.content.as_rich().is_some())
                .collect()
        };
        assert_eq!(
            rich_positions(&docs.api_openai),
            [true, false, false, false]
        );
        assert_eq!(rich_positions(&docs.api_gemini), [true, false, false]);
        assert_eq!(
            rich_positions(&docs.api_claude),
            [true, false, false, false]
        );
        assert_eq!(rich_positions(&docs.api_groq), [true, false, false]);
        assert_eq!(
            rich_positions(&docs.api_vertex),
            [true, false, false, false, true, false]
        );
        assert_eq!(docs.download.steps.len(), 2);
        assert!(docs
            .download
            .steps
            .iter()
            .all(|s| s.content.as_plain().is_some()));
    }
}

#[test]
fn rich_text_leaves_are_exact() {
    let link = |href: &str, text: &str| Seg::Link {
        href: href.to_owned(),
        text: text.to_owned(),
    };
    let text = |t: &str| Seg::Text { text: t.to_owned() };
    let code = |t: &str| Seg::Code { text: t.to_owned() };

    let ja = site_dict(SiteLocale::Ja);
    let en = site_dict(SiteLocale::En);
    assert_eq!(
        ja.amadeus.docs.download.fallback_standard.segments,
        [
            text("上記のボタンからダウンロードできない場合は、"),
            link(
                "https://github.com/dsuv-sg/RealAmadeusUnity/releases",
                "RealAmadeusUnity Releases"
            ),
            text("（Unity版・レガシー）から直接ダウンロードしてください。"),
        ]
    );
    assert_eq!(
        en.amadeus.docs.download.fallback_lightweight.segments,
        [
            text("If the download buttons above do not work, please download directly from "),
            link(
                "https://github.com/dsuv-sg/RealAmadeusQT/releases",
                "RealAmadeusQT Releases"
            ),
            text(" (Qt / Legacy)."),
        ]
    );
    assert_eq!(
        ja.amadeus.docs.api_openai.steps[0].content,
        StepContent::Rich(RichText {
            segments: vec![
                link(
                    "https://platform.openai.com/",
                    "OpenAI Platform (platform.openai.com)"
                ),
                text(" にアクセスし、アカウントを登録します。"),
            ]
        })
    );
    assert_eq!(
        en.amadeus.docs.api_claude.steps[0].content.plain_text(),
        "Register at the Anthropic Console (console.anthropic.com)."
    );
    let installer =
        "https://dl.google.com/dl/cloudsdk/channels/rapid/GoogleCloudSDKInstaller.exe?hl=ja";
    assert_eq!(
        en.amadeus.docs.api_vertex.steps[4]
            .content
            .as_rich()
            .map(|r| r.segments.clone()),
        Some(vec![
            text("Download and install the "),
            link(installer, "Google Cloud CLI (gcloud) Installer"),
            text(". After installation, open Command Prompt (cmd) and run "),
            code("gcloud auth login"),
            text(" to authenticate."),
        ])
    );
    assert_eq!(
        ja.amadeus.docs.api_vertex.steps[4].content.plain_text(),
        "Google Cloud CLI (gcloud) Installer をダウンロード、インストールしてセットアップします。その後、コマンドプロンプト(cmd)を開き、gcloud auth loginを実行してログインします。"
    );
    assert_eq!(code("x").text(), "x");
    assert_eq!(StepContent::default().as_plain(), Some(""));
    assert_eq!(
        LINK_CLASS,
        "text-[#d49e38] underline hover:text-[#f0b95b] transition-colors"
    );
    assert_eq!((LINK_TARGET, LINK_REL), ("_blank", "noopener noreferrer"));
}

#[test]
fn path_accessors() {
    use SiteLocale::{En, Ja};
    assert_eq!(site_t(Ja, "amadeus.docs.sidebar.download"), "ダウンロード");
    assert_eq!(site_t(En, "amadeus.docs.sidebar.download"), "Download");
    assert_eq!(site_t(En, "amadeus.docs.sidebar.apiOpenAI"), "OpenAI API");
    assert_eq!(
        site_t(Ja, "amadeus.docs.changelog.versions.2.product"),
        "RealAmadeusQT"
    );
    assert_eq!(
        site_t(En, "amadeus.docs.apiOpenAI.steps.1.content"),
        "You must register a credit card (Pay as you go) via the Billing section."
    );
    assert_eq!(
        site_get(Ja, "amadeus.docs.download.fileStructure.windowsQT")
            .map(|s| s.starts_with("解凍後")),
        Some(true)
    );

    // Missing paths, non-canonical indices and non-string nodes fall back to the path.
    for path in [
        "amadeus.docs.nope",
        "amadeus.docs.changelog.versions.6.version",
        "amadeus.docs.changelog.versions.01.version",
        "amadeus.docs.changelog.versions.+1.version",
        "amadeus.docs.changelog.versions.-1.version",
        "amadeus.docs.sidebar.download.extra",
        "amadeus.docs.sidebar",
        "amadeus.docs.download.fallbackStandard",
        "",
        ".",
    ] {
        assert_eq!(site_get(Ja, path), None, "{path}");
        assert_eq!(site_t(Ja, path), path, "{path}");
    }
    assert!(site_value(Ja, "").is_some_and(Value::is_object));
    assert!(site_value(Ja, "amadeus.docs.changelog.versions.0").is_some_and(Value::is_object));

    assert_eq!(
        site_strings(En, "amadeus.docs.apiGroq.modelList.available"),
        site_dict(En).amadeus.docs.api_groq.model_list.available
    );
    assert_eq!(site_strings(Ja, "amadeus.docs.api.steps").len(), 4);
    assert!(site_strings(Ja, "amadeus.docs.title").is_empty());
    assert!(site_strings(Ja, "missing").is_empty());

    assert_eq!(
        site_rich(Ja, "amadeus.docs.download.fallbackStandard"),
        Some(
            site_dict(Ja)
                .amadeus
                .docs
                .download
                .fallback_standard
                .clone()
        )
    );
    assert_eq!(
        site_rich(En, "amadeus.docs.apiGemini.steps.1.content").map(|r| r.plain_text()),
        Some("Select 'Get API key' and generate a new key for a new project.".to_owned())
    );
    assert_eq!(site_rich(En, "amadeus.docs.apiGemini.steps"), None);
    assert_eq!(site_rich(En, "amadeus.docs.sidebar"), None);
    assert_eq!(site_rich(En, "missing"), None);
}

#[test]
fn typed_and_path_access_agree_on_every_string_leaf() {
    fn leaves(value: &Value, path: String, out: &mut Vec<(String, String)>) {
        match value {
            Value::String(s) => out.push((path, s.clone())),
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    leaves(item, format!("{path}.{i}"), out);
                }
            }
            Value::Object(map) if !is_rich(value) => {
                for (k, v) in map {
                    let child = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{path}.{k}")
                    };
                    leaves(v, child, out);
                }
            }
            _ => {}
        }
    }
    for locale in SiteLocale::ALL {
        let mut out = Vec::new();
        leaves(&raw(locale), String::new(), &mut out);
        assert_eq!(out.len(), 260, "{} string leaves", locale.code());
        for (path, value) in out {
            assert_eq!(site_get(locale, &path), Some(value.as_str()), "{path}");
        }
    }
}

#[test]
fn site_json_files_are_clean() {
    for locale in SiteLocale::ALL {
        let raw = SITE_LOCALE_JSON[locale.slot()];
        assert!(!raw.starts_with('\u{FEFF}') && !raw.contains('\r') && raw.ends_with('\n'));
    }
}

/// Strict sequential matcher over V1 source text. Before each expected token only whitespace
/// and commas may be skipped, so a successful walk proves that the source spells out exactly
/// the expected tokens, in order, with nothing else in between.
struct Literal<'a> {
    src: &'a str,
    pos: usize,
}

impl Literal<'_> {
    fn skip_separators(&mut self) {
        let rest = &self.src[self.pos..];
        let kept = rest.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
        self.pos += rest.len() - kept.len();
    }

    fn error(&self, what: &str) -> String {
        format!(
            "line {}: {what}, found {:?}",
            self.src[..self.pos].matches('\n').count() + 1,
            self.src[self.pos..].chars().take(80).collect::<String>()
        )
    }

    fn expect(&mut self, token: &str) -> Result<(), String> {
        self.skip_separators();
        if self.src[self.pos..].starts_with(token) {
            self.pos += token.len();
            Ok(())
        } else {
            Err(self.error(&format!("expected {token:?}")))
        }
    }

    fn finish(&mut self) -> Result<(), String> {
        self.skip_separators();
        if self.pos == self.src.len() {
            Ok(())
        } else {
            Err(self.error("expected the end of the literal"))
        }
    }
}

/// V1 JSX spelling of a rich-text leaf: `<>…</>`, every link with the uniform V1 attributes
/// ([`LINK_TARGET`], [`LINK_REL`], [`LINK_CLASS`]) and `<code>` without props.
fn jsx(segments: &[Node]) -> Result<String, String> {
    let mut out = String::from("<>");
    for segment in segments {
        let Node::Obj(fields) = segment else {
            return Err(format!("segment is not an object: {segment:?}"));
        };
        let field = |name: &str| {
            fields.iter().find_map(|(key, value)| match value {
                Node::Str(text) if key == name => Some(text.as_str()),
                _ => None,
            })
        };
        match (fields.len(), field("type"), field("text"), field("href")) {
            (2, Some("text"), Some(text), None) => out.push_str(text),
            (2, Some("code"), Some(text), None) => {
                out.push_str("<code>");
                out.push_str(text);
                out.push_str("</code>");
            }
            (3, Some("link"), Some(text), Some(href)) => out.push_str(&format!(
                "<a href=\"{href}\" target=\"{LINK_TARGET}\" rel=\"{LINK_REL}\" className=\"{LINK_CLASS}\">{text}</a>"
            )),
            _ => return Err(format!("unexpected segment {segment:?}")),
        }
    }
    out.push_str("</>");
    Ok(out)
}

/// Expects `node` (under `key` when it is an object entry) in V1 object-literal syntax:
/// `key: "text"`, `key: [ … ]`, `key: { … }` or `key: <>…</>`.
fn expect_node(lit: &mut Literal<'_>, key: Option<&str>, node: &Node) -> Result<(), String> {
    let prefix = key.map(|key| format!("{key}: ")).unwrap_or_default();
    if let Some(segments) = node.rich_segments() {
        return lit.expect(&format!("{prefix}{}", jsx(segments)?));
    }
    match node {
        Node::Str(text) => lit.expect(&format!("{prefix}\"{text}\"")),
        Node::Arr(items) => {
            lit.expect(&format!("{prefix}["))?;
            for item in items {
                expect_node(lit, None, item)?;
            }
            lit.expect("]")
        }
        Node::Obj(entries) => {
            lit.expect(&format!("{prefix}{{"))?;
            for (key, value) in entries {
                expect_node(lit, Some(key), value)?;
            }
            lit.expect("}")
        }
    }
}

/// Walks the V1 `translations` object literal `{ ja: {…}, en: {…} }` against the locale
/// roots, then requires the end of the literal.
fn expect_translations(lit: &mut Literal<'_>, roots: &[(&str, Node)]) -> Result<(), String> {
    lit.expect("{")?;
    for (code, root) in roots {
        expect_node(lit, Some(code), root)?;
    }
    lit.expect("}")?;
    lit.finish()
}

#[test]
fn literal_matcher_rejects_swaps_wrong_keys_reordering_and_extra_source() {
    let link = format!(
        "<a href=\"https://x/\" target=\"{LINK_TARGET}\" rel=\"{LINK_REL}\" className=\"{LINK_CLASS}\">X</a>"
    );
    let source = r#"{
    ja: {
        a: { x: "甲", y: "乙" },
        list: ["p", "q"],
        rich: <>LINK 前<code>c</code></>
    },
    en: {
        a: { x: "A", y: "B" },
        list: ["p", "q"],
        rich: <>LINK pre<code>c</code></>
    }
}"#
    .replace("LINK", &link);
    let ja = r#"{"a":{"x":"甲","y":"乙"},"list":["p","q"],"rich":{"$rich":[{"type":"link","href":"https://x/","text":"X"},{"type":"text","text":" 前"},{"type":"code","text":"c"}]}}"#;
    let en = r#"{"a":{"x":"A","y":"B"},"list":["p","q"],"rich":{"$rich":[{"type":"link","href":"https://x/","text":"X"},{"type":"text","text":" pre"},{"type":"code","text":"c"}]}}"#;
    let check = |ja: &str, en: &str| {
        let roots = [
            ("ja", parse_ordered(ja, "ja test JSON")),
            ("en", parse_ordered(en, "en test JSON")),
        ];
        let mut lit = Literal {
            src: &source,
            pos: 0,
        };
        expect_translations(&mut lit, &roots)
    };

    assert_eq!(check(ja, en), Ok(()));
    // ja ↔ en swapped: the first mismatch is reported with its V1 line.
    let swapped = check(en, ja);
    let expected = r#"line 3: expected "x: \"A\"", found "x: \"甲\", y: \"乙\" },"#;
    assert!(
        swapped.as_ref().is_err_and(|err| err.starts_with(expected)),
        "{swapped:?}"
    );
    let xy = r#""x":"甲","y":"乙""#;
    let ja_rich = r#"{"$rich":[{"type":"link","href":"https://x/","text":"X"},{"type":"text","text":" 前"},{"type":"code","text":"c"}]}"#;
    let mutations = [
        (
            "value under the wrong key",
            ja.replace(xy, r#""x":"乙","y":"甲""#),
        ),
        ("keys reordered", ja.replace(xy, r#""y":"乙","x":"甲""#)),
        (
            "V1 leaf missing from the data",
            ja.replace(r#","y":"乙""#, ""),
        ),
        (
            "data leaf missing from V1",
            ja.replace(xy, r#""x":"甲","y":"乙","z":"丙""#),
        ),
        (
            "wrong nesting",
            ja.replace(
                r#""y":"乙"},"list":["p","q"]"#,
                r#""y":"乙","list":["p","q"]}"#,
            ),
        ),
        (
            "array element missing",
            ja.replace(r#"["p","q"]"#, r#"["p"]"#),
        ),
        (
            "link target changed",
            ja.replace("https://x/", "https://y/"),
        ),
        (
            "code became text",
            ja.replace(r#""type":"code""#, r#""type":"text""#),
        ),
        ("rich became plain", ja.replace(ja_rich, r#"" 前c""#)),
    ];
    for (what, mutated) in mutations {
        assert_ne!(mutated, ja, "{what}: mutation did not apply");
        assert!(check(&mutated, en).is_err(), "{what}: not detected");
    }
    // Extra V1 source after the last locale.
    let extended = format!("{source} x");
    let roots = [
        ("ja", parse_ordered(ja, "ja test JSON")),
        ("en", parse_ordered(en, "en test JSON")),
    ];
    let mut lit = Literal {
        src: &extended,
        pos: 0,
    };
    assert!(expect_translations(&mut lit, &roots)
        .is_err_and(|err| err.contains("expected the end of the literal")));
}

/// Drift guard against the V1 source when it is checked out around this workspace: the V1
/// `translations` object literal in `hooks/useTranslation.tsx` must spell out exactly the
/// embedded files, token by token: same locales, keys, key order, nesting, array lengths,
/// strings and JSX (link attributes included), with nothing else in it. Skipped when the V1
/// tree is absent.
#[test]
fn site_data_matches_the_v1_object_literal_exactly() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../hooks/useTranslation.tsx");
    let Ok(source) = std::fs::read_to_string(&path) else {
        println!("V1 source not found at {}; skipped", path.display());
        return;
    };
    let open = "const translations = ";
    let (Some(start), Some(end)) = (source.find(open), source.find("} as any;")) else {
        panic!("`translations` literal not found in {}", path.display());
    };
    assert!(start < end, "malformed `translations` literal");
    let mut lit = Literal {
        src: &source[..=end],
        pos: start + open.len(),
    };
    let roots = SiteLocale::ALL.map(|locale| (locale.code(), ordered(locale)));
    assert_eq!(
        expect_translations(&mut lit, &roots),
        Ok(()),
        "{}",
        path.display()
    );
}
