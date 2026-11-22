//! V1 `MiniGamePanel` (rendered by ChatPanel, `RealAmadeusApp.tsx` 2532–2827). OWNED BY: panels
//! engineer.
//!
//! The game logic is `amadeus_core::minigame::MiniGameSession`; this component owns the DOM, the
//! visibility effects and the lip-sync timers:
//!
//! * Always mounted; `div.mini-game-panel` fades (0.25 s, z 50); in the mobile layout it is a
//!   fixed 46 % bottom sheet (inline style + the mobile CSS).
//! * Shown → Kurisu's emotion follows the state (menu SMILE, playing THINKING, result = tier
//!   emotion) and is sent through `on_emotion_change` while visible (also on every later change).
//!   Hidden (and once at mount) → the lip-sync stops (`on_speak_value_change(0)`).
//! * An answer speaks its comment: `on_speak_value_change(Math.random())` immediately and every
//!   120 ms for `clamp(len × 90, 800, 3000)` ms, then 0. The result comment is not spoken.
//! * START / RETRY notify `game_play`; finishing notifies `game_win` (score > 0) then
//!   `game_perfect` (10/10) through `managers::notify_achievement` (toast + achievements panel).
//! * Backspace or Escape closes while visible; there is no keyboard answering. Closing keeps the
//!   game; reopening resumes it.

use std::time::Duration;

use leptos::prelude::*;

use amadeus_core::minigame::{
    localize, menu_desc_text, mg_t, speak_duration_ms, GameState, MiniGameSession, MiniGameText,
    MG_MENU_BACK_KEY, MG_MENU_TITLE_KEY, MG_RESULT_TITLE_KEY, MG_RETRY_KEY, MG_START_KEY,
};

use super::common::{close_label, fire, minigame_style, use_keydown_while_visible, visible_memo};
use crate::app::managers;
use crate::hooks::viewport::use_is_mobile_viewport;

/// Which content block is rendered (V1 `gameState` + the `activeQuestion` guard).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Menu,
    Playing,
    /// `gameState === "playing"` without an active question: nothing is rendered.
    Empty,
    Result,
}

