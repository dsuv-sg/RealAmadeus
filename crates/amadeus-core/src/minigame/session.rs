//! The V1 `MiniGamePanel` state machine (`RealAmadeusApp.tsx` 2532–2827) without its UI: game
//! state, questions, score, the selected answer, the reveal flag and Kurisu's emotion
//! (`kurisuEmotion`). The UI keeps the timers (lip-sync), the visibility and the DOM.
//!
//! ```text
//! Menu ──start_game──▶ Playing ──select_answer──▶ (revealed) ──next_question──▶ Playing … ──▶ Result
//!  ▲                                                                                        │
//!  └──────────────────────────── back_to_menu ◀────────────── start_game (retry) ◀──────────┘
//! ```
//!
//! V1 details kept: the score of a finished game counts as a "win" when at least one answer was
//! right (`game_win` for `score > 0`, then `game_perfect` for 10/10, in that order); closing the
//! panel keeps the game, and [`MiniGameSession::on_show`] recomputes the emotion when it is shown
//! again; `back_to_menu` keeps the questions and the score.

use super::{
    generate_questions, localize, mg_t, question_counter_text, result_comment, result_emotion,
    score_label_text, score_result_text, wrong_answer_comment, MiniGameQuestion, MG_NEXT_KEY,
    MG_SHOW_RESULT_KEY, TOTAL_QUESTIONS,
};
use crate::achievements::AchievementEvent;
use crate::random_events::Rng;
use crate::types::Emotion;

/// V1 `gameState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    /// Title screen (initial).
    #[default]
    Menu,
    /// A question is shown.
    Playing,
    /// Final score screen.
    Result,
}

impl GameState {
    /// V1 string id (`"menu"`, `"playing"`, `"result"`).
    pub fn as_str(self) -> &'static str {
        match self {
            GameState::Menu => "menu",
            GameState::Playing => "playing",
            GameState::Result => "result",
        }
    }
}

/// Result of [`MiniGameSession::select_answer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnswerOutcome {
    /// The selected index was the right answer (score incremented, emotion SMILE; otherwise
    /// ANGRY).
    pub correct: bool,
    /// The selected index.
    pub selected: usize,
    /// Position of the right answer.
    pub correct_index: usize,
}

/// Result of [`MiniGameSession::next_question`].
#[derive(Debug, Clone, PartialEq)]
pub struct NextOutcome {
    /// The game moved to the result screen.
    pub finished: bool,
    /// Achievement events to send, in order (`GameWin` if score > 0, then `GamePerfect` if
    /// 10/10). Empty unless `finished`.
    pub events: Vec<AchievementEvent>,
}

/// One MiniGamePanel instance's game state (V1 component state + `kurisuEmotion`).
#[derive(Debug, Clone, PartialEq)]
pub struct MiniGameSession {
    state: GameState,
    questions: Vec<MiniGameQuestion>,
    current: usize,
    score: u32,
    selected: Option<usize>,
    revealed: bool,
    emotion: Emotion,
}

impl Default for MiniGameSession {
    fn default() -> Self {
        Self::new()
    }
}

impl MiniGameSession {
    /// Initial state: menu, no questions, emotion SMILE.
    pub fn new() -> Self {
        Self {
            state: GameState::Menu,
            questions: Vec::new(),
            current: 0,
            score: 0,
            selected: None,
            revealed: false,
            emotion: Emotion::Smile,
        }
    }

    /// Current state.
    pub fn state(&self) -> GameState {
        self.state
    }

    /// The generated questions of the current game.
    pub fn questions(&self) -> &[MiniGameQuestion] {
        &self.questions
    }

    /// Index of the current question (`currentQuestion`).
    pub fn current_index(&self) -> usize {
        self.current
    }

    /// Right answers so far.
    pub fn score(&self) -> u32 {
        self.score
    }

    /// Selected answer of the current question (`selectedAnswer`, V1 `-1` = `None`).
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// The current question's answer has been revealed (`answerRevealed`).
    pub fn is_revealed(&self) -> bool {
        self.revealed
    }

    /// Kurisu's emotion while the panel is visible (`kurisuEmotion`).
    pub fn emotion(&self) -> Emotion {
        self.emotion
    }

    /// V1 `startGame()` (START and RETRY): new questions ([`generate_questions`], 78 random
    /// numbers), index 0, score 0, nothing selected, state `Playing`, emotion THINKING. Returns
    /// the achievement event V1 sends (`game_play`). The UI stops the lip-sync first.
    pub fn start_game(&mut self, rng: &mut dyn Rng) -> AchievementEvent {
        self.start_with_questions(generate_questions(rng))
    }

    /// [`start_game`](Self::start_game) with given questions (tests, replays).
    pub fn start_with_questions(&mut self, questions: Vec<MiniGameQuestion>) -> AchievementEvent {
        self.questions = questions;
        self.current = 0;
        self.score = 0;
        self.selected = None;
        self.revealed = false;
        self.state = GameState::Playing;
        self.emotion = Emotion::Thinking;
        AchievementEvent::GamePlay
    }

