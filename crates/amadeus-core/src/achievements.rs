//! Achievements: port of V1 `lib/amadeus/services/achievement-manager.ts`.
//!
//! * 28 definitions ([`ACHIEVEMENT_DEFINITIONS`], display order); only
//!   `secret_reading_steiner` is secret. Titles/descriptions come from the locale JSON
//!   (`ach_<id>_title` / `ach_<id>_desc`) with the in-code Japanese as fallback, translated with
//!   the language **at call time** (V1 read `Config_Language` inside `getAllAchievements` and
//!   `unlock`), so callers pass the current `Config_Language`.
//! * Storage: [`STORAGE_KEY`] (`amadeus_achievements`), compact JSON
//!   `{"unlocked":[…],"dates":{…},"seenEmotions":[…],"lastLaunchDate":"YYYY-MM-DD","launchStreak":N}`
//!   in exactly that key order; `dates` keeps JS object key order. Loading is lenient
//!   (`normalizeState`), a parse error yields the fresh state.
//! * Construction loads the state and updates the launch streak (same day: keep, next day:
//!   +1, otherwise 1), saves, and unlocks `streak_3`/`streak_7` silently (no listener can exist
//!   yet; V1 showed no toast either). [`AchievementManager::startup_unlocks`] lists them.
//! * [`AchievementManager::notify_event`] reproduces `checkAutoUnlocks` exactly, including the
//!   unlock order (which decides the toast order). Every new unlock emits
//!   [`achievement_unlocked`](AchievementManager::achievement_unlocked) then
//!   [`achievements_changed`](AchievementManager::achievements_changed).
//! * Language changes: forward `AppSettings::settings_changed` keys to
//!   [`handle_setting_changed`](AchievementManager::handle_setting_changed) (V1 re-emitted
//!   `achievementsChanged` for `Config_Language`).

use std::cell::RefCell;
use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::Serialize;

use crate::i18n;
use crate::memory::{parse_strict_date_day, LocalDateTime};
use crate::settings::jsnum::js_number_to_string;
use crate::settings::keys;
use crate::storage::KvStore;
use crate::types::Emitter;

/// localStorage key (V1 `STORAGE_KEY`).
pub const STORAGE_KEY: &str = keys::ACHIEVEMENTS_STORAGE_KEY;
/// Title shown for a locked secret achievement (literal, never translated).
pub const SECRET_TITLE: &str = "???";
/// i18n key of the locked secret achievement description.
pub const SECRET_HINT_KEY: &str = "achievement_secret_hint";
/// Fallback of [`SECRET_HINT_KEY`].
pub const SECRET_HINT_DEFAULT: &str = "シークレットアチーブメント";

/// One achievement definition (V1 `AchievementDef`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AchievementDef {
    /// Stable id (persisted in `unlocked` / `dates`).
    pub id: &'static str,
    /// i18n key of the title.
    pub title_key: &'static str,
    /// i18n key of the description.
    pub desc_key: &'static str,
    /// In-code Japanese title.
    pub default_title: &'static str,
    /// In-code Japanese description.
    pub default_desc: &'static str,
    /// Icon id (stored, never rendered by V1).
    pub icon: &'static str,
    /// Shown as `???` until unlocked.
    pub secret: bool,
}

impl AchievementDef {
    /// Localized title.
    pub fn title(&self, lang: i64) -> &'static str {
        i18n::t(self.title_key, self.default_title, lang)
    }

    /// Localized description.
    pub fn description(&self, lang: i64) -> &'static str {
        i18n::t(self.desc_key, self.default_desc, lang)
    }
}

const fn def(
    id: &'static str,
    title_key: &'static str,
    desc_key: &'static str,
    default_title: &'static str,
    default_desc: &'static str,
    icon: &'static str,
    secret: bool,
) -> AchievementDef {
    AchievementDef {
        id,
        title_key,
        desc_key,
        default_title,
        default_desc,
        icon,
        secret,
    }
}

