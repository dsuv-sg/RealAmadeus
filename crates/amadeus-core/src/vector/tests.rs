//! Unit tests of the vector store. Golden values come from running the V1 TypeScript
//! (`lib/amadeus/services/vector-database.ts`) under Node 22 with mocked `fetch` and UUIDs.

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::json;

use super::*;
use crate::random_events::SequenceRng;
use crate::storage::MemoryStore;

const V1_STORE: &str = r#"{"version":1,"entries":[{"id":"00000000-0000-4000-8000-000000000001","content":"猫が好き","category":"general","embedding":[1,0,0],"metadata":{}},{"id":"00000000-0000-4000-8000-000000000002","content":"犬が好き","category":"pets","embedding":[0.9,0.1,0],"metadata":{"n":1.5,"source":"test"}},{"id":"00000000-0000-4000-8000-000000000003","content":"東京在住","category":"place","embedding":[0,1,0],"metadata":{}},{"id":"00000000-0000-4000-8000-000000000004","content":"ints","category":"general","embedding":[1,2,3],"metadata":{"a":[1,2]}},{"id":"00000000-0000-4000-8000-000000000005","content":"tiny","category":"general","embedding":[1e-7,0.25,-0.5],"metadata":{}}]}"#;
const V1_REMOVED: &str = r#"{"version":1,"entries":[{"id":"00000000-0000-4000-8000-000000000001","content":"猫が好き","category":"general","embedding":[1,0,0],"metadata":{}},{"id":"00000000-0000-4000-8000-000000000003","content":"東京在住","category":"place","embedding":[0,1,0],"metadata":{}},{"id":"00000000-0000-4000-8000-000000000004","content":"ints","category":"general","embedding":[1,2,3],"metadata":{"a":[1,2]}},{"id":"00000000-0000-4000-8000-000000000005","content":"tiny","category":"general","embedding":[1e-7,0.25,-0.5],"metadata":{}}]}"#;

fn id(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

/// The V1 test server: known texts embed, anything else fails (`[]`).
fn embed(text: &str) -> Vec<f64> {
    match text {
        "猫が好き" => vec![1.0, 0.0, 0.0],
        "犬が好き" => vec![0.9, 0.1, 0.0],
        "東京在住" => vec![0.0, 1.0, 0.0],
        "query-cat" => vec![1.0, 0.05, 0.0],
        "bad" => vec![f64::NAN, 0.0, 2.0],
        "ints" => vec![1.0, 2.0, 3.0],
        "tiny" => vec![1e-7, 0.25, -0.5],
        _ => Vec::new(),
    }
}

fn settings_store() -> MemoryStore {
    let store = MemoryStore::new();
    store.set("Config_Experimental_RAG_Enabled", "1");
    store.set("Config_Experimental_RAG_Threshold", "0.5");
    store.set("Config_Experimental_RAG_TopK", "2");
    store
}

/// Golden: spec Appendix G cosine / lexical / rounding.
#[test]
fn similarity_matches_v1() {
    assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]), 1.0);
    assert_eq!(
        cosine_similarity(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]),
        0.974_631_846_197_076_2
    );
    assert_eq!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
    assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0, 0.0, 0.0]), 0.0);
    assert_eq!(cosine_similarity(&[0.0, 0.0], &[0.0, 0.0]), 0.0);
    assert_eq!(cosine_similarity(&[], &[]), 0.0);
    assert!(cosine_similarity(&[f64::NAN, 1.0], &[1.0, 1.0]).is_nan());
    assert_eq!(
        lexical_similarity("Dr Pepper 好き", "dr pepperが好きなユーザー"),
        1.0
    );
    assert_eq!(
        lexical_similarity("紅莉栖の誕生日", "ユーザーは紅莉栖の誕生日を知っている"),
        1.0
    );
    assert_eq!(lexical_similarity("   ", "x"), 0.0);
    assert_eq!(lexical_similarity("a b c", "abc"), 1.0);
    assert_eq!(lexical_similarity("Cat 好き", "猫が好き"), 0.5);
    assert_eq!(lexical_similarity("\u{FEFF}a\u{3000}z", "A"), 0.5);
    assert_eq!(js_round(0.704_999_9 * 100.0), 70.0);
    assert_eq!(js_round(2.5), 3.0);
    assert_eq!(js_round(-2.5), -2.0);
    assert_eq!(js_round(0.499_999_999_999_999_94), 0.0);
}

