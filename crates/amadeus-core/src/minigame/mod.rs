//! Steins;Gate quiz: port of V1 `lib/amadeus/minigame.ts` (data and pure logic, itself ported
//! from the Qt `MiniGamePanel.qml`) and, in [`session`], the state machine of the V1
//! `MiniGamePanel` component.
//!
//! * [`QUESTION_POOL`]: the 49 entries verbatim; every text is a `(i18n key, Japanese default)`
//!   pair ([`MiniGameText`]) and the right answer is always `a[0]` in the source.
//! * [`generate_questions`] consumes **exactly 78** random numbers in V1 order: a 48-swap
//!   Fisher–Yates over the pool indices, then 3 swaps per question for the answer order.
//! * Displayed text always comes from the locale JSON ([`localize`] = `t(key, default, lang)`);
//!   the in-code defaults are only fallbacks (49 comments, one question, one answer, the four
//!   result comments and `mg_wrong` differ in `ja.json`).
//! * Achievement events (`game_play`, `game_win`, `game_perfect`) are returned by the session as
//!   [`AchievementEvent`](crate::achievements::AchievementEvent)s; the caller forwards them to
//!   `AchievementManager::notify_event`. Lip-sync timers stay in the UI ([`speak_duration_ms`]).

mod pool;
pub mod session;

use crate::i18n;
use crate::jscompat::utf16_len;
use crate::random_events::{random_index, Rng};
use crate::types::Emotion;

pub use pool::QUESTION_POOL;
pub use session::{AnswerOutcome, GameState, MiniGameSession, NextOutcome};

/// `[i18nKey, jpDefault]` text pair (V1 `MiniGameText`).
pub type MiniGameText = (&'static str, &'static str);

/// Static pool entry exactly as authored (V1 `MiniGameEntry`; `correct` is always 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MiniGameEntry {
    /// Question.
    pub q: MiniGameText,
    /// The four answers (the right one at `correct`).
    pub a: [MiniGameText; 4],
    /// Index of the right answer in `a`.
    pub correct: usize,
    /// Kurisu's comment for a right answer.
    pub c: MiniGameText,
}

/// A generated question: answers shuffled, `correct` recomputed (V1 `MiniGameQuestion`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MiniGameQuestion {
    /// Question.
    pub q: MiniGameText,
    /// Answers in display order.
    pub a: [MiniGameText; 4],
    /// Position of the right answer in `a`.
    pub correct: usize,
    /// Kurisu's comment for a right answer.
    pub c: MiniGameText,
}

/// Questions per game (Qt `totalQuestions`).
pub const TOTAL_QUESTIONS: usize = 10;

/// `t(key, default, lang)` for a text pair (V1 panel helper `tt(pair)`).
pub fn localize(text: MiniGameText, lang: i64) -> &'static str {
    i18n::t(text.0, text.1, lang)
}

/// In-place Fisher–Yates shuffle (V1 `shuffle`, Qt `shuffleArray`): for `i = len-1 … 1`,
/// `j = floor(random() * (i + 1))`, swap. Consumes `len - 1` random numbers. An out-of-contract
/// random value is clamped into `0..=i` instead of corrupting the array.
pub fn shuffle<T>(arr: &mut [T], rng: &mut dyn Rng) {
    for i in (1..arr.len()).rev() {
        let random = rng.next_f64();
        let j = random_index(random, i + 1).unwrap_or(if random >= 1.0 { i } else { 0 });
        arr.swap(i, j);
    }
}

/// V1 `generateQuestions()`: shuffle the 49 pool indices, take the first
/// [`TOTAL_QUESTIONS`], and shuffle each question's four answer positions, recomputing `correct`
/// (the new position of the source's right answer). Exactly 78 random numbers, all pool swaps
/// first, then the answer swaps in question order.
pub fn generate_questions(rng: &mut dyn Rng) -> Vec<MiniGameQuestion> {
    let mut indices: Vec<usize> = (0..QUESTION_POOL.len()).collect();
    shuffle(&mut indices, rng);
    let count = TOTAL_QUESTIONS.min(indices.len());
    let mut questions = Vec::with_capacity(count);
    for &index in &indices[..count] {
        let src = &QUESTION_POOL[index];
        let mut order = [0_usize, 1, 2, 3];
        shuffle(&mut order, rng);
        let mut correct = 0;
        for (k, &position) in order.iter().enumerate() {
            if position == src.correct {
                correct = k;
            }
        }
        questions.push(MiniGameQuestion {
            q: src.q,
            a: order.map(|position| src.a[position]),
            correct,
            c: src.c,
        });
    }
    questions
}

