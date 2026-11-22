//! Unit tests of the memory module. Golden values come from running the V1 TypeScript
//! (`lib/amadeus/memory-manager.ts`) under Node 22 with TZ=Asia/Tokyo and the clock fixed at
//! 2026-11-16 21:34:56 (the spec Appendix G setup).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sha2::{Digest, Sha256};

use super::*;
use crate::protocol::{ChatMessage, ChatRole};
use crate::random_events::SequenceRng;
use crate::storage::MemoryStore;

/// Clock of the V1 golden runs (Node 22, TZ=Asia/Tokyo).
const NOW: LocalDateTime = LocalDateTime::new(2026, 11, 16, 21, 34, 56);

fn sha256(s: &str) -> String {
    Sha256::digest(s.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn stored(store: &MemoryStore) -> String {
    store.get(STORAGE_KEY).unwrap_or_default()
}

/// Golden: spec-lib-core Appendix G "MemoryManager" (V1 under Node 22).
#[test]
fn appendix_g_scenario() {
    let store = MemoryStore::new();
    store.set(STORAGE_KEY, G_INITIAL);
    let m = MemoryManager::new(&store, &NOW);
    assert_eq!(stored(&store), G_INITIAL, "loading never writes back");
    assert_eq!(m.memory_context(), G_CONTEXT);
    let mut rng = SequenceRng::new([0.5]);
    assert_eq!(m.dynamic_context(3.0, &NOW, &mut rng), G_DYNAMIC);
    assert_eq!(rng.calls(), 1, "one random number per mood hint");
    assert_eq!(time_context(NOW.hour), "夜");
    m.record_interaction(&NOW);
    assert_eq!(m.total_interactions(), 42);
    m.record_emotion("ANGRY");
    m.record_emotion("ANGRY");
    assert!(m.is_emotion_repeated());
    assert!(!m.set_user_name(""));
    assert_eq!(m.user_name(), "Salieri");
    assert!(m.set_user_name("  オカリン "));
    assert_eq!(m.user_name(), "オカリン");
    m.add_user_fact("dr pepperが好き", &NOW);
    m.add_episodic_memory("映画を見た", &["映画", "夜", "映画 "], &NOW);
    let long = "これは五十文字を超える非常に長いメッセージです。".repeat(3);
    let mut history = vec![ChatMessage::system("SYS"), ChatMessage::user(long)];
    for i in 2..=32 {
        let content = format!("msg{i}");
        history.push(if i % 2 == 0 {
            ChatMessage::assistant(content)
        } else {
            ChatMessage::user(content)
        });
    }
    assert_eq!(m.trim_conversation_history(&mut history, 30, &NOW), 6);
    assert_eq!(history.len(), 27);
    assert_eq!(
        history[..3],
        [
            ChatMessage::system("SYS"),
            ChatMessage::user("msg7"),
            ChatMessage::assistant("msg8")
        ]
    );
    assert_eq!(m.memory_context(), G_CONTEXT_AFTER);
    assert_eq!(stored(&store), G_STORED_AFTER);
    assert_eq!(m.to_json(), G_STORED_AFTER);
    m.clear_all_memory();
    assert_eq!(stored(&store), "{}");
    assert_eq!(m.memory_context(), G_CONTEXT_EMPTY);
    // A reload of the V1 output gives the same prompt.
    store.set(STORAGE_KEY, G_STORED_AFTER);
    let reloaded = MemoryManager::new(&store, &NOW);
    assert_eq!(reloaded.memory_context(), G_CONTEXT_AFTER);
}

/// Golden: V1 key order on a fresh memory follows the first mutation of each field; a pushed
/// episode is written `timestamp`-first until the next push re-normalizes it.
#[test]
fn fresh_memory_key_order_matches_v1() {
    let store = MemoryStore::new();
    let m = MemoryManager::new(&store, &NOW);
    assert_eq!(m.payload(), MemoryPayload::default());
    assert_eq!(m.to_json(), "{}");
    m.record_emotion("SMILE");
    assert_eq!(stored(&store), X_FRESH1);
    m.set_user_name(" Salieri ");
    m.add_user_fact("  東京在住  ", &NOW);
    m.add_episodic_memory(
        "  一緒に実験した ",
        &["実験", " 実験", "ラボ", "LAB", "lab", "  "],
        &NOW,
    );
    m.record_interaction(&NOW);
    assert_eq!(stored(&store), X_FRESH2);
    m.add_episodic_memory::<&str>("二つ目", &[], &NOW);
    assert_eq!(stored(&store), X_FRESH3);
    assert_eq!(m.memory_context(), X_FRESH_CTX);
}

/// Golden: V1 load normalization, coercion and reset cases (state after `setUserName("U")`).
#[test]
fn load_normalization_matches_v1() {
    for (label, raw, expected) in X_RESETS {
        let store = MemoryStore::new();
        store.set(STORAGE_KEY, raw);
        let m = MemoryManager::new(&store, &NOW);
        assert!(m.set_user_name("U"));
        assert_eq!(stored(&store), expected, "{label}");
    }
    // Missing or empty value: `{}`.
    let store = MemoryStore::new();
    store.set(STORAGE_KEY, "");
    let m = MemoryManager::new(&store, &NOW);
    m.set_user_name("U");
    assert_eq!(stored(&store), r#"{"userName":"U"}"#);
}

/// V2 coercions of wrongly typed top-level fields (V1 crashed or concatenated strings).
#[test]
fn corrupted_top_level_fields_are_coerced() {
    let store = MemoryStore::new();
    store.set(
        STORAGE_KEY,
        r#"{"userName":42,"totalInteractions":"5","lastSessionDate":5,"recentEmotions":["A",null,3,{"x":1}]}"#,
    );
    let m = MemoryManager::new(&store, &NOW);
    assert_eq!(m.user_name(), "42");
    assert_eq!(m.total_interactions(), 0);
    m.record_interaction(&NOW);
    assert_eq!(m.total_interactions(), 1);
    let payload = m.payload();
    assert_eq!(
        payload.recent_emotions,
        Some(vec![
            "A".to_owned(),
            String::new(),
            "3".to_owned(),
            "[object Object]".to_owned()
        ])
    );
    assert!(m.memory_context().contains("「42」と呼びなさい"));
    assert!(m
        .memory_context()
        .contains("【最近の感情傾向】A→→3→[object Object]（"));
    let store = MemoryStore::new();
    store.set(STORAGE_KEY, r#"{"totalInteractions":2.9,"userName":true}"#);
    let m = MemoryManager::new(&store, &NOW);
    assert_eq!((m.total_interactions(), m.user_name()), (2, String::new()));
}

/// Golden: V1 pruning (50 facts FIFO, 10 summaries, 50 episodes, 10 emotions) and the prompt.
#[test]
fn pruning_limits_match_v1() {
    let store = MemoryStore::new();
    let m = MemoryManager::new(&store, &NOW);
    for i in 1..=52 {
        m.add_user_fact(&format!("fact{i}"), &NOW);
    }
    m.add_user_fact("FACT1", &NOW);
    m.add_user_fact("fact3", &NOW);
    for i in 1..=12 {
        m.add_conversation_summary(&format!("sum{i}"), &NOW);
    }
    for i in 1..=45 {
        m.add_episodic_memory(&format!("ep{i}"), &[format!("t{i}")], &NOW);
    }
    for i in 1..=12 {
        m.record_emotion(&format!("E{i}"));
    }
    let json = stored(&store);
    assert_eq!(crate::jscompat::utf16_len(&json), 6178);
    assert_eq!(
        sha256(&json),
        "4afde5106d2887a3ff244f6bd26265279d23530d2b621ecad88b1e7297a7f44e"
    );
    let payload = m.payload();
    assert_eq!(
        payload.user_facts.as_ref().map(Vec::len),
        Some(MAX_USER_FACTS)
    );
    assert_eq!(
        payload.conversation_summaries.as_ref().map(Vec::len),
        Some(MAX_CONVERSATION_SUMMARIES)
    );
    assert_eq!(
        payload.episodic_memory.as_ref().map(Vec::len),
        Some(MAX_EPISODIC_MEMORIES)
    );
    assert_eq!(
        payload.recent_emotions.as_ref().map(Vec::len),
        Some(EMOTION_HISTORY_SIZE)
    );
    assert_eq!(m.memory_context(), X_PRUNE_CTX);
    assert!(!m.is_emotion_repeated());
}

/// Golden: V1 `trimConversationHistory` edge cases.
#[test]
fn trim_conversation_history_matches_v1() {
    let store = MemoryStore::new();
    let m = MemoryManager::new(&store, &NOW);
    // A stray system message at index 1 is removed and counted.
    let mut h = vec![ChatMessage::system("S"), ChatMessage::system("stray")];
    for i in 0..31 {
        let content = format!("{}{i}", "あ".repeat(60));
        h.push(if i % 2 == 1 {
            ChatMessage::assistant(content)
        } else {
            ChatMessage::user(content)
        });
    }
    assert_eq!(m.trim_conversation_history(&mut h, 30, &NOW), 5);
    assert_eq!(h.len(), 28);
    assert_eq!(h[1], ChatMessage::user(format!("{}4", "あ".repeat(60))));
    assert_eq!(stored(&store), X_TRIM1_STORE);
    // Without a system prompt, index 0 is never trimmed.
    let mut h2 = vec![ChatMessage::user("no system")];
    for i in 0..30 {
        h2.push(ChatMessage::assistant(format!("m{i}")));
    }
    assert_eq!(m.trim_conversation_history(&mut h2, 30, &NOW), 5);
    assert_eq!(h2.len(), 26);
    assert_eq!(
        (&h2[0], &h2[1]),
        (
            &ChatMessage::user("no system"),
            &ChatMessage::assistant("m5")
        )
    );
    let mut h3 = vec![ChatMessage::system("S"), ChatMessage::user("x")];
    assert_eq!(m.trim_conversation_history(&mut h3, 30, &NOW), 0);
    // The removal stops when only index 0 is left; the summary is trimmed when stored.
    let mut h4 = vec![ChatMessage::system("S")];
    for i in 0..5 {
        h4.push(ChatMessage::user(format!("u{i}   ")));
    }
    assert_eq!(m.trim_conversation_history(&mut h4, 2, &NOW), 5);
    assert_eq!(h4.len(), 1);
    assert_eq!(
        sha256(&stored(&store)),
        "50a5ca04016f323d902976034c889ae4cb3f1e154bcc9b68c65a7c194bb5a0d7"
    );
}

/// Deviation (spec §1.1): a cut inside a surrogate pair drops the half instead of storing a
/// lone surrogate; the 300-unit summary cap counts UTF-16 units.
#[test]
fn trim_counts_utf16_units() {
    let store = MemoryStore::new();
    let m = MemoryManager::new(&store, &NOW);
    let mut h = vec![ChatMessage::system("S")];
    h.push(ChatMessage::user(format!("{}😀tail", "a".repeat(49))));
    for _ in 0..5 {
        h.push(ChatMessage::assistant("😀".repeat(25)));
    }
    assert_eq!(m.trim_conversation_history(&mut h, 0, &NOW), 6);
    assert_eq!(h, [ChatMessage::system("S")]);
    let summaries = m.payload().conversation_summaries.unwrap_or_default();
    let expected_head = format!(
        "[11/16 21:34の会話] user: {}... / assistant: {} / ",
        "a".repeat(49),
        "😀".repeat(25)
    );
    assert!(summaries[0].starts_with(&expected_head), "{}", summaries[0]);
    assert!(summaries[0].ends_with("😀..."));
    // 300 units would split an emoji: 299 units + "...".
    assert_eq!(crate::jscompat::utf16_len(&summaries[0]), 302);
}

/// Golden: V1 itself stores a lone surrogate when a 50-unit preview or the 300-unit cap splits
/// an emoji (`JSON.stringify` writes `\ud83d`), and V1's `JSON.parse` loads it with every other
/// field. V2 loads the same memory with only the lone halves dropped (exactly what V2's own
/// trim stores) instead of resetting it; the next save is V1's bytes minus the `\ud83d`s.
#[test]
fn v1_lone_surrogate_escapes_load() {
    let without_half = |s: &str| s.replace("\\ud83d", "");
    let store = MemoryStore::new();
    store.set(STORAGE_KEY, X_LONE_STORED);
    let m = MemoryManager::new(&store, &NOW);
    assert_eq!(stored(&store), X_LONE_STORED, "loading never writes back");
    assert_eq!(m.user_name(), "オカリン");
    assert_eq!(m.total_interactions(), 5);
    let v1_ctx: String =
        serde_json::from_str(&without_half(X_LONE_V1_CTX_JSON)).unwrap_or_default();
    assert!(v1_ctx.contains("- 東京在住（強度:1 / 信頼度:低）"));
    assert_eq!(m.memory_context(), v1_ctx);

    // The loaded summaries are exactly what V2's own trim stores for the same histories.
    let fresh = MemoryStore::new();
    let v2 = MemoryManager::new(&fresh, &NOW);
    let mut h1 = vec![
        ChatMessage::system("SYS"),
        ChatMessage::user(format!("{}😀 楽しかった", "a".repeat(49))),
    ];
    for i in 0..30 {
        let content = format!("msg{i}");
        h1.push(if i % 2 == 1 {
            ChatMessage::user(content)
        } else {
            ChatMessage::assistant(content)
        });
    }
    assert_eq!(v2.trim_conversation_history(&mut h1, 30, &NOW), 5);
    let mut h2 = vec![ChatMessage::system("SYS"), ChatMessage::user("xx")];
    for i in 0..5 {
        let content = "😀".repeat(25);
        h2.push(if i % 2 == 1 {
            ChatMessage::user(content)
        } else {
            ChatMessage::assistant(content)
        });
    }
    assert_eq!(v2.trim_conversation_history(&mut h2, 2, &NOW), 6);
    let loaded = m.payload();
    assert_eq!(
        loaded.conversation_summaries,
        v2.payload().conversation_summaries
    );
    assert_eq!(
        loaded.episodic_memory.as_ref().map(Vec::len),
        Some(2),
        "the mirrored episodes load too"
    );

    m.record_emotion("SMILE");
    assert_eq!(stored(&store), without_half(X_LONE_V1_AFTER));
}

/// Only unpaired surrogate escapes inside string literals are dropped; everything V1's
/// `JSON.parse` rejects is still rejected (memory reset).
#[test]
fn lone_surrogate_escapes_are_dropped_before_parsing() {
    let dropped = [
        (r#"["\ud83d"]"#, r#"[""]"#),
        (r#"["a\uD83Db"]"#, r#"["ab"]"#),
        (r#"["\ude00x"]"#, r#"["x"]"#),
        (r#"["\ud83d😀"]"#, r#"["😀"]"#),
        (r#"["\ude00\ud83d"]"#, r#"[""]"#),
        (r#"["\ud83dA"]"#, r#"["A"]"#),
        (r#"["\\\ud83d"]"#, r#"["\\"]"#),
        (r#"["\ud83d\n"]"#, r#"["\n"]"#),
        (r#"{"\ud800k":"\"\udfff"}"#, r#"{"k":"\""}"#),
        (r#"["é\ud83d😀"]"#, r#"["é😀"]"#),
    ];
    for (raw, expected) in dropped {
        assert_eq!(drop_lone_surrogate_escapes(raw), expected, "{raw}");
        assert!(
            serde_json::from_str::<Value>(expected).is_ok(),
            "{expected}"
        );
    }
    // Pairs, literal backslashes, escapes outside strings and text with an invalid escape stay
    // as they are (dropping `\uD8D6` would turn `\ud03` into the valid `퀾`).
    for raw in [
        r#"{"a":"😀"}"#,
        r#"["😀", "􏿿"]"#,
        r#"["\\ud83d"]"#,
        r#"[1]\ud83d"#,
        r#"["\ud83g"]"#,
        r#"["\ud83"]"#,
        r#"["\ud03\uD8D6e"]"#,
        r#"["\ud83d\q"]"#,
    ] {
        assert!(
            matches!(drop_lone_surrogate_escapes(raw), Cow::Borrowed(s) if s == raw),
            "{raw}"
        );
    }
    let loaded = parse_memory_json(
        r#"{"userName":"A\ud83d","conversationSummaries":["\udc00s"],"episodicMemory":[{"summary":"e\ud83d","tags":["t\ud83d"]}]}"#,
        "2026-11-16",
    );
    let loaded = loaded.unwrap_or_default();
    assert_eq!(loaded.user_name.as_deref(), Some("A"));
    assert_eq!(loaded.conversation_summaries, Some(vec!["s".to_owned()]));
    assert_eq!(
        loaded.episodic_memory,
        Some(vec![EpisodicMemory {
            timestamp: "2026-11-16".into(),
            summary: "e".into(),
            tags: vec!["t".into()],
        }])
    );
    // V1 throws on these too: still a reset.
    for raw in [
        r#"{"userName":"A"}\ud83d"#,
        r#"{"userName":"\ud83g"}"#,
        r#"{"userName":"\ud03\ud83de"}"#,
    ] {
        assert_eq!(parse_memory_json(raw, "2026-11-16"), None, "{raw}");
    }
}

/// Golden: V1 `getDynamicContext` for every hour (`random() = (h % 5 + 0.5) / 5`, turn 2.5 at
/// 03:00), serialized with `JSON.stringify`.
#[test]
fn dynamic_context_matches_v1_for_every_hour() {
    let mut outputs = Vec::new();
    for hour in 0..24_u32 {
        let now = LocalDateTime::new(2027, 1, 2, hour, 5, 9);
        let mut rng = SequenceRng::new([(f64::from(hour % 5) + 0.5) / 5.0]);
        let turn = if hour == 3 { 2.5 } else { f64::from(hour) };
        outputs.push(dynamic_context(turn, &now, &mut rng));
    }
    let json = serde_json::to_string(&outputs).unwrap_or_default();
    assert_eq!(
        sha256(&json),
        "08a931590bcd19a8e127ab2d5e47910db1fd5044643ac7d731f22e1398c7feec"
    );
    let mut bad_rng = SequenceRng::new([1.0, f64::NAN, -0.5]);
    for _ in 0..3 {
        assert!(dynamic_context(1.0, &NOW, &mut bad_rng).ends_with(MOOD_HINTS[0]));
    }
    let table: Vec<&str> = (0..24).map(time_context).collect();
    assert_eq!(table[4], "深夜");
    assert_eq!(table[5], "朝");
    assert_eq!(table[10], "午前中");
    assert_eq!(table[12], "昼");
    assert_eq!(table[14], "午後");
    assert_eq!(table[17], "夕方");
    assert_eq!(table[20], "夜");
}

/// Golden: fact order by strength, date, then ja collation; legacy summaries when there is no
/// episode; same-day episodes ordered by timestamp string descending.
#[test]
fn prompt_order_matches_v1() {
    let store = MemoryStore::new();
    store.set(
        STORAGE_KEY,
        r#"{"userName":"SALIERI","totalInteractions":0,"lastSessionDate":"","recentEmotions":["A","B"],"userFacts":[{"content":"ゲーム","strength":1,"lastUpdated":"2026-11-16"},{"content":"アニメ","strength":1,"lastUpdated":"2026-11-16"},{"content":"東京","strength":1,"lastUpdated":"2026-11-16"},{"content":"猫","strength":1,"lastUpdated":"2026-11-16"},{"content":"名前","strength":1,"lastUpdated":"2026-11-16"},{"content":"abc","strength":1,"lastUpdated":"2026-11-16"},{"content":"ABC","strength":1,"lastUpdated":"2026-11-15"},{"content":"10代","strength":2,"lastUpdated":"2026-11-01"},{"content":"Zebra","strength":1,"lastUpdated":"2026-11-17"}],"conversationSummaries":["s1","s2","s3","s4"],"episodicMemory":[]}"#,
    );
    assert_eq!(
        MemoryManager::new(&store, &NOW).memory_context(),
        X_TIES_CTX
    );
    store.set(
        STORAGE_KEY,
        r#"{"episodicMemory":[{"summary":"a","timestamp":"2026-11-16"},{"summary":"b","timestamp":"2026-11-16 23:00"},{"summary":"c","timestamp":"2026-11-15"},{"summary":"d","timestamp":"2026-11-16T01:00"},{"summary":"e","timestamp":"2026-11-17"}]}"#,
    );
    assert_eq!(
        MemoryManager::new(&store, &NOW).memory_context(),
        X_EPISODES_CTX
    );
}

/// The collator decides ties only; an injected one (the browser's `Intl.Collator("ja")`)
/// replaces the fallback.
#[test]
fn collator_injection() {
    let store = MemoryStore::new();
    let m = MemoryManager::new(&store, &NOW);
    for fact in ["b", "a", "c"] {
        m.add_user_fact(fact, &NOW);
    }
    m.add_user_fact("c", &NOW);
    let facts = |m: &MemoryManager<'_, MemoryStore>| -> Vec<String> {
        m.memory_context()
            .lines()
            .filter(|l| l.starts_with("- "))
            .map(str::to_owned)
            .collect()
    };
    assert_eq!(
        facts(&m),
        [
            "- c（強度:2 / 信頼度:中）",
            "- a（強度:1 / 信頼度:低）",
            "- b（強度:1 / 信頼度:低）"
        ]
    );
    m.set_collator(|a: &str, b: &str| b.cmp(a));
    assert_eq!(
        facts(&m)[1..],
        ["- b（強度:1 / 信頼度:低）", "- a（強度:1 / 信頼度:低）"]
    );
    // An inconsistent comparator cannot make the sort panic.
    let flip = Cell::new(false);
    m.set_collator(move |_: &str, _: &str| {
        flip.set(!flip.get());
        if flip.get() {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });
    assert_eq!(facts(&m).len(), 3);
}

#[test]
fn helpers_match_v1() {
    assert_eq!(
        dedupe_tags(&["実験", "実験 ", "ラボ", "ラボ"]),
        ["実験", "ラボ"]
    );
    assert_eq!(dedupe_tags(&["x", "X"]), ["x"]);
    assert_eq!(
        dedupe_tags(&["\u{FEFF} a ", "A", "\u{85}"]),
        ["a", "\u{85}"]
    );
    assert_eq!(normalize_fact_key("  Dr Pepper  "), "dr pepper");
    assert_eq!(normalize_fact_key("ΑΣ"), "ας");
    assert!(is_newer_date_string("2026-11-16", "2026-11-15"));
    assert!(
        is_newer_date_string("2026-02-30", "2026-03-01 x"),
        "Feb 30 is Mar 2"
    );
    assert!(!is_newer_date_string("2026-03-01", "2026-02-30"));
    assert!(is_newer_date_string("2026-11-16", "bad"));
    assert!(!is_newer_date_string("bad", "2026-11-16"));
    assert!(is_newer_date_string("b", "a"));
    assert!(!is_newer_date_string("a", "b"));
    assert!(
        is_newer_date_string("\u{FF61}", "\u{1F600}"),
        "UTF-16 code unit order"
    );
    assert_eq!(certainty_label(4), "高");
    assert_eq!(certainty_label(2), "中");
    assert_eq!(certainty_label(1), "低");
    assert_eq!(utf16_prefix("a😀b", 2), "a");
    assert_eq!(utf16_prefix("a😀b", 3), "a😀");
    assert_eq!(ellipsize("abc", 3), "abc");
    assert_eq!(ellipsize("abcd", 3), "abc...");
}

#[test]
fn signals_and_reentrancy() {
    // Handlers must be `'static`, so the store lives for the whole test run.
    let store: &'static MemoryStore = Box::leak(Box::new(MemoryStore::new()));
    let m = Rc::new(MemoryManager::new(store, &NOW));
    let names = Rc::new(RefCell::new(Vec::<String>::new()));
    let stats = Rc::new(Cell::new(0));
    let (m2, names2) = (Rc::clone(&m), Rc::clone(&names));
    m.user_name_changed().connect(move |_| {
        // Re-entrant read inside the handler.
        names2.borrow_mut().push(m2.user_name());
    });
    let stats2 = Rc::clone(&stats);
    m.stats_changed()
        .connect(move |_| stats2.set(stats2.get() + 1));
    assert!(!m.set_user_name("   "));
    assert!(m.set_user_name("Salieri"));
    m.record_interaction(&NOW);
    m.record_emotion("SMILE");
    m.add_user_fact("x", &NOW);
    assert_eq!(*names.borrow(), ["Salieri"]);
    assert_eq!(stats.get(), 1);
    m.clear_all_memory();
    assert_eq!(*names.borrow(), ["Salieri", ""]);
    assert_eq!(stats.get(), 2);
}

/// What a browser's `String(error)` gives for a full `localStorage`.
const QUOTA_ERROR: &str = "QuotaExceededError: The quota has been exceeded.";

/// A `localStorage` whose writes fail while `fail` is set (quota exceeded). Only `try_set` is
/// allowed: V1's `saveMemory` caught the exception, so the manager must learn about failures.
#[derive(Default)]
struct FullStore {
    inner: MemoryStore,
    fail: Cell<bool>,
}

impl KvStore for FullStore {
    fn get(&self, key: &str) -> Option<String> {
        self.inner.get(key)
    }
    fn set(&self, _key: &str, _value: &str) {
        panic!("the memory manager must save through try_set");
    }
    fn try_set(&self, key: &str, value: &str) -> Result<(), String> {
        if self.fail.get() {
            return Err(QUOTA_ERROR.to_owned());
        }
        self.inner.set(key, value);
        Ok(())
    }
    fn remove(&self, key: &str) {
        self.inner.remove(key);
    }
    fn keys(&self) -> Vec<String> {
        self.inner.keys()
    }
}

#[test]
fn save_failed_warning_is_v1_console_text() {
    assert_eq!(
        save_failed_warning(QUOTA_ERROR),
        "[MemoryManager] Failed to persist memory to localStorage: QuotaExceededError: The quota has been exceeded."
    );
    assert_eq!(
        save_failed_warning(""),
        "[MemoryManager] Failed to persist memory to localStorage: "
    );
}

/// V1 `saveMemory()`: every failed write logs the warning, the change stays in memory, nothing
/// throws, and the next successful write stores the whole state.
#[test]
fn failed_saves_warn_every_time_and_never_crash() {
    // Handlers must be `'static`, so the store lives for the whole test run.
    let store: &'static FullStore = Box::leak(Box::default());
    let m = Rc::new(MemoryManager::new(store, &NOW));
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let (weak, log2) = (Rc::downgrade(&m), Rc::clone(&log));
    m.save_failed().connect(move |error: &str| {
        // Re-entrant calls inside the handler: the state borrow is already released.
        let m = weak.upgrade().unwrap();
        assert!(m.to_json().starts_with('{'));
        log2.borrow_mut().push(save_failed_warning(error));
    });
    let log2 = Rc::clone(&log);
    m.user_name_changed()
        .connect(move |_| log2.borrow_mut().push("user_name_changed".to_owned()));
    let log2 = Rc::clone(&log);
    m.stats_changed()
        .connect(move |_| log2.borrow_mut().push("stats_changed".to_owned()));
    let warning = save_failed_warning(QUOTA_ERROR);
    let warnings =
        |log: &RefCell<Vec<String>>| log.borrow().iter().filter(|l| **l == warning).count();

    // A working store: no warning.
    assert!(m.set_user_name("Okabe"));
    assert_eq!(*log.borrow(), ["user_name_changed"]);
    let persisted = store.get(STORAGE_KEY).unwrap();
    assert_eq!(persisted, m.to_json());
    log.borrow_mut().clear();

    store.fail.set(true);
    // V1 order: the warning (inside saveMemory) comes before the mutator's signal.
    assert!(m.set_user_name("Salieri"));
    assert_eq!(*log.borrow(), [warning.as_str(), "user_name_changed"]);
    assert_eq!(m.user_name(), "Salieri", "the change stays in memory");
    m.record_interaction(&NOW);
    assert_eq!(log.borrow()[2..], [warning.as_str(), "stats_changed"]);
    m.record_emotion("SMILE");
    m.add_user_fact("Dr Pepperが好き", &NOW);
    m.add_conversation_summary("要約テスト", &NOW);
    m.add_episodic_memory("実験した", &["ラボ"], &NOW);
    // Ignored (blank) inputs do not save, so they do not warn.
    assert!(!m.set_user_name("  "));
    m.add_user_fact(" ", &NOW);
    m.add_episodic_memory("", &["x"], &NOW);
    assert_eq!(warnings(&log), 6, "one warning per failed write");
    assert_eq!(
        store.get(STORAGE_KEY).unwrap(),
        persisted,
        "nothing new was stored"
    );
    assert_eq!(m.total_interactions(), 1);
    assert!(m.memory_context().contains("Dr Pepperが好き"));

    // Writes work again: no retry until the next save, which stores the whole state.
    store.fail.set(false);
    m.record_emotion("SMILE");
    assert_eq!(warnings(&log), 6);
    assert_eq!(store.get(STORAGE_KEY).unwrap(), m.to_json());
    let saved = parse_memory_json(&store.get(STORAGE_KEY).unwrap(), &NOW.date_iso()).unwrap();
    assert_eq!(saved, m.payload());
    assert_eq!(saved.user_name.as_deref(), Some("Salieri"));

    // `clearAllMemory()` and the submit path warn too.
    store.fail.set(true);
    m.clear_all_memory();
    assert_eq!(warnings(&log), 7);
    assert_eq!(m.to_json(), "{}");
    let mut history = vec![ChatMessage::system("SYS")];
    m.record_user_turn(&mut history, ChatMessage::user("hi"), &NOW);
    assert_eq!(warnings(&log), 8);
    assert_eq!(m.total_interactions(), 1);
    assert_ne!(store.get(STORAGE_KEY).unwrap(), "{}");
}

#[test]
fn chat_integration() {
    let store = MemoryStore::new();
    let m = MemoryManager::new(&store, &NOW);
    let cleaned = m.apply_memory_commands(
        "[SMILE] 覚えておくわ。[SAVE_FACT: ユーザーの名前は岡部] [SAVE_EVENT: {\"summary\": \"一緒に実験した\", \"tags\": \"実験、ラボ\"}]",
        &NOW,
    );
    assert_eq!(cleaned, "[SMILE] 覚えておくわ。");
    assert_eq!(m.user_name(), "岡部");
    let payload = m.payload();
    assert_eq!(
        payload.user_facts,
        Some(vec![UserFact {
            content: "ユーザーの名前は岡部".into(),
            strength: 1,
            last_updated: "2026-11-16".into()
        }])
    );
    assert_eq!(
        payload.episodic_memory,
        Some(vec![EpisodicMemory {
            timestamp: "2026-11-16".into(),
            summary: "一緒に実験した".into(),
            tags: vec!["実験".into(), "ラボ".into()]
        }])
    );

    let base = "PERSONA";
    let initial = m.initial_history(base);
    assert_eq!(initial.len(), 1);
    assert_eq!(initial[0].role, ChatRole::System);
    assert_eq!(
        initial[0].content,
        format!("PERSONA\n\n{}", m.memory_context())
    );

    // V1 submit order: request from the untrimmed history, then the stored-history update.
    let mut history = initial.clone();
    let user = ChatMessage::user("こんにちは");
    let mut request_history = history.clone();
    request_history.push(user.clone());
    let mut rng = SequenceRng::new([0.0, 0.99]);
    let request = m.update_system_prompt(&request_history, base, 0.0, &NOW, &mut rng);
    assert!(request[0]
        .content
        .ends_with("会話ターン数: 0\n（今は少しリラックスしている）"));
    assert!(!request[0].content.contains("【累計やりとり回数】"));
    assert_eq!(request[1], user);
    assert_eq!(m.record_user_turn(&mut history, user, &NOW), 0);
    assert_eq!(m.total_interactions(), 1);
    let stored_history = m.update_system_prompt(&history, base, 0.0, &NOW, &mut rng);
    assert!(stored_history[0]
        .content
        .contains("【累計やりとり回数】1回"));
    assert!(stored_history[0].content.ends_with("（いつも通りの調子）"));
    assert_eq!(rng.calls(), 2);
    // Without a system message at index 0 nothing is replaced.
    let imported = vec![ChatMessage::user("a")];
    assert_eq!(
        m.update_system_prompt(&imported, base, 1.0, &NOW, &mut rng),
        imported
    );
}

// ---- V1 golden outputs ------------------------------------------------------------------------

const G_INITIAL: &str = "{\"userName\":\"Salieri\",\"totalInteractions\":41,\"lastSessionDate\":\"2026-11-15 20:00\",\"recentEmotions\":[\"SMILE\",\"SMILE\",\"ANGRY\"],\"userFacts\":[{\"content\":\"Dr Pepperが好き\",\"strength\":3,\"lastUpdated\":\"2026-11-10\"},{\"content\":\"dr pepperが好き \",\"strength\":\"2\",\"lastUpdated\":\"2026-11-12\"},{\"content\":\"猫を飼っている\",\"strength\":1,\"lastUpdated\":\"bad\"},{\"content\":\"  \",\"strength\":5},{\"content\":\"ゲームが好き\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"アニメが好き\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"東京在住\",\"strength\":4,\"lastUpdated\":\"2026-11-01\"}],\"conversationSummaries\":[\"古い要約\"],\"episodicMemory\":[{\"timestamp\":\"2026-11-14\",\"summary\":\"一緒に実験した\",\"tags\":[\"実験\",\"実験 \",\"ラボ\",\"ラボ\"]},{\"timestamp\":\"2026-11-16\",\"summary\":\"A\",\"tags\":[]},{\"timestamp\":\"2026-11-16\",\"summary\":\"B\"},{\"timestamp\":\"2026-11-16\",\"summary\":\"C\",\"tags\":[\"x\",\"X\"]},{\"timestamp\":\"2026-11-16\",\"summary\":\"D\"},{\"timestamp\":\"2026-11-16\",\"summary\":\"E\"},{\"timestamp\":\"\",\"summary\":\"時刻なし\"}]}";
const G_CONTEXT: &str = "【ユーザー情報】ユーザーのことは「あなた」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。\n【ユーザーについて知っていること（重要度順）】\n- Dr Pepperが好き（強度:5 / 信頼度:高）\n- 東京在住（強度:4 / 信頼度:高）\n- アニメが好き（強度:1 / 信頼度:低）\n- ゲームが好き（強度:1 / 信頼度:低）\n- 猫を飼っている（強度:1 / 信頼度:低）\n【最近のエピソード記憶】\n- [2026-11-16] A\n- [2026-11-16] B\n- [2026-11-16] C（タグ: x）\n- [2026-11-16] D\n【前回のセッション】2026-11-15 20:00\n【累計やりとり回数】41回\n【最近の感情傾向】SMILE→SMILE→ANGRY（同じ感情が続きすぎないように意識して）";
const G_DYNAMIC: &str = "【現在の状況】現在日時（クライアント時刻）: 2026/11/16 21:34:56 (夜) / 会話ターン数: 3\n（少し眠そう）";
const G_CONTEXT_AFTER: &str = "【ユーザー情報】ユーザーのことは「オカリン」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。\n【ユーザーについて知っていること（重要度順）】\n- Dr Pepperが好き（強度:6 / 信頼度:高）\n- 東京在住（強度:4 / 信頼度:高）\n- アニメが好き（強度:1 / 信頼度:低）\n- ゲームが好き（強度:1 / 信頼度:低）\n- 猫を飼っている（強度:1 / 信頼度:低）\n【最近のエピソード記憶】\n- [2026-11-16] A\n- [2026-11-16] B\n- [2026-11-16] C（タグ: x）\n- [2026-11-16] D\n【前回のセッション】2026-11-16 21:34\n【累計やりとり回数】42回\n【最近の感情傾向】SMILE→SMILE→ANGRY→ANGRY→ANGRY（同じ感情が続きすぎないように意識して）";
const G_STORED_AFTER: &str = "{\"userName\":\"オカリン\",\"totalInteractions\":42,\"lastSessionDate\":\"2026-11-16 21:34\",\"recentEmotions\":[\"SMILE\",\"SMILE\",\"ANGRY\",\"ANGRY\",\"ANGRY\"],\"userFacts\":[{\"content\":\"Dr Pepperが好き\",\"strength\":6,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"猫を飼っている\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"ゲームが好き\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"アニメが好き\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"東京在住\",\"strength\":4,\"lastUpdated\":\"2026-11-01\"}],\"conversationSummaries\":[\"古い要約\",\"[11/16 21:34の会話] user: これは五十文字を超える非常に長いメッセージです。これは五十文字を超える非常に長いメッセージです。これ... / assistant: msg2 / user: msg3 / assistant: msg4 / user: msg5 / assistant: msg6\"],\"episodicMemory\":[{\"summary\":\"一緒に実験した\",\"timestamp\":\"2026-11-14\",\"tags\":[\"実験\",\"ラボ\"]},{\"summary\":\"A\",\"timestamp\":\"2026-11-16\",\"tags\":[]},{\"summary\":\"B\",\"timestamp\":\"2026-11-16\",\"tags\":[]},{\"summary\":\"C\",\"timestamp\":\"2026-11-16\",\"tags\":[\"x\"]},{\"summary\":\"D\",\"timestamp\":\"2026-11-16\",\"tags\":[]},{\"summary\":\"E\",\"timestamp\":\"2026-11-16\",\"tags\":[]},{\"summary\":\"時刻なし\",\"timestamp\":\"2026-11-16\",\"tags\":[]},{\"summary\":\"映画を見た\",\"timestamp\":\"2026-11-16\",\"tags\":[\"映画\",\"夜\"]},{\"timestamp\":\"2026-11-16\",\"summary\":\"[11/16 21:34の会話] user: これは五十文字を超える非常に長いメッセージです。これは五十文字を超える非常に長いメッセージです。これ... / assistant: msg2 / user: msg3 / assistant: msg4 / user: msg5 / assistant: msg6\",\"tags\":[\"要約\"]}]}";
const G_CONTEXT_EMPTY: &str = "【ユーザー情報】ユーザーのことは「あなた」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。";
const X_FRESH1: &str = "{\"recentEmotions\":[\"SMILE\"]}";
const X_FRESH2: &str = "{\"recentEmotions\":[\"SMILE\"],\"userName\":\"Salieri\",\"userFacts\":[{\"content\":\"東京在住\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"}],\"episodicMemory\":[{\"timestamp\":\"2026-11-16\",\"summary\":\"一緒に実験した\",\"tags\":[\"実験\",\"ラボ\",\"LAB\"]}],\"totalInteractions\":1,\"lastSessionDate\":\"2026-11-16 21:34\"}";
const X_FRESH3: &str = "{\"recentEmotions\":[\"SMILE\"],\"userName\":\"Salieri\",\"userFacts\":[{\"content\":\"東京在住\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"}],\"episodicMemory\":[{\"summary\":\"一緒に実験した\",\"timestamp\":\"2026-11-16\",\"tags\":[\"実験\",\"ラボ\",\"LAB\"]},{\"timestamp\":\"2026-11-16\",\"summary\":\"二つ目\",\"tags\":[]}],\"totalInteractions\":1,\"lastSessionDate\":\"2026-11-16 21:34\"}";
const X_TRIM1_STORE: &str = "{\"conversationSummaries\":[\"[11/16 21:34の会話] system: stray / user: ああああああああああああああああああああああああああああああああああああああああああああああああああ... / assistant: ああああああああああああああああああああああああああああああああああああああああああああああああああ... / user: ああああああああああああああああああああああああああああああああああああああああああああああああああ... / assistant: ああああああああああああああああああああああああああああああああああああああああああああああああああ...\"],\"episodicMemory\":[{\"timestamp\":\"2026-11-16\",\"summary\":\"[11/16 21:34の会話] system: stray / user: ああああああああああああああああああああああああああああああああああああああああああああああああああ... / assistant: ああああああああああああああああああああああああああああああああああああああああああああああああああ... / user: ああああああああああああああああああああああああああああああああああああああああああああああああああ... / assistant: ああああああああああああああああああああああああああああああああああああああああああああああああああ...\",\"tags\":[\"要約\"]}]}";
const X_FRESH_CTX: &str = "【ユーザー情報】ユーザーのことは「あなた」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。\n【ユーザーについて知っていること（重要度順）】\n- 東京在住（強度:1 / 信頼度:低）\n【最近のエピソード記憶】\n- [2026-11-16] 一緒に実験した（タグ: 実験, ラボ, LAB）\n- [2026-11-16] 二つ目\n【前回のセッション】2026-11-16 21:34\n【累計やりとり回数】1回";
const X_PRUNE_CTX: &str = "【ユーザー情報】ユーザーのことは「あなた」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。\n【ユーザーについて知っていること（重要度順）】\n- FACT1（強度:1 / 信頼度:低）\n- fact10（強度:1 / 信頼度:低）\n- fact11（強度:1 / 信頼度:低）\n- fact12（強度:1 / 信頼度:低）\n- fact13（強度:1 / 信頼度:低）\n- fact14（強度:1 / 信頼度:低）\n【最近のエピソード記憶】\n- [2026-11-16] sum8（タグ: 要約）\n- [2026-11-16] sum9（タグ: 要約）\n- [2026-11-16] sum10（タグ: 要約）\n- [2026-11-16] sum11（タグ: 要約）\n【最近の感情傾向】E3→E4→E5→E6→E7→E8→E9→E10→E11→E12（同じ感情が続きすぎないように意識して）";
const X_TIES_CTX: &str = "【ユーザー情報】ユーザーのことは「あなた」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。\n【ユーザーについて知っていること（重要度順）】\n- abc（強度:2 / 信頼度:中）\n- 10代（強度:2 / 信頼度:中）\n- Zebra（強度:1 / 信頼度:低）\n- アニメ（強度:1 / 信頼度:低）\n- ゲーム（強度:1 / 信頼度:低）\n- 東京（強度:1 / 信頼度:低）\n【過去の会話の記憶】\n- s2\n- s3\n- s4";
const X_EPISODES_CTX: &str = "【ユーザー情報】ユーザーのことは「あなた」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。\n【最近のエピソード記憶】\n- [2026-11-17] e\n- [2026-11-16T01:00] d\n- [2026-11-16 23:00] b\n- [2026-11-16] a";
/// V1-written value with lone-surrogate escapes (spec Appendix G clock): a 50-unit preview cut
/// and a 300-unit summary cut inside an emoji, each in a summary and its mirrored episode.
const X_LONE_STORED: &str = "{\"userName\":\"オカリン\",\"userFacts\":[{\"content\":\"Dr Pepperが好き\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"東京在住\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"}],\"totalInteractions\":5,\"lastSessionDate\":\"2026-11-16 21:34\",\"recentEmotions\":[\"SMILE\"],\"conversationSummaries\":[\"[11/16 21:34の会話] user: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\\ud83d... / assistant: msg0 / user: msg1 / assistant: msg2 / user: msg3\",\"[11/16 21:34の会話] user: xx / assistant: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / user: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / assistant: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / user: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / assistant: 😀😀😀😀😀😀😀\\ud83d...\"],\"episodicMemory\":[{\"summary\":\"[11/16 21:34の会話] user: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\\ud83d... / assistant: msg0 / user: msg1 / assistant: msg2 / user: msg3\",\"timestamp\":\"2026-11-16\",\"tags\":[\"要約\"]},{\"timestamp\":\"2026-11-16\",\"summary\":\"[11/16 21:34の会話] user: xx / assistant: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / user: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / assistant: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / user: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / assistant: 😀😀😀😀😀😀😀\\ud83d...\",\"tags\":[\"要約\"]}]}";
/// V1 `JSON.stringify(getMemoryContext())` after loading [`X_LONE_STORED`] (lone halves kept).
const X_LONE_V1_CTX_JSON: &str = "\"【ユーザー情報】ユーザーのことは「オカリン」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。\\n【ユーザーについて知っていること（重要度順）】\\n- Dr Pepperが好き（強度:1 / 信頼度:低）\\n- 東京在住（強度:1 / 信頼度:低）\\n【最近のエピソード記憶】\\n- [2026-11-16] [11/16 21:34の会話] user: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\\ud83d... / assistant: msg0 / user: msg1 / assistant: msg2 / user: msg3（タグ: 要約）\\n- [2026-11-16] [11/16 21:34の会話] user: xx / assistant: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / user: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / assistant: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / user: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / assistant: 😀😀😀😀😀😀😀\\ud83d...（タグ: 要約）\\n【前回のセッション】2026-11-16 21:34\\n【累計やりとり回数】5回\"";
/// V1 stored value after loading [`X_LONE_STORED`] and `recordEmotion("SMILE")`.
const X_LONE_V1_AFTER: &str = "{\"userName\":\"オカリン\",\"totalInteractions\":5,\"lastSessionDate\":\"2026-11-16 21:34\",\"recentEmotions\":[\"SMILE\",\"SMILE\"],\"userFacts\":[{\"content\":\"Dr Pepperが好き\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"東京在住\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"}],\"conversationSummaries\":[\"[11/16 21:34の会話] user: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\\ud83d... / assistant: msg0 / user: msg1 / assistant: msg2 / user: msg3\",\"[11/16 21:34の会話] user: xx / assistant: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / user: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / assistant: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / user: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / assistant: 😀😀😀😀😀😀😀\\ud83d...\"],\"episodicMemory\":[{\"summary\":\"[11/16 21:34の会話] user: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\\ud83d... / assistant: msg0 / user: msg1 / assistant: msg2 / user: msg3\",\"timestamp\":\"2026-11-16\",\"tags\":[\"要約\"]},{\"summary\":\"[11/16 21:34の会話] user: xx / assistant: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / user: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / assistant: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / user: 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀 / assistant: 😀😀😀😀😀😀😀\\ud83d...\",\"timestamp\":\"2026-11-16\",\"tags\":[\"要約\"]}]}";
const X_RESETS: [(&str, &str, &str); 21] = [
    ("null", "null", "{\"userName\":\"U\"}"),
    ("number", "5", "{\"userName\":\"U\",\"totalInteractions\":0,\"lastSessionDate\":\"\",\"recentEmotions\":[],\"userFacts\":[],\"conversationSummaries\":[],\"episodicMemory\":[]}"),
    ("string", "\"x\"", "{\"userName\":\"U\",\"totalInteractions\":0,\"lastSessionDate\":\"\",\"recentEmotions\":[],\"userFacts\":[],\"conversationSummaries\":[],\"episodicMemory\":[]}"),
    ("array", "[]", "{\"userName\":\"U\",\"totalInteractions\":0,\"lastSessionDate\":\"\",\"recentEmotions\":[],\"userFacts\":[],\"conversationSummaries\":[],\"episodicMemory\":[]}"),
    ("bool", "true", "{\"userName\":\"U\",\"totalInteractions\":0,\"lastSessionDate\":\"\",\"recentEmotions\":[],\"userFacts\":[],\"conversationSummaries\":[],\"episodicMemory\":[]}"),
    ("badjson", "{", "{\"userName\":\"U\"}"),
    ("factsNum", "{\"userFacts\":5}", "{\"userName\":\"U\"}"),
    ("factsObj", "{\"userFacts\":{\"a\":1}}", "{\"userName\":\"U\"}"),
    ("factsStr", "{\"userFacts\":\"abc\",\"episodicMemory\":\"x\"}", "{\"userName\":\"U\",\"totalInteractions\":0,\"lastSessionDate\":\"\",\"recentEmotions\":[],\"userFacts\":[],\"conversationSummaries\":[],\"episodicMemory\":[]}"),
    ("tagNum", "{\"episodicMemory\":[{\"summary\":\"s\",\"tags\":[1]}]}", "{\"userName\":\"U\"}"),
    ("tagNull", "{\"episodicMemory\":[{\"summary\":\"s\",\"tags\":[null]}]}", "{\"userName\":\"U\"}"),
    ("tagsNotArray", "{\"episodicMemory\":[{\"summary\":\"s\",\"tags\":\"a,b\"}]}", "{\"userName\":\"U\",\"totalInteractions\":0,\"lastSessionDate\":\"\",\"recentEmotions\":[],\"userFacts\":[],\"conversationSummaries\":[],\"episodicMemory\":[{\"summary\":\"s\",\"timestamp\":\"2026-11-16\",\"tags\":[]}]}"),
    ("toStringSummary", "{\"conversationSummaries\":[{\"toString\":\"x\"}]}", "{\"userName\":\"U\"}"),
    ("toStringContent", "{\"userFacts\":[{\"content\":{\"toString\":1}}]}", "{\"userName\":\"U\"}"),
    ("toStringEmptyContent", "{\"userFacts\":[{\"content\":\"  \",\"strength\":{\"toString\":1}}]}", "{\"userName\":\"U\",\"totalInteractions\":0,\"lastSessionDate\":\"\",\"recentEmotions\":[],\"userFacts\":[],\"conversationSummaries\":[],\"episodicMemory\":[]}"),
    ("summariesMixed", "{\"conversationSummaries\":[\"a\",1,null,true,[1,null,[2]],{\"x\":1}]}", "{\"userName\":\"U\",\"totalInteractions\":0,\"lastSessionDate\":\"\",\"recentEmotions\":[],\"userFacts\":[],\"conversationSummaries\":[\"a\",\"1\",\"null\",\"true\",\"1,,2\",\"[object Object]\"],\"episodicMemory\":[]}"),
    ("strengths", "{\"userFacts\":[{\"content\":\"a\",\"strength\":\"3\"},{\"content\":\"b\",\"strength\":2.7},{\"content\":\"c\",\"strength\":\"4.9\"},{\"content\":\"d\",\"strength\":0},{\"content\":\"e\",\"strength\":-5},{\"content\":\"f\",\"strength\":null},{\"content\":\"g\",\"strength\":\"abc\"},{\"content\":\"h\",\"strength\":1e21},{\"content\":\"i\",\"strength\":true},{\"content\":\"j\",\"strength\":[5]},{\"content\":\"k\"},{\"content\":\"l\",\"strength\":\"-0\"}]}", "{\"userName\":\"U\",\"totalInteractions\":0,\"lastSessionDate\":\"\",\"recentEmotions\":[],\"userFacts\":[{\"content\":\"a\",\"strength\":3,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"b\",\"strength\":2,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"c\",\"strength\":4,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"d\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"e\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"f\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"g\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"h\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"i\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"j\",\"strength\":5,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"k\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"l\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"}],\"conversationSummaries\":[],\"episodicMemory\":[]}"),
    ("dates", "{\"userFacts\":[{\"content\":\"x\",\"strength\":1,\"lastUpdated\":\"2026-02-30\"},{\"content\":\"X\",\"strength\":1,\"lastUpdated\":\"2026-03-01\"},{\"content\":\"y\",\"lastUpdated\":\" 2026-11-01 extra\"},{\"content\":\"z\",\"lastUpdated\":\"2026/11/01\"},{\"content\":5,\"lastUpdated\":\"0050-01-01\"},{\"content\":null}]}", "{\"userName\":\"U\",\"totalInteractions\":0,\"lastSessionDate\":\"\",\"recentEmotions\":[],\"userFacts\":[{\"content\":\"x\",\"strength\":2,\"lastUpdated\":\"2026-02-30\"},{\"content\":\"y\",\"strength\":1,\"lastUpdated\":\"2026-11-01 extra\"},{\"content\":\"z\",\"strength\":1,\"lastUpdated\":\"2026-11-16\"},{\"content\":\"5\",\"strength\":1,\"lastUpdated\":\"0050-01-01\"}],\"conversationSummaries\":[],\"episodicMemory\":[]}"),
    ("mergeOlder", "{\"userFacts\":[{\"content\":\"Cat\",\"strength\":2,\"lastUpdated\":\"2026-11-12\"},{\"content\":\"cat \",\"strength\":3,\"lastUpdated\":\"2026-11-10\"}]}", "{\"userName\":\"U\",\"totalInteractions\":0,\"lastSessionDate\":\"\",\"recentEmotions\":[],\"userFacts\":[{\"content\":\"Cat\",\"strength\":5,\"lastUpdated\":\"2026-11-12\"}],\"conversationSummaries\":[],\"episodicMemory\":[]}"),
    ("episodes", "{\"episodicMemory\":[{\"summary\":\"  a  \",\"timestamp\":\" 2026-11-01 \",\"tags\":[\"T\",\"t\",\" t \"]},{\"summary\":5,\"timestamp\":\"bad\"},{\"summary\":\"\",\"timestamp\":\"2026-11-01\"},null,7,\"str\",{\"summary\":\"z\",\"timestamp\":\"2026-11-16T09:00:00Z\",\"tags\":[]}]}", "{\"userName\":\"U\",\"totalInteractions\":0,\"lastSessionDate\":\"\",\"recentEmotions\":[],\"userFacts\":[],\"conversationSummaries\":[],\"episodicMemory\":[{\"summary\":\"a\",\"timestamp\":\"2026-11-01\",\"tags\":[\"T\"]},{\"summary\":\"5\",\"timestamp\":\"2026-11-16\",\"tags\":[]},{\"summary\":\"z\",\"timestamp\":\"2026-11-16T09:00:00Z\",\"tags\":[]}]}"),
    ("primitiveFields", "{\"userName\":\"  Okabe  \",\"totalInteractions\":7,\"lastSessionDate\":\"x\",\"recentEmotions\":[\"A\",\"B\"],\"extra\":1}", "{\"userName\":\"U\",\"totalInteractions\":7,\"lastSessionDate\":\"x\",\"recentEmotions\":[\"A\",\"B\"],\"userFacts\":[],\"conversationSummaries\":[],\"episodicMemory\":[]}"),
];
