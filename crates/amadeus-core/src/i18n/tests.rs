//! System A tests: data parity with V1 and exact `t()` / `tr()` / `fill()` semantics.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::dict::{parse, APP_LOCALE_JSON};
use super::*;

/// Key counts of V2 `crates/amadeus-core/locales/app/*.json` (V1 + `changelog_v20`), in index order.
const EXPECTED_KEY_COUNTS: [usize; LANG_COUNT] = [
    538, 538, 533, 534, 534, 533, 533, 533, 534, 534, 534, 534, 534,
];

const FREEAPI_KEYS: [&str; 4] = [
    "freeapi_quota_exhausted",
    "freeapi_quota_label",
    "freeapi_quota_remaining",
    "freeapi_quota_usage",
];

/// Out-of-range indices that must behave like index 0 in `t()`.
const OUT_OF_RANGE: [i64; 5] = [-1, 13, 100, i64::MAX, i64::MIN];

/// Independent oracle: the raw file parsed as a generic JSON object.
fn oracle(lang: Lang) -> Map<String, Value> {
    match serde_json::from_str::<Value>(APP_LOCALE_JSON[lang as usize]) {
        Ok(Value::Object(map)) => map,
        other => panic!("{}.json is not a JSON object: {other:?}", lang.code()),
    }
}

/// Keys V1 itself lacks relative to `ja.json` (spec §2.14), sorted.
fn known_v1_gaps(lang: Lang) -> Vec<&'static str> {
    let mut gaps = match lang {
        Lang::Ja | Lang::En => Vec::new(),
        _ => FREEAPI_KEYS.to_vec(),
    };
    if matches!(lang, Lang::Zh | Lang::Fr | Lang::De | Lang::Ru) {
        gaps.push("sleeping_placeholder");
    }
    gaps.sort_unstable();
    gaps
}

fn all_indices() -> impl Iterator<Item = i64> {
    (0..LANG_COUNT as i64).chain(OUT_OF_RANGE)
}

#[test]
fn every_app_locale_parses_with_v1_key_counts() {
    for lang in Lang::ALL {
        let parsed = parse(APP_LOCALE_JSON[lang as usize]);
        assert!(parsed.is_ok(), "{}.json: {parsed:?}", lang.code());
        let expected = EXPECTED_KEY_COUNTS[lang as usize];
        assert_eq!(key_count(lang), expected, "{}.json key count", lang.code());
        assert_eq!(
            oracle(lang).len(),
            expected,
            "{}.json oracle count",
            lang.code()
        );
    }
}

#[test]
fn key_parity_against_fallback_language_reports_v1_gaps() {
    // `ja.json` is the reference set (the dictionary used for out-of-range indices). V1 data
    // is itself inconsistent (FreeAPI keys exist only in ja/en, `sleeping_placeholder` is
    // missing in zh/fr/de/ru). Those gaps are reported, not treated as failures; the test
    // only fails if the data drifts from V1 (which would change visible text).
    let mut report = String::new();
    for lang in Lang::ALL {
        let missing = missing_keys(lang);
        let extra = extra_keys(lang);
        if !missing.is_empty() || !extra.is_empty() {
            report.push_str(&format!(
                "{} ({}): missing vs ja = {missing:?}, extra = {extra:?}\n",
                lang.code(),
                key_count(lang)
            ));
        }
        assert!(
            extra.is_empty(),
            "{}.json has keys ja.json lacks: {extra:?}",
            lang.code()
        );
        assert_eq!(
            missing,
            known_v1_gaps(lang),
            "{}.json drifted from V1",
            lang.code()
        );
    }
    println!("System A key parity report (V1 data, reference ja.json):\n{report}");
}

#[test]
fn t_matches_oracle_for_every_language_and_key() {
    let reference = oracle(Lang::Ja);
    for lang in Lang::ALL {
        let data = oracle(lang);
        for key in reference.keys() {
            match data.get(key) {
                Some(Value::String(value)) => {
                    assert_eq!(t(key, "D", lang), value.as_str(), "{}:{key}", lang.code());
                    assert_eq!(t(key, "", lang), value.as_str(), "{}:{key}", lang.code());
                    assert_eq!(lookup(key, lang), Some(value.as_str()));
                    assert!(has_key(key, lang));
                }
                Some(other) => panic!("{}:{key} is not a string: {other:?}", lang.code()),
                None => {
                    assert_eq!(t(key, "D", lang), "D", "{}:{key}", lang.code());
                    assert_eq!(t(key, "", lang), key.as_str(), "{}:{key}", lang.code());
                    assert_eq!(lookup(key, lang), None);
                    assert!(!has_key(key, lang));
                }
            }
        }
    }
}

