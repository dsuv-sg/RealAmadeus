//! ChatPanel state and behaviour: V1 `ChatPanel`'s state, refs and callbacks
//! (`RealAmadeusApp.tsx` L828–1860, spec-app-part2 §4–§8) behind one `Copy + Send + Sync`
//! handle ([`Chat`]) that view closures, effects, `Callback`s and service handlers capture.
//!
//! * Rendered state and effect dependencies are `RwSignal`s written through [`put`], which —
//!   like React's `setState` bail-out — notifies only on a real change, so the effects re-run
//!   exactly when V1's dependency arrays changed.
//! * V1's refs and the non-rendered state live in [`Inner`] (a local `StoredValue`). Its borrow
//!   is never held while calling services, callbacks or the managers: `TTSService.synthesize`,
//!   `AudioService.playFromFile` / `stop` and the STT start/stop call back synchronously.
//! * The AI / TTS handlers reproduce V1's **effective** behaviour (spec §6), except for the
//!   streaming path, where DECISIONS D23 adopts spec §6.4 Option B over V1's frozen first-render
//!   closures. A streamed reply leaves `waitingAPI` and is typed out (V1 kept it hidden forever,
//!   CHAT-028), and the stream latency is measured against the live text (CHAT-029). Typing
//!   and the TTS capture of a streamed reply both wait until its leading tag is decided. The TTS
//!   guard and tail flush live in [`super::tts_queue`] (TTS-006, TTS-003), the skip in
//!   [`super::typewriter`]. `submitMessage` also clears the typewriter, which V1 never did, so a
//!   streamed reply starts typing at its own first character instead of at the previous text's
//!   index. Nothing can observe that reset: the typewriter only runs in `typing`, and every other
//!   way into `typing` resets it too.
//! * What a streamed reply types and speaks is
//!   [`stream_display_prefix`] of the raw
//!   stream: always a prefix of the text the completion will display, so memory commands, think
//!   blocks and half-received tags are never typed or spoken (CHAT-028). V1's
//!   `stripEmotionTag(streamBuffer)` value is still computed by the detector, but only its
//!   emptiness is used, for the latency report. The typewriter redraws the page if the
//!   completion text still differs, holds the backlog pages until the stream completes, and
//!   logs a page skipped during the stream only once that page is final.
//! * While the stream is open, V1's cancel stays available in every state of the dialogue
//!   (STR-005, [`cancel_available`]). V1 could always cancel a stream, because it never left
//!   `waitingAPI`.
//! * Methods that may run after the panel is gone (timers, late service events, the bridge) use
//!   the `try_` accessors and do nothing then.

use amadeus_core::achievements::AchievementEvent;
use amadeus_core::chat_utils::{
    api_error_message_for_platform, parse_dev_command, parse_emotion_tag, stream_display_prefix,
    strip_emotion_tag, strip_think_blocks, DevCommand, StreamEmotionDetector,
    DEV_EMOTION_HELP_TEXT, EMPTY_REPLY_TEXT,
};
use amadeus_core::jscompat::js_trim;
use amadeus_core::persona::system_prompt_from_settings;
use amadeus_core::protocol::ChatMessage;
use amadeus_core::providers::{llm_status_from_settings, uses_streaming};
use amadeus_core::settings::keys;
use amadeus_core::types::LlmStatus;
use leptos::prelude::*;

use super::listener::set_js_timeout;
use super::tts_queue::{self, TtsHost, TtsQueue};
use super::typewriter::{StepOutput, Typewriter};
use crate::app::state::OverlayPanel;
use crate::hooks::app_settings;
use crate::i18n::config_language_untracked;
#[cfg(not(test))]
use crate::{app::managers, env::now_ms, services};
// Native unit tests run without a browser, where the clock and the managers' dates (`js_sys`)
// and the services (`window`) cannot run: they use the stand-ins of `tests::native`.
#[cfg(test)]
use self::tests::native::{managers, now_ms, services};

/// Idle emotion (60 s without activity in `inputReady` / `waitForAdvance`).
pub const SLEEPING: &str = "SLEEPING";
/// Default emotion.
pub const NORMAL: &str = "NORMAL";
/// Emotion shown with an API error.
pub const ANGRY: &str = "ANGRY";
/// `.chat-stt-hint` after a transcript (hard-coded Japanese in V1).
pub const STT_OK_TEXT: &str = "音声入力: OK";
/// `.chat-stt-hint` after an STT error.
pub const STT_FAILED_TEXT: &str = "音声入力に失敗しました";
/// Idle ticks before falling asleep.
const SLEEP_AFTER_SECONDS: u32 = 60;
/// Activity this soon after falling asleep only resets the counter (ms).
const WAKE_GUARD_MS: f64 = 800.0;

/// V1 `chatState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatState {
    /// `"inputReady"`: the input bar is shown.
    #[default]
    InputReady,
    /// `"waitingAPI"`: request in flight, animated dots.
    WaitingApi,
    /// `"typing"`: the typewriter runs (pages, `▼` pauses).
    Typing,
    /// `"waitForAdvance"`: everything typed, `▼`.
    WaitForAdvance,
}

/// V1 `achievementToast` (`serial` replaces React's object identity, which restarted the 4 s).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Toast {
    /// Localized title (shown as `★ {title}`).
    pub title: String,
    /// Localized description.
    pub description: String,
    /// `.achievement-toast.visible`.
    pub visible: bool,
    /// Incremented on every unlock.
    pub serial: u64,
}

/// React's `setState(value)` bail-out: notifies subscribers only when the value changed. A
/// disposed signal is ignored.
pub fn put<T: PartialEq + Send + Sync + 'static>(signal: RwSignal<T>, value: T) {
    signal.maybe_update(|current| {
        if *current == value {
            false
        } else {
            *current = value;
            true
        }
    });
}

/// V1 refs and non-rendered state.
#[derive(Default)]
pub struct Inner {
    /// `idleSecondsRef`.
    idle_seconds: u32,
    /// `sleepTimestampRef` (`Date.now()` when SLEEPING was entered).
    sleep_timestamp: f64,
    /// `lastMousePosRef` (initially `{x: -1, y: -1}`).
    last_mouse: (i32, i32),
    /// `streamRawBufferRef` + `streamEmotionParsedRef`.
    stream: StreamEmotionDetector,
    /// Every token of the streamed reply so far (the source of the D23 display).
    stream_raw: String,
    /// V1's `fullText.length === 0` while streaming: the detector's display (V1's never
    /// rendered `fullText`) is still empty. Only the latency report reads it.
    stream_text_empty: bool,
    /// `requestStartTimeRef`.
    request_start: f64,
    /// The `tts*Ref`s.
    tts: TtsQueue,
    /// `fullText`, `typeIndex`, `typewriterStartIndex`, … (see [`Typewriter`]).
    typewriter: Typewriter,
    /// `typingTimeoutRef`.
    typing_timeout: Option<TimeoutHandle>,
    /// `lastLanguageKeyRef`.
    last_language_key: String,
    /// `startupEventShownRef`.
    startup_event_shown: bool,
    /// `conversationHistory`.
    history: Vec<ChatMessage>,
}

/// The props the controller needs.
#[derive(Clone, Copy)]
pub struct ChatProps {
    /// `menuPanelOpen` (memoized: re-runs only on a real change).
    pub menu_open: Memo<bool>,
    /// `onBackLogAdd`.
    pub on_back_log_add: Callback<(String, String)>,
    /// `onLlmStatusUpdate`.
    pub on_llm_status_update: Callback<LlmStatus>,
    /// `setActiveOverlay` (mobile only).
    pub set_active_overlay: Option<Callback<OverlayPanel>>,
}