/// V1 `ACHIEVEMENT_DEFINITIONS` (verbatim, display order).
#[rustfmt::skip]
pub const ACHIEVEMENT_DEFINITIONS: [AchievementDef; 28] = [
    def("first_chat", "ach_first_chat_title", "ach_first_chat_desc", "初めまして", "初めて紅莉栖と会話した", "chat", false),
    def("chat_10", "ach_chat_10_title", "ach_chat_10_desc", "常連", "10回会話した", "chat", false),
    def("chat_50", "ach_chat_50_title", "ach_chat_50_desc", "親友", "50回会話した", "chat", false),
    def("chat_100", "ach_chat_100_title", "ach_chat_100_desc", "ラボメン", "100回会話した", "chat", false),
    def("first_angry", "ach_first_angry_title", "ach_first_angry_desc", "怒らせた", "初めて紅莉栖を怒らせた", "angry", false),
    def("first_blush", "ach_first_blush_title", "ach_first_blush_desc", "照れさせた", "初めて紅莉栖を照れさせた", "blush", false),
    def("first_smile", "ach_first_smile_title", "ach_first_smile_desc", "笑顔", "初めて紅莉栖を笑顔にした", "smile", false),
    def("first_normal", "ach_first_normal_title", "ach_first_normal_desc", "平常心", "初めて紅莉栖の平常な姿を見た", "normal", false),
    def("first_sad", "ach_first_sad_title", "ach_first_sad_desc", "悲しませた", "初めて紅莉栖を悲しませた", "sad", false),
    def("first_surprised", "ach_first_surprised_title", "ach_first_surprised_desc", "驚かせた", "初めて紅莉栖を驚かせた", "surprised", false),
    def("first_wink", "ach_first_wink_title", "ach_first_wink_desc", "ウインク", "初めて紅莉栖にウインクされた", "wink", false),
    def("first_disgust", "ach_first_disgust_title", "ach_first_disgust_desc", "呆れさせた", "初めて紅莉栖を呆れさせた", "disgust", false),
    def("first_smug", "ach_first_smug_title", "ach_first_smug_desc", "ドヤ顔", "初めて紅莉栖のドヤ顔を見た", "smug", false),
    def("first_thinking", "ach_first_thinking_title", "ach_first_thinking_desc", "考えさせた", "初めて紅莉栖を考え込ませた", "thinking", false),
    def("first_panic", "ach_first_panic_title", "ach_first_panic_desc", "パニック", "初めて紅莉栖をパニックにした", "panic", false),
    def("all_emotions", "ach_all_emotions_title", "ach_all_emotions_desc", "感情マスター", "全ての感情を引き出した", "emotions", false),
    def("night_owl", "ach_night_owl_title", "ach_night_owl_desc", "夜更かし", "深夜(0-4時)に会話した", "night", false),
    def("early_bird", "ach_early_bird_title", "ach_early_bird_desc", "早起き", "朝(5-7時)に会話した", "morning", false),
    def("streak_3", "ach_streak_3_title", "ach_streak_3_desc", "3日連続", "3日連続で起動した", "streak", false),
    def("streak_7", "ach_streak_7_title", "ach_streak_7_desc", "1週間", "7日連続で起動した", "streak", false),
    def("first_game", "ach_first_game_title", "ach_first_game_desc", "ゲーマー", "初めてミニゲームをプレイした", "game", false),
    def("game_win", "ach_game_win_title", "ach_game_win_desc", "勝利", "ミニゲームで勝利した", "game", false),
    def("game_perfect", "ach_game_perfect_title", "ach_game_perfect_desc", "パーフェクト", "ミニゲームで全問正解した", "game", false),
    def("export_chat", "ach_export_chat_title", "ach_export_chat_desc", "記録者", "会話記録をエクスポートした", "export", false),
    def("import_chat", "ach_import_chat_title", "ach_import_chat_desc", "時間旅行者", "会話記録をインポートした", "import", false),
    def("first_event", "ach_first_event_title", "ach_first_event_desc", "偶然の出会い", "起動時ランダムイベントが発生した", "event", false),
    def("dev_user", "ach_dev_user_title", "ach_dev_user_desc", "開発者", "開発コマンドを使用した", "dev", false),
    def("secret_reading_steiner", "ach_secret_reading_steiner_title", "ach_secret_reading_steiner_desc", "リーディングシュタイナー", "世界線を超えた秘密を発見した", "secret", true),
];

