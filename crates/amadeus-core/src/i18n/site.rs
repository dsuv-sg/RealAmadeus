//! System B — the landing/docs site dictionary (V1 `hooks/useTranslation.tsx`).
//!
//! Used by the `/amadeus` landing page, `/amadeus/download`, `/amadeus/changelog` and
//! `/amadeus/docs/**` (and the DSUV portfolio root, which owns `common`/`home`). The app at
//! `/amadeus/app` never uses it (it has System A, see [`crate::i18n`]).
//!
//! # V1 semantics (`LanguageProvider` / `useLanguage`)
//!
//! * Locales are `'ja' | 'en'` ([`SiteLocale`]).
//! * The initial locale is **`ja`** on every full page load ([`SiteLocale::INITIAL`]). The
//!   choice is **never persisted**: there is no localStorage key, cookie, URL parameter or
//!   `navigator.language` detection. V1 keeps it in React state of a provider that wraps the
//!   whole site, so it survives client-side navigation between site pages only.
//! * `toggleLanguage` flips ja → en → ja ([`SiteLocale::toggled`]). Every toggle button shows
//!   the *target* language: `locale === 'ja' ? 'EN' : 'JP'` ([`SiteLocale::toggle_label`]).
//!   Inline page ternaries `locale === 'ja' ? a : b` map to [`SiteLocale::pick`].
//! * `t` is the dictionary object of the current locale: [`site_dict`] mirrors its property
//!   paths (`t.amadeus.docs.sidebar.download` → `site_dict(l).amadeus.docs.sidebar.download`).
//!   Both locales have identical shapes (checked by the tests). V1 has no fallback for a
//!   missing path (it would throw a `TypeError`); the path accessors here never panic and
//!   [`site_t`] falls back to the path itself.
//!
//! # Data
//!
//! `crates/amadeus-core/locales/site/{ja,en}.json` were generated from the V1 object literal
//! with the TypeScript compiler AST. They keep V1 key names (camelCase), key order and array
//! order, including the subtrees V1 never renders (marked "unused in V1" below). When the V1
//! tree is checked out around this workspace, a test re-reads the V1 `translations` literal
//! and requires it to spell out exactly these files, token by token (locales, keys, order,
//! nesting, strings, JSX and link attributes, nothing extra). JSX leaves (rich text) are encoded as `{"$rich": [segment, …]}`
//! with the segments `{"type":"text","text":…}`, `{"type":"link","href":…,"text":…}` and
//! `{"type":"code","text":…}`. Every V1 link has the same attributes ([`LINK_TARGET`],
//! [`LINK_REL`], [`LINK_CLASS`]); `<code>` has no class.
//!
//! The typed structs deserialize leniently (unknown keys are ignored, missing keys become
//! empty), so a schema slip cannot blank the whole dictionary at runtime; exactness is
//! enforced by the tests instead (typed → JSON round trip must equal the embedded file).
//!
//! # V2 additions
//!
//! Texts that only V2 shows (the V2 section of `/docs/download`, the `/download` asset labels
//! and the «旧バージョン (V1) / Legacy (V1)» heading both pages share) are not V1 keys, so
//! they cannot go into the JSON files. They are Rust data in both locales: [`site_v2`].

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::release::AssetKind;

/// `target` attribute of every rich-text link in V1.
pub const LINK_TARGET: &str = "_blank";
/// `rel` attribute of every rich-text link in V1.
pub const LINK_REL: &str = "noopener noreferrer";
/// `className` of every rich-text link in V1.
pub const LINK_CLASS: &str = "text-[#d49e38] underline hover:text-[#f0b95b] transition-colors";

/// Raw JSON sources, indexed by [`SiteLocale::slot`].
static SITE_LOCALE_JSON: [&str; 2] = [
    include_str!("../../locales/site/ja.json"),
    include_str!("../../locales/site/en.json"),
];

static SITE_DICTS: [OnceLock<SiteDict>; 2] = [const { OnceLock::new() }; 2];
static SITE_VALUES: [OnceLock<Value>; 2] = [const { OnceLock::new() }; 2];