/// The ChatPanel handle (all fields are arena handles).
#[derive(Clone, Copy)]
pub struct Chat {
    /// Props.
    pub props: ChatProps,
    /// `chatState`.
    pub chat_state: RwSignal<ChatState>,
    /// `currentEmotionTag`.
    pub emotion: RwSignal<String>,
    /// `currentSpokenChar`.
    pub spoken_char: RwSignal<String>,
    /// `chatInput`.
    pub chat_input: RwSignal<String>,
    /// `dialogueText`.
    pub dialogue_text: RwSignal<String>,
    /// `waitingIndicator`.
    pub waiting_indicator: RwSignal<&'static str>,
    /// `waitingForInput`.
    pub waiting_for_input: RwSignal<bool>,
    /// Mirror of `typeIndex` (for `isActivelySpeaking`).
    pub type_index: RwSignal<usize>,
    /// Mirror of `fullText.length` (for `isActivelySpeaking`).
    pub full_text_len: RwSignal<usize>,
    /// `sttRecording`.
    pub stt_recording: RwSignal<bool>,
    /// `sttRecordingLevel`.
    pub stt_level: RwSignal<f64>,
    /// `sttStatusText`.
    pub stt_status: RwSignal<String>,
    /// `miniGameOpen`.
    pub mini_game_open: RwSignal<bool>,
    /// `miniGameEmotion`.
    pub mini_game_emotion: RwSignal<String>,
    /// `miniSpeak`.
    pub mini_speak: RwSignal<f64>,
    /// `achievementToast`.
    pub toast: RwSignal<Toast>,
    /// Bumped on every `setConversationHistory` (V1 `submitMessage` identity, effect E14).
    pub history_rev: RwSignal<u64>,
    /// `turnCount`.
    pub turn_count: RwSignal<f64>,
    /// `ttsSessionId` (only re-renders in V1; effect E14 dependency).
    pub tts_session: RwSignal<u64>,
    /// D23: the streamed reply of the last message is still arriving (from the streaming
    /// submit until its completion, an error or a cancel). Keeps V1's cancel available while
    /// the reply is typed (STR-005).
    pub stream_open: RwSignal<bool>,
    inner: StoredValue<Inner, LocalStorage>,
}

/// The values V1's global keydown listener (effect E14) captured when it was registered.
#[derive(Debug, Clone, PartialEq)]
pub struct KeySnapshot {
    /// `chatInput`.
    pub chat_input: String,
    /// `chatState`.
    pub chat_state: ChatState,
    /// `menuPanelOpen`.
    pub menu_open: bool,
    /// `miniGameOpen`.
    pub mini_game_open: bool,
    /// [`Chat::stream_open`] (D23, STR-005; re-registers the listener like V1's dependencies).
    pub stream_open: bool,
}

/// What effect E14's listener does with a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatKeyAction {
    /// Nothing.
    Ignore,
    /// `preventDefault()` + V1's cancel ([`Chat::cancel_waiting`]).
    Cancel,
    /// `submitMessage(chatInput)`.
    Submit,
    /// `onAdvance()`.
    Advance,
}

/// V1's Ctrl+C / キャンセル precondition, `chatState === "waitingAPI"`, extended by D23 to a
/// streamed reply whose stream is still open in any state of the dialogue (STR-005). V1 never
/// left `waitingAPI` during a stream, so it could always cancel one. `inputReady` (reachable
/// with an open stream only through an import) keeps V1's behaviour: no cancel, no hint.
pub fn cancel_available(state: ChatState, stream_open: bool) -> bool {
    state == ChatState::WaitingApi || (stream_open && state != ChatState::InputReady)
}

/// Effect E14's decision for `key` (`ctrl` = `event.ctrlKey`) with the captured values.
pub fn chat_key_action(key: &str, ctrl: bool, snapshot: &KeySnapshot) -> ChatKeyAction {
    if snapshot.menu_open || snapshot.mini_game_open {
        return ChatKeyAction::Ignore;
    }
    if key.to_lowercase() == "c"
        && ctrl
        && cancel_available(snapshot.chat_state, snapshot.stream_open)
    {
        return ChatKeyAction::Cancel;
    }
    if key != "Enter" {
        return ChatKeyAction::Ignore;
    }
    if snapshot.chat_state != ChatState::InputReady {
        ChatKeyAction::Advance
    } else if js_trim(&snapshot.chat_input).is_empty() {
        ChatKeyAction::Ignore
    } else {
        ChatKeyAction::Submit
    }
}

/// `AppSettings.getFloat("Config_TextSpeed", 1.0)` (read for every typed character).
fn text_speed() -> f64 {
    app_settings().get_float(keys::TEXT_SPEED.key, keys::TEXT_SPEED.default)
}

impl Chat {
    /// Creates the state of a fresh mount (V1 initial `useState` values).
    pub fn new(props: ChatProps) -> Self {
        Self {
            props,
            chat_state: RwSignal::new(ChatState::InputReady),
            emotion: RwSignal::new(NORMAL.to_owned()),
            spoken_char: RwSignal::new(String::new()),
            chat_input: RwSignal::new(String::new()),
            dialogue_text: RwSignal::new(String::new()),
            waiting_indicator: RwSignal::new(""),
            waiting_for_input: RwSignal::new(false),
            type_index: RwSignal::new(0),
            full_text_len: RwSignal::new(0),
            stt_recording: RwSignal::new(false),
            stt_level: RwSignal::new(0.0),
            stt_status: RwSignal::new(String::new()),
            mini_game_open: RwSignal::new(false),
            mini_game_emotion: RwSignal::new(NORMAL.to_owned()),
            mini_speak: RwSignal::new(0.0),
            toast: RwSignal::new(Toast::default()),
            history_rev: RwSignal::new(0),
            turn_count: RwSignal::new(0.0),
            tts_session: RwSignal::new(0),
            stream_open: RwSignal::new(false),
            inner: StoredValue::new_local(Inner {
                last_mouse: (-1, -1),
                ..Inner::default()
            }),
        }
    }

    fn with_inner<R>(self, f: impl FnOnce(&mut Inner) -> R) -> Option<R> {
        self.inner.try_update_value(f)
    }

    // -----------------------------------------------------------------------------------------
    // Small state helpers
    // -----------------------------------------------------------------------------------------

    /// `setCurrentEmotionTag(tag)`.
    pub fn set_emotion(self, tag: &str) {
        put(self.emotion, tag.to_owned());
    }

    fn back_log(self, role: &str, message: String) {
        let _ = self
            .props
            .on_back_log_add
            .try_run((role.to_owned(), message));
    }

    /// V1 `updateStatus(latencyMs)`: `onLlmStatusUpdate(getLlmStatusFromSettings(latencyMs))`.
    pub fn update_status(self, latency_ms: f64) {
        let status = llm_status_from_settings(app_settings(), latency_ms as i64);
        let _ = self.props.on_llm_status_update.try_run(status);
    }

    /// `conversationHistoryRef.current` (bridge `getHistory`).
    pub fn history_snapshot(self) -> Vec<ChatMessage> {
        self.inner
            .try_with_value(|inner| inner.history.clone())
            .unwrap_or_default()
    }

    /// `setConversationHistory(history)`.
    fn set_history(self, history: Vec<ChatMessage>) {
        self.with_inner(|inner| inner.history = history);
        self.history_rev.update(|rev| *rev = rev.wrapping_add(1));
    }