#[test]
fn out_of_range_index_uses_japanese_dictionary() {
    let reference = oracle(Lang::Ja);
    for index in OUT_OF_RANGE {
        assert_eq!(Lang::from_index_or_ja(index), Lang::Ja);
        for (key, value) in &reference {
            assert_eq!(Some(t(key, "D", index)), value.as_str(), "{index}:{key}");
        }
        assert_eq!(t("no_such_key", "D", index), "D");
        assert_eq!(t("no_such_key", "", index), "no_such_key");
    }
    // Works with any integer type that converts into i64, and with `Lang`.
    assert_eq!(t("close", "x", 1i32), "Close");
    assert_eq!(t("close", "x", 2u8), "关闭");
    assert_eq!(t("close", "x", Lang::Ar), "إغلاق");
    assert_eq!(t("close", "x", -5i32), "閉じる");
}

#[test]
fn untranslated_keys_are_absent_everywhere() {
    for key in UNTRANSLATED_KEYS {
        for index in all_indices() {
            assert!(
                !has_key(key, index),
                "{key} unexpectedly translated for {index}"
            );
            assert_eq!(t(key, "", index), key);
        }
    }
}

#[test]
fn placeholder_sets_are_identical_across_languages() {
    fn placeholders(value: &str) -> BTreeSet<String> {
        let bytes = value.as_bytes();
        bytes
            .windows(2)
            .filter(|w| w[0] == b'%' && w[1].is_ascii_digit())
            .map(|w| String::from_utf8_lossy(w).into_owned())
            .collect()
    }
    let reference = oracle(Lang::Ja);
    let mut with_placeholders = BTreeSet::new();
    for key in reference.keys() {
        let mut sets = Lang::ALL
            .iter()
            .filter_map(|lang| lookup(key, *lang).map(placeholders));
        let Some(first) = sets.next() else { continue };
        if !first.is_empty() {
            with_placeholders.insert(key.as_str());
        }
        for other in sets {
            assert_eq!(first, other, "placeholder mismatch for {key}");
        }
    }
    let expected: BTreeSet<&str> = [
        "freeapi_quota_remaining",
        "freeapi_quota_usage",
        "mg_menu_desc",
        "mg_score_result",
        "mg_wrong",
        "update_confirm",
    ]
    .into_iter()
    .collect();
    assert_eq!(with_placeholders, expected);
}

