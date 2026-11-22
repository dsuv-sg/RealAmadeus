//! The V1 ChatPanel typewriter (`step()` of effect E6, `RealAmadeusApp.tsx` L1099–1199,
//! spec-app-part2 §9) as a pure state machine.
//!
//! V1 indexes `fullText` in **UTF-16 code units** (`charAt`, `.length`, `.slice`), so this engine
//! does too: an astral character (emoji) takes two steps and for one step the page ends in a lone
//! high surrogate, exactly like V1 (rendered here as U+FFFD, as the browser rendered V1's lone
//! surrogate). Backlog pages never split a pair: they end at a BMP pause character or at the end.
//!
//! The engine owns the non-rendered typewriter state (`fullText`, `typeIndex`,
//! `typewriterStartIndex`, `typewriterClearPageOnResume`, `lastLoggedPageText`, `skipTyping`,
//! `streamComplete`); the rendered values (`dialogueText`, `waitingIndicator`,
//! `currentSpokenChar`, `waitingForInput`, `chatState`) are returned in [`StepOutput`] and owned by
//! the panel. Rust state is updated immediately, which is what V1's "set, then the next timer tick
//! sees the re-rendered refs" amounted to.
//!
//! Deliberate changes for streamed replies (DECISIONS D23, spec §6.4). For non-streamed text
//! `streamComplete` is always true, so none of them applies there:
//!
//! * A skip while the stream is still open keeps typing (the next step is polled in 50 ms). Only
//!   a skip after the stream is complete goes to `waitForAdvance` (spec §6.4 item 3). V1's
//!   `streamComplete || old typeIndex >= len` either stalled (no reschedule) or dropped the rest
//!   of the stream.
//! * Backlog pages (CHAT-028): a skip while the stream is open logs nothing, because that page
//!   is still growing. It is logged once it is final, at its pause or by the end branch. Pages
//!   paused while the stream is open are held and handed out in order by
//!   [`complete_stream`](Typewriter::complete_stream). A cancelled or failed stream therefore
//!   leaves no backlog rows: [`reset`](Typewriter::reset) drops them.
//! * Redraw (CHAT-028): when the streamed text no longer starts with what was typed,
//!   [`set_full_text`](Typewriter::set_full_text) restarts the page, so stale text never stays
//!   on screen.
//! * The last page stays on screen. A pause typed at the end of the text received so far
//!   (`nextIndex == len` while the stream is open) still sets V1's clear flag
//!   (`|| !streamComplete`), but on resume the page is cleared only once more text has
//!   arrived; until then it stays and the stream is polled every 50 ms. If the stream
//!   completes without more text, the flag is dropped and the end branch keeps the page, as
//!   after a pause at the end of a non-streamed text. V1 cleared it at once, so its last `▼`
//!   would have shown an empty box (never visible in V1, which did not display streamed
//!   replies). A skip at that point has nothing to finish and is dropped. The end branch
//!   therefore logs the page only once (CHAT-028).

use amadeus_core::chat_utils::{char_delay_for_char, is_pause_character};
use amadeus_core::jscompat::js_trim;

/// `▼`, shown at every pause and at the end.
pub const PAUSE_INDICATOR: &str = "▼";
/// Delay after the clear-page branch (ms).
pub const CLEAR_PAGE_DELAY_MS: f64 = 10.0;
/// Poll interval while the text is fully typed but the stream is incomplete (ms).
pub const STREAM_POLL_MS: f64 = 50.0;

/// What one [`Typewriter::step`] asks the panel to do (`None` = that setter is not called).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StepOutput {
    /// `setDialogueText(…)`.
    pub dialogue: Option<String>,
    /// `setCurrentSpokenChar(…)` (the last call of the step wins, like a React batch).
    pub spoken_char: Option<String>,
    /// `setWaitingIndicator(…)`.
    pub indicator: Option<&'static str>,
    /// `setWaitingForInput(…)`.
    pub waiting_for_input: Option<bool>,
    /// `onBackLogAdd("assistant", …)`.
    pub backlog: Option<String>,
    /// `setChatState("waitForAdvance")`.
    pub wait_for_advance: bool,
    /// `typingTimeoutRef.current = window.setTimeout(step, ms)`.
    pub schedule_ms: Option<f64>,
}

/// Non-rendered typewriter state (see the module docs).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Typewriter {
    text: Vec<u16>,
    type_index: usize,
    start_index: usize,
    clear_page_on_resume: bool,
    last_logged_page: String,
    skip: bool,
    stream_complete: bool,
    /// Pages paused while the stream was open, with their start index (D23, CHAT-028).
    held_pages: Vec<(usize, String)>,
}

/// One UTF-16 unit as a `char` for the pause/delay rules. A surrogate becomes U+FFFD, which —
/// like V1's lone surrogate — is no pause character, no closer and has the base delay.
fn unit_char(unit: u16) -> char {
    char::from_u32(u32::from(unit)).unwrap_or(char::REPLACEMENT_CHARACTER)
}

/// V1 `isPauseCharacter(char, nextChar)` where `""` (`None`) is the end of the text, or — for
/// `char` — the empty string `charAt` returns past the end (never a pause).
fn is_pause(unit: Option<u16>, next: Option<u16>) -> bool {
    unit.is_some_and(|u| is_pause_character(unit_char(u), next.map(unit_char)))
}

/// V1 `charDelayForChar(char, textSpeed)`; `""` gets the base delay.
fn delay_for(unit: Option<u16>, text_speed: f64) -> f64 {
    char_delay_for_char(unit.map_or('\0', unit_char), text_speed)
}

/// `[" ", "\n", "\r", "\t"].includes(c)` (skipped at the start of a page).
fn is_page_start_whitespace(unit: u16) -> bool {
    matches!(unit, 0x20 | 0x0A | 0x0D | 0x09)
}

impl Typewriter {
    /// Empty typewriter (V1 initial state).
    pub fn new() -> Self {
        Self::default()
    }

    /// The engine part of V1's "typewriter reset block" (`handleApiSuccess`, `handleApiError`,
    /// the startup greeting, `/dev emotion help`): `fullText = text`, `typeIndex = 0`,
    /// `typewriterStartIndex = 0`, `skipTyping = false`, `typewriterClearPageOnResume = false`,
    /// `lastLoggedPageText = ""`, `streamComplete = stream_complete`. Pages held for the backlog
    /// of an interrupted stream are dropped (D23).
    pub fn reset(&mut self, text: &str, stream_complete: bool) {
        self.text = text.encode_utf16().collect();
        self.type_index = 0;
        self.start_index = 0;
        self.skip = false;
        self.clear_page_on_resume = false;
        self.last_logged_page.clear();
        self.stream_complete = stream_complete;
        self.held_pages.clear();
    }