    /// V1 `finishAssistantMessage(displayText)`: the history stores the display text.
    fn finish_assistant_message(self, display: &str) {
        self.with_inner(|inner| inner.history.push(ChatMessage::assistant(display)));
        self.history_rev.update(|rev| *rev = rev.wrapping_add(1));
    }

    fn sync_typewriter(self) {
        if let Some((index, len)) =
            self.with_inner(|inner| (inner.typewriter.type_index(), inner.typewriter.len()))
        {
            put(self.type_index, index);
            put(self.full_text_len, len);
        }
    }

    /// `setFullText(text)` alone (streaming). D23: when the text no longer starts with what was
    /// typed, the typewriter restarts the page; the stale page, its `▼` and its pause are
    /// cleared here so that the page is redrawn from the new text right away.
    fn set_full_text(self, text: &str) {
        let restarted = self
            .with_inner(|inner| inner.typewriter.set_full_text(text))
            .unwrap_or(false);
        self.sync_typewriter();
        if restarted {
            put(self.dialogue_text, String::new());
            put(self.waiting_indicator, "");
            put(self.waiting_for_input, false);
        }
    }

    /// `setFullText(text)` + V1's "typewriter reset block" + `streamComplete = true` +
    /// `chatState = "typing"` (success, error, startup greeting, `/dev emotion help`).
    fn show_text(self, text: &str) {
        self.with_inner(|inner| inner.typewriter.reset(text, true));
        self.sync_typewriter();
        put(self.waiting_for_input, false);
        put(self.dialogue_text, String::new());
        put(self.waiting_indicator, "");
        put(self.chat_state, ChatState::Typing);
    }

    /// V1 `isActivelySpeaking` (tracked; `ttsPlaybackBusyRef` is sampled, not tracked).
    pub fn is_actively_speaking(self) -> bool {
        if self.chat_state.get() != ChatState::Typing || self.waiting_for_input.get() {
            return false;
        }
        let len = self.full_text_len.get();
        let index = self.type_index.get();
        (index < len && len > 0)
            || self
                .inner
                .try_with_value(|inner| inner.tts.playback_busy())
                .unwrap_or(false)
    }

    // -----------------------------------------------------------------------------------------
    // Idle / sleep (E1–E3)
    // -----------------------------------------------------------------------------------------

    /// V1 `resetIdleTimer()`.
    pub fn reset_idle_timer(self) {
        let Some(sleep_timestamp) = self.with_inner(|inner| {
            if inner.idle_seconds > 0 {
                inner.idle_seconds = 0;
            }
            inner.sleep_timestamp
        }) else {
            return;
        };
        if now_ms() - sleep_timestamp < WAKE_GUARD_MS {
            return;
        }
        self.emotion.maybe_update(|emotion| {
            if emotion == SLEEPING {
                NORMAL.clone_into(emotion);
                true
            } else {
                false
            }
        });
    }

    /// E1 `mousemove`: ignored when the coordinates did not change.
    pub fn on_mouse_move(self, x: i32, y: i32) {
        let moved = self
            .with_inner(|inner| {
                if inner.last_mouse == (x, y) {
                    false
                } else {
                    inner.last_mouse = (x, y);
                    true
                }
            })
            .unwrap_or(false);
        if moved {
            self.reset_idle_timer();
        }
    }