// ---------------------------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------------------------

/// Result-comment tier (V1 `ResultCommentTier`), from the final score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResultTier {
    /// 10 / 10.
    Perfect,
    /// 6–9.
    Good,
    /// 1–5.
    Ok,
    /// 0.
    Bad,
}

impl ResultTier {
    /// V1 `RESULT_COMMENT_TIERS` order.
    pub const ALL: [ResultTier; 4] = [
        ResultTier::Perfect,
        ResultTier::Good,
        ResultTier::Ok,
        ResultTier::Bad,
    ];

    /// Tier of a score (`== 10` perfect, `>= 6` good, `>= 1` ok, else bad).
    pub fn from_score(score: u32) -> Self {
        if score as usize == TOTAL_QUESTIONS {
            ResultTier::Perfect
        } else if score >= 6 {
            ResultTier::Good
        } else if score >= 1 {
            ResultTier::Ok
        } else {
            ResultTier::Bad
        }
    }

    /// Tier name (`"perfect"`, `"good"`, `"ok"`, `"bad"`).
    pub fn as_str(self) -> &'static str {
        match self {
            ResultTier::Perfect => "perfect",
            ResultTier::Good => "good",
            ResultTier::Ok => "ok",
            ResultTier::Bad => "bad",
        }
    }

    /// i18n key (V1 `RESULT_COMMENT_KEYS`).
    pub fn key(self) -> &'static str {
        match self {
            ResultTier::Perfect => "mg_result_perfect",
            ResultTier::Good => "mg_result_good",
            ResultTier::Ok => "mg_result_ok",
            ResultTier::Bad => "mg_result_bad",
        }
    }

    /// In-code Japanese fallback (V1 `RESULT_COMMENT_DEFAULTS`; `ja.json` differs).
    pub fn default_text(self) -> &'static str {
        match self {
            ResultTier::Perfect => "「パーフェクト！…まあ、当然の結果よね。」",
            ResultTier::Good => "「ふーん、まあまあね。悪くないわ。」",
            ResultTier::Ok => "「…もう少し勉強した方がいいんじゃない？」",
            ResultTier::Bad => "「はぁ…全滅？本当にシュタゲ好きなの？」",
        }
    }

    /// Kurisu's emotion on the result screen (V1 `resultEmotion`).
    pub fn emotion(self) -> Emotion {
        match self {
            ResultTier::Perfect => Emotion::Smug,
            ResultTier::Good => Emotion::Smile,
            ResultTier::Ok => Emotion::Normal,
            ResultTier::Bad => Emotion::Disgust,
        }
    }
}

/// V1 `resultEmotion(score)`: 10 → SMUG, ≥ 6 → SMILE, ≥ 1 → NORMAL, else DISGUST.
pub fn result_emotion(score: u32) -> Emotion {
    ResultTier::from_score(score).emotion()
}

/// V1 `resultCommentKey(score)`.
pub fn result_comment_key(score: u32) -> &'static str {
    ResultTier::from_score(score).key()
}

/// V1 panel `resultCommentDefaultForKey(key)`: the tier default of a result key, the `bad`
/// default for any other key.
pub fn result_comment_default_for_key(key: &str) -> &'static str {
    ResultTier::ALL
        .into_iter()
        .find(|tier| tier.key() == key)
        .unwrap_or(ResultTier::Bad)
        .default_text()
}

/// Result-screen comment: `t(resultCommentKey(score), resultCommentDefaultForKey(key), lang)`.
pub fn result_comment(score: u32, lang: i64) -> &'static str {
    let key = result_comment_key(score);
    i18n::t(key, result_comment_default_for_key(key), lang)
}

// ---------------------------------------------------------------------------------------------
// UI strings
// ---------------------------------------------------------------------------------------------

/// Wrong-answer comment key (V1 `MG_WRONG_KEY`); `%1` = the localized right answer.
pub const MG_WRONG_KEY: &str = "mg_wrong";
/// In-code fallback of [`MG_WRONG_KEY`] (`ja.json` differs).
pub const MG_WRONG_DEFAULT: &str = "はぁ…不正解よ。答えは「%1」だったわ。";