/// Locale of the landing/docs site (V1 `type Locale = 'ja' | 'en'`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SiteLocale {
    /// Japanese, the initial locale.
    #[default]
    Ja,
    /// English.
    En,
}

impl SiteLocale {
    /// Both locales, in V1 declaration order.
    pub const ALL: [SiteLocale; 2] = [SiteLocale::Ja, SiteLocale::En];

    /// Locale after a full page load (V1 `useState<Locale>('ja')`). Never persisted.
    pub const INITIAL: SiteLocale = SiteLocale::Ja;

    /// `"ja"` or `"en"`.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            SiteLocale::Ja => "ja",
            SiteLocale::En => "en",
        }
    }

    /// Exact (case-sensitive) inverse of [`SiteLocale::code`].
    #[must_use]
    pub fn from_code(code: &str) -> Option<SiteLocale> {
        Self::ALL.into_iter().find(|locale| locale.code() == code)
    }

    /// V1 `toggleLanguage`: `prev === 'ja' ? 'en' : 'ja'`.
    #[must_use]
    pub fn toggled(self) -> SiteLocale {
        match self {
            SiteLocale::Ja => SiteLocale::En,
            SiteLocale::En => SiteLocale::Ja,
        }
    }

    /// Label of the toggle button (the target language): `locale === 'ja' ? 'EN' : 'JP'`.
    #[must_use]
    pub fn toggle_label(self) -> &'static str {
        match self {
            SiteLocale::Ja => "EN",
            SiteLocale::En => "JP",
        }
    }

    /// V1 inline ternary `locale === 'ja' ? ja : en` (e.g. the changelog page's
    /// `'ダウンロードへ' : 'BACK TO DOWNLOAD'` and `'戻る' : 'Return'`).
    #[must_use]
    pub fn pick<'a>(self, ja: &'a str, en: &'a str) -> &'a str {
        match self {
            SiteLocale::Ja => ja,
            SiteLocale::En => en,
        }
    }

    fn slot(self) -> usize {
        match self {
            SiteLocale::Ja => 0,
            SiteLocale::En => 1,
        }
    }
}

// ---------------------------------------------------------------------------
// Rich text
// ---------------------------------------------------------------------------

/// One segment of a rich-text (V1 JSX) leaf.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Seg {
    /// Plain text; leading/trailing spaces are significant.
    Text {
        /// The text.
        text: String,
    },
    /// `<a href target="_blank" rel="noopener noreferrer" className=LINK_CLASS>text</a>`.
    Link {
        /// Link target URL.
        href: String,
        /// Link text.
        text: String,
    },
    /// `<code>text</code>` (no class).
    Code {
        /// Code text.
        text: String,
    },
}

impl Seg {
    /// The visible text of the segment.
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Seg::Text { text } | Seg::Link { text, .. } | Seg::Code { text } => text,
        }
    }
}

/// A rich-text leaf (V1 JSX fragment), serialized as `{"$rich": [segments]}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RichText {
    /// The fragment's children in order.
    #[serde(rename = "$rich")]
    pub segments: Vec<Seg>,
}

impl RichText {
    /// Concatenated visible text (what `textContent` would return).
    #[must_use]
    pub fn plain_text(&self) -> String {
        self.segments.iter().map(Seg::text).collect()
    }
}

/// Content of a docs step: a plain string or rich text (V1 `string | JSX`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StepContent {
    /// Plain string.
    Plain(String),
    /// Rich text (JSX in V1).
    Rich(RichText),
}

impl Default for StepContent {
    fn default() -> Self {
        StepContent::Plain(String::new())
    }
}

impl StepContent {
    /// The string if this is plain content.
    #[must_use]
    pub fn as_plain(&self) -> Option<&str> {
        match self {
            StepContent::Plain(text) => Some(text),
            StepContent::Rich(_) => None,
        }
    }