    /// One E2 tick with the `chatState` / `currentEmotionTag` the interval was created with.
    pub fn idle_tick(self, state: ChatState, emotion: &str) {
        if !matches!(state, ChatState::InputReady | ChatState::WaitForAdvance) {
            return;
        }
        let fall_asleep = self
            .with_inner(|inner| {
                inner.idle_seconds = inner.idle_seconds.saturating_add(1);
                if inner.idle_seconds >= SLEEP_AFTER_SECONDS && emotion != SLEEPING {
                    inner.sleep_timestamp = now_ms();
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false);
        if fall_asleep {
            self.set_emotion(SLEEPING);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Mount (E4, E11–E13)
    // -----------------------------------------------------------------------------------------

    /// E4 step 1: `[system: persona + "\n\n" + memoryContext]` (no dynamic context).
    pub fn init_history(self) {
        let base = system_prompt_from_settings(app_settings());
        self.set_history(managers::memory().initial_history(&base));
    }

    /// E11: re-applies `updateSystemPrompt` when `"${configLanguage}:${chatLanguageSetting}"`
    /// changes (the first run only stores the key).
    pub fn on_language_key(self, key: String) {
        let changed = self
            .with_inner(|inner| {
                if inner.last_language_key.is_empty() {
                    inner.last_language_key = key;
                    return false;
                }
                if inner.last_language_key == key {
                    return false;
                }
                inner.last_language_key = key;
                true
            })
            .unwrap_or(false);
        if changed {
            let history = self.update_system_prompt(&self.history_snapshot());
            self.set_history(history);
        }
    }

    /// V1 `updateSystemPrompt(history)` with the current `turnCount`.
    fn update_system_prompt(self, history: &[ChatMessage]) -> Vec<ChatMessage> {
        let turn = self.turn_count.try_get_untracked().unwrap_or(0.0);
        self.update_system_prompt_with_turn(history, turn)
    }

    fn update_system_prompt_with_turn(
        self,
        history: &[ChatMessage],
        turn: f64,
    ) -> Vec<ChatMessage> {
        let base = system_prompt_from_settings(app_settings());
        managers::memory().update_system_prompt(
            history,
            &base,
            turn,
            &managers::local_now(),
            &mut managers::rng(),
        )
    }

    /// E12: the startup greeting (once per mount): a random event typed out with its emotion,
    /// not sent to the LLM, no TTS, unlocks `first_event`.
    pub fn startup_event(self) {
        let first = self
            .with_inner(|inner| !std::mem::replace(&mut inner.startup_event_shown, true))
            .unwrap_or(false);
        if !first {
            return;
        }
        let events = managers::random_events();
        events.try_trigger_event(&mut managers::rng(), config_language_untracked());
        let text = events.event_text(config_language_untracked());
        let emotion = events.event_emotion();
        if text.is_empty() {
            return;
        }
        self.set_emotion(emotion);
        self.show_text(text);
        managers::notify_achievement(&AchievementEvent::RandomEvent);
    }

    /// Bridge `getTurnCount`.
    pub fn turn_count_value(self) -> f64 {
        self.turn_count.try_get_untracked().unwrap_or(0.0)
    }

    /// Bridge `applyImported(messages, turnCount)`: replace the history and the turn count,
    /// `inputReady`, NORMAL (typewriter, TTS and requests untouched).
    pub fn apply_imported(self, messages: Vec<ChatMessage>, turn_count: f64) {
        self.set_history(messages);
        put(self.turn_count, turn_count);
        put(self.chat_state, ChatState::InputReady);
        self.set_emotion(NORMAL);
    }

    // -----------------------------------------------------------------------------------------
    // Sending (submitMessage, dev commands)
    // -----------------------------------------------------------------------------------------

    /// V1 `submitMessage(raw)`.
    pub fn submit_message(self, raw: &str) {
        let text = js_trim(raw);
        if text.is_empty() || self.chat_state.try_get_untracked() != Some(ChatState::InputReady) {
            return;
        }
        if let Some(command) = parse_dev_command(text) {
            self.run_dev_command(command);
            return;
        }

        let user_message = ChatMessage::user(text);
        put(self.chat_input, String::new());
        let turn = self.turn_count_value();
        let history = self.history_snapshot();
        // The request (`updateSystemPrompt([...conversationHistory, userMessage])`) is built from
        // the untrimmed history, with the memory context from before `recordInteraction()` and
        // the pre-increment turn count: V1 deferred the stored-history updater to the next
        // render, after the request had been sent (spec-lib-core §7.10).
        let mut request = history.clone();
        request.push(user_message.clone());
        let next_messages = self.update_system_prompt_with_turn(&request, turn);

        put(self.turn_count, turn + 1.0);
        // The stored history (V1's state updater, run exactly once): append, trim to 30,
        // `recordInteraction()`, refresh the system prompt (still the old turn count).
        let mut stored = history;
        managers::memory().record_user_turn(&mut stored, user_message, &managers::local_now());
        let stored = self.update_system_prompt_with_turn(&stored, turn);
        self.set_history(stored);
        self.back_log("user", text.to_owned());

        let started = now_ms();
        self.with_inner(|inner| {
            inner.request_start = started;
            inner.stream.reset();
            inner.stream_raw.clear();
            inner.stream_text_empty = true;
            // `setStreamComplete(false)`, plus the D23 typewriter reset (see the module docs).
            inner.typewriter.reset("", false);
        });
        self.sync_typewriter();
        put(self.dialogue_text, String::new());
        put(self.waiting_indicator, ".");
        put(self.chat_state, ChatState::WaitingApi);
        self.reset_tts_session();

        // Default 0 here (8 elsewhere), as in V1; the key always exists after initializeDefaults.
        let provider = app_settings().get_int(keys::API_PROVIDER.key, 0);
        let streaming = uses_streaming(provider);
        put(self.stream_open, streaming);
        if streaming {
            services::ai().send_chat_streaming(next_messages);
        } else {
            services::ai().send_chat(next_messages);
        }
    }

    /// The `/dev …` branch of `submitMessage` (never reaches the backlog, history or API).
    fn run_dev_command(self, command: DevCommand) {
        match command {
            DevCommand::EmotionHelp => {
                put(self.chat_input, String::new());
                self.show_text(DEV_EMOTION_HELP_TEXT);
            }
            DevCommand::SetEmotion(target) => {
                if let Some(target) = target {
                    self.set_emotion(&target);
                }
                put(self.chat_input, String::new());
                managers::notify_achievement(&AchievementEvent::DevCommand);
            }
            DevCommand::MotionReset => {
                self.set_emotion(NORMAL);
                put(self.chat_input, String::new());
                managers::notify_achievement(&AchievementEvent::DevCommand);
            }
            DevCommand::AchievementsReset => {
                put(self.chat_input, String::new());
                managers::achievements().reset_all();
                managers::notify_achievement(&AchievementEvent::DevCommand);
            }
            DevCommand::Steiner => {
                put(self.chat_input, String::new());
                managers::notify_achievement(&AchievementEvent::SecretSteiner);
            }
        }
    }

    /// V1 `onAdvance()` (dialogue click, Enter outside `inputReady`).
    pub fn on_advance(self) {
        match self.chat_state.try_get_untracked() {
            Some(ChatState::Typing) => {
                if self.waiting_for_input.try_get_untracked().unwrap_or(false) {
                    put(self.waiting_for_input, false);
                } else {
                    self.with_inner(|inner| inner.typewriter.request_skip());
                }
            }
            Some(ChatState::WaitForAdvance) => {
                put(self.chat_state, ChatState::InputReady);
                self.set_emotion(NORMAL);
                put(self.waiting_indicator, "");
            }
            _ => {}
        }
    }

    /// `Ctrl+C` (PC) / `キャンセル` (mobile) when [`cancel_available`]: V1's cancel, which aborts
    /// only a stream (a pending `sendChat` still answers later, V1 Q-C1) and does not stop TTS.
    /// D23 (STR-005): while the streamed reply is being typed, its typewriter stops too and
    /// nothing of the reply is kept. Its held backlog pages are dropped. History, memory
    /// commands and achievements only happen at completion, which never comes for a cancelled
    /// stream.
    pub fn cancel_waiting(self) {
        services::ai().cancel_streaming();
        if self.stream_open.try_get_untracked().unwrap_or(false) {
            self.stop_stream_reply();
        }
        put(self.chat_state, ChatState::InputReady);
        self.set_emotion(NORMAL);
        put(self.waiting_indicator, "");
        put(self.chat_input, String::new());
    }

    /// The D23 part of a cancel: the stream is closed and its typed reply discarded.
    fn stop_stream_reply(self) {
        put(self.stream_open, false);
        self.clear_typing_timeout();
        self.with_inner(|inner| inner.typewriter.reset("", false));
        self.sync_typewriter();
        put(self.waiting_for_input, false);
        put(self.dialogue_text, String::new());
    }

    /// V1 `toggleSttRecording()` (mic button; hidden unless `Config_AudioInputEnabled` was 1 at
    /// mount, DECISIONS D14). Enabling STT here persists `Config_Experimental_STT_Enabled = 1`.
    pub fn toggle_stt(self) {
        if self.chat_state.try_get_untracked() != Some(ChatState::InputReady) {
            return;
        }
        put(self.stt_status, String::new());
        let stt = services::stt();
        if !stt.enabled() {
            stt.set_enabled(true);
        }
        if stt.is_recording() {
            stt.stop_recording();
            return;
        }
        stt.start_recording();
    }

    /// Effect E14's listener body with the values captured at registration.
    pub fn on_global_keydown(self, event: &web_sys::KeyboardEvent, snapshot: &KeySnapshot) {
        match chat_key_action(&event.key(), event.ctrl_key(), snapshot) {
            ChatKeyAction::Cancel => {
                event.prevent_default();
                self.cancel_waiting();
            }
            ChatKeyAction::Submit => self.submit_message(&snapshot.chat_input),
            ChatKeyAction::Advance => self.on_advance(),
            ChatKeyAction::Ignore => {}
        }
    }

    // -----------------------------------------------------------------------------------------
    // AI replies (handlers connected by E4)
    // -----------------------------------------------------------------------------------------

    /// `AIService.responseReceived`: `updateStatus(Date.now() - start)`, then `handleApiSuccess`.
    pub fn on_response_received(self, response: &str) {
        let started = self.with_inner(|inner| inner.request_start).unwrap_or(0.0);
        self.update_status(now_ms() - started);
        self.handle_api_success(response);
    }

    /// V1 `handleApiSuccess(response)` (non-streaming reply; arrives in any state).
    fn handle_api_success(self, response: &str) {
        let no_think = strip_think_blocks(response);
        let memory = managers::memory();
        let mut cleaned = memory.apply_memory_commands(js_trim(&no_think), &managers::local_now());
        if cleaned.is_empty() {
            cleaned = EMPTY_REPLY_TEXT.to_owned();
        }
        let tag = parse_emotion_tag(&cleaned);
        self.set_emotion(tag);
        memory.record_emotion(tag);
        managers::notify_achievement(&AchievementEvent::Emotion(tag.to_owned()));
        managers::notify_achievement(&AchievementEvent::Chat {
            total_interactions: memory.total_interactions(),
        });
        let mut display = strip_emotion_tag(&cleaned);
        if display.is_empty() {
            display = EMPTY_REPLY_TEXT.to_owned();
        }
        self.finish_assistant_message(&display);
        self.show_text(&display);
        tts_queue::speak_reply(&self, &display);
    }

    /// V1 `handleStreamToken(token)` with live state (D23, spec §6.4 Option B).
    pub fn on_stream_token(self, token: &str) {
        let Some((parsed, started, no_text)) = self.with_inner(|inner| {
            (
                inner.stream.is_parsed(),
                inner.request_start,
                inner.stream_text_empty,
            )
        }) else {
            return;
        };
        // `!streamEmotionParsedRef.current && fullText.length === 0` with the live `fullText`
        // (D23): the time to the first token, repeated while nothing displayable has arrived.
        // V1's frozen `fullText` was always "", so it reported on every token until the emotion
        // was decided (spec §6.2).
        if !parsed && no_text {
            self.update_status(now_ms() - started);
        }
        let Some((update, parsed, display)) = self.with_inner(|inner| {
            let update = inner.stream.push_token(token);
            inner.stream_text_empty = update.display.is_empty();
            inner.stream_raw.push_str(token);
            let display = stream_display_prefix(&inner.stream_raw);
            (update, inner.stream.is_parsed(), display)
        }) else {
            return;
        };
        if let Some(emotion) = update.emotion {
            self.set_emotion(emotion.as_str());
            managers::memory().record_emotion(emotion.as_str());
        }
        // CHAT-028: only the part of the reply that the completion will display, cleaned like
        // the completion (memory commands, think blocks and tags removed, undecided text held
        // back), is typed and spoken.
        self.set_full_text(&display);
        // `if (chatState === "waitingAPI") setChatState("typing")` with the live state (V1's
        // frozen `chatState` was "inputReady", so it never fired: CHAT-028). The switch waits
        // until the leading tag is decided and there is text to show (the §6.4 hardening).
        if parsed && !display.is_empty() {
            self.begin_stream_typing();
        }
        // The TTS capture uses the same `parsed` gate and the same text (D23), so nothing that
        // is not displayed is spoken (V1 captured the raw display and spoke e.g. "[SMIOkay…",
        // think blocks and memory commands).
        tts_queue::capture_stream_token(&self, &display, parsed);
    }

    /// D23: the streamed reply leaves `waitingAPI` and is typed out. This happens only from the
    /// live `waitingAPI`, so after a cancel the rest of a stream stays hidden. The typewriter was
    /// cleared by `submit_message` and its text follows the stream. As in V1's reset block, the
    /// pause flag, the page and the dots are cleared.
    fn begin_stream_typing(self) {
        if self.chat_state.try_get_untracked() != Some(ChatState::WaitingApi) {
            return;
        }
        put(self.waiting_for_input, false);
        put(self.dialogue_text, String::new());
        put(self.waiting_indicator, "");
        put(self.chat_state, ChatState::Typing);
    }

    /// V1 `handleStreamComplete(fullResponse)` with live state (D23, spec §6.4 Option B).
    pub fn on_stream_complete(self, full_response: &str) {
        let Some((raw, parsed)) = self.with_inner(|inner| {
            (
                inner.stream.completion_raw(full_response),
                inner.stream.is_parsed(),
            )
        }) else {
            return;
        };
        let memory = managers::memory();
        // Memory commands are taken from the raw text, `<think>` blocks included.
        let mut cleaned =
            strip_emotion_tag(&memory.apply_memory_commands(&raw, &managers::local_now()));
        if cleaned.is_empty() {
            cleaned = EMPTY_REPLY_TEXT.to_owned();
        }
        let mut emotion_for_event = self.emotion.try_get_untracked().unwrap_or_default();
        if !parsed {
            let tag = parse_emotion_tag(&raw);
            tag.clone_into(&mut emotion_for_event);
            self.set_emotion(tag);
            memory.record_emotion(tag);
        }
        managers::notify_achievement(&AchievementEvent::Emotion(emotion_for_event));
        managers::notify_achievement(&AchievementEvent::Chat {
            total_interactions: memory.total_interactions(),
        });

        // The typed part is a prefix of `cleaned` (CHAT-028); if not (a completion text that is
        // not the streamed tokens), the page is redrawn.
        self.set_full_text(&cleaned);
        tts_queue::capture_stream_text(&self, &cleaned, true);
        self.finish_assistant_message(&cleaned);
        // `setStreamComplete(true)`; the pages paused during the stream reach the backlog now.
        let held = self
            .with_inner(|inner| inner.typewriter.complete_stream())
            .unwrap_or_default();
        for page in held {
            self.back_log("assistant", page);
        }
        put(self.stream_open, false);
        // `if (chatState === "waitingAPI") setChatState("typing")` with the live state: a reply
        // that never showed text while streaming is typed now.
        self.begin_stream_typing();
        self.with_inner(|inner| {
            inner.stream.reset();
            inner.stream_raw.clear();
        });
    }

    /// V1 `handleApiError(error)`: ANGRY + Kurisu's message typed out, TTS cancelled, nothing
    /// stored in the history or memory. A stream ends with its error; what it had typed is
    /// replaced and its held backlog pages are dropped by the typewriter reset.
    pub fn on_api_error(self, error: &str) {
        let message =
            api_error_message_for_platform(error, crate::platform::platform().is_desktop());
        self.set_emotion(ANGRY);
        put(self.stream_open, false);
        self.show_text(message);
        self.reset_tts_session();
    }

    // -----------------------------------------------------------------------------------------
    // Typewriter (E6)
    // -----------------------------------------------------------------------------------------

    /// V1 `step()`.
    pub fn typewriter_step(self) {
        let menu_open = self.props.menu_open.try_get_untracked().unwrap_or(false);
        let waiting = self.waiting_for_input.try_get_untracked().unwrap_or(false);
        let Some(out) =
            self.with_inner(|inner| inner.typewriter.step(menu_open, waiting, text_speed))
        else {
            return;
        };
        self.apply_step(out);
    }

    fn apply_step(self, out: StepOutput) {
        if let Some(text) = out.dialogue {
            put(self.dialogue_text, text);
        }
        if let Some(spoken) = out.spoken_char {
            put(self.spoken_char, spoken);
        }
        if let Some(indicator) = out.indicator {
            put(self.waiting_indicator, indicator);
        }
        if let Some(waiting) = out.waiting_for_input {
            put(self.waiting_for_input, waiting);
        }
        self.sync_typewriter();
        if out.wait_for_advance {
            put(self.chat_state, ChatState::WaitForAdvance);
        }
        if let Some(page) = out.backlog {
            self.back_log("assistant", page);
        }
        if let Some(ms) = out.schedule_ms {
            let handle = set_js_timeout(move || self.typewriter_tick(), ms);
            self.with_inner(|inner| inner.typing_timeout = handle);
        }
    }

    fn typewriter_tick(self) {
        self.with_inner(|inner| inner.typing_timeout = None);
        self.typewriter_step();
    }

    /// `window.clearTimeout(typingTimeoutRef.current)`.
    pub fn clear_typing_timeout(self) {
        if let Some(handle) = self
            .with_inner(|inner| inner.typing_timeout.take())
            .flatten()
        {
            handle.clear();
        }
    }

    /// E7's timer with the values the effect saw.
    pub fn auto_advance(self, state: ChatState, waiting_for_input: bool) {
        if state == ChatState::WaitForAdvance {
            put(self.chat_state, ChatState::InputReady);
            self.set_emotion(NORMAL);
            put(self.waiting_indicator, "");
        } else if waiting_for_input {
            put(self.waiting_for_input, false);
        }
    }

    // -----------------------------------------------------------------------------------------
    // TTS (§11) and STT (§13) handlers
    // -----------------------------------------------------------------------------------------

    /// V1 `prepareLiveTTSSession()` / `cancelLiveTTS()`.
    fn reset_tts_session(self) {
        self.tts_session
            .update(|session| *session = session.wrapping_add(1));
        self.with_inner(|inner| inner.tts.reset());
        services::audio().stop();
    }

    /// `STTService.transcriptionComplete(text)`: the untrimmed text replaces the input.
    pub fn on_transcription(self, text: &str) {
        if !js_trim(text).is_empty() {
            put(self.chat_input, text.to_owned());
            put(self.stt_status, STT_OK_TEXT.to_owned());
        }
    }

    /// `STTService.transcriptionError()`.
    pub fn on_transcription_error(self) {
        put(self.stt_status, STT_FAILED_TEXT.to_owned());
    }

    /// `STTService.recordingStateChanged()`.
    pub fn on_recording_state_changed(self) {
        let recording = services::stt().is_recording();
        put(self.stt_recording, recording);
        if !recording {
            put(self.stt_level, 0.0);
        }
    }

    /// `STTService.recordingProgress(level)`.
    pub fn on_recording_progress(self, level: f64) {
        put(self.stt_level, level);
    }

    // -----------------------------------------------------------------------------------------
    // Achievement toast (E9, E10)
    // -----------------------------------------------------------------------------------------

    /// `AchievementManager.achievementUnlocked` → toast (the last of a batch wins).
    pub fn on_achievement_unlocked(self, title: &str, description: &str) {
        self.toast.update(|toast| {
            title.clone_into(&mut toast.title);
            description.clone_into(&mut toast.description);
            toast.visible = true;
            toast.serial = toast.serial.wrapping_add(1);
        });
    }

    /// The 4 s timer: hide, keeping the texts.
    pub fn hide_toast(self) {
        self.toast
            .maybe_update(|toast| std::mem::replace(&mut toast.visible, false));
    }
}

impl TtsHost for Chat {
    fn with_queue<R>(&self, f: impl FnOnce(&mut TtsQueue) -> R) -> Option<R> {
        self.inner.try_update_value(|inner| f(&mut inner.tts))
    }

    fn tts_enabled(&self) -> bool {
        services::tts().enabled()
    }

    fn synthesize(&self, text: &str) {
        services::tts().synthesize(text);
    }

    fn play(&self, url: &str) {
        services::audio().play_from_file(url);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use ChatKeyAction::{Advance, Cancel, Ignore, Submit};
    use ChatState::{InputReady, Typing, WaitForAdvance, WaitingApi};

    /// Stand-ins for what the controller reaches outside the panel (see the `use` at the top of
    /// the file). Native tests have no browser: `js_sys` (the clock, the dates the managers are
    /// built with) and `window` (the services) panic there.
    pub(super) mod native {
        /// `Date.now()`, fixed.
        pub fn now_ms() -> f64 {
            1_800_000_000_000.0
        }

        /// `crate::app::managers` over the in-memory storage, at a fixed local time.
        pub mod managers {
            use std::cell::RefCell;

            use amadeus_core::achievements::{AchievementEvent, AchievementManager};
            use amadeus_core::memory::{LocalDateTime, MemoryManager};
            use amadeus_core::random_events::RandomEventService;

            use crate::app::managers::{Achievements, Memory};
            use crate::i18n::config_language_untracked;
            use crate::storage::local_store;

            thread_local! {
                static MEMORY: &'static Memory =
                    Box::leak(Box::new(MemoryManager::new(local_store(), &local_now())));
                static ACHIEVEMENTS: &'static Achievements =
                    Box::leak(Box::new(AchievementManager::new(local_store(), &local_now())));
                static RANDOM_EVENTS: &'static RandomEventService =
                    Box::leak(Box::new(RandomEventService::new(local_now().hour)));
                /// Every `notify_achievement` event, in order.
                pub static EVENTS: RefCell<Vec<AchievementEvent>> =
                    const { RefCell::new(Vec::new()) };
            }

            pub fn memory() -> &'static Memory {
                MEMORY.with(|memory| *memory)
            }

            pub fn achievements() -> &'static Achievements {
                ACHIEVEMENTS.with(|achievements| *achievements)
            }

            pub fn random_events() -> &'static RandomEventService {
                RANDOM_EVENTS.with(|events| *events)
            }

            pub fn local_now() -> LocalDateTime {
                LocalDateTime::new(2026, 11, 22, 12, 0, 0)
            }

            pub fn rng() -> impl FnMut() -> f64 {
                || 0.5
            }

            pub fn notify_achievement(event: &AchievementEvent) -> Vec<&'static str> {
                EVENTS.with(|events| events.borrow_mut().push(event.clone()));
                achievements().notify_event(event, &local_now(), config_language_untracked())
            }
        }

        /// The services: every call is recorded and nothing is sent. The test answers the
        /// syntheses (`tts_queue::on_synthesis_*`).
        pub mod services {
            use std::cell::{Cell, RefCell};

            use amadeus_core::protocol::ChatMessage;

            thread_local! {
                /// Every call, in order (`"synthesize 覚えておくわ。"`).
                static CALLS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
                /// `TTSService.enabled`.
                pub static TTS_ENABLED: Cell<bool> = const { Cell::new(false) };
            }

            fn record(call: impl Into<String>) {
                CALLS.with(|calls| calls.borrow_mut().push(call.into()));
            }

            /// The calls since the last time.
            pub fn take_calls() -> Vec<String> {
                CALLS.with(|calls| std::mem::take(&mut *calls.borrow_mut()))
            }

            pub struct Ai;
            pub struct Audio;
            pub struct Stt;
            pub struct Tts;

            pub fn ai() -> Ai {
                Ai
            }

            pub fn audio() -> Audio {
                Audio
            }

            pub fn stt() -> Stt {
                Stt
            }

            pub fn tts() -> Tts {
                Tts
            }

            impl Ai {
                pub fn send_chat(&self, _messages: Vec<ChatMessage>) {
                    record("send_chat");
                }

                pub fn send_chat_streaming(&self, _messages: Vec<ChatMessage>) {
                    record("send_chat_streaming");
                }

                pub fn cancel_streaming(&self) {
                    record("cancel_streaming");
                }
            }

            impl Audio {
                pub fn stop(&self) {
                    record("audio stop");
                }

                pub fn play_from_file(&self, url: &str) {
                    record(format!("play {url}"));
                }
            }

            impl Stt {
                pub fn enabled(&self) -> bool {
                    false
                }

                pub fn set_enabled(&self, _enabled: bool) {}

                pub fn is_recording(&self) -> bool {
                    false
                }

                pub fn start_recording(&self) {}

                pub fn stop_recording(&self) {}
            }

            impl Tts {
                pub fn enabled(&self) -> bool {
                    TTS_ENABLED.get()
                }

                pub fn synthesize(&self, text: &str) {
                    record(format!("synthesize {text}"));
                }
            }
        }
    }

    fn snapshot(chat_state: ChatState, stream_open: bool) -> KeySnapshot {
        KeySnapshot {
            chat_input: String::new(),
            chat_state,
            menu_open: false,
            mini_game_open: false,
            stream_open,
        }
    }

    #[test]
    fn cancel_is_available_while_the_stream_is_open() {
        // V1: `waitingAPI` only, with or without a stream.
        assert!(cancel_available(WaitingApi, false));
        assert!(cancel_available(WaitingApi, true));
        for state in [InputReady, Typing, WaitForAdvance] {
            assert!(!cancel_available(state, false), "{state:?}");
        }
        // D23 (STR-005): a streamed reply typed while its stream is still open.
        assert!(cancel_available(Typing, true));
        assert!(cancel_available(WaitForAdvance, true));
        assert!(
            !cancel_available(InputReady, true),
            "the input bar keeps V1's behaviour (an import during a stream)"
        );
    }

    #[test]
    fn global_keys_follow_the_captured_snapshot() {
        let typing_open = snapshot(Typing, true);
        assert_eq!(chat_key_action("c", true, &typing_open), Cancel);
        assert_eq!(chat_key_action("C", true, &typing_open), Cancel);
        assert_eq!(chat_key_action("c", false, &typing_open), Ignore);
        assert_eq!(
            chat_key_action("c", true, &snapshot(WaitingApi, false)),
            Cancel
        );
        // After the completion the listener is re-registered without the stream: no cancel.
        assert_eq!(chat_key_action("c", true, &snapshot(Typing, false)), Ignore);
        assert_eq!(
            chat_key_action("c", true, &snapshot(WaitForAdvance, false)),
            Ignore
        );
        assert_eq!(
            chat_key_action("c", true, &snapshot(InputReady, true)),
            Ignore
        );
        // The menu and the mini game block every key (V1).
        for blocked in [
            KeySnapshot {
                menu_open: true,
                ..typing_open.clone()
            },
            KeySnapshot {
                mini_game_open: true,
                ..typing_open.clone()
            },
        ] {
            assert_eq!(chat_key_action("c", true, &blocked), Ignore);
            assert_eq!(chat_key_action("Enter", false, &blocked), Ignore);
        }
        // Enter: advance outside `inputReady`, submit a non-blank input there.
        assert_eq!(chat_key_action("Enter", false, &typing_open), Advance);
        assert_eq!(
            chat_key_action("Enter", true, &snapshot(WaitingApi, false)),
            Advance
        );
        assert_eq!(
            chat_key_action("Enter", false, &snapshot(InputReady, false)),
            Ignore
        );
        let input = KeySnapshot {
            chat_input: " hi ".into(),
            ..snapshot(InputReady, false)
        };
        assert_eq!(chat_key_action("Enter", false, &input), Submit);
        assert_eq!(chat_key_action("c", true, &input), Ignore);
        let blank = KeySnapshot {
            chat_input: "\u{3000} ".into(),
            ..snapshot(InputReady, false)
        };
        assert_eq!(chat_key_action("Enter", false, &blank), Ignore);
        assert_eq!(chat_key_action("x", false, &typing_open), Ignore);
    }

    /// The `(role, message)` rows a test panel sent to the backlog.
    type BacklogRows = Arc<Mutex<Vec<(String, String)>>>;

    /// A panel whose backlog rows are recorded.
    fn panel() -> (Chat, BacklogRows) {
        let rows = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&rows);
        let chat = Chat::new(ChatProps {
            menu_open: Memo::new(|_| false),
            on_back_log_add: Callback::new(move |row: (String, String)| {
                sink.lock().unwrap().push(row);
            }),
            on_llm_status_update: Callback::new(|_: LlmStatus| {}),
            set_active_overlay: None,
        });
        (chat, rows)
    }