/// `totalCount` (28).
pub const TOTAL_COUNT: usize = ACHIEVEMENT_DEFINITIONS.len();

/// V1 `EMOTION_ACHIEVEMENTS` (emotion tag → achievement id, V1 key order).
pub const EMOTION_ACHIEVEMENTS: [(&str, &str); 11] = [
    ("ANGRY", "first_angry"),
    ("BLUSH", "first_blush"),
    ("SMILE", "first_smile"),
    ("NORMAL", "first_normal"),
    ("SAD", "first_sad"),
    ("SURPRISED", "first_surprised"),
    ("WINK", "first_wink"),
    ("DISGUST", "first_disgust"),
    ("SMUG", "first_smug"),
    ("THINKING", "first_thinking"),
    ("PANIC", "first_panic"),
];

/// V1 `ALL_EMOTIONS` (canonical order); all must be seen for `all_emotions`.
pub const ALL_EMOTIONS: [&str; 11] = [
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

/// Definition of an id.
pub fn definition(id: &str) -> Option<&'static AchievementDef> {
    ACHIEVEMENT_DEFINITIONS.iter().find(|d| d.id == id)
}

/// Achievement of an emotion tag (exact, upper-case match), V1 `EMOTION_ACHIEVEMENTS[emotion]`.
pub fn emotion_achievement_id(emotion: &str) -> Option<&'static str> {
    EMOTION_ACHIEVEMENTS
        .iter()
        .find(|(tag, _)| *tag == emotion)
        .map(|(_, id)| *id)
}

/// One row of the achievement panel (V1 `AchievementView`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementView {
    /// Id.
    pub id: &'static str,
    /// Icon id (not rendered).
    pub icon: &'static str,
    /// Secret flag.
    pub secret: bool,
    /// Unlocked (`unlocked.includes(id)`).
    pub unlocked: bool,
    /// Localized title, or `???` for a locked secret.
    pub title: String,
    /// Localized description, or the secret hint for a locked secret.
    pub description: String,
    /// `dates[id] ?? ""` (`YYYY-MM-DD HH:mm`).
    pub unlocked_at: String,
}

impl AchievementView {
    /// Icon text: `★` unlocked, `?` locked secret, `☆` locked.
    pub fn icon_glyph(&self) -> &'static str {
        if self.unlocked {
            "★"
        } else if self.secret {
            "?"
        } else {
            "☆"
        }
    }

    /// Row class: `achievement-row unlocked` / `achievement-row locked`.
    pub fn row_class(&self) -> &'static str {
        if self.unlocked {
            "achievement-row unlocked"
        } else {
            "achievement-row locked"
        }
    }

    /// Icon class: `achievement-icon unlocked` / `achievement-icon locked`.
    pub fn icon_class(&self) -> &'static str {
        if self.unlocked {
            "achievement-icon unlocked"
        } else {
            "achievement-icon locked"
        }
    }

    /// The date is shown only when unlocked and non-empty.
    pub fn shows_date(&self) -> bool {
        self.unlocked && !self.unlocked_at.is_empty()
    }
}

/// An achievement trigger (V1 `notifyEvent(eventType, data)`).
#[derive(Debug, Clone, PartialEq)]
pub enum AchievementEvent {
    /// `"chat"` with `totalInteractions` (after each assistant reply).
    Chat {
        /// `MemoryManager::total_interactions()`.
        total_interactions: i64,
    },
    /// `"emotion"` with the reply's emotion (any string; only the 11 tags unlock).
    Emotion(String),
    /// `"startup"` with a streak (never sent by the V1 UI).
    Startup {
        /// Launch streak.
        streak: f64,
    },
    /// `"game_play"`.
    GamePlay,
    /// `"game_win"` (final score > 0).
    GameWin,
    /// `"game_perfect"` (10/10).
    GamePerfect,
    /// `"export"`.
    Export,
    /// `"import"`.
    Import,
    /// `"random_event"` (startup greeting shown).
    RandomEvent,
    /// `"dev_command"`.
    DevCommand,
    /// `"secret_steiner"` (`/dev steiner`).
    SecretSteiner,
}