    /// V1 `selectAnswer(index)`: ignored (`None`) once revealed or without a current question.
    /// Otherwise marks `index` selected and revealed; a right answer adds 1 and sets SMILE, a
    /// wrong one sets ANGRY. The UI then speaks [`current_comment`](Self::current_comment) for
    /// [`speak_duration_ms`](super::speak_duration_ms).
    pub fn select_answer(&mut self, index: usize) -> Option<AnswerOutcome> {
        if self.revealed {
            return None;
        }
        let question = self.questions.get(self.current)?;
        let correct_index = question.correct;
        let correct = index == correct_index;
        self.selected = Some(index);
        self.revealed = true;
        if correct {
            self.score += 1;
            self.emotion = Emotion::Smile;
        } else {
            self.emotion = Emotion::Angry;
        }
        Some(AnswerOutcome {
            correct,
            selected: index,
            correct_index,
        })
    }

    /// V1 `nextQuestion()` (NEXT / SHOW RESULT button, shown only after a reveal): advances and
    /// clears the selection; past the last question the state becomes `Result` with
    /// [`result_emotion`], otherwise the emotion is THINKING. The UI stops the lip-sync first.
    pub fn next_question(&mut self) -> NextOutcome {
        let next = self.current + 1;
        self.current = next;
        self.selected = None;
        self.revealed = false;
        if next >= self.questions.len() {
            self.state = GameState::Result;
            self.emotion = result_emotion(self.score);
            let mut events = Vec::new();
            if self.score > 0 {
                events.push(AchievementEvent::GameWin);
            }
            if self.score as usize == TOTAL_QUESTIONS {
                events.push(AchievementEvent::GamePerfect);
            }
            NextOutcome {
                finished: true,
                events,
            }
        } else {
            self.emotion = Emotion::Thinking;
            NextOutcome {
                finished: false,
                events: Vec::new(),
            }
        }
    }

    /// Result screen MENU button: state `Menu`, emotion SMILE (questions and score untouched).
    pub fn back_to_menu(&mut self) {
        self.state = GameState::Menu;
        self.emotion = Emotion::Smile;
    }

    /// Panel became visible (Qt `onVisibleChanged` parity): playing → THINKING, result →
    /// [`result_emotion`], menu → SMILE. Returns the new emotion (forward it to Live2D while the
    /// panel is visible).
    pub fn on_show(&mut self) -> Emotion {
        self.emotion = match self.state {
            GameState::Playing => Emotion::Thinking,
            GameState::Result => result_emotion(self.score),
            GameState::Menu => Emotion::Smile,
        };
        self.emotion
    }

    /// `currentQuestion < questions.length ? questions[currentQuestion] : undefined`. The playing
    /// screen renders only when this is `Some`.
    pub fn active_question(&self) -> Option<&MiniGameQuestion> {
        self.questions.get(self.current)
    }

    /// Comment bubble text (`currentComment`, recomputed on render so it follows language
    /// changes): `""` unless revealed; the localized `c` for a right answer, otherwise `mg_wrong`
    /// with `%1` = the localized right answer.
    pub fn current_comment(&self, lang: i64) -> String {
        if !self.revealed {
            return String::new();
        }
        let Some(question) = self.active_question() else {
            return String::new();
        };
        if self.selected == Some(question.correct) {
            localize(question.c, lang).to_owned()
        } else {
            let correct_text = question
                .a
                .get(question.correct)
                .map_or("", |answer| localize(*answer, lang));
            wrong_answer_comment(correct_text, lang)
        }
    }

    /// CSS class of answer button `index`: `mg-answer`, `mg-answer revealed`,
    /// `mg-answer revealed correct` (the right answer once revealed) or
    /// `mg-answer revealed wrong` (the selected wrong answer).
    pub fn answer_class(&self, index: usize) -> &'static str {
        if !self.revealed {
            return "mg-answer";
        }
        let correct = self.active_question().map(|q| q.correct);
        if correct == Some(index) {
            "mg-answer revealed correct"
        } else if self.selected == Some(index) {
            "mg-answer revealed wrong"
        } else {
            "mg-answer revealed"
        }
    }

    /// `Q{n} / 10`.
    pub fn counter_text(&self) -> String {
        question_counter_text(self.current)
    }

    /// `{mg_score}: {score}`.
    pub fn score_label(&self, lang: i64) -> String {
        score_label_text(self.score, lang)
    }

    /// NEXT button label: `mg_show_result` on the last question (`index + 1 >= 10`), else
    /// `mg_next`.
    pub fn next_button_label(&self, lang: i64) -> &'static str {
        if self.current + 1 >= TOTAL_QUESTIONS {
            mg_t(MG_SHOW_RESULT_KEY, lang)
        } else {
            mg_t(MG_NEXT_KEY, lang)
        }
    }

    /// Result line `{score} / 10 正解` (localized).
    pub fn score_result(&self, lang: i64) -> String {
        score_result_text(self.score, lang)
    }

    /// Result comment of the current score (localized).
    pub fn result_comment(&self, lang: i64) -> &'static str {
        result_comment(self.score, lang)
    }
}
