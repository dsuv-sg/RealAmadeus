//! Unit tests of the quiz. Golden question sets come from V1 `generateQuestions()` under Node 22.

use super::*;
use crate::achievements::AchievementEvent;
use crate::i18n::{self, LANG_COUNT};
use crate::random_events::{Mulberry32, SequenceRng};

/// `(question key, answer suffixes in display order, correct)` of a generated game.
fn summary(questions: &[MiniGameQuestion]) -> Vec<(&'static str, String, usize)> {
    questions
        .iter()
        .map(|q| {
            let order: String =
                q.a.iter()
                    .map(|(key, _)| &key[key.rfind('_').map_or(0, |i| i + 1)..])
                    .collect();
            (q.q.0, order, q.correct)
        })
        .collect()
}

#[test]
fn pool_matches_v1() {
    assert_eq!(QUESTION_POOL.len(), 49);
    assert_eq!(TOTAL_QUESTIONS, 10);
    let mut keys = Vec::new();
    for entry in &QUESTION_POOL {
        assert_eq!(entry.correct, 0);
        let stem = entry.q.0.strip_suffix("_q").unwrap_or_default();
        assert!(stem.starts_with("mgq_"), "{stem}");
        for (k, answer) in entry.a.iter().enumerate() {
            assert_eq!(answer.0, format!("{stem}_a{k}"));
        }
        assert_eq!(entry.c.0, format!("{stem}_c"));
        keys.push(entry.q);
        keys.extend(entry.a);
        keys.push(entry.c);
    }
    assert_eq!(keys.len(), 294);
    for (key, _) in &keys {
        for lang in 0..LANG_COUNT as i64 {
            assert!(i18n::has_key(key, lang), "{key} missing in locale {lang}");
        }
    }
    // ja.json overrides 51 in-code defaults: every comment, one question and one answer.
    let overridden: Vec<&str> = keys
        .iter()
        .filter(|text| localize(**text, 0) != text.1)
        .map(|text| text.0)
        .collect();
    assert_eq!(overridden.len(), 51);
    assert_eq!(
        overridden
            .iter()
            .filter(|k| !k.ends_with("_c"))
            .collect::<Vec<_>>(),
        [&"mgq_bday_faris_q", &"mgq_fg9_a0"]
    );
    assert_eq!(
        QUESTION_POOL[18].q.1,
        "フェイリス・ニャンニアンの誕生日は？"
    );
    assert_eq!(
        localize(QUESTION_POOL[18].q, 0),
        "フェイリス・ニャンニャンの誕生日は？"
    );
}

/// Golden: spec Appendix G `generateQuestions` with mulberry32(42) (78 random numbers).
#[test]
fn generate_questions_mulberry42() {
    let mut rng = Mulberry32::new(42);
    let mut counted = SequenceRng::new((0..78).map(|_| rng.next_f64()).collect::<Vec<_>>())
        .with_fallback(f64::NAN);
    let questions = generate_questions(&mut counted);
    assert_eq!(counted.calls(), 78);
    let expected = [
        ("mgq_bday_ruka_q", "a3a2a0a1", 2),
        ("mgq_bday_maho_q", "a0a1a3a2", 0),
        ("mgq_reading_steiner_q", "a2a1a0a3", 2),
        ("mgq_mayuri_catchphrase_q", "a3a0a2a1", 1),
        ("mgq_birthstone_kurisu_q", "a1a2a3a0", 3),
        ("mgq_univ_q", "a0a2a3a1", 0),
        ("mgq_ibn5100_q", "a3a1a2a0", 3),
        ("mgq_height_kurisu_q", "a3a0a2a1", 1),
        ("mgq_fg1_q", "a1a0a3a2", 1),
        ("mgq_bday_mayuri_q", "a0a2a1a3", 0),
    ];
    let got = summary(&questions);
    for (g, e) in got.iter().zip(expected) {
        assert_eq!((g.0, g.1.as_str(), g.2), e);
    }
    assert_eq!(got.len(), 10);
    for q in &questions {
        assert!(
            q.a[q.correct].0.ends_with("_a0"),
            "correct points at the source a0"
        );
    }
}