impl AchievementEvent {
    /// V1 event type string.
    pub fn event_type(&self) -> &'static str {
        match self {
            AchievementEvent::Chat { .. } => "chat",
            AchievementEvent::Emotion(_) => "emotion",
            AchievementEvent::Startup { .. } => "startup",
            AchievementEvent::GamePlay => "game_play",
            AchievementEvent::GameWin => "game_win",
            AchievementEvent::GamePerfect => "game_perfect",
            AchievementEvent::Export => "export",
            AchievementEvent::Import => "import",
            AchievementEvent::RandomEvent => "random_event",
            AchievementEvent::DevCommand => "dev_command",
            AchievementEvent::SecretSteiner => "secret_steiner",
        }
    }

    /// From a V1 `(eventType, data)` pair (`data` fields default like V1: `?? 0` / `?? ""`);
    /// `None` for unknown event types (V1 ignored them).
    pub fn from_v1(event_type: &str, data: &AchievementEventData) -> Option<Self> {
        Some(match event_type {
            "chat" => AchievementEvent::Chat {
                total_interactions: data.total_interactions.unwrap_or(0),
            },
            "emotion" => AchievementEvent::Emotion(data.emotion.clone().unwrap_or_default()),
            "startup" => AchievementEvent::Startup {
                streak: data.streak.unwrap_or(0.0),
            },
            "game_play" => AchievementEvent::GamePlay,
            "game_win" => AchievementEvent::GameWin,
            "game_perfect" => AchievementEvent::GamePerfect,
            "export" => AchievementEvent::Export,
            "import" => AchievementEvent::Import,
            "random_event" => AchievementEvent::RandomEvent,
            "dev_command" => AchievementEvent::DevCommand,
            "secret_steiner" => AchievementEvent::SecretSteiner,
            _ => return None,
        })
    }
}

/// V1 `AchievementEventData`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AchievementEventData {
    /// `totalInteractions`.
    pub total_interactions: Option<i64>,
    /// `emotion`.
    pub emotion: Option<String>,
    /// `streak`.
    pub streak: Option<f64>,
}

/// Payload of [`AchievementManager::achievement_unlocked`] (V1 `(id, title, description)`,
/// localized at unlock time; `""` for an id without definition).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementUnlocked {
    /// Id.
    pub id: String,
    /// Localized title.
    pub title: String,
    /// Localized description.
    pub description: String,
}

// ---------------------------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------------------------

/// V1 `AchievementState`.
#[derive(Debug, Clone, PartialEq)]
pub struct AchievementState {
    /// Unlocked ids in unlock order (unknown legacy ids are kept).
    pub unlocked: Vec<String>,
    /// `dates[id]` = `YYYY-MM-DD HH:mm`, in JS key order (see [`AchievementState::to_json`]).
    pub dates: Vec<(String, String)>,
    /// Emotions seen in replies (first-seen order; any string).
    pub seen_emotions: Vec<String>,
    /// Local `YYYY-MM-DD` of the last launch.
    pub last_launch_date: String,
    /// Consecutive launch days (any finite number is accepted from storage, like V1).
    pub launch_streak: f64,
}

impl Default for AchievementState {
    fn default() -> Self {
        Self::fresh()
    }
}

impl AchievementState {
    /// V1 `freshState()`.
    pub fn fresh() -> Self {
        Self {
            unlocked: Vec::new(),
            dates: Vec::new(),
            seen_emotions: Vec::new(),
            last_launch_date: String::new(),
            launch_streak: 0.0,
        }
    }