/// Per-language golden values copied from the V1 files (spec §2.16 and the app specs).
#[test]
fn golden_lookups_for_all_languages() {
    let goldens: [(&str, [&str; LANG_COUNT]); 7] = [
        (
            "cancel_hint",
            [
                "CTRL+Cでキャンセル",
                "CTRL+C to cancel",
                "按 CTRL+C 取消",
                "CTRL+C로 취소",
                "CTRL+C para cancelar",
                "CTRL+C pour annuler",
                "CTRL+C zum Abbrechen",
                "CTRL+C для отмены",
                "CTRL+C для скасування",
                "CTRL+C para cancelar",
                "İptal etmek için CTRL+C",
                "CTRL+C לביטול",
                "CTRL+C للإلغاء",
            ],
        ),
        (
            "amadeus_kurisu",
            [
                "アマデウス紅莉栖",
                "Amadeus Kurisu",
                "阿玛迪斯·红莉栖",
                "아마데우스 쿠리스",
                "Amadeus Kurisu",
                "Amadeus Kurisu",
                "Amadeus Kurisu",
                "Амадеус Курису",
                "Амадеус Курісу",
                "Amadeus Kurisu",
                "Amadeus Kurisu",
                "אמדאוס קוריסו",
                "أماديوس كوريسو",
            ],
        ),
        (
            "mg_menu_desc",
            [
                "シュタインズ・ゲートから%1問出題！",
                "%1 questions from Steins;Gate!",
                "命运石之门系列共%1道题！",
                "슈타인즈 게이트에 관한 퀴즈 %1문항!",
                "¡%1 preguntas de Steins;Gate!",
                "%1 questions tirées de Steins;Gate !",
                "%1 Fragen zu Steins;Gate!",
                "%1 вопросов по Steins;Gate!",
                "%1 питань по Steins;Gate!",
                "%1 perguntas de Steins;Gate!",
                "Steins;Gate serisinden %1 soru!",
                "%1 שאלות בנושא שטיינס;גייט!",
                "%1 أسئلة من شتاينز;غيت!",
            ],
        ),
        (
            "mg_score_result",
            [
                "%1 / %2 正解",
                "%1 / %2 correct",
                "%1 / %2 正确",
                "%1 / %2 정답",
                "%1 / %2 correctas",
                "%1 / %2 correctes",
                "%1 / %2 richtig",
                "%1 / %2 верно",
                "%1 / %2 правильно",
                "%1 / %2 corretas",
                "%1 / %2 doğru",
                "%1 / %2 נכונות",
                "%1 / %2 صحيحة",
            ],
        ),
        (
            "mg_wrong",
            [
                "はぁ……ハズレよ、ハズレ。脳のシナプスが死滅してんじゃない？正解は「%1」よ。",
                "Sigh... Wrong. The answer was '%1'.",
                "唉……答错了。正确答案是'%1'。",
                "하아... 틀렸어. 정답은 '%1'였어.",
                "Ay... Incorrecto. La respuesta era '%1'.",
                "Pff... Faux. La réponse était '%1'.",
                "Seufz... Falsch. Die Antwort war '%1'.",
                "Вздох... Неверно. Правильный ответ был '%1'.",
                "Зітхання... Неправильно. Правильна відповідь була '%1'.",
                "Aff... Errado. A resposta era '%1'.",
                "Of... Yanlış. Cevap '%1' idi.",
                "אנחה... לא נכון. התשובה הייתה '%1'.",
                "تنهيدة... خطأ. كانت الإجابة '%1'.",
            ],
        ),
        (
            "update_confirm",
            [
                "最新バージョン %1 が利用可能です。\n更新しますか？",
                "New version %1 is available.\nDo you want to update?",
                "检测到新版本 %1。\n是否进行更新？",
                "새로운 버전 %1을 사용할 수 있습니다.\n업데이트하시겠습니까?",
                "Nueva versión %1 disponible.\n¿Desea actualizar?",
                "La nouvelle version %1 est disponible.\nVoulez-vous mettre à jour ?",
                "Neue Version %1 ist verfügbar.\nMöchten Sie aktualisieren?",
                "Доступна новая версия %1.\nХотите обновить?",
                "Доступна нова версія %1.\nБажаєте оновити?",
                "Nova versão %1 disponível.\nDeseja atualizar?",
                "Yeni sürüm %1 mevcut.\nGüncellemek istiyor musunuz?",
                "גרסה חדשה %1 זמינה.\nהאם ברצונך לעדכן?",
                "الإصدار الجديد %1 متاح.\nهل تريد التحديث؟",
            ],
        ),
        (
            // Leading ASCII space is meaningful (concatenated after the imported count).
            "import_turns",
            [
                " ターン",
                " turns",
                " 轮对话",
                " 턴",
                " turnos",
                " tours",
                " Runden",
                " ходов",
                " ходів",
                " turnos",
                " tur",
                " תורות",
                " أدوار",
            ],
        ),
    ];
    for (key, values) in goldens {
        for (lang, expected) in Lang::ALL.into_iter().zip(values) {
            assert_eq!(t(key, "", lang), expected, "{}:{key}", lang.code());
        }
    }
    // Trailing spaces preserved, Japanese punctuation preserved, 65,000 working-tree value.
    assert_eq!(
        t("export_success", "エクスポート完了: ", 0),
        "エクスポート完了: "
    );
    assert_eq!(t("export_success", "", 5), "Exportation terminée : ");
    assert_eq!(
        t("freeapi_quota_exhausted", "", 0),
        "全体リクエスト上限（65,000回）に達しました。サービスを停止しています。"
    );
    assert_eq!(
        t("freeapi_quota_exhausted", "", 1),
        "Global request limit (65,000) reached. Service suspended."
    );
    // Two U+200B ZERO WIDTH SPACE characters in he.json are kept byte-for-byte.
    assert!(t("changelog_v101", "", Lang::He).contains("בעת \u{200B}\u{200B}מזעור"));
}