    /// `setFullText(text)` alone (streaming tokens and completion).
    ///
    /// D23 safety net (CHAT-028): if `text` no longer starts with the part already typed, the
    /// typing restarts at the current page start, or at 0 when the text before the page changed
    /// too. The pending clear-page flag and the logged-page memory are reset and the pages held
    /// from that point on are dropped, so the page is redrawn and logged from the new text.
    /// Returns `true` when that happened. The panel then clears the stale page and its pause.
    pub fn set_full_text(&mut self, text: &str) -> bool {
        let new: Vec<u16> = text.encode_utf16().collect();
        let keeps = |upto: usize| {
            let upto = upto.min(self.text.len());
            new.get(..upto) == Some(&self.text[..upto])
        };
        let restart = if keeps(self.type_index) {
            None
        } else if keeps(self.start_index) && self.start_index <= self.text.len() {
            Some(self.start_index)
        } else {
            Some(0)
        };
        self.text = new;
        let Some(point) = restart else {
            return false;
        };
        self.start_index = point;
        self.type_index = point;
        self.clear_page_on_resume = false;
        self.last_logged_page.clear();
        self.held_pages.retain(|(start, _)| *start < point);
        true
    }

    /// `setStreamComplete(true)` at the end of a stream. Returns the pages held while it was
    /// open, in order, for the backlog (D23, CHAT-028).
    pub fn complete_stream(&mut self) -> Vec<String> {
        self.stream_complete = true;
        std::mem::take(&mut self.held_pages)
            .into_iter()
            .map(|(_, page)| page)
            .collect()
    }

    /// `onBackLogAdd("assistant", page)` + `lastLoggedPageText = page`; held while the stream is
    /// open (D23).
    fn log_page(&mut self, out: &mut StepOutput, page: String) {
        if self.stream_complete {
            out.backlog = Some(page.clone());
        } else {
            self.held_pages.push((self.start_index, page.clone()));
        }
        self.last_logged_page = page;
    }

    /// `setSkipTyping(true)` (`onAdvance` while typing and not paused).
    pub fn request_skip(&mut self) {
        self.skip = true;
    }

    /// `typeIndex` (UTF-16 units; may exceed [`len`](Self::len) by one, like V1, when a page
    /// consisted only of whitespace).
    pub fn type_index(&self) -> usize {
        self.type_index
    }

    /// `fullText.length` (UTF-16 units).
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// `fullText.length === 0`.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// `fullText`.
    pub fn full_text(&self) -> String {
        String::from_utf16_lossy(&self.text)
    }

    /// `fullText.slice(from, to)` (clamped like JS; an empty string when `from >= to`).
    fn slice(&self, from: usize, to: usize) -> String {
        let len = self.text.len();
        let end = to.min(len);
        let start = from.min(end);
        String::from_utf16_lossy(&self.text[start..end])
    }