    /// V1 `loadState()`: `normalizeState(JSON.parse(raw))`, the fresh state on a parse error.
    /// Lenient: a non-object root → fresh; `unlocked`/`seenEmotions` keep string elements;
    /// `dates` keeps string values of a plain object; `lastLaunchDate` must be a string;
    /// `launchStreak` a (finite) number.
    pub fn parse(raw: &str) -> Self {
        let mut state = Self::fresh();
        let Ok(JsonValue::Object(root)) = serde_json::from_str::<JsonValue>(raw) else {
            return state;
        };
        for (key, value) in root {
            match (key.as_str(), value) {
                ("unlocked", JsonValue::Array(items)) => state.unlocked = strings(items),
                ("seenEmotions", JsonValue::Array(items)) => state.seen_emotions = strings(items),
                ("dates", JsonValue::Object(entries)) => {
                    state.dates = entries
                        .into_iter()
                        .filter_map(|(id, value)| match value {
                            // `state.dates["__proto__"] = v` hits the prototype setter in V1.
                            JsonValue::String(date) if id != "__proto__" => Some((id, date)),
                            _ => None,
                        })
                        .collect();
                }
                ("lastLaunchDate", JsonValue::String(date)) => state.last_launch_date = date,
                ("launchStreak", JsonValue::Number(streak)) if streak.is_finite() => {
                    state.launch_streak = streak;
                }
                _ => {}
            }
        }
        state
    }

    /// `JSON.stringify(state)`: fixed key order, `dates` in JS property order (array-index keys
    /// ascending first, then the others in insertion order), numbers JS-formatted.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\"unlocked\":");
        push_string_array(&mut out, &self.unlocked);
        out.push_str(",\"dates\":{");
        let mut index_keys: Vec<(u32, &(String, String))> = self
            .dates
            .iter()
            .filter_map(|entry| array_index(&entry.0).map(|n| (n, entry)))
            .collect();
        index_keys.sort_by_key(|(n, _)| *n);
        let ordered = index_keys
            .into_iter()
            .map(|(_, entry)| entry)
            .chain(self.dates.iter().filter(|(k, _)| array_index(k).is_none()));
        for (i, (id, date)) in ordered.enumerate() {
            if i > 0 {
                out.push(',');
            }
            push_string(&mut out, id);
            out.push(':');
            push_string(&mut out, date);
        }
        out.push_str("},\"seenEmotions\":");
        push_string_array(&mut out, &self.seen_emotions);
        out.push_str(",\"lastLaunchDate\":");
        push_string(&mut out, &self.last_launch_date);
        out.push_str(",\"launchStreak\":");
        if self.launch_streak.is_finite() {
            out.push_str(&js_number_to_string(self.launch_streak));
        } else {
            out.push_str("null");
        }
        out.push('}');
        out
    }

    /// `dates[id]`.
    pub fn date_of(&self, id: &str) -> Option<&str> {
        self.dates
            .iter()
            .find(|(key, _)| key == id)
            .map(|(_, date)| date.as_str())
    }

    /// `dates[id] = date` (an existing key keeps its position).
    fn set_date(&mut self, id: &str, date: String) {
        match self.dates.iter_mut().find(|(key, _)| key == id) {
            Some(entry) => entry.1 = date,
            None => self.dates.push((id.to_owned(), date)),
        }
    }
}

fn strings(items: Vec<JsonValue>) -> Vec<String> {
    items
        .into_iter()
        .filter_map(|item| match item {
            JsonValue::String(s) => Some(s),
            _ => None,
        })
        .collect()
}

/// JS "array index" property key: canonical decimal `0 ..= 2^32 - 2`.
fn array_index(key: &str) -> Option<u32> {
    if key.is_empty() || (key.len() > 1 && key.starts_with('0')) {
        return None;
    }
    if !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    key.parse::<u32>().ok().filter(|&n| n != u32::MAX)
}

fn push_string(out: &mut String, s: &str) {
    match serde_json::to_string(s) {
        Ok(quoted) => out.push_str(&quoted),
        Err(_) => out.push_str("\"\""),
    }
}