    /// The rich text if this is rich content.
    #[must_use]
    pub fn as_rich(&self) -> Option<&RichText> {
        match self {
            StepContent::Plain(_) => None,
            StepContent::Rich(rich) => Some(rich),
        }
    }

    /// Concatenated visible text.
    #[must_use]
    pub fn plain_text(&self) -> String {
        match self {
            StepContent::Plain(text) => text.clone(),
            StepContent::Rich(rich) => rich.plain_text(),
        }
    }
}

// ---------------------------------------------------------------------------
// Typed dictionary (mirrors the V1 object literal exactly)
// ---------------------------------------------------------------------------

/// The whole dictionary of one locale (V1 `translations[locale]`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SiteDict {
    /// `common.*` (portfolio root page; partly unused in V1).
    pub common: Common,
    /// `home.*` (DSUV portfolio root page `app/page.tsx` only).
    pub home: Home,
    /// `amadeus.*` (landing, download, changelog and docs pages).
    pub amadeus: AmadeusSite,
}

/// `common.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Common {
    /// `common.returnMain` (unused in V1).
    pub return_main: String,
    /// `common.systemOnline` (portfolio root page).
    pub system_online: String,
    /// `common.terminateSession` (unused in V1).
    pub terminate_session: String,
}

/// `home.*` (portfolio root page).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Home {
    /// `home.role`.
    pub role: String,
    /// `home.tagline`.
    pub tagline: String,
    /// `home.activeProjects`.
    pub active_projects: String,
    /// `home.projects.*`.
    pub projects: HomeProjects,
    /// `home.socials.*`.
    pub socials: HomeSocials,
}

/// `home.projects.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HomeProjects {
    /// `home.projects.amadeus`.
    pub amadeus: TitleDesc,
    /// `home.projects.elvelt`.
    pub elvelt: TitleDesc,
}

/// `{ title, desc }` pair.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TitleDesc {
    /// `title`.
    pub title: String,
    /// `desc`.
    pub desc: String,
}

/// `home.socials.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HomeSocials {
    /// `home.socials.twitter`.
    pub twitter: Social,
    /// `home.socials.github`.
    pub github: Social,
    /// `home.socials.youtube`.
    pub youtube: Social,
    /// `home.socials.support`.
    pub support: Social,
}

/// `{ label, sub }` of a social link.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Social {
    /// `label`.
    pub label: String,
    /// `sub`.
    pub sub: String,
}

/// `amadeus.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AmadeusSite {
    /// `amadeus.status.*` (unused in V1).
    pub status: AmadeusStatus,
    /// `amadeus.button.*` (unused in V1).
    pub button: AmadeusButton,
    /// `amadeus.panels.*` (unused in V1).
    pub panels: Panels,
    /// `amadeus.hud.*` (unused in V1).
    pub hud: Hud,
    /// `amadeus.accessRestricted.*` (unused in V1).
    pub access_restricted: AccessRestricted,
    /// `amadeus.login.*` (unused in V1).
    pub login: SiteLogin,
    /// `amadeus.preparation.*` (unused in V1).
    pub preparation: Preparation,
    /// `amadeus.release.*` (landing, download and changelog pages).
    pub release: Release,
    /// `amadeus.docs.*`.
    pub docs: Docs,
}

/// `amadeus.status.*` (unused in V1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AmadeusStatus {
    /// `system`.
    pub system: String,
    /// `ver`.
    pub ver: String,
    /// `connecting`.
    pub connecting: String,
}

/// `amadeus.button.*` (unused in V1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AmadeusButton {
    /// `init`.
    pub init: String,
}

/// `amadeus.panels.*` (unused in V1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Panels {
    /// `goal`.
    pub goal: TitleContent,
    /// `status`.
    pub status: TitleContent,
    /// `access`.
    pub access: TitleContent,
}

/// `{ title, content }` pair with plain content.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TitleContent {
    /// `title`.
    pub title: String,
    /// `content`.
    pub content: String,
}

/// `amadeus.hud.*` (unused in V1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Hud {
    /// `logic`.
    pub logic: String,
    /// `voice`.
    pub voice: String,
    /// `memory`.
    pub memory: String,
}