/// Golden: the V1 add/search/remove scenario (store JSON byte-identical, same results).
#[test]
fn scenario_matches_v1() {
    let store = settings_store();
    let settings = AppSettings::new(&store);
    let db = VectorDatabase::new(&store, &settings);
    assert!(db.enabled() && db.needs_initial_health_check());
    assert_eq!((db.top_k(), db.threshold()), (2, 0.5));
    let errors = Rc::new(RefCell::new(Vec::<String>::new()));
    let sink = Rc::clone(&errors);
    db.search_error()
        .connect(move |m: &str| sink.borrow_mut().push(m.to_owned()));
    let results = Rc::new(RefCell::new(Vec::<Vec<VectorSearchResult>>::new()));
    let sink = Rc::clone(&results);
    db.search_complete()
        .connect(move |r: &[VectorSearchResult]| sink.borrow_mut().push(r.to_vec()));
    let added = Rc::new(RefCell::new(Vec::<String>::new()));
    let sink = Rc::clone(&added);
    db.entry_added()
        .connect(move |id: &str| sink.borrow_mut().push(id.to_owned()));

    let mut next_id = 0;
    let mut add = |content: &str, category: &str, metadata: serde_json::Value| {
        if db.begin_add_entry(content) {
            let embedding = embed(content);
            // V1 draws a UUID only after a successful embedding.
            let new_id = if embedding.is_empty() {
                String::new()
            } else {
                next_id += 1;
                id(next_id)
            };
            let metadata = match metadata {
                serde_json::Value::Object(map) => map,
                _ => Map::new(),
            };
            db.finish_add_entry(content, category, metadata, embedding, new_id);
        }
    };
    add("猫が好き", DEFAULT_CATEGORY, json!({}));
    add("犬が好き", "pets", json!({"n": 1.5, "source": "test"}));
    add("東京在住", "place", json!({}));
    add("unknown text", DEFAULT_CATEGORY, json!({}));
    add("   ", DEFAULT_CATEGORY, json!({}));
    add("ints", DEFAULT_CATEGORY, json!({"a": [1, 2]}));
    add("tiny", DEFAULT_CATEGORY, json!({}));
    assert_eq!(store.get(STORAGE_KEY).as_deref(), Some(V1_STORE));
    assert_eq!(*errors.borrow(), [EMBEDDING_FAILED_MESSAGE]);
    assert_eq!(added.borrow().len(), 5);
    assert_eq!(db.entry_count(), 5);

    for query in ["query-cat", "Cat 好き", "   "] {
        if db.begin_search(query) {
            db.finish_search(query, &embed(query));
        }
    }
    let results = results.borrow();
    let summary: Vec<Vec<(String, f64)>> = results
        .iter()
        .map(|r| r.iter().map(|x| (x.id.clone(), x.similarity)).collect())
        .collect();
    assert_eq!(
        summary,
        [
            vec![
                (id(1), 0.998_752_338_877_844_6),
                (id(2), 0.998_158_391_825_579_2)
            ],
            vec![(id(1), 0.5), (id(2), 0.5)],
            vec![]
        ]
    );
    assert_eq!(results[0][1].category, "pets");
    assert_eq!(results[0][1].metadata.get("source"), Some(&json!("test")));
    assert_eq!(
        context_for_prompt(&results[0]),
        "【関連する過去の記憶】\n- 猫が好き (類似度: 100%)\n- 犬が好き (類似度: 100%)"
    );
    assert!(db.remove_entry(&id(2)));
    assert!(!db.remove_entry(&id(2)));
    assert_eq!(store.get(STORAGE_KEY).as_deref(), Some(V1_REMOVED));

    // A reload of the V1 file gives the same entries.
    let again = VectorDatabase::new(&store, &settings);
    assert_eq!(again.entries(), db.entries());
    assert_eq!(again.to_json(), V1_REMOVED);

    // NaN embedding items are written as null.
    db.finish_add_entry("bad", DEFAULT_CATEGORY, Map::new(), embed("bad"), id(6));
    assert!(db
        .to_json()
        .ends_with(r#""embedding":[null,0,2],"metadata":{}}]}"#));
    db.clear_all();
    assert_eq!(
        store.get(STORAGE_KEY).as_deref(),
        Some(r#"{"version":1,"entries":[]}"#)
    );
}

/// Golden: V1 `getContextForPrompt` rounding (`Math.round`, `-0` prints as `0`).
#[test]
fn context_rounding_matches_v1() {
    let result = |content: &str, similarity: f64| VectorSearchResult {
        id: "1".into(),
        content: content.into(),
        category: "c".into(),
        similarity,
        metadata: Map::new(),
    };
    assert_eq!(context_for_prompt(&[]), "");
    assert_eq!(
        context_for_prompt(&[
            result("a", 0.705),
            result("b", -0.005),
            result("c", 0.704_999_9),
            result("d", 1.000_000_000_000_000_2),
        ]),
        "【関連する過去の記憶】\n- a (類似度: 71%)\n- b (類似度: 0%)\n- c (類似度: 70%)\n- d (類似度: 100%)"
    );
}

#[test]
fn search_order_threshold_and_top_k() {
    let entry = |n: u32, embedding: Vec<f64>| VectorEntry {
        id: id(n),
        content: format!("entry {n}"),
        category: DEFAULT_CATEGORY.into(),
        embedding,
        metadata: Map::new(),
    };
    let entries = vec![
        entry(1, vec![0.0, 1.0]),
        entry(2, vec![1.0, 1.0]),
        entry(3, vec![1.0, 0.0]),
        entry(4, vec![2.0, 2.0]),
        entry(5, vec![1.0, 0.0, 0.0]),
    ];
    let ids = |results: Vec<VectorSearchResult>| -> Vec<String> {
        results.into_iter().map(|r| r.id).collect()
    };
    let query = [1.0, 0.0];
    // Descending, stable among equal similarities (2 and 4 tie), threshold inclusive.
    assert_eq!(
        ids(search_entries(&entries, "q", &query, 0.0, 10)),
        [id(3), id(2), id(4), id(1), id(5)]
    );
    assert_eq!(
        ids(search_entries(
            &entries,
            "q",
            &query,
            0.707_106_781_186_547_5,
            10
        )),
        [id(3), id(2), id(4)]
    );
    assert_eq!(
        ids(search_entries(&entries, "q", &query, 0.0, 2)),
        [id(3), id(2)]
    );
    // JS `slice(0, k)`: 0 → none, negative → drop from the end.
    assert!(search_entries(&entries, "q", &query, 0.0, 0).is_empty());
    assert_eq!(
        ids(search_entries(&entries, "q", &query, 0.0, -1)),
        [id(3), id(2), id(4), id(1)]
    );
    assert!(search_entries(&entries, "q", &query, 0.0, -9).is_empty());
    // Empty query embedding → lexical fallback over the content.
    assert_eq!(
        ids(search_entries(&entries, "ENTRY 4", &[], 0.75, 5)),
        [id(4)]
    );
}

#[test]
fn settings_sync_and_setters() {
    let store = MemoryStore::new();
    let settings = AppSettings::new(&store);
    let db = VectorDatabase::new(&store, &settings);
    assert_eq!(db.config(), RagConfig::default());
    assert!(!db.needs_initial_health_check());
    let log = Rc::new(RefCell::new(Vec::<&str>::new()));
    for (name, emitter) in [
        ("enabled", db.enabled_changed()),
        ("url", db.server_url_changed()),
        ("topK", db.top_k_changed()),
        ("threshold", db.threshold_changed()),
        ("available", db.server_available_changed()),
    ] {
        let sink = Rc::clone(&log);
        emitter.connect(move |_| sink.borrow_mut().push(name));
    }
    // Disabled: add/search are no-ops; search reports an empty result.
    let completed = Rc::new(RefCell::new(0));
    let sink = Rc::clone(&completed);
    db.search_complete()
        .connect(move |r: &[VectorSearchResult]| {
            assert!(r.is_empty());
            *sink.borrow_mut() += 1;
        });
    assert!(!db.begin_add_entry("x"));
    assert!(!db.begin_search("x"));
    assert_eq!(*completed.borrow(), 1);

    assert!(db.set_enabled(true, &settings));
    assert!(!db.set_enabled(true, &settings));
    assert_eq!(
        store.get("Config_Experimental_RAG_Enabled").as_deref(),
        Some("1")
    );
    assert!(db.set_server_url("http://h:1", &settings));
    assert!(!db.set_server_url("http://h:1", &settings));
    assert!(db.set_top_k(25.7, &settings));
    assert_eq!(db.top_k(), 20);
    assert!(!db.set_top_k(20.0, &settings));
    assert!(!db.set_top_k(f64::NAN, &settings));
    assert!(db.set_top_k(-3.0, &settings));
    assert_eq!(
        store.get("Config_Experimental_RAG_TopK").as_deref(),
        Some("1")
    );
    assert!(db.set_threshold(1.5, &settings));
    assert_eq!(db.threshold(), 1.0);
    assert!(!db.set_threshold(1.0 - 1e-17, &settings));
    assert!(db.set_threshold(0.1 + 0.2, &settings));
    assert_eq!(
        store.get("Config_Experimental_RAG_Threshold").as_deref(),
        Some("0.30000000000000004")
    );
    assert!(!db.set_threshold(f64::NAN, &settings));
    db.set_server_available(true);
    db.set_server_available(true);
    db.set_server_available(false);
    assert_eq!(
        *log.borrow(),
        [
            "enabled",
            "url",
            "topK",
            "topK",
            "threshold",
            "threshold",
            "available",
            "available"
        ]
    );

    // V1 dialog path: settings are written first and re-synced through the subscription, so a
    // setter afterwards sees "no change" for in-range values (no signal, no clamp).
    log.borrow_mut().clear();
    settings.set_int("Config_Experimental_RAG_TopK", 5);
    db.handle_setting_changed("Config_Experimental_RAG_TopK", &settings);
    assert_eq!(db.top_k(), 5);
    assert!(!db.set_top_k(5.0, &settings));
    settings.set_int("Config_Experimental_RAG_TopK", -3);
    db.handle_setting_changed("Config_Experimental_RAG_TopK", &settings);
    assert_eq!(db.top_k(), -3, "unclamped when synced");
    assert!(
        db.set_top_k(-3.0, &settings),
        "the setter clamps out-of-range values"
    );
    assert_eq!(db.top_k(), 1);
    assert_eq!(*log.borrow(), ["topK"]);
    store.set("Config_Experimental_RAG_ServerUrl", "http://other");
    db.handle_setting_changed("Config_Language", &settings);
    assert_eq!(db.server_url(), "http://h:1", "other keys do not re-sync");
    db.handle_setting_changed("Config_Experimental_RAG_ServerUrl", &settings);
    assert_eq!(db.server_url(), "http://other");
    assert!(!db.set_server_url("http://other", &settings));
}

#[test]
fn load_is_lenient_like_v1() {
    let cases: [(&str, usize); 10] = [
        (
            r#"{"entries":[{"id":"a","embedding":[1]},{"id":"","embedding":[1]},{"embedding":[1]},{"id":"b","embedding":[]},{"id":"c"},5,"s",[1]]}"#,
            1,
        ),
        (r#"{"entries":[{"id":"a","embedding":[1]},null]}"#, 0),
        (r#"{"entries":{"id":"a"}}"#, 0),
        (r#"{"entries":"abc"}"#, 0),
        (r#"{"entries":null}"#, 0),
        (r#"{"version":1}"#, 0),
        ("null", 0),
        ("[1]", 0),
        ("{", 0),
        // Deviation: V1's `Map` kept the number id 7 and the string id "7" apart.
        (
            r#"{"entries":[{"id":7,"embedding":["1.5",null,true,"x",[2],"0x10"," 3 ","1e3","."],"content":5,"category":null,"metadata":[1]},{"id":"7","embedding":[2]}]}"#,
            1,
        ),
    ];
    for (raw, count) in cases {
        let store = MemoryStore::new();
        let settings = AppSettings::new(&store);
        let db = VectorDatabase::new(&store, &settings);
        db.finish_add_entry("seed", "general", Map::new(), vec![1.0], "seed".into());
        store.set(STORAGE_KEY, raw);
        db.load();
        assert_eq!(db.entry_count(), count, "{raw}");
    }
    // Coercions and Map semantics (same id keeps its position, last value wins).
    let store = MemoryStore::new();
    store.set(
        STORAGE_KEY,
        r#"{"entries":[{"id":7,"embedding":["1.5",null,true,"x",[2],"0x10"," 3 ","1e3","."],"content":5,"category":null,"metadata":[1]},{"id":"8","embedding":[1]},{"id":"7","embedding":[2],"content":"last"}]}"#,
    );
    let settings = AppSettings::new(&store);
    let db = VectorDatabase::new(&store, &settings);
    let entries = db.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(
        (entries[0].id.as_str(), entries[0].content.as_str()),
        ("7", "last")
    );
    assert_eq!(entries[1].id, "8");
    // An empty value keeps the current entries.
    store.set(STORAGE_KEY, "");
    db.load();
    assert_eq!(db.entry_count(), 2);
    let parsed = parse_store(
        r#"{"entries":[{"id":7,"embedding":["1.5",null,true,"x",[2],"0x10"," 3 ","1e3","."],"content":5,"category":null,"metadata":[1]}]}"#,
    )
    .unwrap_or_default();
    let e = &parsed[0];
    assert_eq!(
        (e.id.as_str(), e.content.as_str(), e.category.as_str()),
        ("7", "5", "")
    );
    assert!(e.metadata.is_empty());
    let numbers: Vec<String> = e.embedding.iter().map(|n| format!("{n}")).collect();
    assert_eq!(
        numbers,
        ["1.5", "0", "1", "NaN", "2", "16", "3", "1000", "NaN"]
    );
}

#[test]
fn helpers() {
    assert_eq!(clamp_top_k(0.0), Some(1));
    assert_eq!(clamp_top_k(7.9), Some(7));
    assert_eq!(clamp_top_k(f64::INFINITY), Some(20));
    assert_eq!(clamp_top_k(f64::NEG_INFINITY), Some(1));
    assert_eq!(clamp_top_k(f64::NAN), None);
    assert_eq!(clamp_threshold(-1.0), Some(0.0));
    assert_eq!(clamp_threshold(0.25), Some(0.25));
    assert_eq!(clamp_threshold(f64::NAN), None);
    assert_eq!(
        random_uuid_v4(&mut SequenceRng::constant(0.0)),
        "00000000-0000-4000-8000-000000000000"
    );
    let uuid = random_uuid_v4(&mut SequenceRng::constant(0.999_999_999_999));
    assert_eq!(uuid, "ffffffff-ffff-4fff-bfff-ffffffffffff");
    assert_eq!(
        store_json(&[VectorEntry {
            id: "x".into(),
            content: "c".into(),
            category: "g".into(),
            embedding: vec![1.0, 1e21, -0.0, f64::INFINITY],
            metadata: Map::new(),
        }]),
        r#"{"version":1,"entries":[{"id":"x","content":"c","category":"g","embedding":[1,1e+21,0,null],"metadata":{}}]}"#
    );
    for (input, expected) in [
        ("", 0.0),
        ("  12  ", 12.0),
        ("-1.5e2", -150.0),
        (".5", 0.5),
        ("5.", 5.0),
        ("+Infinity", f64::INFINITY),
        ("0b101", 5.0),
        ("0o17", 15.0),
    ] {
        assert_eq!(js_string_to_number(input), expected, "{input:?}");
    }
    for input in ["1e", "e5", "-0x10", "0x", "1_000", "١", "infinity", "12px"] {
        assert!(js_string_to_number(input).is_nan(), "{input:?}");
    }
}