fn push_string_array(out: &mut String, items: &[String]) {
    out.push('[');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_string(out, item);
    }
    out.push(']');
}

/// `JSON.parse` result with JS object semantics: keys in creation order, a repeated key keeps
/// its first position and takes the last value. (`serde_json::Map` order depends on a cargo
/// feature that `amadeus-core` does not control.)
#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl<'de> Deserialize<'de> for JsonValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct JsonVisitor;

        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = JsonValue;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("any JSON value")
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<JsonValue, E> {
                Ok(JsonValue::Bool(v))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<JsonValue, E> {
                Ok(JsonValue::Number(v as f64))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<JsonValue, E> {
                Ok(JsonValue::Number(v as f64))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<JsonValue, E> {
                Ok(JsonValue::Number(v))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<JsonValue, E> {
                Ok(JsonValue::String(v.to_owned()))
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<JsonValue, E> {
                Ok(JsonValue::String(v))
            }

            fn visit_unit<E: de::Error>(self) -> Result<JsonValue, E> {
                Ok(JsonValue::Null)
            }

            fn visit_none<E: de::Error>(self) -> Result<JsonValue, E> {
                Ok(JsonValue::Null)
            }

            fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<JsonValue, D::Error> {
                JsonValue::deserialize(d)
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<JsonValue, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(JsonValue::Array(items))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<JsonValue, A::Error> {
                let mut entries: Vec<(String, JsonValue)> = Vec::new();
                while let Some((key, value)) = map.next_entry::<String, JsonValue>()? {
                    match entries.iter_mut().find(|(k, _)| *k == key) {
                        Some(entry) => entry.1 = value,
                        None => entries.push((key, value)),
                    }
                }
                Ok(JsonValue::Object(entries))
            }
        }

        deserializer.deserialize_any(JsonVisitor)
    }
}

// ---------------------------------------------------------------------------------------------
// The manager
// ---------------------------------------------------------------------------------------------

/// V1 `AchievementManager` singleton over a [`KvStore`].
///
/// All methods take `&self`; signals are emitted after the state change and the storage write,
/// with no borrow held, so handlers may call back into the manager.
pub struct AchievementManager<'a, S: KvStore + ?Sized> {
    store: &'a S,
    state: RefCell<AchievementState>,
    startup_unlocks: Vec<&'static str>,
    achievement_unlocked: Emitter<AchievementUnlocked>,
    achievements_changed: Emitter<()>,
}

impl<S: KvStore + ?Sized> fmt::Debug for AchievementManager<'_, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AchievementManager")
            .field("state", &self.state.borrow())
            .field("startup_unlocks", &self.startup_unlocks)
            .finish_non_exhaustive()
    }
}

impl<'a, S: KvStore + ?Sized> AchievementManager<'a, S> {
    /// V1 constructor: `loadState()` then `updateLaunchStreak()` with the local date of `now`
    /// (always saves; unlocks `streak_3` at a streak ≥ 3 and `streak_7` at ≥ 7, without signals).
    pub fn new(store: &'a S, now: &LocalDateTime) -> Self {
        let state = store
            .get(STORAGE_KEY)
            .map_or_else(AchievementState::fresh, |raw| AchievementState::parse(&raw));
        let mut manager = Self {
            store,
            state: RefCell::new(state),
            startup_unlocks: Vec::new(),
            achievement_unlocked: Emitter::new(),
            achievements_changed: Emitter::new(),
        };
        manager.update_launch_streak(now);
        manager
    }

    fn update_launch_streak(&mut self, now: &LocalDateTime) {
        let today_str = now.date_iso();
        let streak = {
            let mut state = self.state.borrow_mut();
            let today = parse_strict_date_day(&today_str);
            let last = parse_strict_date_day(&state.last_launch_date);
            let mut streak = state.launch_streak;
            match (today, last) {
                (Some(t), Some(l)) if t == l => {}
                (Some(t), Some(l)) if t - l == 1 => streak += 1.0,
                _ => streak = 1.0,
            }
            state.last_launch_date = today_str;
            state.launch_streak = streak;
            self.save(&state);
            streak
        };
        for (threshold, id) in [(3.0, "streak_3"), (7.0, "streak_7")] {
            if streak >= threshold && self.unlock_silently(id, now) {
                self.startup_unlocks.push(id);
            }
        }
    }