/// `amadeus.accessRestricted.*` (unused in V1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AccessRestricted {
    /// `title`.
    pub title: String,
    /// `desc`.
    pub desc: String,
    /// `button`.
    pub button: String,
}

/// `amadeus.login.*` (unused in V1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SiteLogin {
    /// `userId`.
    pub user_id: String,
    /// `password`.
    pub password: String,
    /// `connect`.
    pub connect: String,
    /// `placeholderId`.
    pub placeholder_id: String,
    /// `error`.
    pub error: String,
}

/// `amadeus.preparation.*` (unused in V1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preparation {
    /// `title`.
    pub title: String,
    /// `message`.
    pub message: String,
    /// `status`.
    pub status: String,
}

/// `amadeus.release.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Release {
    /// `title` (landing page).
    pub title: String,
    /// `message` (landing page).
    pub message: String,
    /// `windowsDownload` (download page).
    pub windows_download: String,
    /// `macosDownload` (download page).
    pub macos_download: String,
    /// `linuxDownload` (download page).
    pub linux_download: String,
    /// `lightweightWindowsDownload` (download page).
    pub lightweight_windows_download: String,
    /// `download` (landing and download pages).
    pub download: String,
    /// `githubRepo` (landing page).
    pub github_repo: String,
    /// `docs` (landing and changelog pages).
    pub docs: String,
    /// `webVersion` (landing page).
    pub web_version: String,
}

/// `amadeus.docs.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Docs {
    /// `title` (unused in V1).
    pub title: String,
    /// `back` (docs layout: sidebar link `← {back}` to the landing page).
    pub back: String,
    /// `sidebar.*` (docs layout).
    pub sidebar: DocsSidebar,
    /// `gettingStarted.*` (docs index page).
    pub getting_started: GettingStarted,
    /// `download.*` (docs download page).
    pub download: DocsDownload,
    /// `changelog.*` (landing, download, changelog and docs download pages).
    pub changelog: DocsChangelog,
    /// `loginDoc.*` (docs login page).
    pub login_doc: LoginDoc,
    /// `controls.*` (docs controls page).
    pub controls: ControlsDoc,
    /// `api.*` (docs API page; `modelLabels` also used by every provider page).
    pub api: ApiDoc,
    /// `apiOpenAI.*`.
    #[serde(rename = "apiOpenAI")]
    pub api_openai: ProviderDoc,
    /// `apiGemini.*`.
    pub api_gemini: ProviderDoc,
    /// `apiClaude.*`.
    pub api_claude: ProviderDoc,
    /// `apiGroq.*`.
    pub api_groq: ProviderDoc,
    /// `apiVertex.*`.
    pub api_vertex: ProviderDoc,
}

/// `amadeus.docs.sidebar.*` (docs layout navigation labels).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DocsSidebar {
    /// `gettingStarted` (`/amadeus/docs`).
    pub getting_started: String,
    /// `download` (`/amadeus/docs/download`).
    pub download: String,
    /// `login` (`/amadeus/docs/login`).
    pub login: String,
    /// `controls` (`/amadeus/docs/controls`).
    pub controls: String,
    /// `apiSetup` (`/amadeus/docs/api`).
    pub api_setup: String,
    /// `apiOpenAI` (`/amadeus/docs/api/openai`).
    #[serde(rename = "apiOpenAI")]
    pub api_openai: String,
    /// `apiGemini` (`/amadeus/docs/api/gemini`).
    pub api_gemini: String,
    /// `apiClaude` (`/amadeus/docs/api/claude`).
    pub api_claude: String,
    /// `apiGroq` (`/amadeus/docs/api/groq`).
    pub api_groq: String,
    /// `apiVertex` (`/amadeus/docs/api/vertex`).
    pub api_vertex: String,
}

/// `amadeus.docs.gettingStarted.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GettingStarted {
    /// `title`.
    pub title: String,
    /// `desc`.
    pub desc: String,
    /// `loginInfo`.
    pub login_info: String,
}