#[test]
fn json_defaults_win_over_code_defaults_when_the_key_exists() {
    assert_eq!(
        t(
            "login_access_denied",
            ja_en(0, "アクセスが拒否されました", "ACCESS DENIED"),
            0
        ),
        "ACCESS DENIED"
    );
    assert_eq!(
        t(
            "help_toggle_menu",
            ja_en(1, "メニュー開閉", "Open/Close Menu"),
            1
        ),
        "Toggle Menu"
    );
    assert_eq!(
        tr(
            "※ Vertexを使用するためには、gCloud CLIのインストールが必要です。",
            "* gCloud CLI must be installed to use Vertex.",
            Some("setting_vertex_note"),
            1
        ),
        "* gCloud CLI is required to use Vertex."
    );
}

/// Effective display matrix of spec §2.7 for keys whose call-site default can surface.
#[test]
fn display_matrix_for_keys_with_surfacing_defaults() {
    // (key, ja default, en default) — keys absent from every file, ternary defaults.
    let ternary: [(&str, &str, &str); 12] = [
        ("advance_dialogue", "会話を進める", "Advance dialogue"),
        ("backlog", "ログ", "LOG"),
        ("booting", "起動中", "Booting"),
        (
            "help_right_click_title",
            "Tab / 右クリック",
            "Tab / Right Click",
        ),
        ("help_wasd", "WASD / ↑←↓→", "WASD / Arrows"),
        (
            "login_empty",
            "ユーザーIDとパスワードを入力してください。",
            "Please enter both fields.",
        ),
        ("send", "送信", "SEND"),
        ("setting_title", "設定", "SETTINGS"),
        ("setting_close", "閉じる", "Close"),
        ("setting_apply", "適用", "Apply"),
        ("settings", "設定", "SETTINGS"),
        ("speech_input", "音声入力", "Speech input"),
    ];
    for index in all_indices() {
        for (key, ja, en) in ternary {
            let expected = if index == 1 { en } else { ja };
            assert_eq!(
                t(key, ja_en(index, ja, en), index),
                expected,
                "{index}:{key}"
            );
            assert_eq!(tr(ja, en, Some(key), index), expected, "{index}:{key}");
        }
        // Japanese-only default: Japanese even for English.
        assert_eq!(t("cancel_button", "キャンセル", index), "キャンセル");

        let label = tr(
            "FreeAPIクォータ",
            "FreeAPI Quota",
            Some("freeapi_quota_label"),
            index,
        );
        let usage = tr(
            "%1 / %2 回使用",
            "%1 / %2 requests used",
            Some("freeapi_quota_usage"),
            index,
        );
        let remaining = tr(
            "残り %1 回",
            "%1 remaining",
            Some("freeapi_quota_remaining"),
            index,
        );
        if index == 1 {
            assert_eq!(label, "FreeAPI Quota");
            assert_eq!(usage, "%1 / %2 requests used");
            assert_eq!(remaining, "%1 remaining");
        } else {
            // ja JSON (0 and out-of-range) or the Japanese default (2..=12): same text.
            assert_eq!(label, "FreeAPIクォータ", "{index}");
            assert_eq!(usage, "%1 / %2 回使用", "{index}");
            assert_eq!(remaining, "残り %1 回", "{index}");
        }
        // Present in ja/en only; out-of-range indices use ja.json.
        assert_eq!(
            has_key("freeapi_quota_label", index),
            !(2..=12).contains(&index)
        );

        // `tr` without a key: Japanese only for index 0, English for everything else.
        let vertex = tr(
            "Vertex APIキー / SA JSON",
            "Vertex API Key / SA JSON",
            None,
            index,
        );
        let empty_key = tr(
            "Vertex APIキー / SA JSON",
            "Vertex API Key / SA JSON",
            Some(""),
            index,
        );
        let expected = if index == 0 {
            "Vertex APIキー / SA JSON"
        } else {
            "Vertex API Key / SA JSON"
        };
        assert_eq!(vertex, expected, "{index}");
        assert_eq!(empty_key, expected, "{index}");
    }

    let sleeping_default = "Zzz... (キー入力やマウス移動で起こす)";
    for lang in Lang::ALL {
        let shown = t("sleeping_placeholder", sleeping_default, lang);
        if matches!(lang, Lang::Zh | Lang::Fr | Lang::De | Lang::Ru) {
            assert_eq!(shown, sleeping_default, "{}", lang.code());
        } else {
            assert_ne!(
                lookup("sleeping_placeholder", lang),
                None,
                "{}",
                lang.code()
            );
        }
    }
    assert_eq!(
        t("sleeping_placeholder", sleeping_default, 1),
        "Zzz... (Press any key or move mouse to wake up)"
    );
}