    /// V1 `step()`. `menu_open` / `waiting_for_input` are the panel's current
    /// `menuPanelOpen` / `waitingForInput`; `text_speed` reads `Config_TextSpeed` (V1 read it for
    /// every typed character, so it is only called when a character delay is computed).
    pub fn step(
        &mut self,
        menu_open: bool,
        waiting_for_input: bool,
        text_speed: impl FnOnce() -> f64,
    ) -> StepOutput {
        let mut out = StepOutput::default();
        if menu_open || waiting_for_input {
            return out;
        }

        if self.clear_page_on_resume {
            if self.type_index < self.text.len() {
                self.start_index = self.type_index;
                out.dialogue = Some(String::new());
                self.last_logged_page.clear();
                self.clear_page_on_resume = false;
                out.indicator = Some("");
                out.schedule_ms = Some(CLEAR_PAGE_DELAY_MS);
                return out;
            }
            // D23: nothing follows the pause yet. It was typed at the end of the text received
            // while the stream was open (for non-streamed text the flag means `nextIndex < len`,
            // so this is never reached). A skip has nothing to finish here.
            self.skip = false;
            if !self.stream_complete {
                // The page stays on screen until more text arrives, which clears it above.
                out.schedule_ms = Some(STREAM_POLL_MS);
                return out;
            }
            // The stream completed without more text: that page was the last one, and the end
            // branch keeps it, like after a pause at the end of a non-streamed text.
            self.clear_page_on_resume = false;
        }

        let len = self.text.len();
        if self.skip {
            let finished = self.slice(self.start_index, len);
            out.dialogue = Some(finished.clone());
            self.type_index = len;
            self.skip = false;
            out.waiting_for_input = Some(false);
            if self.stream_complete {
                if !js_trim(&finished).is_empty() && finished != self.last_logged_page {
                    out.backlog = Some(finished.clone());
                    self.last_logged_page = finished;
                }
                out.wait_for_advance = true;
                out.indicator = Some(PAUSE_INDICATOR);
                out.spoken_char = Some(String::new());
            } else {
                // D23: the stream is still open — keep typing what arrives next. The page is
                // still growing, so it is logged later, once final (CHAT-028).
                out.schedule_ms = Some(STREAM_POLL_MS);
            }
            return out;
        }

        let current = self.type_index;
        if current < len {
            let mut next = current;
            if current == self.start_index {
                while next < len && is_page_start_whitespace(self.text[next]) {
                    next += 1;
                }
            }
            let unit = self.text.get(next).copied();
            out.spoken_char = Some(
                unit.map(|u| String::from_utf16_lossy(&[u]))
                    .unwrap_or_default(),
            );
            next += 1;
            let page = self.slice(self.start_index, next);
            self.type_index = next;
            out.dialogue = Some(page.clone());

            let next_unit = self.text.get(next).copied();
            if is_pause(unit, next_unit) {
                out.waiting_for_input = Some(true);
                out.spoken_char = Some(String::new());
                self.clear_page_on_resume = next < len || !self.stream_complete;
                if !js_trim(&page).is_empty() {
                    self.log_page(&mut out, page);
                }
                out.indicator = Some(PAUSE_INDICATOR);
                return out;
            }
            out.schedule_ms = Some(delay_for(unit, text_speed()));
        } else if !self.stream_complete {
            out.schedule_ms = Some(STREAM_POLL_MS);
        } else {
            out.spoken_char = Some(String::new());
            let full = self.slice(0, len);
            if self.last_logged_page != full && !js_trim(&full).is_empty() {
                out.backlog = Some(full.clone());
                self.last_logged_page = full;
            }
            out.wait_for_advance = true;
            out.indicator = Some(PAUSE_INDICATOR);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simulates the panel around the engine: the E6 effect (step immediately on (re)start), the
    /// timer chain, `onAdvance` and the rendered fields.
    #[derive(Default)]
    struct Sim {
        tw: Typewriter,
        typing: bool,
        waiting: bool,
        dialogue: String,
        indicator: &'static str,
        spoken: String,
        backlog: Vec<String>,
        pending: Option<f64>,
        delays: Vec<f64>,
        speed: f64,
        /// Every `▼` pause: the page start, the page, and whether the stream was still open.
        pauses: Vec<(usize, String, bool)>,
        /// The page start and the dialogue at the final `▼` (`waitForAdvance`).
        last: Option<(usize, String)>,
    }

    impl Sim {
        fn show(text: &str) -> Self {
            let mut sim = Sim {
                speed: 1.0,
                ..Sim::default()
            };
            sim.tw.reset(text, true);
            sim.typing = true;
            sim.effect();
            sim
        }

        fn apply(&mut self, out: StepOutput) {
            if let Some(d) = out.dialogue {
                self.dialogue = d;
            }
            if let Some(c) = out.spoken_char {
                self.spoken = c;
            }
            if let Some(i) = out.indicator {
                self.indicator = i;
            }
            let mut rerun = false;
            if let Some(w) = out.waiting_for_input {
                rerun |= w != self.waiting;
                self.waiting = w;
                if w {
                    let open = !self.tw.stream_complete;
                    let page = (self.tw.start_index, self.dialogue.clone(), open);
                    self.pauses.push(page);
                }
            }
            if let Some(b) = out.backlog {
                self.backlog.push(b);
            }
            if out.wait_for_advance {
                self.typing = false;
                self.last = Some((self.tw.start_index, self.dialogue.clone()));
                rerun = true;
            }
            self.pending = out.schedule_ms;
            if let Some(ms) = out.schedule_ms {
                self.delays.push(ms);
            }
            if rerun {
                self.effect();
            }
        }

        /// E6: clear the timer, then step right away when typing, not paused.
        fn effect(&mut self) {
            self.pending = None;
            if self.typing && !self.waiting {
                let speed = self.speed;
                let out = self.tw.step(false, self.waiting, || speed);
                self.apply(out);
            }
        }

        /// Fires the pending timer.
        fn tick(&mut self) -> bool {
            if self.pending.take().is_none() {
                return false;
            }
            let speed = self.speed;
            let out = self.tw.step(false, self.waiting, || speed);
            self.apply(out);
            true
        }

        /// Runs timers until the typewriter pauses or stops.
        fn run(&mut self) {
            while self.tick() {}
        }

        /// V1 `onAdvance` while typing (resume or skip). Returns false when not typing.
        fn advance(&mut self) -> bool {
            if !self.typing {
                return false;
            }
            if self.waiting {
                self.waiting = false;
                self.effect();
            } else {
                self.tw.request_skip();
            }
            true
        }
    }

    #[test]
    fn example_a_two_sentences_double_triangle_and_full_text_log() {
        let mut sim = Sim::show("そうね。あなたは？");
        // The effect typed `そ` synchronously; the rest follows the 50 ms chain.
        sim.run();
        assert_eq!(sim.dialogue, "そうね。");
        assert!(sim.waiting && sim.typing);
        assert_eq!(sim.indicator, PAUSE_INDICATOR);
        assert_eq!(sim.spoken, "");
        assert_eq!(sim.delays, vec![50.0, 50.0, 50.0]);
        assert_eq!(sim.backlog, vec!["そうね。"]);
        // Click: clear page (box empties, ▼ hidden), 10 ms later typing continues.
        assert!(sim.advance());
        assert_eq!(sim.dialogue, "");
        assert_eq!(sim.indicator, "");
        assert_eq!(sim.pending, Some(CLEAR_PAGE_DELAY_MS));
        sim.run();
        assert_eq!(sim.dialogue, "あなたは？");
        assert!(sim.waiting);
        assert_eq!(sim.backlog, vec!["そうね。", "あなたは？"]);
        // Click: nothing to clear → end branch logs the full text again → waitForAdvance ▼.
        assert!(sim.advance());
        assert!(!sim.typing);
        assert_eq!(sim.dialogue, "あなたは？");
        assert_eq!(sim.indicator, PAUSE_INDICATOR);
        assert_eq!(
            sim.backlog,
            vec!["そうね。", "あなたは？", "そうね。あなたは？"]
        );
    }

    #[test]
    fn example_b_no_terminal_punctuation() {
        let mut sim = Sim::show("了解");
        sim.run();
        assert!(!sim.typing, "end branch reached without a pause");
        assert_eq!(sim.dialogue, "了解");
        assert_eq!(sim.indicator, PAUSE_INDICATOR);
        assert_eq!(sim.backlog, vec!["了解"]);
        assert_eq!(sim.delays, vec![50.0, 50.0]);
    }

    #[test]
    fn example_c_leading_space_on_second_page() {
        let mut sim = Sim::show("Hi! How are you?");
        sim.run();
        assert_eq!(sim.dialogue, "Hi!");
        sim.advance();
        sim.run();
        assert_eq!(
            sim.dialogue, " How are you?",
            "page is sliced from the start index"
        );
        sim.advance();
        assert_eq!(
            sim.backlog,
            vec!["Hi!", " How are you?", "Hi! How are you?"]
        );
    }

    #[test]
    fn example_d_blank_lines_kept_in_page() {
        let mut sim = Sim::show("A。\n\nB");
        sim.run();
        assert_eq!(sim.dialogue, "A。");
        sim.advance();
        sim.run();
        assert_eq!(sim.dialogue, "\n\nB");
        assert!(!sim.typing);
        assert_eq!(sim.backlog, vec!["A。", "A。\n\nB"]);
    }

    #[test]
    fn example_e_closing_bracket_and_delays() {
        let mut sim = Sim::show("「そうよ。」って言ったの。");
        sim.run();
        assert_eq!(
            sim.dialogue, "「そうよ。」って言ったの。",
            "。 before 」 does not pause"
        );
        assert!(sim.waiting);
        // 「 そ う よ 。(50 each) 」(75) っ て 言 っ た の (50 each), final 。 pauses.
        assert_eq!(
            sim.delays,
            vec![50.0, 50.0, 50.0, 50.0, 50.0, 75.0, 50.0, 50.0, 50.0, 50.0, 50.0, 50.0]
        );
        sim.advance();
        assert!(
            !sim.typing,
            "double ▼: the click after the last pause reaches the end branch"
        );
        assert_eq!(
            sim.backlog.len(),
            1,
            "same text: the end branch does not log it twice"
        );
    }

    #[test]
    fn example_f_ellipsis_delay() {
        let mut sim = Sim::show("えっと……そうね");
        sim.run();
        assert_eq!(
            sim.delays,
            vec![50.0, 50.0, 50.0, 125.0, 125.0, 50.0, 50.0, 50.0]
        );
        assert_eq!(sim.backlog, vec!["えっと……そうね"]);
    }

    #[test]
    fn example_g_english_periods() {
        let mut sim = Sim::show("e.g. this");
        sim.run();
        assert_eq!(sim.dialogue, "e.g.");
        let mut sim = Sim::show("3.14");
        sim.run();
        assert!(!sim.typing);
        assert_eq!(sim.backlog, vec!["3.14"]);
        let mut sim = Sim::show("Wait...");
        sim.run();
        assert_eq!(sim.dialogue, "Wait...", "only the last . pauses");
        assert!(sim.waiting);
    }

    #[test]
    fn example_h_skip() {
        let mut sim = Sim::show("そうね。あなたは？");
        sim.tick(); // `う`
        assert_eq!(sim.dialogue, "そう");
        assert!(sim.advance()); // skipTyping = true
        sim.tick();
        assert_eq!(sim.dialogue, "そうね。あなたは？");
        assert!(!sim.typing);
        assert!(!sim.waiting);
        assert_eq!(sim.indicator, PAUSE_INDICATOR);
        assert_eq!(sim.backlog, vec!["そうね。あなたは？"]);
        assert_eq!(
            sim.pending, None,
            "a skip of complete text never reschedules"
        );

        // Skipping during page 2 logs only that page (no end-branch duplicate).
        let mut sim = Sim::show("そうね。あなたは？");
        sim.run();
        sim.advance();
        sim.run_until_chars(2);
        assert!(sim.advance());
        sim.tick();
        assert_eq!(sim.dialogue, "あなたは？");
        assert!(!sim.typing);
        assert_eq!(sim.backlog, vec!["そうね。", "あなたは？"]);
    }

    impl Sim {
        /// Fires timers until the page has `n` characters.
        fn run_until_chars(&mut self, n: usize) {
            while self.dialogue.chars().count() < n && self.tick() {}
        }

        /// A stream that started typing with `text` (the panel switched to typing).
        fn streaming(text: &str) -> Self {
            let mut sim = Sim {
                speed: 1.0,
                ..Sim::default()
            };
            sim.tw.reset("", false);
            sim.tw.set_full_text(text);
            sim.typing = true;
            sim.effect();
            sim
        }

        /// `Chat::set_full_text`: a restart also clears the stale page and its pause.
        fn set_text(&mut self, text: &str) -> bool {
            let restarted = self.tw.set_full_text(text);
            if restarted {
                self.dialogue.clear();
                self.indicator = "";
                if self.waiting {
                    self.waiting = false;
                    self.effect();
                }
            }
            restarted
        }

        /// Like [`run`](Self::run) for an open stream: stops at a pause or at the first 50 ms
        /// poll (caught up with the text received so far).
        fn run_open(&mut self) {
            while self.tick() {
                if self.tw.type_index() >= self.tw.len() {
                    break;
                }
            }
        }

        /// `Chat::on_stream_complete`: the held pages go to the backlog.
        fn complete(&mut self, text: &str) {
            self.set_text(text);
            let held = self.tw.complete_stream();
            self.backlog.extend(held);
        }
    }

    #[test]
    fn pages_paused_during_a_stream_are_logged_once_at_completion() {
        let mut sim = Sim::streaming("そうね。あな");
        sim.run();
        assert_eq!(sim.dialogue, "そうね。");
        assert!(sim.waiting);
        assert!(sim.backlog.is_empty(), "held while the stream is open");
        sim.advance();
        sim.run_until_chars(2);
        assert_eq!(sim.dialogue, "あな");
        sim.set_text("そうね。あなたは？");
        sim.complete("そうね。あなたは？");
        assert_eq!(sim.backlog, vec!["そうね。"], "handed out in order");
        sim.run();
        assert_eq!(sim.dialogue, "あなたは？");
        sim.advance();
        assert!(!sim.typing);
        assert_eq!(
            sim.backlog,
            vec!["そうね。", "あなたは？", "そうね。あなたは？"],
            "afterwards exactly the rows of a non-streamed reply"
        );
        assert_eq!(
            Sim::show("そうね。あなたは？").backlog_after_reading(),
            sim.backlog
        );
    }

    impl Sim {
        /// Reads a reply to the end, pressing Enter at every ▼; returns the backlog.
        fn backlog_after_reading(mut self) -> Vec<String> {
            loop {
                self.run();
                if !self.advance() {
                    return self.backlog;
                }
            }
        }
    }

    #[test]
    fn a_cancelled_or_failed_stream_leaves_no_backlog_rows() {
        let mut sim = Sim::streaming("一。二。");
        sim.run();
        assert!(sim.waiting);
        sim.advance();
        sim.run();
        assert_eq!(sim.dialogue, "二。");
        // Cancel (or an error, which types its own message): the typewriter is reset.
        sim.tw.reset("", false);
        assert!(sim.tw.complete_stream().is_empty());
        assert!(sim.backlog.is_empty());
    }

    #[test]
    fn a_shrinking_stream_text_redraws_the_page() {
        // CHAT-028 case 1: the typed part disappears (an open <think> closed, a command was
        // removed at completion). The page restarts from its start with the new text.
        let mut sim = Sim::streaming("<think>ここは内部");
        sim.run_until_chars(5);
        assert_eq!(sim.dialogue, "<thin");
        assert!(sim.set_text("本文です。"));
        assert_eq!(sim.dialogue, "", "the stale page is cleared at once");
        assert_eq!(sim.tw.type_index(), 0);
        sim.complete("本文です。");
        sim.run();
        assert_eq!(sim.dialogue, "本文です。");
        sim.advance();
        assert!(!sim.typing);
        assert_eq!(sim.backlog, vec!["本文です。"]);

        // Only the current page restarts when the text before it is unchanged, also while
        // paused at a stale ▼ (the held stale page is dropped).
        let mut sim = Sim::streaming("そうね。まず、[SAVE_FACT: x");
        sim.run();
        assert_eq!(sim.dialogue, "そうね。");
        sim.advance();
        sim.run_open();
        assert_eq!(sim.dialogue, "まず、[SAVE_FACT: x");
        sim.set_text("そうね。まず、[SAVE_FACT: x]。");
        sim.run();
        assert!(sim.waiting);
        assert_eq!(sim.dialogue, "まず、[SAVE_FACT: x]。");
        assert!(sim.set_text("そうね。まず、本文。"));
        assert!(!sim.waiting, "the stale ▼ is left");
        sim.complete("そうね。まず、本文。");
        sim.run();
        assert_eq!(sim.dialogue, "まず、本文。");
        sim.advance();
        assert!(!sim.typing);
        assert_eq!(
            sim.backlog,
            vec!["そうね。", "まず、本文。", "そうね。まず、本文。"]
        );

        // Text before the page changed too: the whole reply is typed again from the start.
        let mut sim = Sim::streaming("A。B");
        sim.run();
        sim.advance();
        sim.run_open();
        assert_eq!(sim.dialogue, "B");
        assert!(sim.set_text("X。B"));
        assert_eq!(sim.tw.type_index(), 0);
        sim.complete("X。B");
        sim.run();
        assert_eq!(sim.dialogue, "X。");

        // Growing text never restarts.
        let mut sim = Sim::streaming("あい");
        sim.run_open();
        assert!(!sim.set_text("あいう"));
        assert!(!sim.set_text("あいう"));
    }

    /// When the reader presses Enter at a `▼` while the reply is still streaming.
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Enter {
        /// As soon as the `▼` shows.
        Early,
        /// Once the `▼` has been shown for more than this many typewriter ticks.
        Late(usize),
        /// Not before the stream has completed.
        AfterCompletion,
    }

    /// A streamed reply read to its end ([`read_stream`]).
    #[derive(Debug)]
    struct Reading {
        /// The text the completion displays.
        text: String,
        /// [`Sim::pauses`].
        pauses: Vec<(usize, String, bool)>,
        /// [`Sim::last`].
        last: (usize, String),
        backlog: Vec<String>,
        /// Every text the dialogue box showed, in order.
        shown: Vec<String>,
    }

    impl Sim {
        fn record(&self, shown: &mut Vec<String>) {
            if shown.last() != Some(&self.dialogue) {
                shown.push(self.dialogue.clone());
            }
        }
    }

    /// Streams `reply` like the controller: `chunk`-character tokens (the audit mock sends 4),
    /// the D23 display, typing once the leading tag is decided and there is text. Between two
    /// tokens the typewriter gets `ticks` steps of 50 ms; at a `▼` Enter is pressed as `enter`
    /// says and, with `skip_every`, Enter while typing (a skip) on every `skip_every`-th tick.
    /// After the completion (`Chat::on_stream_complete`) the reply is read to its end with
    /// Enter at every `▼`.
    fn read_stream(
        reply: &str,
        chunk: usize,
        ticks: usize,
        enter: Enter,
        skip_every: Option<usize>,
    ) -> Reading {
        use amadeus_core::chat_utils::{
            extract_memory_commands, stream_display_prefix, strip_emotion_tag,
            StreamEmotionDetector, EMPTY_REPLY_TEXT,
        };
        let mut sim = Sim {
            speed: 1.0,
            ..Sim::default()
        };
        sim.tw.reset("", false);
        let mut detector = StreamEmotionDetector::new();
        let mut raw = String::new();
        let mut shown = Vec::new();
        let (mut paused_for, mut tick) = (0, 0);
        let chars: Vec<char> = reply.chars().collect();
        for token in chars.chunks(chunk.max(1)) {
            let token: String = token.iter().collect();
            detector.push_token(&token);
            raw.push_str(&token);
            let display = stream_display_prefix(&raw);
            sim.set_text(&display);
            if detector.is_parsed() && !display.is_empty() && !sim.typing {
                sim.typing = true;
                sim.effect();
            }
            sim.record(&mut shown);
            for _ in 0..ticks {
                tick += 1;
                if sim.waiting {
                    paused_for += 1;
                    let press = match enter {
                        Enter::Early => true,
                        Enter::Late(n) => paused_for > n,
                        Enter::AfterCompletion => false,
                    };
                    if press {
                        paused_for = 0;
                        sim.advance();
                    }
                } else if sim.typing && skip_every.is_some_and(|every| tick % every == 0) {
                    sim.advance();
                } else {
                    sim.tick();
                }
                sim.record(&mut shown);
            }
        }
        let cleaned = extract_memory_commands(&detector.completion_raw(reply)).cleaned_text;
        let mut text = strip_emotion_tag(&cleaned);
        if text.is_empty() {
            text = EMPTY_REPLY_TEXT.to_owned();
        }
        sim.complete(&text);
        if !sim.typing {
            sim.typing = true;
            sim.effect();
        }
        sim.record(&mut shown);
        loop {
            sim.run();
            sim.record(&mut shown);
            if !sim.advance() {
                break;
            }
            sim.record(&mut shown);
        }
        Reading {
            text,
            pauses: sim.pauses,
            last: sim.last.expect("the reply ends at a ▼"),
            backlog: sim.backlog,
            shown,
        }
    }

    /// Checks the pages of a reading: each one starts where the previous one ended, and the
    /// final `▼` shows the rest of the text, the last page, never an empty box.
    fn check_pages(reading: &Reading, case: &str) {
        let text: Vec<u16> = reading.text.encode_utf16().collect();
        let mut next_start = 0;
        for (start, page, _) in &reading.pauses {
            assert_eq!(*start, next_start, "{case}: {reading:#?}");
            next_start = start + page.encode_utf16().count();
        }
        let (start, last) = &reading.last;
        let on_last_pause = reading.pauses.last().is_some_and(|(s, _, _)| s == start);
        assert!(
            on_last_pause || *start == next_start,
            "{case}: {reading:#?}"
        );
        let rest = String::from_utf16_lossy(&text[(*start).min(text.len())..]);
        assert_eq!(*last, rest, "{case}: {reading:#?}");
        assert!(
            !js_trim(last).is_empty(),
            "{case}: the last ▼ shows an empty box: {reading:#?}"
        );
    }

    /// V1's backlog rows for the pages of a reading without skips: every pause page that is
    /// not blank, then the whole text, which the end branch logs unless the last page was
    /// logged at its pause with exactly that text.
    fn expected_backlog(reading: &Reading) -> Vec<String> {
        let mut rows: Vec<String> = reading
            .pauses
            .iter()
            .map(|(_, page, _)| page.clone())
            .filter(|page| !js_trim(page).is_empty())
            .collect();
        let logged_last = reading
            .pauses
            .last()
            .filter(|(start, _, _)| *start == reading.last.0)
            .map(|(_, page, _)| page);
        if logged_last != Some(&reading.text) {
            rows.push(reading.text.clone());
        }
        rows
    }

    /// The audit's streamed replies (`llm_mock.py`).
    const MEMO: &str = "[SMILE] 覚えておくわ。[SAVE_FACT: ユーザーの名前は岡部]\n\n\n\
                        [SAVE_EVENT: {\"summary\": \"ラボで実験した\", \"tags\": [\"ラボ\", \"実験\"]}]";
    const THINKAFTER: &str =
        "[SMILE] <think>ここは内部の思考メモで表示されてはいけない</think>本文です。";
    const FRONTA: &str = "[SMILE] 「本当！」と言ったの。";
    const FRONTB: &str = "[SMILE] Wait... 3.14 ok.";

    #[test]
    fn streamed_replies_never_show_commands_or_think_text() {
        let read = |reply| read_stream(reply, 4, 3, Enter::Early, None);
        let memo = read(MEMO);
        assert!(
            memo.shown
                .iter()
                .all(|s| "覚えておくわ。".starts_with(s.as_str())),
            "{memo:?}"
        );
        assert_eq!(memo.backlog, vec!["覚えておくわ。"]);

        let think = read(THINKAFTER);
        assert!(
            think
                .shown
                .iter()
                .all(|s| "本文です。".starts_with(s.as_str())),
            "{think:?}"
        );
        assert!(
            think.shown.iter().any(|s| s == "本文です。"),
            "the real reply is shown"
        );
        assert_eq!(think.backlog, vec!["本文です。"]);

        let tag_mid = read("前置き [WINK] 後ろ");
        assert!(
            tag_mid
                .shown
                .iter()
                .all(|s| "前置き  後ろ".starts_with(s.as_str())),
            "{tag_mid:?}"
        );
        assert_eq!(tag_mid.backlog, vec!["前置き  後ろ"]);

        let spaced = "[ PANIC ] えっ、ちょっと";
        let reading = read(spaced);
        assert!(
            reading.shown.iter().all(|s| spaced.starts_with(s.as_str())),
            "{reading:?}"
        );
        assert_eq!(reading.last.1, spaced, "no jump at completion");
        assert_eq!(reading.backlog, vec![spaced]);
    }

    #[test]
    fn the_last_page_of_a_streamed_reply_stays_on_screen() {
        // A stream with no typewriter step between two tokens (fast), one, three (the mock's
        // SLOW, 150 ms per token) or twelve (VERYSLOW, 600 ms). Enter at a ▼ at once, a bit
        // later, much later, or only after the completion.
        for ticks in [0, 1, 3, 12] {
            for enter in [
                Enter::Early,
                Enter::Late(1),
                Enter::Late(8),
                Enter::AfterCompletion,
            ] {
                let case = format!("{ticks} ticks per token, Enter {enter:?}");
                let read = |reply| read_stream(reply, 4, ticks, enter, None);
                let memo = read(MEMO);
                assert_eq!(memo.last.1, "覚えておくわ。", "MEMO, {case}: {memo:#?}");
                assert_eq!(memo.backlog, ["覚えておくわ。"], "MEMO, {case}");
                let think = read(THINKAFTER);
                assert_eq!(think.last.1, "本文です。", "THINKAFTER, {case}: {think:#?}");
                assert_eq!(think.backlog, ["本文です。"], "THINKAFTER, {case}");
                // `」` arrives in the token after `！` (`「本当！` / `」と言っ`). When `！` was
                // typed before it (three steps after typing started), the page ends there (the
                // kept frontier split), and the last page is the rest.
                let front_a = read(FRONTA);
                let split = front_a.pauses.len() == 2;
                let expected = if split {
                    "」と言ったの。"
                } else {
                    "「本当！」と言ったの。"
                };
                assert_eq!(front_a.last.1, expected, "FRONTA, {case}: {front_a:#?}");
                assert_eq!(split, ticks >= 3, "FRONTA, {case}: {front_a:#?}");
                let front_b = read(FRONTB);
                assert_eq!(front_b.last.1, " 3.14 ok.", "FRONTB, {case}: {front_b:#?}");
                assert_eq!(
                    front_b.backlog,
                    ["Wait...", " 3.14 ok.", "Wait... 3.14 ok."],
                    "FRONTB, {case}"
                );
                for reading in [&memo, &think, &front_a, &front_b] {
                    check_pages(reading, &case);
                    assert_eq!(reading.backlog, expected_backlog(reading), "{case}");
                }
            }
        }
    }

    /// A small deterministic generator for the simulations.
    struct XorShift(u64);

    impl XorShift {
        fn below(&mut self, n: usize) -> usize {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            (x % n as u64) as usize
        }

        fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
            items[self.below(items.len())]
        }
    }

    #[test]
    fn no_streamed_reply_ends_on_an_empty_box() {
        let leads = [
            "[SMILE] ",
            "",
            "[ PANIC ] ",
            "<think>メモ</think>[SAD] ",
            "[smug]",
        ];
        let words = [
            "そうね",
            "本当",
            "Wait",
            "3.14",
            "ok",
            "あなたは",
            "😀",
            "「えっ",
        ];
        let stops = [
            "。", "！", "？", "!", "?", ".", "……", "\n", "」", "）", "、", " ", "。」",
        ];
        let inserts = ["[WINK]", "<think>x</think>", "[SAVE_FACT: 名前は岡部]", " "];
        let tails = [
            "",
            "[SAVE_FACT: 好物はプリン]",
            "\n\n\n[SAVE_EVENT: {\"summary\": \"ラボ\", \"tags\": [\"ラボ\"]}]",
            "<think>後で考える</think>",
            " [WINK]",
            "[SAVE_FACT: x",
        ];
        let mut rng = XorShift(0x9E37_79B9_7F4A_7C15);
        let mut open_last_pauses = 0;
        for case in 0..4000 {
            let mut reply = rng.pick(&leads).to_owned();
            for _ in 0..1 + rng.below(4) {
                reply.push_str(rng.pick(&words));
                reply.push_str(rng.pick(&stops));
                if rng.below(4) == 0 {
                    reply.push_str(rng.pick(&inserts));
                }
            }
            reply.push_str(rng.pick(&tails));
            let chunk = 1 + rng.below(8);
            let ticks = rng.below(14);
            let enter = match rng.below(3) {
                0 => Enter::Early,
                1 => Enter::Late(rng.below(8)),
                _ => Enter::AfterCompletion,
            };
            let skip_every = (rng.below(4) == 0).then(|| 2 + rng.below(9));
            let reading = read_stream(&reply, chunk, ticks, enter, skip_every);
            let context = format!(
                "case {case}: {reply:?} in {chunk}-char tokens, {ticks} ticks, Enter {enter:?}, \
                 skip every {skip_every:?}"
            );
            check_pages(&reading, &context);
            // Nothing outside the final text is ever shown (a lone high surrogate is a
            // half-typed emoji).
            for shown in &reading.shown {
                let shown = shown.trim_end_matches('\u{FFFD}');
                assert!(reading.text.contains(shown), "{context}: {reading:#?}");
            }
            if skip_every.is_none() {
                assert_eq!(reading.backlog, expected_backlog(&reading), "{context}");
            }
            if reading
                .pauses
                .last()
                .is_some_and(|(start, _, open)| *open && *start == reading.last.0)
            {
                open_last_pauses += 1;
            }
        }
        // The case that used to show an empty box (896 of these 4000 readings before the fix):
        // the last page paused while the stream was still open.
        assert!(open_last_pauses > 800, "{open_last_pauses}");
    }

    /// One non-streamed reply read twice (`read_non_streamed`): the text at every `▼` and the
    /// backlog rows with Enter at every `▼`, then the same with a skip at the second character
    /// of every page.
    type Golden<'a> = (
        &'a str,
        &'a [&'a str],
        &'a [&'a str],
        &'a [&'a str],
        &'a [&'a str],
    );

    /// Recorded with the engine before the last-page hold of streamed replies (D23).
    const NON_STREAMED: [Golden<'static>; 20] = [
        (
            "そうね。あなたは？",
            &["そうね。", "あなたは？", "あなたは？"],
            &["そうね。", "あなたは？", "そうね。あなたは？"],
            &["そうね。あなたは？"],
            &["そうね。あなたは？"],
        ),
        ("了解", &["了解"], &["了解"], &["了解"], &["了解"]),
        (
            "Hi! How are you?",
            &["Hi!", " How are you?", " How are you?"],
            &["Hi!", " How are you?", "Hi! How are you?"],
            &["Hi! How are you?"],
            &["Hi! How are you?"],
        ),
        (
            "A。\n\nB",
            &["A。", "\n\nB"],
            &["A。", "A。\n\nB"],
            &["A。", "\n\nB"],
            &["A。", "\n\nB"],
        ),
        (
            "「そうよ。」って言ったの。",
            &["「そうよ。」って言ったの。", "「そうよ。」って言ったの。"],
            &["「そうよ。」って言ったの。"],
            &["「そうよ。」って言ったの。"],
            &["「そうよ。」って言ったの。"],
        ),
        (
            "えっと……そうね",
            &["えっと……そうね"],
            &["えっと……そうね"],
            &["えっと……そうね"],
            &["えっと……そうね"],
        ),
        (
            "e.g. this 3.14 Wait...",
            &["e.g.", " this 3.14 Wait...", " this 3.14 Wait..."],
            &["e.g.", " this 3.14 Wait...", "e.g. this 3.14 Wait..."],
            &["e.g. this 3.14 Wait..."],
            &["e.g. this 3.14 Wait..."],
        ),
        (
            "A\nB",
            &["A\n", "B"],
            &["A\n", "A\nB"],
            &["A\n", "B"],
            &["A\n", "A\nB"],
        ),
        (
            "「本当！」と言ったの。",
            &["「本当！」と言ったの。", "「本当！」と言ったの。"],
            &["「本当！」と言ったの。"],
            &["「本当！」と言ったの。"],
            &["「本当！」と言ったの。"],
        ),
        (
            "Wait... 3.14 ok.",
            &["Wait...", " 3.14 ok.", " 3.14 ok."],
            &["Wait...", " 3.14 ok.", "Wait... 3.14 ok."],
            &["Wait... 3.14 ok."],
            &["Wait... 3.14 ok."],
        ),
        (
            "覚えておくわ。",
            &["覚えておくわ。", "覚えておくわ。"],
            &["覚えておくわ。"],
            &["覚えておくわ。"],
            &["覚えておくわ。"],
        ),
        (
            "本文です。",
            &["本文です。", "本文です。"],
            &["本文です。"],
            &["本文です。"],
            &["本文です。"],
        ),
        (
            "うん。うん。",
            &["うん。", "うん。", "うん。"],
            &["うん。", "うん。", "うん。うん。"],
            &["うん。うん。"],
            &["うん。うん。"],
        ),
        (
            "A。 ",
            &["A。", " "],
            &["A。", "A。 "],
            &["A。", " "],
            &["A。", "A。 "],
        ),
        (
            "😀😀。終わり",
            &["😀😀。", "終わり"],
            &["😀😀。", "😀😀。終わり"],
            &["😀😀。終わり"],
            &["😀😀。終わり"],
        ),
        (
            "前置き  後ろ",
            &["前置き  後ろ"],
            &["前置き  後ろ"],
            &["前置き  後ろ"],
            &["前置き  後ろ"],
        ),
        (
            "[ PANIC ] えっ、ちょっと",
            &["[ PANIC ] えっ、ちょっと"],
            &["[ PANIC ] えっ、ちょっと"],
            &["[ PANIC ] えっ、ちょっと"],
            &["[ PANIC ] えっ、ちょっと"],
        ),
        (
            "そうね……少し長くなるけど説明するわ。まず、世界線の収束について。",
            &[
                "そうね……少し長くなるけど説明するわ。",
                "まず、世界線の収束について。",
                "まず、世界線の収束について。",
            ],
            &[
                "そうね……少し長くなるけど説明するわ。",
                "まず、世界線の収束について。",
                "そうね……少し長くなるけど説明するわ。まず、世界線の収束について。",
            ],
            &["そうね……少し長くなるけど説明するわ。まず、世界線の収束について。"],
            &["そうね……少し長くなるけど説明するわ。まず、世界線の収束について。"],
        ),
        ("……", &["……"], &["……"], &["……"], &["……"]),
        (
            "ええ!? 本当に？\n\n「嘘でしょ。」",
            &["ええ!", "?", " 本当に？", "\n\n「嘘でしょ。」"],
            &[
                "ええ!",
                "?",
                " 本当に？",
                "ええ!? 本当に？\n\n「嘘でしょ。」",
            ],
            &["ええ!? 本当に？\n\n「嘘でしょ。」"],
            &["ええ!? 本当に？\n\n「嘘でしょ。」"],
        ),
    ];

    /// Reads a non-streamed reply to its end, Enter at every `▼` and, with `skip_at`, a skip
    /// once a page shows that many characters. Returns the text at every `▼` and the backlog.
    fn read_non_streamed(text: &str, skip_at: Option<usize>) -> (Vec<String>, Vec<String>) {
        let mut sim = Sim::show(text);
        let mut marks = Vec::new();
        loop {
            if let Some(chars) = skip_at {
                sim.run_until_chars(chars);
                if sim.typing && !sim.waiting {
                    sim.advance();
                }
            }
            sim.run();
            marks.push(sim.dialogue.clone());
            if !sim.advance() {
                return (marks, sim.backlog);
            }
        }
    }

    #[test]
    fn non_streamed_pages_are_unchanged() {
        for (text, marks, rows, skip_marks, skip_rows) in NON_STREAMED {
            let (read_marks, read_rows) = read_non_streamed(text, None);
            assert_eq!(read_marks, marks, "{text:?}");
            assert_eq!(read_rows, rows, "{text:?}");
            let (read_marks, read_rows) = read_non_streamed(text, Some(2));
            assert_eq!(read_marks, skip_marks, "{text:?} with a skip");
            assert_eq!(read_rows, skip_rows, "{text:?} with a skip");
        }
    }

    #[test]
    fn example_i_newline_pause() {
        let mut sim = Sim::show("A\nB");
        sim.run();
        assert_eq!(sim.dialogue, "A\n");
        sim.advance();
        sim.run();
        assert_eq!(sim.dialogue, "B");
        assert_eq!(sim.backlog, vec!["A\n", "A\nB"]);
    }

    #[test]
    fn help_text_pauses_after_every_line() {
        let help = amadeus_core::chat_utils::DEV_EMOTION_HELP_TEXT;
        let mut sim = Sim::show(help);
        let mut pages = 0;
        loop {
            sim.run();
            if !sim.typing {
                break;
            }
            pages += 1;
            sim.advance();
        }
        assert_eq!(
            pages, 15,
            "15 pages (the blank line is skipped at a page start)"
        );
        assert_eq!(sim.backlog.len(), 16, "15 pages + the full text at the end");
        assert_eq!(sim.backlog.last().map(String::as_str), Some(help));
    }

    #[test]
    fn whitespace_only_tail_overruns_the_index_like_v1() {
        let mut tw = Typewriter::new();
        tw.reset("A。 ", true);
        let first = tw.step(false, false, || 1.0);
        assert_eq!(first.dialogue.as_deref(), Some("A"));
        let pause = tw.step(false, false, || 1.0);
        assert_eq!(pause.waiting_for_input, Some(true));
        // Resume: clear page, then the whitespace run reaches the end: char "", index len + 1.
        let clear = tw.step(false, false, || 1.0);
        assert_eq!(clear.schedule_ms, Some(CLEAR_PAGE_DELAY_MS));
        let overrun = tw.step(false, false, || 1.0);
        assert_eq!(overrun.spoken_char.as_deref(), Some(""));
        assert_eq!(overrun.dialogue.as_deref(), Some(" "));
        assert_eq!(tw.type_index(), 4);
        assert_eq!(overrun.schedule_ms, Some(50.0));
        let end = tw.step(false, false, || 1.0);
        assert!(end.wait_for_advance);
        assert_eq!(end.backlog.as_deref(), Some("A。 "));
    }

    #[test]
    fn astral_characters_take_two_steps() {
        let mut tw = Typewriter::new();
        tw.reset("😀!", true);
        let high = tw.step(false, false, || 1.0);
        assert_eq!(high.dialogue.as_deref(), Some("\u{FFFD}"));
        assert_eq!(high.schedule_ms, Some(50.0));
        let low = tw.step(false, false, || 1.0);
        assert_eq!(low.dialogue.as_deref(), Some("😀"));
        let bang = tw.step(false, false, || 1.0);
        assert_eq!(bang.dialogue.as_deref(), Some("😀!"));
        assert_eq!(bang.waiting_for_input, Some(true));
        assert_eq!(bang.backlog.as_deref(), Some("😀!"));
    }

    #[test]
    fn text_speed_is_read_per_character_and_clamped() {
        let mut tw = Typewriter::new();
        tw.reset("あ、い", true);
        assert_eq!(tw.step(false, false, || 2.0).schedule_ms, Some(25.0));
        assert_eq!(
            tw.step(false, false, || 0.01).schedule_ms,
            Some(1250.0),
            "、 ×2.5 at the 0.1 floor"
        );
        // The last character does not pause; its delay precedes the end branch.
        assert_eq!(tw.step(false, false, || 3.0).schedule_ms, Some(50.0 / 3.0));
    }

    #[test]
    fn guard_menu_and_pause() {
        let mut tw = Typewriter::new();
        tw.reset("abc", true);
        assert_eq!(tw.step(true, false, || 1.0), StepOutput::default());
        assert_eq!(tw.step(false, true, || 1.0), StepOutput::default());
        assert_eq!(tw.type_index(), 0);
    }

    #[test]
    fn incomplete_stream_polls_and_keeps_clear_flag() {
        let mut tw = Typewriter::new();
        tw.reset("", false);
        assert_eq!(
            tw.step(false, false, || 1.0).schedule_ms,
            Some(STREAM_POLL_MS)
        );
        tw.set_full_text("はい。");
        tw.step(false, false, || 1.0);
        tw.step(false, false, || 1.0);
        let pause = tw.step(false, false, || 1.0);
        assert_eq!(pause.waiting_for_input, Some(true));
        assert_eq!(pause.backlog, None, "held while the stream is open");
        // `nextIndex < len || !streamComplete`: a pause at the current end still sets the flag.
        assert!(tw.clear_page_on_resume);
        // Resumed while nothing more has arrived: the page stays (no dialogue change) and the
        // stream is polled, the flag kept. A skip (V1: waitForAdvance, dropping the rest of the
        // stream) has nothing to finish and is dropped.
        tw.request_skip();
        for _ in 0..3 {
            let hold = tw.step(false, false, || 1.0);
            let poll = StepOutput {
                schedule_ms: Some(STREAM_POLL_MS),
                ..StepOutput::default()
            };
            assert_eq!(hold, poll);
            assert!(tw.clear_page_on_resume && !tw.skip);
        }
        // More text: V1's clear now, then the new page is typed.
        tw.set_full_text("はい。そう");
        let clear = tw.step(false, false, || 1.0);
        assert_eq!(clear.dialogue.as_deref(), Some(""));
        assert_eq!(clear.schedule_ms, Some(CLEAR_PAGE_DELAY_MS));
        assert_eq!(
            tw.step(false, false, || 1.0).dialogue.as_deref(),
            Some("そ"),
            "text arriving after the pause is typed"
        );
    }

    #[test]
    fn a_page_paused_at_the_end_of_an_open_stream_stays_and_is_logged_once() {
        // Resumed before the stream completes: the page stays while nothing follows, and the
        // end branch keeps it without logging it a second time.
        let mut sim = Sim::streaming("覚えておくわ。");
        sim.run();
        assert!(sim.waiting);
        sim.advance();
        assert_eq!(
            sim.dialogue, "覚えておくわ。",
            "nothing to clear it for yet"
        );
        assert_eq!(sim.pending, Some(STREAM_POLL_MS));
        sim.tick();
        assert_eq!(sim.dialogue, "覚えておくわ。");
        sim.complete("覚えておくわ。");
        sim.run();
        assert!(!sim.typing);
        assert_eq!(sim.indicator, PAUSE_INDICATOR);
        assert_eq!(
            sim.dialogue, "覚えておくわ。",
            "the last ▼ shows the last page"
        );
        assert_eq!(sim.backlog, vec!["覚えておくわ。"]);
        assert_eq!(
            sim.backlog,
            Sim::show("覚えておくわ。").backlog_after_reading(),
            "the rows of the same non-streamed reply"
        );

        // Still paused when the stream completes (the flag was set while it was open):
        // resuming ends the reply on the same page.
        let mut sim = Sim::streaming("はい。");
        sim.run();
        assert!(sim.waiting);
        sim.complete("はい。");
        sim.advance();
        sim.run();
        assert!(!sim.typing);
        assert_eq!(sim.dialogue, "はい。");
        assert_eq!(sim.backlog, vec!["はい。"]);

        // More text arrives while the page is held: V1's clear, a new page, then V1's
        // full-text row.
        let mut sim = Sim::streaming("はい。");
        sim.run();
        sim.advance();
        sim.tick();
        assert_eq!(sim.dialogue, "はい。");
        sim.set_text("はい。次");
        sim.tick();
        assert_eq!(sim.dialogue, "", "cleared once the next page has text");
        sim.tick();
        assert_eq!(sim.dialogue, "次");
        sim.complete("はい。次");
        sim.run();
        assert!(!sim.typing);
        assert_eq!(sim.dialogue, "次");
        assert_eq!(sim.backlog, vec!["はい。", "はい。次"]);
        assert_eq!(sim.backlog, Sim::show("はい。次").backlog_after_reading());

        // Two identical pages: the second one is still logged.
        let mut sim = Sim::streaming("うん。");
        sim.run();
        sim.advance();
        sim.tick();
        sim.set_text("うん。うん。");
        sim.run();
        assert!(sim.waiting);
        sim.complete("うん。うん。");
        sim.advance();
        sim.run();
        assert!(!sim.typing);
        assert_eq!(sim.dialogue, "うん。");
        assert_eq!(
            sim.backlog,
            Sim::show("うん。うん。").backlog_after_reading()
        );
    }

    #[test]
    fn skip_during_an_open_stream_keeps_typing_until_complete() {
        // A stream is typed as it arrives (the panel switched to typing at its first text).
        let mut sim = Sim::streaming("ねえ、聞いて");
        sim.tick();
        assert_eq!(sim.dialogue, "ねえ");
        // Skip mid-page with the old index below the length (V1: no reschedule, stalled forever).
        assert!(sim.advance());
        sim.tick();
        assert_eq!(sim.dialogue, "ねえ、聞いて");
        assert!(sim.typing, "stays in typing while the stream is open");
        assert!(!sim.waiting);
        assert_eq!(sim.pending, Some(STREAM_POLL_MS));
        assert!(
            sim.backlog.is_empty(),
            "CHAT-028 case 5: the growing page is not logged at the skip"
        );
        // Caught up: poll.
        sim.tick();
        assert_eq!(sim.pending, Some(STREAM_POLL_MS));
        // The rest arrives and the stream completes: typed on the same page up to the pause.
        sim.set_text("ねえ、聞いてる？");
        sim.complete("ねえ、聞いてる？");
        sim.run();
        assert_eq!(sim.dialogue, "ねえ、聞いてる？");
        assert!(sim.waiting && sim.typing);
        assert_eq!(sim.backlog, vec!["ねえ、聞いてる？"]);
        // Resume: the pause ended the complete text, so the end branch runs (box kept).
        sim.advance();
        assert!(!sim.typing, "waitForAdvance once complete");
        assert_eq!(sim.dialogue, "ねえ、聞いてる？");
        assert_eq!(sim.indicator, PAUSE_INDICATOR);
        assert_eq!(
            sim.backlog,
            vec!["ねえ、聞いてる？"],
            "logged once, with its final content"
        );

        // A skip while caught up logs nothing either; the end branch logs the final page.
        let mut sim = Sim::streaming("そうね……");
        sim.run_until_chars(5);
        assert!(sim.advance());
        sim.tick();
        assert!(sim.backlog.is_empty());
        sim.set_text("そうね……少し");
        sim.complete("そうね……少し");
        sim.run();
        assert!(!sim.typing);
        assert_eq!(sim.dialogue, "そうね……少し");
        assert_eq!(sim.backlog, vec!["そうね……少し"]);

        // A skip after completion still ends at once.
        let mut sim = Sim::streaming("あいうえお");
        sim.complete("あいうえお");
        assert!(sim.advance());
        sim.tick();
        assert!(!sim.typing);
        assert_eq!(sim.pending, None);
        assert_eq!(sim.dialogue, "あいうえお");
        assert_eq!(sim.backlog, vec!["あいうえお"]);
    }
}