/// `amadeus.docs.download.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DocsDownload {
    /// `title`.
    pub title: String,
    /// `desc`.
    pub desc: String,
    /// `standardTitle`.
    pub standard_title: String,
    /// `standardDesc`.
    pub standard_desc: String,
    /// `lightweightTitle`.
    pub lightweight_title: String,
    /// `lightweightDesc`.
    pub lightweight_desc: String,
    /// `recommendedEnv`.
    pub recommended_env: String,
    /// `standardEnv`.
    pub standard_env: String,
    /// `lightweightEnv`.
    pub lightweight_env: String,
    /// `windows`.
    pub windows: String,
    /// `mac`.
    pub mac: String,
    /// `linux`.
    pub linux: String,
    /// `downloadBtn`.
    pub download_btn: String,
    /// `releasesPage` (unused in V1).
    pub releases_page: String,
    /// `fallbackStandard` (rich text with a link).
    pub fallback_standard: RichText,
    /// `fallbackLightweight` (rich text with a link).
    pub fallback_lightweight: RichText,
    /// `steps` (2 entries). V1 renders `steps.slice(2)`, i.e. **nothing**; keep that quirk.
    pub steps: Vec<Step>,
    /// `fileStructure.*`.
    pub file_structure: FileStructure,
    /// `notice`.
    pub notice: String,
}

/// `amadeus.docs.download.fileStructure.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FileStructure {
    /// `title`.
    pub title: String,
    /// `windows`.
    pub windows: String,
    /// `mac`.
    pub mac: String,
    /// `linux`.
    pub linux: String,
    /// `windowsQT`.
    #[serde(rename = "windowsQT")]
    pub windows_qt: String,
}

/// `amadeus.docs.changelog.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DocsChangelog {
    /// `title`.
    pub title: String,
    /// `desc`.
    pub desc: String,
    /// `versions` (newest first: V1.4, V1.3, V1.2, V1.1, V1.0.1, V1.0).
    pub versions: Vec<ChangelogVersion>,
}

/// One entry of `amadeus.docs.changelog.versions`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChangelogVersion {
    /// `version`, e.g. `"V1.4"`.
    pub version: String,
    /// `product` (never rendered in V1).
    pub product: String,
    /// `items`.
    pub items: Vec<String>,
}

/// `amadeus.docs.loginDoc.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LoginDoc {
    /// `title`.
    pub title: String,
    /// `desc`.
    pub desc: String,
    /// `idLabel`.
    pub id_label: String,
    /// `idValue`.
    pub id_value: String,
    /// `passLabel`.
    pub pass_label: String,
    /// `passValue`.
    pub pass_value: String,
}

/// `amadeus.docs.controls.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ControlsDoc {
    /// `title`.
    pub title: String,
    /// `desc`.
    pub desc: String,
    /// `items` (5 entries).
    pub items: Vec<ControlItem>,
}

/// One entry of `amadeus.docs.controls.items`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ControlItem {
    /// `key` (key combination label).
    pub key: String,
    /// `desc`.
    pub desc: String,
}

/// `amadeus.docs.api.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ApiDoc {
    /// `title`.
    pub title: String,
    /// `desc`.
    pub desc: String,
    /// `steps` (4 plain strings).
    pub steps: Vec<String>,
    /// `notice`.
    pub notice: String,
    /// `modelLabels.*` (also used by every provider page).
    pub model_labels: ModelLabels,
}

/// `amadeus.docs.api.modelLabels.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelLabels {
    /// `title`.
    pub title: String,
    /// `available`.
    pub available: String,
    /// `recommended`.
    pub recommended: String,
}

/// `amadeus.docs.api{OpenAI,Gemini,Claude,Groq,Vertex}.*`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderDoc {
    /// `title`.
    pub title: String,
    /// `desc`.
    pub desc: String,
    /// `modelList.*`.
    pub model_list: ModelList,
    /// `steps` (`content` is plain or rich text).
    pub steps: Vec<Step>,
}