#[test]
fn fill_reproduces_chained_js_replace() {
    assert_eq!(fill("%1 / %2 正解", &["3", "10"]), "3 / 10 正解");
    assert_eq!(
        fill(t("mg_score_result", "%1 / %2 正解", Lang::He), &["7", "10"]),
        "7 / 10 נכונות"
    );
    assert_eq!(
        fill(
            t("mg_menu_desc", "シュタインズ・ゲートから%1問出題！", 0),
            &["10"]
        ),
        "シュタインズ・ゲートから10問出題！"
    );
    // Literal `%` in the replacement is kept.
    assert_eq!(
        fill(
            t("mg_wrong", "はぁ…不正解よ。答えは「%1」だったわ。", 0),
            &["1.048596%"]
        ),
        "はぁ……ハズレよ、ハズレ。脳のシナプスが死滅してんじゃない？正解は「1.048596%」よ。"
    );
    // First occurrence only, applied sequentially on the intermediate result.
    assert_eq!(fill("%1 and %1", &["a"]), "a and %1");
    assert_eq!(fill("%1 %2", &["%2", "x"]), "x %2");
    assert_eq!(fill("%2 %1", &["a", "b"]), "b a");
    assert_eq!(fill("no placeholders", &["x"]), "no placeholders");
    assert_eq!(fill("%1", &[]), "%1");
    // JS replacement patterns.
    assert_eq!(fill("[%1]", &["$&"]), "[%1]");
    assert_eq!(fill("a%1b", &["$`"]), "aab");
    assert_eq!(fill("a%1b", &["$'"]), "abb");
    assert_eq!(fill("a%1b", &["$$"]), "a$b");
    assert_eq!(fill("a%1b", &["$1"]), "a$1b");
    assert_eq!(fill("a%1b", &["$<x>"]), "a$<x>b");
    assert_eq!(fill("a%1b", &["x$"]), "ax$b");
    assert_eq!(fill("a%1b", &["$$$"]), "a$$b");
}

#[test]
fn replace_first_js_edge_cases() {
    assert_eq!(replace_first_js("abc", "", "x"), "xabc");
    assert_eq!(replace_first_js("abc", "", "$'"), "abcabc");
    assert_eq!(replace_first_js("abc", "z", "x"), "abc");
    assert_eq!(replace_first_js("あ%1い%1", "%1", "う"), "あうい%1");
    assert_eq!(replace_first_js("xay", "a", "[$`|$&|$']"), "x[x|a|y]y");
}

#[test]
fn lang_index_and_code_mapping() {
    for (i, lang) in Lang::ALL.into_iter().enumerate() {
        let index = i as i64;
        assert_eq!(Lang::from_index(index), Some(lang));
        assert_eq!(usize::from(lang.index()), i);
        assert_eq!(i64::from(lang), index);
        assert_eq!(lang.code(), LANGUAGE_CODES[i]);
        assert_eq!(Lang::from_code(lang.code()), Some(lang));
        assert_eq!(lang.native_name(), LANGUAGE_NAMES[i]);
        assert_eq!(lang.chat_name(), CHAT_LANGUAGE_NAMES[i]);
        assert_eq!(lang.is_rtl(), matches!(lang.code(), "he" | "ar"));
        assert_eq!(is_rtl_index(index), lang.is_rtl());
    }
    for index in OUT_OF_RANGE {
        assert_eq!(Lang::from_index(index), None);
        assert!(!is_rtl_index(index));
    }
    assert_eq!(Lang::from_code("JA"), None);
    assert_eq!(Lang::from_code("ja-JP"), None);
    assert_eq!(Lang::default(), Lang::Ja);
    assert_eq!(
        LANGUAGE_CODES.join(","),
        "ja,en,zh,ko,es,fr,de,ru,uk,pt,tr,he,ar"
    );
    assert_eq!(LANGUAGE_NAMES[11], "עברית");
    assert_eq!(LANGUAGE_NAMES[12], "العربية");
    assert_eq!(CONFIG_LANGUAGE_KEY, "Config_Language");
    assert_eq!(CONFIG_CHAT_LANGUAGE_KEY, "Config_ChatLanguage");
}