    #[test]
    fn cancel_while_a_streamed_reply_is_typed() {
        let owner = Owner::new();
        owner.with(|| {
            let (chat, rows) = panel();
            // A streamed reply is being typed: its first page is paused at ▼ (held for the
            // backlog), the stream is still open.
            chat.with_inner(|inner| {
                inner.typewriter.reset("", false);
                inner.typewriter.set_full_text("そうね。あなたは");
                for _ in 0..4 {
                    inner.typewriter.step(false, false, || 1.0);
                }
            });
            chat.sync_typewriter();
            put(chat.chat_state, Typing);
            put(chat.stream_open, true);
            put(chat.waiting_for_input, true);
            put(chat.waiting_indicator, "▼");
            put(chat.dialogue_text, "そうね。".to_owned());
            put(chat.emotion, "SMILE".to_owned());
            put(chat.chat_input, "draft".to_owned());

            chat.cancel_waiting();

            // V1's cancel ...
            assert_eq!(chat.chat_state.get_untracked(), InputReady);
            assert_eq!(chat.emotion.get_untracked(), NORMAL);
            assert_eq!(chat.waiting_indicator.get_untracked(), "");
            assert_eq!(chat.chat_input.get_untracked(), "");
            // ... plus the stopped stream reply: closed, typewriter cleared, nothing kept.
            assert!(!chat.stream_open.get_untracked());
            assert!(!chat.waiting_for_input.get_untracked());
            assert_eq!(chat.dialogue_text.get_untracked(), "");
            assert_eq!(chat.full_text_len.get_untracked(), 0);
            assert_eq!(chat.type_index.get_untracked(), 0);
            let held = chat
                .with_inner(|inner| inner.typewriter.complete_stream())
                .unwrap_or_default();
            assert!(held.is_empty(), "the paused page never reaches the backlog");
            assert!(rows.lock().unwrap().is_empty());
            assert!(chat.history_snapshot().is_empty());
            assert!(!cancel_available(
                chat.chat_state.get_untracked(),
                chat.stream_open.get_untracked()
            ));
        });
    }