/// `modelList.*` of a provider page.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelList {
    /// `available` model ids.
    pub available: Vec<String>,
    /// `recommended` model id.
    pub recommended: String,
}

/// `{ title, content }` docs step.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Step {
    /// `title`.
    pub title: String,
    /// `content`.
    pub content: StepContent,
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

fn parse_site_dict(json: &str) -> Result<SiteDict, serde_json::Error> {
    serde_json::from_str(json)
}

/// The typed dictionary of `locale` (V1 `t` from `useLanguage()`), parsed once on first use.
/// The embedded data always parses (checked by the tests); a failure would degrade to empty
/// strings instead of panicking.
#[must_use]
pub fn site_dict(locale: SiteLocale) -> &'static SiteDict {
    let slot = locale.slot();
    SITE_DICTS[slot].get_or_init(|| parse_site_dict(SITE_LOCALE_JSON[slot]).unwrap_or_default())
}

fn site_root(locale: SiteLocale) -> &'static Value {
    let slot = locale.slot();
    SITE_VALUES[slot]
        .get_or_init(|| serde_json::from_str(SITE_LOCALE_JSON[slot]).unwrap_or(Value::Null))
}

/// Canonical array index (`"0"`, `"12"`; no sign, no leading zeros), like a JS array index.
fn parse_index(segment: &str) -> Option<usize> {
    let canonical = !segment.is_empty()
        && segment.bytes().all(|b| b.is_ascii_digit())
        && (segment == "0" || !segment.starts_with('0'));
    if canonical {
        segment.parse().ok()
    } else {
        None
    }
}

/// The raw JSON node at a dot-separated V1 property path, e.g.
/// `"amadeus.docs.apiOpenAI.steps.0.content"` (array elements are addressed by index). An
/// empty path returns the root. `None` if any segment is missing.
#[must_use]
pub fn site_value(locale: SiteLocale, path: &str) -> Option<&'static Value> {
    let mut node = site_root(locale);
    if path.is_empty() {
        return Some(node);
    }
    for segment in path.split('.') {
        node = match node {
            Value::Object(map) => map.get(segment)?,
            Value::Array(items) => items.get(parse_index(segment)?)?,
            _ => return None,
        };
    }
    Some(node)
}

/// The string leaf at `path` (see [`site_value`]); `None` if the path is missing or is not a
/// plain string (rich-text leaves are objects: use [`site_rich`] or [`site_dict`]).
#[must_use]
pub fn site_get(locale: SiteLocale, path: &str) -> Option<&'static str> {
    site_value(locale, path)?.as_str()
}

/// Path-based translation, e.g. `site_t(SiteLocale::Ja, "amadeus.docs.sidebar.download")`
/// → `"ダウンロード"`. Falls back to the path itself when it does not name a string leaf
/// (V1 would throw instead; this never panics). The result borrows from `path` only for the
/// fallback; dictionary values are `'static`.
#[must_use]
pub fn site_t(locale: SiteLocale, path: &str) -> &str {
    site_get(locale, path).unwrap_or(path)
}

/// The string elements of the array at `path` (e.g.
/// `"amadeus.docs.apiGroq.modelList.available"`); empty if the path is missing or not an
/// array. Non-string elements are skipped.
#[must_use]
pub fn site_strings(locale: SiteLocale, path: &str) -> Vec<&'static str> {
    site_value(locale, path)
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |items| {
            items.iter().filter_map(Value::as_str).collect()
        })
}

