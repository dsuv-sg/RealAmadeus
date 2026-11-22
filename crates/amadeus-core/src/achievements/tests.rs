//! Unit tests of the achievements. Golden values come from running the V1 TypeScript
//! (`lib/amadeus/services/achievement-manager.ts`) under Node 22 with TZ=Asia/Tokyo.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sha2::{Digest, Sha256};

use super::*;
use crate::i18n::LANG_COUNT;
use crate::storage::MemoryStore;

fn sha256(s: &str) -> String {
    Sha256::digest(s.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

const fn at(day: u32, hour: u32, minute: u32) -> LocalDateTime {
    LocalDateTime::new(2026, 11, day, hour, minute, 0)
}

fn stored(store: &MemoryStore) -> String {
    store.get(STORAGE_KEY).unwrap_or_default()
}

fn boot(
    raw: Option<&str>,
    now: LocalDateTime,
) -> (
    &'static MemoryStore,
    AchievementManager<'static, MemoryStore>,
) {
    let store: &'static MemoryStore = Box::leak(Box::new(MemoryStore::new()));
    if let Some(raw) = raw {
        store.set(STORAGE_KEY, raw);
    }
    let manager = AchievementManager::new(store, &now);
    (store, manager)
}

#[test]
fn definitions_match_v1() {
    assert_eq!(TOTAL_COUNT, 28);
    let ids: Vec<&str> = ACHIEVEMENT_DEFINITIONS.iter().map(|d| d.id).collect();
    assert_eq!(ids[0], "first_chat");
    assert_eq!(ids[15], "all_emotions");
    assert_eq!(ids[27], "secret_reading_steiner");
    let secrets: Vec<&str> = ACHIEVEMENT_DEFINITIONS
        .iter()
        .filter(|d| d.secret)
        .map(|d| d.id)
        .collect();
    assert_eq!(secrets, ["secret_reading_steiner"]);
    for d in &ACHIEVEMENT_DEFINITIONS {
        assert_eq!(d.title_key, format!("ach_{}_title", d.id));
        assert_eq!(d.desc_key, format!("ach_{}_desc", d.id));
        for lang in 0..LANG_COUNT as i64 {
            assert!(i18n::has_key(d.title_key, lang) && i18n::has_key(d.desc_key, lang));
        }
        // No achievement string differs between the code and ja.json.
        assert_eq!(
            (d.title(0), d.description(0)),
            (d.default_title, d.default_desc)
        );
    }
    assert!(i18n::has_key(SECRET_HINT_KEY, 0));
    for (emotion, id) in EMOTION_ACHIEVEMENTS {
        assert_eq!(id, format!("first_{}", emotion.to_lowercase()));
        assert_eq!(emotion_achievement_id(emotion), Some(id));
    }
    assert_eq!(emotion_achievement_id("smile"), None);
    assert_eq!(emotion_achievement_id("SLEEPING"), None);
    assert_eq!(ALL_EMOTIONS, crate::chat_utils::EMOTION_TAGS);
    assert_eq!(definition("ghost"), None);
}

/// Golden: V1 constructor (load + launch streak) for various stored states at 2026-11-16.
#[test]
fn launch_streak_matches_v1() {
    let (store, m) = boot(None, at(16, 21, 34));
    assert_eq!(stored(store), FRESH_JSON);
    assert!(m.startup_unlocks().is_empty());

    let (store, m) = boot(
        Some(
            r#"{"unlocked":["first_chat"],"dates":{"first_chat":"2026-11-15 10:00"},"seenEmotions":[],"lastLaunchDate":"2026-11-15","launchStreak":2}"#,
        ),
        at(16, 8, 0),
    );
    assert_eq!(stored(store), STREAK3_JSON);
    assert_eq!(
        (m.startup_unlocks(), m.unlocked_count()),
        (&["streak_3"][..], 2)
    );

    let noon = at(16, 12, 0);
    let cases = [
        (
            r#"{"lastLaunchDate":"2026-11-16","launchStreak":0}"#,
            SAMEDAY0_JSON,
        ),
        (
            r#"{"lastLaunchDate":"2026-11-10","launchStreak":5}"#,
            GAP_JSON,
        ),
        (
            r#"{"lastLaunchDate":"2026-11-15","launchStreak":6,"unlocked":["streak_3"],"dates":{"streak_3":"2026-11-12 09:00"}}"#,
            STREAK7_JSON,
        ),
        (
            r#"{"lastLaunchDate":"2026-11-15","launchStreak":2.5}"#,
            FLOAT_JSON,
        ),
        (
            r#"{"lastLaunchDate":"2026-11-17","launchStreak":4}"#,
            FUTURE_JSON,
        ),
        (
            r#"{"lastLaunchDate":"0099-11-15","launchStreak":4}"#,
            Y99_JSON,
        ),
        ("not json", GARBAGE_JSON),
        ("null", NULL_JSON),
        ("[1,2]", ARRAY_JSON),
        (
            r#"{"unlocked":["a",1,null,"b",{"x":1}],"dates":{"b":"2026-01-01 00:00","2":"x","c":5,"10":"y","__proto__":"p","a":"2025-12-31 23:59","b":"dup"},"seenEmotions":"SMILE","lastLaunchDate":20261115,"launchStreak":"3","extra":true}"#,
            LENIENT_JSON,
        ),
    ];
    for (raw, expected) in cases {
        let (store, _) = boot(Some(raw), noon);
        assert_eq!(stored(store), expected, "{raw}");
    }
    let (_, m) = boot(
        Some(r#"{"lastLaunchDate":"2026-11-15","launchStreak":6}"#),
        noon,
    );
    assert_eq!(m.startup_unlocks(), ["streak_3", "streak_7"]);
    assert_eq!(m.launch_streak(), 7.0);
}

/// Golden: a full V1 event sequence (emitted toasts, final JSON, signal count, reset, dev).
#[test]
fn event_sequence_matches_v1() {
    let (store, m) = boot(None, at(16, 2, 10));
    let toasts = Rc::new(RefCell::new(Vec::<AchievementUnlocked>::new()));
    let sink = Rc::clone(&toasts);
    m.achievement_unlocked()
        .connect(move |u: &AchievementUnlocked| sink.borrow_mut().push(u.clone()));
    let changed = Rc::new(Cell::new(0));
    let counter = Rc::clone(&changed);
    m.achievements_changed()
        .connect(move |_| counter.set(counter.get() + 1));

    let night = at(16, 2, 10);
    let morning = at(16, 6, 30);
    let emotion = |e: &str| AchievementEvent::Emotion(e.to_owned());
    let mut unlocked = Vec::new();
    unlocked.extend(m.notify_event(&AchievementEvent::RandomEvent, &night, 1));
    unlocked.extend(m.notify_event(&emotion("SMILE"), &night, 1));
    unlocked.extend(m.notify_event(
        &AchievementEvent::Chat {
            total_interactions: 1,
        },
        &night,
        1,
    ));
    unlocked.extend(m.notify_event(
        &AchievementEvent::Chat {
            total_interactions: 10,
        },
        &morning,
        1,
    ));
    for e in [
        "SMILE",
        "SLEEPING",
        "",
        "NORMAL",
        "ANGRY",
        "SAD",
        "SURPRISED",
        "BLUSH",
        "WINK",
        "DISGUST",
        "SMUG",
        "THINKING",
        "PANIC",
    ] {
        unlocked.extend(m.notify_event(&emotion(e), &morning, 1));
    }
    unlocked.extend(m.notify_event(
        &AchievementEvent::Chat {
            total_interactions: 100,
        },
        &morning,
        1,
    ));
    unlocked.extend(m.notify_event(&AchievementEvent::Startup { streak: 7.0 }, &morning, 1));
    assert_eq!(
        AchievementEvent::from_v1("unknown_event", &AchievementEventData::default()),
        None
    );
    m.handle_setting_changed("Config_Language");
    m.handle_setting_changed("Config_TextSpeed");
    for event in [
        AchievementEvent::GamePlay,
        AchievementEvent::GameWin,
        AchievementEvent::GamePerfect,
        AchievementEvent::Export,
        AchievementEvent::Import,
        AchievementEvent::DevCommand,
        AchievementEvent::SecretSteiner,
    ] {
        unlocked.extend(m.notify_event(&event, &morning, 0));
    }
    {
        let toasts = toasts.borrow();
        assert_eq!(toasts.len(), TOASTS.len());
        for (got, (id, title, description)) in toasts.iter().zip(TOASTS) {
            assert_eq!(
                (
                    got.id.as_str(),
                    got.title.as_str(),
                    got.description.as_str()
                ),
                (id, title, description)
            );
        }
    }
    let ids: Vec<&str> = TOASTS.iter().map(|t| t.0).collect();
    assert_eq!(unlocked, ids, "notify_event returns the toast order");
    assert_eq!(changed.get(), 29);
    assert_eq!(
        sha256(&stored(store)),
        "9db44a26031c417c203ebecc7be1ffaac7b5f59117c5f4d9255615fcabb4cd71"
    );
    assert_eq!((m.unlocked_count(), m.total_count()), (28, 28));
    // Repeated events unlock nothing.
    assert!(m
        .notify_event(&AchievementEvent::DevCommand, &morning, 0)
        .is_empty());

    m.reset_all();
    assert_eq!(stored(store), RESET_JSON);
    assert_eq!(changed.get(), 30);
    m.notify_event(&AchievementEvent::DevCommand, &morning, 0);
    assert_eq!(stored(store), DEV_JSON);
}

/// Golden: V1 `getAllAchievements()` rows for several languages (an unknown stored id counts in
/// `unlockedCount`).
#[test]
fn views_match_v1() {
    let (_, m) = boot(
        Some(
            r#"{"unlocked":["first_chat","ghost"],"dates":{"first_chat":"2026-11-15 10:00"},"seenEmotions":[],"lastLaunchDate":"2026-11-16","launchStreak":1}"#,
        ),
        at(16, 12, 0),
    );
    for (lang, expected) in [(0, VIEWS0), (1, VIEWS1), (2, VIEWS2), (13, VIEWS13)] {
        let views = m.all_achievements(lang);
        assert_eq!(views.len(), 28);
        let picked = [&views[0], &views[1], &views[27]];
        assert_eq!(
            serde_json::to_string(&picked).unwrap_or_default(),
            expected,
            "{lang}"
        );
        assert_eq!(m.unlocked_count(), 2);
    }
    let views = m.all_achievements(0);
    assert_eq!(
        (
            views[0].icon_glyph(),
            views[1].icon_glyph(),
            views[27].icon_glyph()
        ),
        ("★", "☆", "?")
    );
    assert_eq!(views[0].row_class(), "achievement-row unlocked");
    assert_eq!(views[1].icon_class(), "achievement-icon locked");
    assert!(views[0].shows_date() && !views[1].shows_date());
    assert!(m.unlock("secret_reading_steiner", &at(16, 12, 0), 1));
    assert!(!m.unlock("secret_reading_steiner", &at(16, 12, 5), 1));
    let secret = m.all_achievements(1).pop();
    assert_eq!(
        secret
            .map(|v| serde_json::to_string(&v).unwrap_or_default())
            .as_deref(),
        Some(SECRET_EN)
    );
    assert!(m.is_unlocked("ghost"));
}

#[test]
fn unknown_ids_and_reentrancy() {
    let (store, m) = boot(None, at(16, 12, 0));
    let m = Rc::new(m);
    let seen = Rc::new(RefCell::new(Vec::<(String, String, usize)>::new()));
    let (m2, seen2) = (Rc::clone(&m), Rc::clone(&seen));
    m.achievement_unlocked()
        .connect(move |u: &AchievementUnlocked| {
            // Handlers may read and even unlock from inside the emission.
            let rows = m2.all_achievements(0).iter().filter(|v| v.unlocked).count();
            seen2
                .borrow_mut()
                .push((u.id.clone(), u.title.clone(), rows));
            if u.id == "first_game" {
                m2.unlock("game_win", &at(16, 12, 1), 0);
            }
        });
    assert!(m.unlock("legacy_id", &at(16, 12, 0), 0));
    m.notify_event(&AchievementEvent::GamePlay, &at(16, 12, 0), 0);
    assert_eq!(
        *seen.borrow(),
        [
            ("legacy_id".to_owned(), String::new(), 0),
            ("first_game".to_owned(), "ゲーマー".to_owned(), 1),
            ("game_win".to_owned(), "勝利".to_owned(), 2),
        ]
    );
    assert_eq!(
        stored(store),
        r#"{"unlocked":["legacy_id","first_game","game_win"],"dates":{"legacy_id":"2026-11-16 12:00","first_game":"2026-11-16 12:00","game_win":"2026-11-16 12:01"},"seenEmotions":[],"lastLaunchDate":"2026-11-16","launchStreak":1}"#
    );
    assert_eq!(m.unlocked_count(), 3);
}

#[test]
fn state_parsing_and_events() {
    let state = AchievementState::parse(
        r#"{"dates":{"4294967295":"a","4294967294":"b","01":"c","0":"d"},"launchStreak":-2}"#,
    );
    assert_eq!(
        state.to_json(),
        r#"{"unlocked":[],"dates":{"0":"d","4294967294":"b","4294967295":"a","01":"c"},"seenEmotions":[],"lastLaunchDate":"","launchStreak":-2}"#
    );
    assert_eq!(state.date_of("01"), Some("c"));
    assert_eq!(
        AchievementState::parse("{\"launchStreak\":1e400}"),
        AchievementState::fresh()
    );
    let data = AchievementEventData {
        total_interactions: Some(5),
        emotion: Some("SAD".into()),
        streak: None,
    };
    for name in [
        "chat",
        "emotion",
        "startup",
        "game_play",
        "game_win",
        "game_perfect",
        "export",
        "import",
        "random_event",
        "dev_command",
        "secret_steiner",
    ] {
        let event = AchievementEvent::from_v1(name, &data);
        assert_eq!(event.as_ref().map(AchievementEvent::event_type), Some(name));
    }
    assert_eq!(
        AchievementEvent::from_v1("chat", &AchievementEventData::default()),
        Some(AchievementEvent::Chat {
            total_interactions: 0
        })
    );
    assert_eq!(
        AchievementEvent::from_v1("startup", &data),
        Some(AchievementEvent::Startup { streak: 0.0 })
    );
}

// ---- V1 golden outputs ------------------------------------------------------------------------

const TOASTS: [(&str, &str, &str); 28] = [
    (
        "first_event",
        "Chance Encounter",
        "A random event occurred at startup",
    ),
    (
        "first_smile",
        "Smile",
        "Made Kurisu smile for the first time",
    ),
    (
        "first_chat",
        "Nice to Meet You",
        "Had your first conversation with Kurisu",
    ),
    ("night_owl", "Night Owl", "Chatted late at night (0-4 AM)"),
    ("chat_10", "Regular", "Had 10 conversations"),
    (
        "early_bird",
        "Early Bird",
        "Chatted in the morning (5-7 AM)",
    ),
    (
        "first_normal",
        "Calm & Collected",
        "Saw Kurisu in her normal state for the first time",
    ),
    (
        "first_angry",
        "You Made Her Mad",
        "Made Kurisu angry for the first time",
    ),
    (
        "first_sad",
        "You Made Her Sad",
        "Made Kurisu sad for the first time",
    ),
    (
        "first_surprised",
        "Surprise!",
        "Surprised Kurisu for the first time",
    ),
    (
        "first_blush",
        "You Made Her Blush",
        "Made Kurisu blush for the first time",
    ),
    (
        "first_wink",
        "Wink",
        "Got a wink from Kurisu for the first time",
    ),
    (
        "first_disgust",
        "Disgusted",
        "Disgusted Kurisu for the first time",
    ),
    (
        "first_smug",
        "Smug Face",
        "Saw Kurisu's smug face for the first time",
    ),
    (
        "first_thinking",
        "Deep Thought",
        "Made Kurisu think deeply for the first time",
    ),
    (
        "first_panic",
        "Panic!",
        "Made Kurisu panic for the first time",
    ),
    ("all_emotions", "Emotion Master", "Drew out every emotion"),
    ("chat_50", "Close Friend", "Had 50 conversations"),
    ("chat_100", "Lab Member", "Had 100 conversations"),
    ("streak_3", "3-Day Streak", "Launched 3 days in a row"),
    ("streak_7", "One Week", "Launched 7 days in a row"),
    ("first_game", "ゲーマー", "初めてミニゲームをプレイした"),
    ("game_win", "勝利", "ミニゲームで勝利した"),
    ("game_perfect", "パーフェクト", "ミニゲームで全問正解した"),
    ("export_chat", "記録者", "会話記録をエクスポートした"),
    ("import_chat", "時間旅行者", "会話記録をインポートした"),
    ("dev_user", "開発者", "開発コマンドを使用した"),
    (
        "secret_reading_steiner",
        "リーディングシュタイナー",
        "世界線を超えた秘密を発見した",
    ),
];
const FRESH_JSON: &str = "{\"unlocked\":[],\"dates\":{},\"seenEmotions\":[],\"lastLaunchDate\":\"2026-11-16\",\"launchStreak\":1}";
const SAMEDAY0_JSON: &str = "{\"unlocked\":[],\"dates\":{},\"seenEmotions\":[],\"lastLaunchDate\":\"2026-11-16\",\"launchStreak\":0}";
const GAP_JSON: &str = "{\"unlocked\":[],\"dates\":{},\"seenEmotions\":[],\"lastLaunchDate\":\"2026-11-16\",\"launchStreak\":1}";
const FLOAT_JSON: &str = "{\"unlocked\":[\"streak_3\"],\"dates\":{\"streak_3\":\"2026-11-16 12:00\"},\"seenEmotions\":[],\"lastLaunchDate\":\"2026-11-16\",\"launchStreak\":3.5}";
const FUTURE_JSON: &str = "{\"unlocked\":[],\"dates\":{},\"seenEmotions\":[],\"lastLaunchDate\":\"2026-11-16\",\"launchStreak\":1}";
const Y99_JSON: &str = "{\"unlocked\":[],\"dates\":{},\"seenEmotions\":[],\"lastLaunchDate\":\"2026-11-16\",\"launchStreak\":1}";
const GARBAGE_JSON: &str = "{\"unlocked\":[],\"dates\":{},\"seenEmotions\":[],\"lastLaunchDate\":\"2026-11-16\",\"launchStreak\":1}";
const NULL_JSON: &str = "{\"unlocked\":[],\"dates\":{},\"seenEmotions\":[],\"lastLaunchDate\":\"2026-11-16\",\"launchStreak\":1}";
const ARRAY_JSON: &str = "{\"unlocked\":[],\"dates\":{},\"seenEmotions\":[],\"lastLaunchDate\":\"2026-11-16\",\"launchStreak\":1}";
const LENIENT_JSON: &str = "{\"unlocked\":[\"a\",\"b\"],\"dates\":{\"2\":\"x\",\"10\":\"y\",\"b\":\"dup\",\"a\":\"2025-12-31 23:59\"},\"seenEmotions\":[],\"lastLaunchDate\":\"2026-11-16\",\"launchStreak\":1}";
const RESET_JSON: &str =
    "{\"unlocked\":[],\"dates\":{},\"seenEmotions\":[],\"lastLaunchDate\":\"\",\"launchStreak\":0}";
const DEV_JSON: &str = "{\"unlocked\":[\"dev_user\"],\"dates\":{\"dev_user\":\"2026-11-16 06:30\"},\"seenEmotions\":[],\"lastLaunchDate\":\"\",\"launchStreak\":0}";
const STREAK3_JSON: &str = "{\"unlocked\":[\"first_chat\",\"streak_3\"],\"dates\":{\"first_chat\":\"2026-11-15 10:00\",\"streak_3\":\"2026-11-16 08:00\"},\"seenEmotions\":[],\"lastLaunchDate\":\"2026-11-16\",\"launchStreak\":3}";
const STREAK7_JSON: &str = "{\"unlocked\":[\"streak_3\",\"streak_7\"],\"dates\":{\"streak_3\":\"2026-11-12 09:00\",\"streak_7\":\"2026-11-16 12:00\"},\"seenEmotions\":[],\"lastLaunchDate\":\"2026-11-16\",\"launchStreak\":7}";
const VIEWS0: &str = "[{\"id\":\"first_chat\",\"icon\":\"chat\",\"secret\":false,\"unlocked\":true,\"title\":\"初めまして\",\"description\":\"初めて紅莉栖と会話した\",\"unlockedAt\":\"2026-11-15 10:00\"},{\"id\":\"chat_10\",\"icon\":\"chat\",\"secret\":false,\"unlocked\":false,\"title\":\"常連\",\"description\":\"10回会話した\",\"unlockedAt\":\"\"},{\"id\":\"secret_reading_steiner\",\"icon\":\"secret\",\"secret\":true,\"unlocked\":false,\"title\":\"???\",\"description\":\"シークレットアチーブメント\",\"unlockedAt\":\"\"}]";
const VIEWS1: &str = "[{\"id\":\"first_chat\",\"icon\":\"chat\",\"secret\":false,\"unlocked\":true,\"title\":\"Nice to Meet You\",\"description\":\"Had your first conversation with Kurisu\",\"unlockedAt\":\"2026-11-15 10:00\"},{\"id\":\"chat_10\",\"icon\":\"chat\",\"secret\":false,\"unlocked\":false,\"title\":\"Regular\",\"description\":\"Had 10 conversations\",\"unlockedAt\":\"\"},{\"id\":\"secret_reading_steiner\",\"icon\":\"secret\",\"secret\":true,\"unlocked\":false,\"title\":\"???\",\"description\":\"Secret Achievement\",\"unlockedAt\":\"\"}]";
const VIEWS2: &str = "[{\"id\":\"first_chat\",\"icon\":\"chat\",\"secret\":false,\"unlocked\":true,\"title\":\"初次见面\",\"description\":\"与红莉栖进行了第一次对话\",\"unlockedAt\":\"2026-11-15 10:00\"},{\"id\":\"chat_10\",\"icon\":\"chat\",\"secret\":false,\"unlocked\":false,\"title\":\"常客\",\"description\":\"进行了10次对话\",\"unlockedAt\":\"\"},{\"id\":\"secret_reading_steiner\",\"icon\":\"secret\",\"secret\":true,\"unlocked\":false,\"title\":\"???\",\"description\":\"隐藏成就\",\"unlockedAt\":\"\"}]";
const VIEWS13: &str = "[{\"id\":\"first_chat\",\"icon\":\"chat\",\"secret\":false,\"unlocked\":true,\"title\":\"初めまして\",\"description\":\"初めて紅莉栖と会話した\",\"unlockedAt\":\"2026-11-15 10:00\"},{\"id\":\"chat_10\",\"icon\":\"chat\",\"secret\":false,\"unlocked\":false,\"title\":\"常連\",\"description\":\"10回会話した\",\"unlockedAt\":\"\"},{\"id\":\"secret_reading_steiner\",\"icon\":\"secret\",\"secret\":true,\"unlocked\":false,\"title\":\"???\",\"description\":\"シークレットアチーブメント\",\"unlockedAt\":\"\"}]";
const SECRET_EN: &str = "{\"id\":\"secret_reading_steiner\",\"icon\":\"secret\",\"secret\":true,\"unlocked\":true,\"title\":\"Reading Steiner\",\"description\":\"Discovered a secret that transcends world lines\",\"unlockedAt\":\"2026-11-16 12:00\"}";