    #[test]
    fn cancel_in_waiting_api_without_a_stream_is_unchanged() {
        let owner = Owner::new();
        owner.with(|| {
            let (chat, rows) = panel();
            // A pending `sendChat` (V1 Q-C1): the typewriter keeps its state.
            chat.with_inner(|inner| inner.typewriter.reset("", false));
            put(chat.chat_state, WaitingApi);
            put(chat.waiting_indicator, "..");
            put(chat.chat_input, "draft".to_owned());
            chat.with_inner(|inner| inner.typewriter.set_full_text("late"));
            chat.cancel_waiting();
            assert_eq!(chat.chat_state.get_untracked(), InputReady);
            assert_eq!(chat.emotion.get_untracked(), NORMAL);
            assert_eq!(chat.waiting_indicator.get_untracked(), "");
            assert_eq!(chat.chat_input.get_untracked(), "");
            assert_eq!(
                chat.with_inner(|inner| inner.typewriter.full_text()),
                Some("late".to_owned()),
                "only a stream reply is stopped"
            );
            assert!(rows.lock().unwrap().is_empty());
        });
    }

    /// Effect E6 and the typewriter's timer, by hand: the steps of `Chat::typewriter_step`,
    /// with each scheduled step fired by the test instead of a browser timer.
    struct Driver {
        chat: Chat,
        /// A next step is scheduled.
        pending: bool,
        /// E6's dependencies (`chatState`, `waitingForInput`) at its last run.
        deps: Option<(ChatState, bool)>,
    }