/// Rich text at `path`: a `{"$rich": …}` leaf, or a plain string as a single text segment
/// (so step contents render uniformly). `None` if missing or of another kind.
#[must_use]
pub fn site_rich(locale: SiteLocale, path: &str) -> Option<RichText> {
    match site_value(locale, path)? {
        Value::String(text) => Some(RichText {
            segments: vec![Seg::Text { text: text.clone() }],
        }),
        node @ Value::Object(map) if map.contains_key("$rich") => RichText::deserialize(node).ok(),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// V2 additions (not in V1's dictionary)
// ---------------------------------------------------------------------------

/// The V2-only texts of one locale (see "V2 additions" in the module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SiteV2 {
    /// Heading of the V1 builds: `/download`'s collapsible section and the V1 part of
    /// `/docs/download`.
    pub legacy_v1: &'static str,
    /// Labels of the V2 release assets (`/download` cards, `/docs/download` platform boxes).
    pub assets: AssetLabels,
    /// The V2 section on top of `/docs/download`.
    pub docs_download: DocsDownloadV2,
}

/// Label of each V2 release asset kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetLabels {
    /// [`AssetKind::Nsis`].
    pub nsis: &'static str,
    /// [`AssetKind::Msi`].
    pub msi: &'static str,
    /// [`AssetKind::Dmg`].
    pub dmg: &'static str,
    /// [`AssetKind::AppArchive`].
    pub app_archive: &'static str,
    /// [`AssetKind::AppImage`].
    pub app_image: &'static str,
    /// [`AssetKind::Deb`].
    pub deb: &'static str,
    /// [`AssetKind::Rpm`].
    pub rpm: &'static str,
    /// [`AssetKind::WebServer`].
    pub web_server: &'static str,
}

impl AssetLabels {
    /// The label of `kind`.
    #[must_use]
    pub fn label(&self, kind: AssetKind) -> &'static str {
        match kind {
            AssetKind::Nsis => self.nsis,
            AssetKind::Msi => self.msi,
            AssetKind::Dmg => self.dmg,
            AssetKind::AppArchive => self.app_archive,
            AssetKind::AppImage => self.app_image,
            AssetKind::Deb => self.deb,
            AssetKind::Rpm => self.rpm,
            AssetKind::WebServer => self.web_server,
        }
    }
}

/// The V2 (Rust edition) section on top of `/docs/download`. Its title follows V1's
/// `standardTitle` / `lightweightTitle` pattern (edition, then product).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocsDownloadV2 {
    /// Section title.
    pub title: &'static str,
    /// Section description: what V2 is and that its builds are on the download page.
    pub desc: &'static str,
    /// Text of the link to `{base}/download`.
    pub download_page: &'static str,
}

/// [`SiteV2`] by [`SiteLocale::slot`].
static SITE_V2: [SiteV2; 2] = [
    SiteV2 {
        legacy_v1: "レガシー版 (V1 / Unity・Qt)",
        assets: AssetLabels {
            nsis: "インストーラー (.exe)",
            msi: "MSIパッケージ (.msi)",
            dmg: "ディスクイメージ (.dmg)",
            app_archive: "アプリアーカイブ (.app.tar.gz)",
            app_image: "AppImage",
            deb: "Debian / Ubuntu (.deb)",
            rpm: "Fedora / openSUSE (.rpm)",
            web_server: "セルフホスト版",
        },
        docs_download: DocsDownloadV2 {
            title: "Rust版 RealAmadeus V2",
            desc: "Rustで全面的に書き直された最新版です。Windows/macOS/Linux向けのデスクトップアプリと、Web版のセルフホスト用パッケージをダウンロードページで提供しています。",
            download_page: "ダウンロードページへ",
        },
    },
    SiteV2 {
        legacy_v1: "Legacy (V1 / Unity & Qt)",
        assets: AssetLabels {
            nsis: "Installer (.exe)",
            msi: "MSI Package (.msi)",
            dmg: "Disk Image (.dmg)",
            app_archive: "App Archive (.app.tar.gz)",
            app_image: "AppImage",
            deb: "Debian / Ubuntu (.deb)",
            rpm: "Fedora / openSUSE (.rpm)",
            web_server: "Self-host Server",
        },
        docs_download: DocsDownloadV2 {
            title: "Rust Edition RealAmadeus V2",
            desc: "The latest version, completely rewritten in Rust. Desktop apps for Windows, macOS, and Linux and a self-host package for the web version are available on the Download page.",
            download_page: "GO TO DOWNLOAD PAGE",
        },
    },
];