    fn save(&self, state: &AchievementState) {
        self.store.set(STORAGE_KEY, &state.to_json());
    }

    /// Stores a new unlock; `false` when it was already unlocked.
    fn unlock_silently(&self, id: &str, now: &LocalDateTime) -> bool {
        let mut state = self.state.borrow_mut();
        if state.unlocked.iter().any(|u| u == id) {
            return false;
        }
        state.unlocked.push(id.to_owned());
        state.set_date(id, now.date_time_minutes());
        self.save(&state);
        true
    }

    /// The underlying store.
    pub fn store(&self) -> &'a S {
        self.store
    }

    /// Ids unlocked by the launch streak during construction (no signal was emitted for them).
    pub fn startup_unlocks(&self) -> &[&'static str] {
        &self.startup_unlocks
    }

    /// V1 `achievementUnlocked(id, title, description)` (ChatPanel's toast).
    pub fn achievement_unlocked(&self) -> &Emitter<AchievementUnlocked> {
        &self.achievement_unlocked
    }

    /// V1 `achievementsChanged()` (after unlocks, `reset_all`, and `Config_Language` changes).
    pub fn achievements_changed(&self) -> &Emitter<()> {
        &self.achievements_changed
    }

    /// V1 `settingsChanged` subscription: `Config_Language` → `achievements_changed`.
    pub fn handle_setting_changed(&self, key: &str) {
        if key == keys::LANGUAGE.key {
            self.achievements_changed.emit(&());
        }
    }

    /// Snapshot of the state.
    pub fn state(&self) -> AchievementState {
        self.state.borrow().clone()
    }

    /// V1 `unlockedCount`: length of the stored list (unknown legacy ids included, so it can
    /// exceed [`TOTAL_COUNT`]).
    pub fn unlocked_count(&self) -> usize {
        self.state.borrow().unlocked.len()
    }

    /// V1 `totalCount` (28).
    pub fn total_count(&self) -> usize {
        TOTAL_COUNT
    }

    /// Current launch streak.
    pub fn launch_streak(&self) -> f64 {
        self.state.borrow().launch_streak
    }

    /// V1 `isUnlocked(id)`.
    pub fn is_unlocked(&self, id: &str) -> bool {
        self.state.borrow().unlocked.iter().any(|u| u == id)
    }

    /// V1 `getAllAchievements()` in definition order, translated with `lang`
    /// (`Config_Language` now).
    pub fn all_achievements(&self, lang: i64) -> Vec<AchievementView> {
        let state = self.state.borrow();
        ACHIEVEMENT_DEFINITIONS
            .iter()
            .map(|d| {
                let unlocked = state.unlocked.iter().any(|u| u == d.id);
                let (title, description) = if d.secret && !unlocked {
                    (
                        SECRET_TITLE.to_owned(),
                        i18n::t(SECRET_HINT_KEY, SECRET_HINT_DEFAULT, lang).to_owned(),
                    )
                } else {
                    (d.title(lang).to_owned(), d.description(lang).to_owned())
                };
                AchievementView {
                    id: d.id,
                    icon: d.icon,
                    secret: d.secret,
                    unlocked,
                    title,
                    description,
                    unlocked_at: state.date_of(d.id).unwrap_or("").to_owned(),
                }
            })
            .collect()
    }

    /// V1 `unlock(id)`: no-op (`false`) when already unlocked; otherwise appends the id, stamps
    /// `dates[id]` with `now` (`YYYY-MM-DD HH:mm`), saves, emits `achievement_unlocked` (title and
    /// description in `lang`, `""` for an unknown id, which is still stored) and then
    /// `achievements_changed`.
    pub fn unlock(&self, id: &str, now: &LocalDateTime, lang: i64) -> bool {
        if !self.unlock_silently(id, now) {
            return false;
        }
        let (title, description) =
            definition(id).map_or(("", ""), |d| (d.title(lang), d.description(lang)));
        self.achievement_unlocked.emit(&AchievementUnlocked {
            id: id.to_owned(),
            title: title.to_owned(),
            description: description.to_owned(),
        });
        self.achievements_changed.emit(&());
        true
    }

    /// V1 `notifyEvent` / `checkAutoUnlocks`, in V1 order:
    /// * `Chat`: ≥ 1 `first_chat`, ≥ 10 `chat_10`, ≥ 50 `chat_50`, ≥ 100 `chat_100`, then the
    ///   local hour of `now`: 0–3 `night_owl`, 5–6 `early_bird`;
    /// * `Emotion`: its `first_<emotion>`, then the emotion is added to `seenEmotions` (any
    ///   string, saved), then `all_emotions` once all 11 tags were seen;
    /// * `Startup`: streak ≥ 3 `streak_3`, ≥ 7 `streak_7`;
    /// * the others map to `first_game`, `game_win`, `game_perfect`, `export_chat`,
    ///   `import_chat`, `first_event`, `dev_user`, `secret_reading_steiner`.
    ///
    /// Returns the newly unlocked ids in unlock (= toast) order.
    pub fn notify_event(
        &self,
        event: &AchievementEvent,
        now: &LocalDateTime,
        lang: i64,
    ) -> Vec<&'static str> {
        let mut unlocked = Vec::new();
        let mut try_unlock = |id: &'static str| {
            if self.unlock(id, now, lang) {
                unlocked.push(id);
            }
        };
        match event {
            AchievementEvent::Chat { total_interactions } => {
                let total = *total_interactions;
                for (threshold, id) in [
                    (1, "first_chat"),
                    (10, "chat_10"),
                    (50, "chat_50"),
                    (100, "chat_100"),
                ] {
                    if total >= threshold {
                        try_unlock(id);
                    }
                }
                if now.hour < 4 {
                    try_unlock("night_owl");
                }
                if (5..7).contains(&now.hour) {
                    try_unlock("early_bird");
                }
            }
            AchievementEvent::Emotion(emotion) => {
                if let Some(id) = emotion_achievement_id(emotion) {
                    try_unlock(id);
                }
                {
                    let mut state = self.state.borrow_mut();
                    if !state.seen_emotions.iter().any(|e| e == emotion) {
                        state.seen_emotions.push(emotion.clone());
                        self.save(&state);
                    }
                }
                let all_seen = {
                    let state = self.state.borrow();
                    ALL_EMOTIONS
                        .iter()
                        .all(|name| state.seen_emotions.iter().any(|e| e == name))
                };
                if all_seen {
                    try_unlock("all_emotions");
                }
            }
            AchievementEvent::Startup { streak } => {
                if *streak >= 3.0 {
                    try_unlock("streak_3");
                }
                if *streak >= 7.0 {
                    try_unlock("streak_7");
                }
            }
            AchievementEvent::GamePlay => try_unlock("first_game"),
            AchievementEvent::GameWin => try_unlock("game_win"),
            AchievementEvent::GamePerfect => try_unlock("game_perfect"),
            AchievementEvent::Export => try_unlock("export_chat"),
            AchievementEvent::Import => try_unlock("import_chat"),
            AchievementEvent::RandomEvent => try_unlock("first_event"),
            AchievementEvent::DevCommand => try_unlock("dev_user"),
            AchievementEvent::SecretSteiner => try_unlock("secret_reading_steiner"),
        }
        unlocked
    }

    /// V1 `resetAll()` (`/dev achievements reset`): fresh state — also clearing
    /// `lastLaunchDate` and `launchStreak` — saved, then `achievements_changed`.
    pub fn reset_all(&self) {
        {
            let mut state = self.state.borrow_mut();
            *state = AchievementState::fresh();
            self.save(&state);
        }
        self.achievements_changed.emit(&());
    }
}

#[cfg(test)]
mod tests;