#[component]
pub fn MiniGamePanel(
    #[prop(into)] visible: Signal<bool>,
    #[prop(into)] config_language: Signal<i32>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_emotion_change: Callback<String>,
    #[prop(into)] on_speak_value_change: Callback<f64>,
) -> impl IntoView {
    let visible = visible_memo(visible);
    let is_mobile = use_is_mobile_viewport();
    let session = RwSignal::new(MiniGameSession::new());
    let emotion = Memo::new(move |_| session.with(MiniGameSession::emotion));
    let revealed = Memo::new(move |_| session.with(MiniGameSession::is_revealed));
    let stage = Memo::new(move |_| {
        session.with(|s| match s.state() {
            GameState::Menu => Stage::Menu,
            GameState::Playing if s.active_question().is_some() => Stage::Playing,
            GameState::Playing => Stage::Empty,
            GameState::Result => Stage::Result,
        })
    });
    let lang = move || i64::from(config_language.get());

    // `speakIntervalRef` / `speakStopTimeoutRef`.
    let speak_interval = StoredValue::new(None::<IntervalHandle>);
    let speak_stop = StoredValue::new(None::<TimeoutHandle>);

    let stop_speaking = move || {
        if let Some(interval) = speak_interval.try_update_value(Option::take).flatten() {
            interval.clear();
        }
        if let Some(timeout) = speak_stop.try_update_value(Option::take).flatten() {
            timeout.clear();
        }
        fire(on_speak_value_change, 0.0);
    };
    let start_speaking = move |text: &str| {
        if let Some(interval) = speak_interval.try_get_value().flatten() {
            interval.clear();
        }
        if let Some(timeout) = speak_stop.try_get_value().flatten() {
            timeout.clear();
        }
        let ms = speak_duration_ms(text);
        fire(on_speak_value_change, js_sys::Math::random());
        let interval = set_interval_with_handle(
            move || fire(on_speak_value_change, js_sys::Math::random()),
            Duration::from_millis(120),
        )
        .ok();
        speak_interval.try_set_value(interval);
        let timeout = set_timeout_with_handle(
            move || {
                if let Some(interval) = speak_interval.try_update_value(Option::take).flatten() {
                    interval.clear();
                }
                speak_stop.try_set_value(None);
                fire(on_speak_value_change, 0.0);
            },
            Duration::from_millis(u64::from(ms)),
        )
        .ok();
        speak_stop.try_set_value(timeout);
    };

    // 1. "QT onVisibleChanged parity": shown → recompute the emotion from the state; hidden (and
    //    at mount) → stop speaking.
    Effect::new(move |_| {
        if visible.get() {
            session.update(|s| {
                s.on_show();
            });
        } else {
            stop_speaking();
        }
    });
    // 2. Forward the emotion while visible (V1 deps `[visible, kurisuEmotion]`).
    Effect::new(move |_| {
        let current = emotion.get();
        if visible.get() {
            fire(on_emotion_change, current.as_str().to_owned());
        }
    });
    // 3. Unmount: clear the timers without callbacks.
    on_cleanup(move || {
        if let Some(interval) = speak_interval.try_update_value(Option::take).flatten() {
            interval.clear();
        }
        if let Some(timeout) = speak_stop.try_update_value(Option::take).flatten() {
            timeout.clear();
        }
    });
    // 4. Keys while visible.
    use_keydown_while_visible(visible, move |event| {
        let key = event.key();
        if key == "Backspace" || key == "Escape" {
            event.prevent_default();
            fire(on_close, ());
        }
    });

    // START / RETRY.
    let start_game = move |_| {
        stop_speaking();
        let mut rng = managers::rng();
        if let Some(event) = session.try_update(|s| s.start_game(&mut rng)) {
            managers::notify_achievement(&event);
        }
    };
    let select_answer = move |index: usize| {
        if session
            .try_update(|s| s.select_answer(index))
            .flatten()
            .is_some()
        {
            let comment = session
                .with_untracked(|s| s.current_comment(i64::from(config_language.get_untracked())));
            start_speaking(&comment);
        }
    };
    // NEXT / SHOW RESULT.
    let next_question = move |_| {
        stop_speaking();
        if let Some(outcome) = session.try_update(MiniGameSession::next_question) {
            for event in &outcome.events {
                managers::notify_achievement(event);
            }
        }
    };
    let back_to_menu = move |_| session.update(MiniGameSession::back_to_menu);

    // `${activeQuestion.q[0]}-${index}` keys: new buttons per question, patched on reveal.
    let answers = move || {
        session.with(|s| {
            s.active_question()
                .map(|question| {
                    question
                        .a
                        .iter()
                        .enumerate()
                        .map(|(index, text)| (question.q.0, index, *text))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    };
    let answer_button = move |(_, index, text): (&'static str, usize, MiniGameText)| {
        view! {
            <button
                type="button"
                class=move || session.with(|s| s.answer_class(index))
                disabled=move || revealed.get()
                on:click=move |_| select_answer(index)
            >
                <span class="mg-answer-text">{move || localize(text, lang())}</span>
            </button>
        }
    };
    let comment = Memo::new(move |_| session.with(|s| s.current_comment(lang())));

    let menu_view = move || {
        view! {
            <div class="mini-game-menu">
                <div class="mg-menu-title">{move || mg_t(MG_MENU_TITLE_KEY, lang())}</div>
                <div class="mg-menu-desc">{move || menu_desc_text(lang())}</div>
                <button type="button" class="mg-start-button" on:click=start_game>
                    {move || mg_t(MG_START_KEY, lang())}
                </button>
            </div>
        }
    };
    let question_text = move || {
        session.with(|s| {
            s.active_question()
                .map(|q| localize(q.q, lang()))
                .unwrap_or_default()
        })
    };
    let playing_view = move || {
        view! {
            <div class="mini-game-playing">
                <div class="mg-header">
                    <span class="mg-question-counter">{move || session.with(MiniGameSession::counter_text)}</span>
                    <span class="mg-score-label">{move || session.with(|s| s.score_label(lang()))}</span>
                </div>
                <div class="mg-question-box">{question_text}</div>
                <div class="mg-answers">
                    <For each=answers key=|(question, index, _)| (*question, *index) children=answer_button/>
                </div>
                <div class="mg-spacer"></div>
                <Show when=move || revealed.get()>
                    <button type="button" class="mg-next-button" on:click=next_question>
                        {move || session.with(|s| s.next_button_label(lang()))}
                    </button>
                </Show>
            </div>
        }
    };
    let result_view = move || {
        view! {
            <div class="mini-game-result">
                <div class="mg-result-title">{move || mg_t(MG_RESULT_TITLE_KEY, lang())}</div>
                <div class="mg-score-result">{move || session.with(|s| s.score_result(lang()))}</div>
                <div class="mg-result-comment">{move || session.with(|s| s.result_comment(lang()))}</div>
                <div class="mg-result-buttons">
                    <button type="button" class="mg-retry-button" on:click=start_game>
                        {move || mg_t(MG_RETRY_KEY, lang())}
                    </button>
                    <button type="button" class="mg-menu-back-button" on:click=back_to_menu>
                        {move || mg_t(MG_MENU_BACK_KEY, lang())}
                    </button>
                </div>
            </div>
        }
    };
    // `{gameState === "menu" && …}{gameState === "playing" && activeQuestion && …}{gameState ===
    // "result" && …}`: the block is rebuilt only when the stage changes.
    let content = move || match stage.get() {
        Stage::Menu => menu_view().into_any(),
        Stage::Playing => playing_view().into_any(),
        Stage::Empty => ().into_any(),
        Stage::Result => result_view().into_any(),
    };

    view! {
        <div class="mini-game-panel" style=move || minigame_style(visible.get(), is_mobile.get())>
            <div class="mini-game-title">"MINI GAME"</div>
            <div class="mini-game-content">{content}</div>
            <Show when=move || revealed.get() && comment.with(|c| !c.is_empty())>
                <div class="mg-comment-bubble">{move || comment.get()}</div>
            </Show>
            <button type="button" class="overlay-close-button mg-close-button" on:click=move |_| fire(on_close, ())>
                {move || close_label(config_language.get())}
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use amadeus_core::achievements::AchievementEvent;
    use amadeus_core::i18n;
    use amadeus_core::minigame::{
        result_comment, result_emotion, score_result_text, speak_duration_ms, wrong_answer_comment,
        MG_NEXT_KEY, MG_SCORE_KEY, MG_SHOW_RESULT_KEY,
    };
    use amadeus_core::random_events::SequenceRng;

    use super::*;

    /// spec-app-part3 Appendix C.1 (`mg_*` short keys), index = language.
    const C1: [(&str, [&str; 13]); 10] = [
        (
            MG_MENU_TITLE_KEY,
            [
                "紅莉栖とシュタゲクイズ",
                "Steins;Gate Quiz with Kurisu",
                "红莉栖与命运石之门问答",
                "크리스와 슈타게 퀴즈",
                "Quiz de Steins;Gate con Kurisu",
                "Quiz Steins;Gate avec Kurisu",
                "Steins;Gate-Quiz mit Kurisu",
                "Викторина по Steins;Gate с Курису",
                "Вікторина по Steins;Gate з Курісу",
                "Quiz de Steins;Gate com a Kurisu",
                "Kurisu ile Steins;Gate Bilgi Yarışması",
                "חידון שטיינס;גייט עם קוריסו",
                "مسابقة شتاينز;غيت مع كوريسو",
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
            MG_START_KEY,
            [
                "スタート",
                "Start",
                "开始",
                "시작",
                "Empezar",
                "Commencer",
                "Start",
                "Начать",
                "Почати",
                "Começar",
                "Başlat",
                "התחל",
                "ابدأ",
            ],
        ),
        (
            MG_SCORE_KEY,
            [
                "スコア",
                "Score",
                "得分",
                "점수",
                "Puntuación",
                "Score",
                "Punktzahl",
                "Счёт",
                "Рахунок",
                "Pontuação",
                "Puan",
                "ציון",
                "النقاط",
            ],
        ),
        (
            MG_NEXT_KEY,
            [
                "次の問題",
                "Next Question",
                "下一题",
                "다음 문제",
                "Siguiente pregunta",
                "Question suivante",
                "Nächste Frage",
                "Следующий вопрос",
                "Наступне запитання",
                "Próxima pergunta",
                "Sonraki Soru",
                "השאלה הבאה",
                "السؤال التالي",
            ],
        ),
        (
            MG_SHOW_RESULT_KEY,
            [
                "結果を見る",
                "See Results",
                "查看结果",
                "결과 보기",
                "Ver resultados",
                "Voir les résultats",
                "Ergebnisse ansehen",
                "Показать результаты",
                "Показати результати",
                "Ver resultados",
                "Sonuçları Gör",
                "הצג תוצאות",
                "عرض النتائج",
            ],
        ),
        (
            MG_RESULT_TITLE_KEY,
            [
                "結果",
                "Results",
                "结果",
                "결과",
                "Resultados",
                "Résultats",
                "Ergebnisse",
                "Результаты",
                "Результати",
                "Resultados",
                "Sonuçlar",
                "תוצאות",
                "النتائج",
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
            MG_RETRY_KEY,
            [
                "もう一度",
                "Try Again",
                "再试一次",
                "다시 하기",
                "Reintentar",
                "Réessayer",
                "Nochmal versuchen",
                "Ещё раз",
                "Спробувати ще",
                "Tentar de novo",
                "Tekrar Dene",
                "נסה שוב",
                "حاول مجددًا",
            ],
        ),
        (
            MG_MENU_BACK_KEY,
            [
                "メニューへ",
                "Menu",
                "菜单",
                "메뉴",
                "Menú",
                "Menu",
                "Menü",
                "Меню",
                "Меню",
                "Menu",
                "Menü",
                "תפריט",
                "القائمة",
            ],
        ),
    ];

    /// Appendix C.2 `mg_wrong`.
    const WRONG: [&str; 13] = [
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
    ];

    /// Appendix C.2 result tiers: (score, emotion, texts).
    const TIERS: [(u32, &str, [&str; 13]); 4] = [
        (
            10,
            "SMUG",
            [
                "「パーフェクト！……ま、まあ、私のことだし当然の結果よねっ！あんたにしてはよくやったわ、褒めてあげる！」",
                "Perfect! ...Well, obviously.",
                "满分！……哼，理所当然啦。",
                "완벽해! ...뭐, 당연하지.",
                "¡Perfecto! ...Bueno, obvio.",
                "Parfait ! ...Eh bien, évidemment.",
                "Perfekt! ...Naja, offensichtlich.",
                "Идеально! ...Ну, разумеется.",
                "Ідеально! ...Ну, звісно.",
                "Perfeito! ...Bem, óbvio.",
                "Mükemmel! ...Eh, tabii ki.",
                "מושלם! ...טוב, מובן מאליו.",
                "مثالي! ...حسنًا، بالطبع.",
            ],
        ),
        (
            6,
            "SMILE",
            [
                "「ふーん、まあまあね。天才の私には遠く及ばないけど、一般人にしては合格点なんじゃない？」",
                "Hmm, not bad. Not bad at all.",
                "嗯，还不错。还算可以。",
                "흠, 나쁘지 않네. 꽤 괜찮아.",
                "Hmm, nada mal. Nada mal en absoluto.",
                "Hmm, pas mal. Pas mal du tout.",
                "Hmm, nicht schlecht. Gar nicht schlecht.",
                "Хм, неплохо. Совсем неплохо.",
                "Хм, непогано. Зовсім непогано.",
                "Hmm, nada mal. Nada mal mesmo.",
                "Hmm, fena değil. Hiç fena değil.",
                "המם, לא רע. בכלל לא רע.",
                "همم، ليس سيئًا. ليس سيئًا أبدًا.",
            ],
        ),
        (
            1,
            "NORMAL",
            [
                "「……あんた、もしかして岡部並みに記憶力が退化してんじゃないの？もう少し脳トレしなさい！」",
                "...Wait, is your memory deteriorating like Okabe's? You need to do some brain training!",
                "……你该不会像冈部那样记忆力衰退了吧？多去做做脑力训练！",
                "……너, 설마 오카베처럼 기억력이 감퇴한 거 아니야? 뇌 운동 좀 더 해!",
                "...¿Acaso se te está deteriorando la memoria como a Okabe? ¡Deberías entrenar un poco más el cerebro!",
                "...Ta mémoire flancherait-elle comme celle d'Okabe ? Tu devrais faire un peu de gymnastique cérébrale !",
                "...Lässt dein Gedächtnis etwa nach wie bei Okabe? Du solltest dein Gehirn etwas mehr trainieren!",
                "...У тебя что, память ухудшилась, как у Окабэ? Потренируй мозг!",
                "...У тебе що, пам'ять погіршилась, як в Окабе? Потренуй мізок!",
                "...Sua memória está falhando como a do Okabe? Você deveria treinar mais o cérebro!",
                "...Yoksa hafızan Okabe gibi zayıflıyor mu? Beynini biraz daha çalıştırmalısın!",
                "...האם הזיכרון שלך נחלش כמו של אוקאבה? כדאי שתאמן קצת את המוח!",
                "...هل تراجعت ذاكرتك مثل أوكابي؟ يجب أن تمرن عقلك أكثر قليلاً!",
            ],
        ),
        (
            0,
            "DISGUST",
            [
                "「はぁ……全滅？あんたの海馬、ちょっとスキャンしてバグがないか調べた方がいいわね……」",
                "Sigh... A total wipeout? Do you even like Steins;Gate?",
                "唉……全军覆没？你到底喜不喜欢Steins;Gate啊？",
                "하아... 전멸이야? 정말 Steins;Gate 좋아하는 거 맞아?",
                "Ay... ¿Un desastre total? ¿De verdad te gusta Steins;Gate?",
                "Pff... Un échec total ? Tu aimes vraiment Steins;Gate ?",
                "Seufz... Ein Totalausfall? Magst du Steins;Gate überhaupt?",
                "Вздох... Полный провал? Тебе вообще нравится Steins;Gate?",
                "Зітхання... Повний провал? Тобі взагалі подобається Steins;Gate?",
                "Aff... Um desastre total? Você gosta mesmo de Steins;Gate?",
                "Of... Tam bir fiyasko? Steins;Gate'i gerçekten seviyor musun?",
                "אנחה... כישלון מוחלט? אתה בכלל אוהב את Steins;Gate?",
                "تنهيدة... فشل ذريع؟ هل تحب Steins;Gate أصلًا؟",
            ],
        ),
    ];

    #[test]
    fn ui_strings_in_all_languages() {
        for (key, values) in C1 {
            for (lang, want) in values.iter().enumerate() {
                assert_eq!(mg_t(key, lang as i64), *want, "{key} lang {lang}");
            }
        }
        // Rendered forms (`replace("%1", "10")`, `replace("%1", score).replace("%2", "10")`).
        assert_eq!(menu_desc_text(0), "シュタインズ・ゲートから10問出題！");
        assert_eq!(menu_desc_text(5), "10 questions tirées de Steins;Gate !");
        assert_eq!(score_result_text(3, 0), "3 / 10 正解");
        assert_eq!(score_result_text(10, 12), "10 / 10 صحيحة");
        // Out of range → ja.json.
        assert_eq!(mg_t(MG_START_KEY, 13), "スタート");
    }

    #[test]
    fn wrong_comment_in_all_languages() {
        for (lang, template) in WRONG.iter().enumerate() {
            assert_eq!(
                i18n::t("mg_wrong", "", lang as i64),
                *template,
                "lang {lang}"
            );
            assert_eq!(
                wrong_answer_comment("X", lang as i64),
                template.replacen("%1", "X", 1),
                "lang {lang}"
            );
        }
    }

    #[test]
    fn result_tiers_in_all_languages() {
        for (score, emotion, texts) in TIERS {
            assert_eq!(result_emotion(score).as_str(), emotion, "score {score}");
            for (lang, want) in texts.iter().enumerate() {
                assert_eq!(
                    result_comment(score, lang as i64),
                    *want,
                    "score {score} lang {lang}"
                );
            }
        }
        // Tier boundaries: 10 perfect, 6–9 good, 1–5 ok, 0 bad.
        for (score, tier_score) in [(9, 6), (7, 6), (5, 1), (2, 1)] {
            assert_eq!(
                result_comment(score, 1),
                result_comment(tier_score, 1),
                "score {score}"
            );
        }
    }

    #[test]
    fn speak_duration_is_clamped() {
        // max(800, min(3000, utf16_len * 90)).
        assert_eq!(speak_duration_ms(""), 800);
        assert_eq!(speak_duration_ms("123456789"), 810);
        assert_eq!(
            speak_duration_ms("電話レンジ（仮）。命名センスは問わないで。"),
            1890
        );
        assert_eq!(speak_duration_ms(&"a".repeat(40)), 3000);
    }

    #[test]
    fn session_drives_the_v1_flow() {
        // A constant-0 RNG keeps the pool order; every question's right answer is then a[0]
        // shuffled to position 1 (Fisher–Yates with j = 0).
        let mut session = MiniGameSession::new();
        let mut rng = SequenceRng::new(vec![0.0]);
        assert_eq!(session.start_game(&mut rng), AchievementEvent::GamePlay);
        assert_eq!(session.state(), GameState::Playing);
        assert_eq!(session.emotion().as_str(), "THINKING");
        let mut events = Vec::new();
        for _ in 0..10 {
            let correct = session.active_question().map(|q| q.correct).unwrap_or(0);
            assert!(session.select_answer(correct).is_some_and(|o| o.correct));
            assert!(
                session.select_answer(correct).is_none(),
                "answers lock after a reveal"
            );
            assert_eq!(session.emotion().as_str(), "SMILE");
            events = session.next_question().events;
        }
        assert_eq!(session.state(), GameState::Result);
        assert_eq!(
            events,
            vec![AchievementEvent::GameWin, AchievementEvent::GamePerfect]
        );
        assert_eq!(session.emotion().as_str(), "SMUG");
        session.back_to_menu();
        assert_eq!(session.state(), GameState::Menu);
        assert_eq!(session.on_show().as_str(), "SMILE");
    }

    #[test]
    fn wrong_answers_and_resume() {
        let mut session = MiniGameSession::new();
        let mut rng = SequenceRng::new(vec![0.0]);
        session.start_game(&mut rng);
        let correct = session.active_question().map(|q| q.correct).unwrap_or(0);
        let wrong = (correct + 1) % 4;
        assert!(session.select_answer(wrong).is_some_and(|o| !o.correct));
        assert_eq!(session.emotion().as_str(), "ANGRY");
        assert_eq!(session.answer_class(correct), "mg-answer revealed correct");
        assert_eq!(session.answer_class(wrong), "mg-answer revealed wrong");
        assert_eq!(
            session.answer_class((correct + 2) % 4),
            "mg-answer revealed"
        );
        assert!(session
            .current_comment(1)
            .starts_with("Sigh... Wrong. The answer was '"));
        // Closing and reopening while playing resumes with THINKING (V1 visible effect).
        assert_eq!(session.on_show().as_str(), "THINKING");
        for _ in 0..10 {
            session.next_question();
        }
        assert_eq!(session.state(), GameState::Result);
        assert_eq!(session.score(), 0);
        assert_eq!(session.on_show().as_str(), "DISGUST");
        assert_eq!(session.result_comment(0), TIERS[3].2[0]);
    }
}