/// The V2-only site texts of `locale`.
#[must_use]
pub fn site_v2(locale: SiteLocale) -> &'static SiteV2 {
    &SITE_V2[locale.slot()]
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod v2_tests {
    use super::*;
    use crate::release::{self, Platform};

    #[test]
    fn v2_texts_exist_in_both_locales() {
        let ja = site_v2(SiteLocale::Ja);
        let en = site_v2(SiteLocale::En);
        for v2 in [ja, en] {
            let d = &v2.docs_download;
            for text in [v2.legacy_v1, d.title, d.desc, d.download_page] {
                assert!(!text.trim().is_empty() && text.trim() == text, "{text:?}");
            }
            for spec in &release::ASSETS {
                assert!(!v2.assets.label(spec.kind).is_empty(), "{:?}", spec.kind);
            }
        }
        assert_eq!(
            (ja.legacy_v1, en.legacy_v1),
            ("レガシー版 (V1 / Unity・Qt)", "Legacy (V1 / Unity & Qt)")
        );
        assert_eq!(ja.docs_download.title, "Rust版 RealAmadeus V2");
        assert_eq!(en.docs_download.title, "Rust Edition RealAmadeus V2");
        assert_eq!(ja.docs_download.download_page, "ダウンロードページへ");
        assert_eq!(en.docs_download.download_page, "GO TO DOWNLOAD PAGE");
        assert_ne!(ja.docs_download, en.docs_download);
    }

    #[test]
    fn v2_section_title_follows_the_v1_edition_titles() {
        // V1: "レガシー版（Unity） RealAmadeusUnity" / "Legacy Edition (Unity) RealAmadeusUnity".
        for locale in SiteLocale::ALL {
            let v1 = &site_dict(locale).amadeus.docs.download;
            let v2 = site_v2(locale).docs_download.title;
            let (v1_edition, v1_product) = v1.standard_title.rsplit_once(' ').unwrap_or_default();
            assert_eq!(v1_product, "RealAmadeusUnity", "{}", locale.code());
            assert!(!v1_edition.is_empty());
            assert!(v2.ends_with(" RealAmadeus V2"), "{v2}");
        }
    }

    #[test]
    fn v2_desc_names_every_platform_of_the_release_table() {
        for locale in SiteLocale::ALL {
            let desc = site_v2(locale).docs_download.desc;
            for platform in [Platform::Windows, Platform::MacOs, Platform::Linux] {
                assert!(
                    desc.contains(platform.label()),
                    "{}: {platform:?}",
                    locale.code()
                );
            }
            assert!(desc.contains("Rust"));
        }
        assert!(site_v2(SiteLocale::Ja)
            .docs_download
            .desc
            .contains("ダウンロードページ"));
        assert!(site_v2(SiteLocale::En)
            .docs_download
            .desc
            .contains("Download page"));
    }

    #[test]
    fn asset_labels_name_the_file_type_of_each_asset() {
        for locale in SiteLocale::ALL {
            let labels = &site_v2(locale).assets;
            for spec in &release::ASSETS {
                let label = labels.label(spec.kind);
                // `-setup.exe` → `exe`, `.app.tar.gz` → `app.tar.gz`, `.AppImage` → `AppImage`.
                let extension = spec
                    .suffix
                    .trim_start_matches("-setup")
                    .trim_start_matches('.');
                if spec.kind == AssetKind::WebServer {
                    assert!(label.contains("セルフホスト") || label.contains("Self-host"));
                } else {
                    assert!(label.contains(extension), "{label:?} lacks {extension:?}");
                }
            }
        }
        let ja = &site_v2(SiteLocale::Ja).assets;
        let en = &site_v2(SiteLocale::En).assets;
        assert_eq!(ja.label(AssetKind::Nsis), "インストーラー (.exe)");
        assert_eq!(en.label(AssetKind::Msi), "MSI Package (.msi)");
        assert_eq!(en.label(AssetKind::WebServer), "Self-host Server");
        assert_eq!(ja.label(AssetKind::Rpm), en.label(AssetKind::Rpm));
    }
}