/// `mg_menu_title`.
pub const MG_MENU_TITLE_KEY: &str = "mg_menu_title";
/// `mg_menu_desc` (`%1` = [`TOTAL_QUESTIONS`]).
pub const MG_MENU_DESC_KEY: &str = "mg_menu_desc";
/// `mg_start`.
pub const MG_START_KEY: &str = "mg_start";
/// `mg_score`.
pub const MG_SCORE_KEY: &str = "mg_score";
/// `mg_next`.
pub const MG_NEXT_KEY: &str = "mg_next";
/// `mg_show_result`.
pub const MG_SHOW_RESULT_KEY: &str = "mg_show_result";
/// `mg_result_title`.
pub const MG_RESULT_TITLE_KEY: &str = "mg_result_title";
/// `mg_score_result` (`%1` = score, `%2` = [`TOTAL_QUESTIONS`]).
pub const MG_SCORE_RESULT_KEY: &str = "mg_score_result";
/// `mg_retry`.
pub const MG_RETRY_KEY: &str = "mg_retry";
/// `mg_menu_back`.
pub const MG_MENU_BACK_KEY: &str = "mg_menu_back";

/// V1 `MG_UI_DEFAULTS` (in-code Japanese fallbacks, keyed by i18n key, V1 order).
pub const MG_UI_DEFAULTS: [(&str, &str); 10] = [
    (MG_MENU_TITLE_KEY, "紅莉栖とシュタゲクイズ"),
    (MG_MENU_DESC_KEY, "シュタインズ・ゲートから%1問出題！"),
    (MG_START_KEY, "スタート"),
    (MG_SCORE_KEY, "スコア"),
    (MG_NEXT_KEY, "次の問題"),
    (MG_SHOW_RESULT_KEY, "結果を見る"),
    (MG_RESULT_TITLE_KEY, "結果"),
    (MG_SCORE_RESULT_KEY, "%1 / %2 正解"),
    (MG_RETRY_KEY, "もう一度"),
    (MG_MENU_BACK_KEY, "メニューへ"),
];

/// In-code fallback of a UI key (`None` for keys outside [`MG_UI_DEFAULTS`]).
pub fn mg_ui_default(key: &str) -> Option<&'static str> {
    MG_UI_DEFAULTS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, default)| *default)
}

/// V1 panel helper `mgT(key)` = `t(key, MG_UI_DEFAULTS[key], lang)`.
pub fn mg_t(key: &str, lang: i64) -> &str {
    i18n::t(key, mg_ui_default(key).unwrap_or(""), lang)
}

/// Menu description: `mgT("mg_menu_desc").replace("%1", "10")`.
pub fn menu_desc_text(lang: i64) -> String {
    i18n::fill(mg_t(MG_MENU_DESC_KEY, lang), &["10"])
}

/// Result line: `mgT("mg_score_result").replace("%1", score).replace("%2", "10")`.
pub fn score_result_text(score: u32, lang: i64) -> String {
    i18n::fill(mg_t(MG_SCORE_RESULT_KEY, lang), &[&score.to_string(), "10"])
}

/// Score label while playing: `` `${mgT("mg_score")}: ${score}` `` (ASCII `": "` in every
/// language).
pub fn score_label_text(score: u32, lang: i64) -> String {
    format!("{}: {score}", mg_t(MG_SCORE_KEY, lang))
}

/// Question counter `` `Q${index + 1} / 10` `` (uses the constant, like V1).
pub fn question_counter_text(index: usize) -> String {
    format!("Q{} / {TOTAL_QUESTIONS}", index + 1)
}

/// Wrong-answer comment: `t("mg_wrong", MG_WRONG_DEFAULT, lang).replace("%1", correct_text)`
/// (first occurrence, JS replacement patterns honoured).
pub fn wrong_answer_comment(correct_text: &str, lang: i64) -> String {
    i18n::replace_first_js(
        i18n::t(MG_WRONG_KEY, MG_WRONG_DEFAULT, lang),
        "%1",
        correct_text,
    )
}

/// Lip-sync duration of a spoken comment: `max(800, min(3000, text.length * 90))` ms, with the
/// length in UTF-16 units. (Mouth values are `Math.random()` every 120 ms, drawn by the UI.)
pub fn speak_duration_ms(text: &str) -> u32 {
    let units = u32::try_from(utf16_len(text)).unwrap_or(u32::MAX);
    units.saturating_mul(90).clamp(800, 3000)
}

#[cfg(test)]
mod tests;