    impl Driver {
        fn new(chat: Chat) -> Self {
            let mut driver = Self {
                chat,
                pending: false,
                deps: None,
            };
            driver.effect();
            driver
        }

        /// E6, when a dependency changed: the scheduled step is dropped and, while typing and
        /// not paused, the typewriter steps at once.
        fn effect(&mut self) {
            let chat = self.chat;
            let deps = (
                chat.chat_state.get_untracked(),
                chat.waiting_for_input.get_untracked(),
            );
            if self.deps == Some(deps) {
                return;
            }
            self.deps = Some(deps);
            self.pending = false;
            if deps == (Typing, false) {
                self.step();
            }
        }

        fn step(&mut self) {
            let chat = self.chat;
            let waiting = chat.waiting_for_input.get_untracked();
            let mut out = chat
                .with_inner(|inner| inner.typewriter.step(false, waiting, || 1.0))
                .expect("the panel is alive");
            self.pending = out.schedule_ms.take().is_some();
            chat.apply_step(out);
            self.effect();
        }

        /// Fires the scheduled step, if any.
        fn tick(&mut self) -> bool {
            if !std::mem::take(&mut self.pending) {
                return false;
            }
            self.step();
            true
        }

        /// Enter outside the input bar (`onAdvance`).
        fn enter(&mut self) {
            self.chat.on_advance();
            self.effect();
        }
    }