/// Golden: V1 with mulberry32(7), `random() ≡ 0` and `random() ≈ 1`.
#[test]
fn generate_questions_other_sequences() {
    let questions = generate_questions(&mut Mulberry32::new(7));
    let expected = [
        ("mgq_reading_steiner_q", "a1a0a3a2", 1),
        ("mgq_dmail_q", "a3a2a0a1", 2),
        ("mgq_sg_div_q", "a1a2a0a3", 2),
        ("mgq_fg7_q", "a3a0a1a2", 1),
        ("mgq_labmem_q", "a0a1a3a2", 0),
        ("mgq_bday_okabe_q", "a0a2a1a3", 0),
        ("mgq_labmem_suzuha_q", "a3a2a1a0", 3),
        ("mgq_fg2_q", "a0a1a2a3", 0),
        ("mgq_bday_daru_q", "a0a1a3a2", 0),
        ("mgq_fg9_q", "a1a0a3a2", 1),
    ];
    let got = summary(&questions);
    for (g, e) in got.iter().zip(expected) {
        assert_eq!((g.0, g.1.as_str(), g.2), e);
    }

    let zero = generate_questions(&mut SequenceRng::constant(0.0));
    let stems: Vec<&str> = zero.iter().map(|q| q.q.0).collect();
    let pool_stems: Vec<&str> = QUESTION_POOL[1..=10].iter().map(|e| e.q.0).collect();
    assert_eq!(stems, pool_stems);
    assert_eq!(
        zero[0].a.map(|a| a.0),
        ["mgq_fg8_a1", "mgq_fg8_a2", "mgq_fg8_a3", "mgq_fg8_a0"]
    );
    assert!(zero.iter().all(|q| q.correct == 3));

    let high = generate_questions(&mut SequenceRng::constant(0.999_999_999_9));
    for (q, e) in high.iter().zip(&QUESTION_POOL) {
        assert_eq!((q.q, q.a, q.correct), (e.q, e.a, 0));
    }
    // Out-of-contract values never panic.
    for bad in [1.0, -3.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            generate_questions(&mut SequenceRng::constant(bad)).len(),
            10
        );
    }
}

/// Golden: spec Appendix G result tiers and Appendix F texts.
#[test]
fn result_tiers_match_v1() {
    let cases = [
        (10, Emotion::Smug, "mg_result_perfect"),
        (9, Emotion::Smile, "mg_result_good"),
        (6, Emotion::Smile, "mg_result_good"),
        (5, Emotion::Normal, "mg_result_ok"),
        (1, Emotion::Normal, "mg_result_ok"),
        (0, Emotion::Disgust, "mg_result_bad"),
    ];
    for (score, emotion, key) in cases {
        assert_eq!(
            (result_emotion(score), result_comment_key(score)),
            (emotion, key)
        );
    }
    assert_eq!(ResultTier::from_score(11), ResultTier::Good);
    assert_eq!(
        result_comment_default_for_key("mg_result_ok"),
        ResultTier::Ok.default_text()
    );
    assert_eq!(
        result_comment_default_for_key("nope"),
        "「はぁ…全滅？本当にシュタゲ好きなの？」"
    );
    assert_eq!(
        result_comment(10, 0),
        "「パーフェクト！……ま、まあ、私のことだし当然の結果よねっ！あんたにしてはよくやったわ、褒めてあげる！」"
    );
    assert_eq!(result_comment(7, 1), "Hmm, not bad. Not bad at all.");
    assert_eq!(
        result_comment(3, 1),
        "...Wait, is your memory deteriorating like Okabe's? You need to do some brain training!"
    );
    assert_eq!(
        result_comment(0, 0),
        "「はぁ……全滅？あんたの海馬、ちょっとスキャンしてバグがないか調べた方がいいわね……」"
    );
}

/// Golden: spec Appendix F UI strings.
#[test]
fn ui_strings_match_v1() {
    assert_eq!(menu_desc_text(0), "シュタインズ・ゲートから10問出題！");
    assert_eq!(menu_desc_text(1), "10 questions from Steins;Gate!");
    assert_eq!(score_result_text(3, 0), "3 / 10 正解");
    assert_eq!(score_result_text(10, 1), "10 / 10 correct");
    assert_eq!(score_label_text(4, 1), "Score: 4");
    assert_eq!(score_label_text(0, 0), "スコア: 0");
    assert_eq!(question_counter_text(0), "Q1 / 10");
    assert_eq!(question_counter_text(9), "Q10 / 10");
    assert_eq!(mg_t(MG_MENU_TITLE_KEY, 0), "紅莉栖とシュタゲクイズ");
    assert_eq!(mg_t(MG_MENU_TITLE_KEY, 1), "Steins;Gate Quiz with Kurisu");
    assert_eq!(mg_t(MG_RETRY_KEY, 1), "Try Again");
    assert_eq!(mg_t(MG_MENU_BACK_KEY, 0), "メニューへ");
    assert_eq!(mg_ui_default(MG_SCORE_RESULT_KEY), Some("%1 / %2 正解"));
    assert_eq!(mg_ui_default("unknown"), None);
    assert_eq!(
        wrong_answer_comment("Dr. Pepper", 0),
        "はぁ……ハズレよ、ハズレ。脳のシナプスが死滅してんじゃない？正解は「Dr. Pepper」よ。"
    );
    assert_eq!(
        wrong_answer_comment("Dr. Pepper", 1),
        "Sigh... Wrong. The answer was 'Dr. Pepper'."
    );
    assert_eq!(speak_duration_ms(""), 800);
    assert_eq!(speak_duration_ms("0123456789"), 900);
    assert_eq!(speak_duration_ms(&"x".repeat(40)), 3000);
    assert_eq!(speak_duration_ms("😀😀😀😀😀"), 900, "UTF-16 units");
}