#[test]
fn chat_language_helpers() {
    assert_eq!(effective_chat_lang(0, 0), 0);
    assert_eq!(effective_chat_lang(5, 0), 5);
    assert_eq!(effective_chat_lang(5, 1), 0);
    assert_eq!(effective_chat_lang(0, 13), 12);
    assert_eq!(effective_chat_lang(0, 14), 13);
    assert_eq!(effective_chat_lang(3, -1), -2);
    assert_eq!(effective_chat_lang(0, i64::MIN), i64::MIN);
    assert_eq!(chat_language_name(0), "Japanese (日本語)");
    assert_eq!(chat_language_name(Lang::Ar), "Arabic (العربية)");
    // Out of range → English (unlike `t`, which falls back to Japanese).
    assert_eq!(chat_language_name(13), "English");
    assert_eq!(chat_language_name(-2), "English");
    assert_eq!(
        chat_language_name(effective_chat_lang(0, i64::MIN)),
        "English"
    );
}

#[test]
fn chat_language_options_follow_pending_language() {
    let ja = chat_language_options(0);
    assert_eq!(ja.len(), 14);
    assert_eq!(ja[0], "表示言語と同じ");
    assert_eq!(&ja[1..], &LANGUAGE_NAMES[..]);
    assert_eq!(chat_language_options(1)[0], "Same as Display Language");
    assert_eq!(chat_language_options(2)[0], "与显示语言相同");
    assert_eq!(chat_language_options(13)[0], "表示言語と同じ");
}

#[test]
fn embedded_files_have_v1_format() {
    for lang in Lang::ALL {
        let raw = APP_LOCALE_JSON[lang as usize];
        assert!(!raw.starts_with('\u{FEFF}'), "{}: BOM", lang.code());
        assert!(!raw.contains('\r'), "{}: CR", lang.code());
        assert!(raw.ends_with('\n'), "{}: final newline", lang.code());
        assert!(
            !raw.contains('$'),
            "{}: `$` would make fill() semantics matter",
            lang.code()
        );
    }
}

/// Parity with the V1 sources when the V1 tree is checked out next to this
/// workspace (`RealAmadeusRust/` inside the V1 repository): every V1 entry is identical, and
/// `changelog_v20` is the only V2-added key. Skipped otherwise.
#[test]
fn embedded_files_are_byte_identical_to_v1_sources() {
    let v1 = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../lib/amadeus/locales");
    if !v1.is_dir() {
        println!("V1 locales not found at {}; skipped", v1.display());
        return;
    }
    for lang in Lang::ALL {
        let path = v1.join(format!("{}.json", lang.code()));
        let v1_bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) => panic!("cannot read {}: {err}", path.display()),
        };
        let v1_map: Map<String, Value> = serde_json::from_slice(&v1_bytes)
            .unwrap_or_else(|e| panic!("cannot parse {}: {e}", path.display()));
        let v2_map = oracle(lang);
        for (k, v) in &v1_map {
            assert_eq!(
                v2_map.get(k),
                Some(v),
                "{}: key {k} differs from {}",
                lang.code(),
                path.display()
            );
        }
        let added: Vec<&str> = v2_map
            .keys()
            .filter(|k| !v1_map.contains_key(*k))
            .map(String::as_str)
            .collect();
        assert_eq!(
            added,
            ["changelog_v20"],
            "{}: unexpected V2 keys vs {}",
            lang.code(),
            path.display()
        );
    }
}