    /// Fails every synthesis the panel requested (the queue then moves on) until none is
    /// left; returns the texts.
    fn drain_syntheses(chat: Chat) -> Vec<String> {
        let mut spoken = Vec::new();
        loop {
            let texts: Vec<String> = native::services::take_calls()
                .iter()
                .filter_map(|call| call.strip_prefix("synthesize "))
                .map(str::to_owned)
                .collect();
            if texts.is_empty() {
                return spoken;
            }
            for _ in &texts {
                tts_queue::on_synthesis_error(&chat);
            }
            spoken.extend(texts);
        }
    }

    /// Appends `value` unless it is the last entry.
    fn note(list: &mut Vec<String>, value: String) {
        if list.last() != Some(&value) {
            list.push(value);
        }
    }

    /// What a panel made of a streamed reply ([`stream_through_panel`]).
    struct Streamed {
        /// The backlog rows.
        rows: Vec<(String, String)>,
        /// Every text the typewriter held (`fullText`), in order.
        typed: Vec<String>,
        /// Every text the dialogue box showed, in order.
        shown: Vec<String>,
        /// Every synthesized text, in order.
        spoken: Vec<String>,
        /// The dialogue at the final `▼` (`waitForAdvance`).
        last: Option<String>,
        /// The last message of the conversation history.
        history_last: Option<ChatMessage>,
    }

    /// Sends `message` with a streaming provider (5, Ollama) and feeds `reply` to
    /// `Chat::on_stream_token` in 4-character tokens (the audit mock), with three typewriter
    /// steps per token and Enter at every `▼`, then `Chat::on_stream_complete(reply)`; reads
    /// the reply to its end (Enter at every `▼`) and checks that the panel is back at its input
    /// with the stream closed.
    fn stream_through_panel(message: &str, reply: &str) -> Streamed {
        let (chat, rows) = panel();
        chat.init_history();
        native::services::TTS_ENABLED.set(true);
        app_settings().set_int(keys::API_PROVIDER.key, 5);
        chat.submit_message(message);
        assert_eq!(
            native::services::take_calls(),
            ["audio stop", "send_chat_streaming"]
        );
        assert!(chat.stream_open.get_untracked());
        let mut driver = Driver::new(chat);
        let mut streamed = Streamed {
            rows: Vec::new(),
            typed: Vec::new(),
            shown: Vec::new(),
            spoken: Vec::new(),
            last: None,
            history_last: None,
        };
        let observe = |streamed: &mut Streamed| {
            let typed = chat.with_inner(|inner| inner.typewriter.full_text());
            note(&mut streamed.typed, typed.expect("the panel is alive"));
            note(&mut streamed.shown, chat.dialogue_text.get_untracked());
            streamed.spoken.extend(drain_syntheses(chat));
            if chat.chat_state.get_untracked() == WaitForAdvance && streamed.last.is_none() {
                streamed.last = Some(chat.dialogue_text.get_untracked());
            }
        };
        let chars: Vec<char> = reply.chars().collect();
        for token in chars.chunks(4) {
            chat.on_stream_token(&token.iter().collect::<String>());
            driver.effect();
            observe(&mut streamed);
            for _ in 0..3 {
                if chat.waiting_for_input.get_untracked() {
                    driver.enter();
                } else {
                    driver.tick();
                }
                observe(&mut streamed);
            }
        }
        chat.on_stream_complete(reply);
        driver.effect();
        observe(&mut streamed);
        assert!(
            !chat.stream_open.get_untracked(),
            "closed by the completion"
        );
        for _ in 0..100 {
            while driver.tick() {
                observe(&mut streamed);
            }
            if chat.chat_state.get_untracked() == InputReady {
                break;
            }
            driver.enter();
            observe(&mut streamed);
        }
        assert_eq!(chat.chat_state.get_untracked(), InputReady);
        assert!(!chat.stream_open.get_untracked());
        streamed.rows = rows.lock().unwrap().clone();
        streamed.history_last = chat.history_snapshot().last().cloned();
        streamed
    }

    #[test]
    fn streamed_replies_reach_screen_backlog_and_tts_without_commands_or_think_text() {
        // The audit's MEMO and THINKAFTER replies (`llm_mock.py`).
        let memo = "[SMILE] 覚えておくわ。[SAVE_FACT: ユーザーの名前は岡部]\n\n\n\
                    [SAVE_EVENT: {\"summary\": \"ラボで実験した\", \"tags\": [\"ラボ\", \"実験\"]}]";
        let think_after =
            "[SMILE] <think>ここは内部の思考メモで表示されてはいけない</think>本文です。";
        for (message, reply, display) in [
            ("MEMO stream", memo, "覚えておくわ。"),
            ("THINKAFTER stream", think_after, "本文です。"),
        ] {
            let owner = Owner::new();
            owner.with(|| {
                let streamed = stream_through_panel(message, reply);
                // The typewriter only ever holds a prefix of the display: no command, no think
                // text, never rewritten.
                for typed in &streamed.typed {
                    assert!(
                        !typed.contains("[SAVE_") && !typed.contains("<think>"),
                        "{message}: typed {typed:?}"
                    );
                }
                assert!(
                    streamed
                        .typed
                        .windows(2)
                        .all(|pair| pair[1].starts_with(&pair[0])),
                    "{message}: the typed text only grows: {:?}",
                    streamed.typed
                );
                assert_eq!(streamed.typed.last().map(String::as_str), Some(display));
                assert!(
                    streamed
                        .shown
                        .iter()
                        .all(|shown| display.starts_with(shown.as_str())),
                    "{message}: shown {:?}",
                    streamed.shown
                );
                assert_eq!(
                    streamed.last.as_deref(),
                    Some(display),
                    "{message}: the final ▼ shows the last page"
                );
                // The backlog: the message, then the reply's page once.
                let rows: Vec<(&str, &str)> = streamed
                    .rows
                    .iter()
                    .map(|(role, text)| (role.as_str(), text.as_str()))
                    .collect();
                assert_eq!(rows, [("user", message), ("assistant", display)]);
                // TTS hears exactly the display.
                assert_eq!(streamed.spoken, [display], "{message}");
                // The completion stored the display in the history.
                assert_eq!(streamed.history_last, Some(ChatMessage::assistant(display)));
            });
        }
        // ... and applied the memory command, and reported the emotion of each reply.
        let facts: Vec<String> = native::managers::memory()
            .payload()
            .user_facts
            .unwrap_or_default()
            .into_iter()
            .map(|fact| fact.content)
            .collect();
        assert_eq!(facts, ["ユーザーの名前は岡部"]);
        let smiles = native::managers::EVENTS.with(|events| {
            events
                .borrow()
                .iter()
                .filter(|event| **event == AchievementEvent::Emotion("SMILE".to_owned()))
                .count()
        });
        assert_eq!(smiles, 2);
    }
}