#[test]
fn session_state_machine() {
    let mut session = MiniGameSession::new();
    assert_eq!(
        (session.state(), session.emotion(), session.on_show()),
        (GameState::Menu, Emotion::Smile, Emotion::Smile)
    );
    assert_eq!(session.select_answer(0), None, "no question yet");
    let event = session.start_game(&mut Mulberry32::new(42));
    assert_eq!(event, AchievementEvent::GamePlay);
    assert_eq!(
        (session.state(), session.emotion(), session.score()),
        (GameState::Playing, Emotion::Thinking, 0)
    );
    assert_eq!(session.counter_text(), "Q1 / 10");
    assert_eq!(session.next_button_label(1), "Next Question");
    let first = *session.active_question().unwrap_or(&session.questions()[0]);
    assert_eq!(first.q.0, "mgq_bday_ruka_q");
    assert_eq!(session.current_comment(0), "");
    assert_eq!(session.answer_class(0), "mg-answer");

    // Right answer.
    let outcome = session.select_answer(first.correct);
    assert_eq!(
        outcome,
        Some(AnswerOutcome {
            correct: true,
            selected: first.correct,
            correct_index: first.correct
        })
    );
    assert_eq!((session.score(), session.emotion()), (1, Emotion::Smile));
    assert_eq!(session.select_answer(0), None, "locked after the reveal");
    assert_eq!(session.current_comment(0), localize(first.c, 0));
    assert_eq!(
        session.answer_class(first.correct),
        "mg-answer revealed correct"
    );
    assert_eq!(
        session.answer_class((first.correct + 1) % 4),
        "mg-answer revealed"
    );

    // Wrong answer on the second question.
    assert!(!session.next_question().finished);
    assert_eq!(
        (session.emotion(), session.selected()),
        (Emotion::Thinking, None)
    );
    let second = session.questions()[1];
    let wrong = (second.correct + 1) % 4;
    assert_eq!(session.select_answer(wrong).map(|o| o.correct), Some(false));
    assert_eq!((session.score(), session.emotion()), (1, Emotion::Angry));
    assert_eq!(session.answer_class(wrong), "mg-answer revealed wrong");
    assert_eq!(
        session.current_comment(1),
        format!(
            "Sigh... Wrong. The answer was '{}'.",
            localize(second.a[second.correct], 1)
        )
    );

    // Closing and reopening keeps the game.
    assert_eq!(session.on_show(), Emotion::Thinking);

    // Answer the rest correctly → 9 / 10: `game_win` only.
    for index in 2..10 {
        assert!(!session.next_question().finished);
        let q = session.questions()[index];
        if index == 9 {
            assert_eq!(session.counter_text(), "Q10 / 10");
            assert_eq!(session.next_button_label(0), "結果を見る");
        }
        session.select_answer(q.correct);
    }
    let done = session.next_question();
    assert_eq!(
        done,
        NextOutcome {
            finished: true,
            events: vec![AchievementEvent::GameWin]
        }
    );
    assert_eq!(
        (session.state(), session.score(), session.emotion()),
        (GameState::Result, 9, Emotion::Smile)
    );
    assert!(session.active_question().is_none());
    assert_eq!(session.score_result(1), "9 / 10 correct");
    assert_eq!(session.result_comment(1), "Hmm, not bad. Not bad at all.");
    assert_eq!(session.on_show(), Emotion::Smile);
    session.back_to_menu();
    assert_eq!(
        (session.state(), session.emotion(), session.score()),
        (GameState::Menu, Emotion::Smile, 9)
    );

    // A perfect game sends `game_win` then `game_perfect`; a zero game sends nothing.
    let questions = generate_questions(&mut Mulberry32::new(7));
    session.start_with_questions(questions.clone());
    let mut last = NextOutcome {
        finished: false,
        events: Vec::new(),
    };
    for q in &questions {
        session.select_answer(q.correct);
        last = session.next_question();
    }
    assert_eq!(
        last.events,
        [AchievementEvent::GameWin, AchievementEvent::GamePerfect]
    );
    assert_eq!((session.score(), session.emotion()), (10, Emotion::Smug));
    session.start_with_questions(questions.clone());
    for q in &questions {
        session.select_answer((q.correct + 1) % 4);
        last = session.next_question();
    }
    assert!(last.finished && last.events.is_empty());
    assert_eq!(session.emotion(), Emotion::Disgust);
    assert_eq!(GameState::Result.as_str(), "result");
}
