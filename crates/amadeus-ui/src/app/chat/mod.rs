//! V1 `ChatPanel` (`RealAmadeusApp.tsx` L816–1994, spec-app-part2): the conversation screen —
//! Live2D model, dialogue box and typewriter, input bar, MINI GAME / ACHIEVEMENTS triggers, the
//! always-mounted mini game and the achievement toast. OWNED BY: chat engineer (this directory).
//!
//! * [`controller`]: state, refs and V1's callbacks (`submitMessage`, `onAdvance`, the AI / TTS /
//!   STT handlers…), with V1's effective behaviour, except that streamed replies follow
//!   DECISIONS D23 (spec §6.4 Option B) rather than V1's frozen closures.
//! * [`typewriter`]: the pure `step()` engine. [`tts_queue`]: the TTS pipeline.
//! * [`listener`]: owned `window` listeners and JS timer semantics.
//! * [`blocks`]: the dialogue / input / trigger blocks are one branch, swapped so that native Tab
//!   focus lands where it did in V1 (KEY-009).
//!
//! The effects below keep V1's declaration order and dependency arrays (E1–E14, spec §5); they
//! read their dependencies tracked and do the work `untrack`ed. Like V1's `useEffect`, the global
//! keyboard listener (E14) is removed and re-added whenever its dependencies change, so it sees
//! the values of the last "render" and keeps V1's position in the `window` listener list (e.g. the
//! Enter that closes the PC menu never advances the chat). Leptos event delegation is off in this
//! workspace, so the input's own Enter handler runs at the element, before every `window`
//! listener (React's root listener ran before them too).

pub mod blocks;
pub mod controller;
pub mod listener;
pub mod tts_queue;
pub mod typewriter;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use amadeus_core::i18n::{t, tr};
use amadeus_core::jscompat::js_trim;
use amadeus_core::settings::jsnum::js_number_to_string;
use amadeus_core::settings::keys;
use amadeus_core::types::{ConnectionId, LlmStatus};
use leptos::either::EitherOf3;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use self::blocks::{chat_block, ChatBlock};
use self::controller::{cancel_available, put, Chat, ChatProps, ChatState, KeySnapshot, SLEEPING};
use self::listener::{js_math_round, set_js_timeout, WindowListener};
use super::managers;
use super::panels::minigame::MiniGamePanel;
use super::state::{ConversationBridge, OverlayPanel};
use crate::components::live2d_canvas::Live2DCanvas;
use crate::hooks::app_settings;
use crate::hooks::settings::use_app_setting_int;
use crate::hooks::viewport::use_is_mobile_viewport;
use crate::services;

/// E5: the waiting dots (`".".repeat(dots)`).
const DOTS: [&str; 3] = [".", "..", "..."];
/// E5 interval (ms).
const DOTS_INTERVAL_MS: u64 = 400;
/// E2 interval (ms).
const IDLE_TICK_MS: u64 = 1000;
/// E10: toast visible time (ms).
const TOAST_MS: f64 = 4000.0;

thread_local! {
    /// Id of the mount whose bridge is in `conversation_ref` (0 = none).
    static BRIDGE_OWNER: Cell<u64> = const { Cell::new(0) };
    static NEXT_BRIDGE_ID: Cell<u64> = const { Cell::new(0) };
}

/// E4's 13 service subscriptions.
#[derive(Clone, Copy)]
struct Subscriptions {
    response: ConnectionId,
    token: ConnectionId,
    complete: ConnectionId,
    error: ConnectionId,
    synthesis_complete: ConnectionId,
    synthesis_error: ConnectionId,
    transcription: ConnectionId,
    transcription_error: ConnectionId,
    recording_state: ConnectionId,
    recording_progress: ConnectionId,
    playback_started: ConnectionId,
    playback_finished: ConnectionId,
    audio_error: ConnectionId,
}

impl Subscriptions {
    fn connect(chat: Chat) -> Self {
        let ai = services::ai();
        let tts = services::tts();
        let stt = services::stt();
        let audio = services::audio();
        Self {
            response: ai
                .response_received
                .connect(move |response: &str| chat.on_response_received(response)),
            token: ai
                .stream_token
                .connect(move |token: &str| chat.on_stream_token(token)),
            complete: ai
                .stream_complete
                .connect(move |response: &str| chat.on_stream_complete(response)),
            error: ai
                .error_occurred
                .connect(move |error: &str| chat.on_api_error(error)),
            synthesis_complete: tts
                .synthesis_complete
                .connect(move |url: &str| tts_queue::on_synthesis_complete(&chat, url)),
            synthesis_error: tts
                .synthesis_error
                .connect(move |_: &str| tts_queue::on_synthesis_error(&chat)),
            transcription: stt
                .transcription_complete
                .connect(move |text: &str| chat.on_transcription(text)),
            transcription_error: stt
                .transcription_error
                .connect(move |_: &str| chat.on_transcription_error()),
            recording_state: stt
                .recording_state_changed
                .connect(move |_: &()| chat.on_recording_state_changed()),
            recording_progress: stt
                .recording_progress
                .connect(move |level: &f64| chat.on_recording_progress(*level)),
            playback_started: audio
                .playback_started
                .connect(move |_: &()| tts_queue::on_playback_started(&chat)),
            playback_finished: audio
                .playback_finished
                .connect(move |_: &()| tts_queue::on_playback_ended(&chat)),
            audio_error: audio
                .error_occurred
                .connect(move |_: &str| tts_queue::on_playback_ended(&chat)),
        }
    }

    fn disconnect(self) {
        let ai = services::ai();
        ai.response_received.disconnect(self.response);
        ai.stream_token.disconnect(self.token);
        ai.stream_complete.disconnect(self.complete);
        ai.error_occurred.disconnect(self.error);
        let tts = services::tts();
        tts.synthesis_complete.disconnect(self.synthesis_complete);
        tts.synthesis_error.disconnect(self.synthesis_error);
        let stt = services::stt();
        stt.transcription_complete.disconnect(self.transcription);
        stt.transcription_error.disconnect(self.transcription_error);
        stt.recording_state_changed.disconnect(self.recording_state);
        stt.recording_progress.disconnect(self.recording_progress);
        let audio = services::audio();
        audio.playback_started.disconnect(self.playback_started);
        audio.playback_finished.disconnect(self.playback_finished);
        audio.error_occurred.disconnect(self.audio_error);
    }
}

#[allow(clippy::too_many_arguments)]
#[component]
pub fn ChatPanel(
    #[prop(into)] auto_mode: Signal<bool>,
    #[prop(into)] auto_speed: Signal<f64>,
    #[prop(into)] config_language: Signal<i32>,
    #[prop(into)] menu_panel_open: Signal<bool>,
    /// `(role, message)` — V1 `onBackLogAdd`.
    #[prop(into)]
    on_back_log_add: Callback<(String, String)>,
    #[prop(into)] on_llm_status_update: Callback<LlmStatus>,
    #[prop(into)] active_overlay: Signal<OverlayPanel>,
    #[prop(optional, into)] set_active_overlay: Option<Callback<OverlayPanel>>,
    /// V1 `conversationRef`; ChatPanel fills it on mount and clears it on unmount.
    conversation_ref: StoredValue<Option<ConversationBridge>, LocalStorage>,
) -> impl IntoView {
    // Props as memos: the effects must re-run only when a value really changes (V1 deps).
    let menu_open = Memo::new(move |_| menu_panel_open.get());
    let auto_mode = Memo::new(move |_| auto_mode.get());
    let auto_speed = Memo::new(move |_| auto_speed.get());
    let language = Memo::new(move |_| config_language.get());
    let active_overlay = Memo::new(move |_| active_overlay.get());

    let chat = Chat::new(ChatProps {
        menu_open,
        on_back_log_add,
        on_llm_status_update,
        set_active_overlay,
    });
    let is_mobile = use_is_mobile_viewport();
    // `showAudioInput`: read once at mount and never updated. Nothing in V1 writes the key, so the
    // mic button stays hidden unless localStorage was edited (DECISIONS D14).
    let show_audio_input = app_settings().get_int(keys::AUDIO_INPUT_ENABLED.key, 0) == 1;
    let eye_tracking = use_app_setting_int(keys::EYE_TRACKING.key, 0);
    let chat_language_setting = use_app_setting_int(keys::CHAT_LANGUAGE.key, 0);
    let speaking = Memo::new(move |_| chat.is_actively_speaking());

    // E1: window activity listeners (registered once, before E14's keyboard listener).
    let activity = StoredValue::new_local(Vec::<WindowListener>::new());
    Effect::new(move |_| {
        untrack(move || {
            let mut listeners = Vec::with_capacity(5);
            listeners.extend(WindowListener::new(
                "mousemove",
                move |event: web_sys::Event| {
                    if let Some(mouse) = event.dyn_ref::<web_sys::MouseEvent>() {
                        chat.on_mouse_move(mouse.client_x(), mouse.client_y());
                    }
                },
            ));
            for name in ["mousedown", "keydown", "touchstart", "scroll"] {
                listeners.extend(WindowListener::new(name, move |_| chat.reset_idle_timer()));
            }
            activity.try_set_value(listeners);
            on_cleanup(move || {
                activity.try_update_value(Vec::clear);
            });
        });
    });

    // E2: idle tick, re-created whenever the state or the emotion changes.
    Effect::new(move |_| {
        let state = chat.chat_state.get();
        let emotion = chat.emotion.get();
        untrack(move || {
            let handle = set_interval_with_handle(
                move || chat.idle_tick(state, &emotion),
                Duration::from_millis(IDLE_TICK_MS),
            )
            .ok();
            on_cleanup(move || {
                if let Some(handle) = handle {
                    handle.clear();
                }
            });
        });
    });

    // E3: overlay / menu changes count as activity.
    Effect::new(move |_| {
        active_overlay.track();
        menu_open.track();
        untrack(move || chat.reset_idle_timer());
    });

    // E4: initial history, service subscriptions, initial LLM status; unmount: disconnect, stop
    // recording, cancel the stream (TTS audio keeps playing, as in V1).
    Effect::new(move |_| {
        untrack(move || {
            chat.init_history();
            let subscriptions = Subscriptions::connect(chat);
            chat.update_status(0.0);
            on_cleanup(move || {
                subscriptions.disconnect();
                services::stt().stop_recording();
                services::ai().cancel_streaming();
            });
        });
    });

    // E5: waiting dots `.` → `..` → `...` every 400 ms while `waitingAPI`.
    Effect::new(move |_| {
        if chat.chat_state.get() != ChatState::WaitingApi {
            return;
        }
        untrack(move || {
            let dots = Cell::new(1usize);
            let handle = set_interval_with_handle(
                move || {
                    let next = dots.get() % 3 + 1;
                    dots.set(next);
                    put(chat.waiting_indicator, DOTS[next - 1]);
                },
                Duration::from_millis(DOTS_INTERVAL_MS),
            )
            .ok();
            on_cleanup(move || {
                if let Some(handle) = handle {
                    handle.clear();
                }
            });
        });
    });

    // E6: the typewriter. A (re)start clears the pending step and steps right away.
    Effect::new(move |_| {
        let state = chat.chat_state.get();
        let menu = menu_open.get();
        let waiting = chat.waiting_for_input.get();
        untrack(move || {
            chat.clear_typing_timeout();
            if state == ChatState::Typing && !menu && !waiting {
                chat.typewriter_step();
            }
        });
    });
    on_cleanup(move || chat.clear_typing_timeout());

    // E7: auto mode — after `autoSpeed` s, advance a pause or return to the input.
    Effect::new(move |_| {
        let auto = auto_mode.get();
        let speed = auto_speed.get();
        let state = chat.chat_state.get();
        let menu = menu_open.get();
        let waiting = chat.waiting_for_input.get();
        let should_run =
            state == ChatState::WaitForAdvance || (state == ChatState::Typing && waiting);
        if !auto || menu || !should_run {
            return;
        }
        untrack(move || {
            let handle = set_js_timeout(move || chat.auto_advance(state, waiting), speed * 1000.0);
            on_cleanup(move || {
                if let Some(handle) = handle {
                    handle.clear();
                }
            });
        });
    });

    // E9: achievement toast (every unlock; the last one of a synchronous batch stays visible).
    Effect::new(move |_| {
        untrack(move || {
            managers::connect_scoped(
                || managers::achievements().achievement_unlocked(),
                move |unlocked| {
                    chat.on_achievement_unlocked(&unlocked.title, &unlocked.description)
                },
            );
        });
    });

    // E10: hide the toast 4 s after the last unlock.
    Effect::new(move |_| {
        if !chat.toast.with(|toast| toast.visible) {
            return;
        }
        untrack(move || {
            let handle = set_js_timeout(move || chat.hide_toast(), TOAST_MS);
            on_cleanup(move || {
                if let Some(handle) = handle {
                    handle.clear();
                }
            });
        });
    });

    // E11: refresh the system prompt when the display or chat language changes.
    Effect::new(move |_| {
        let key = format!("{}:{}", language.get(), chat_language_setting.get());
        // `updateSystemPrompt` changed identity with `turnCount` (a no-op through the key check).
        chat.turn_count.track();
        untrack(move || chat.on_language_key(key));
    });

    // E12: the startup greeting (random event).
    Effect::new(move |_| untrack(move || chat.startup_event()));

    // E13: the conversation bridge for the root's export / import.
    Effect::new(move |_| {
        untrack(move || {
            let id = NEXT_BRIDGE_ID.with(|next| {
                next.set(next.get().wrapping_add(1));
                next.get()
            });
            BRIDGE_OWNER.with(|owner| owner.set(id));
            conversation_ref.try_set_value(Some(ConversationBridge {
                get_history: Rc::new(move || chat.history_snapshot()),
                get_turn_count: Rc::new(move || chat.turn_count_value()),
                apply_imported: Rc::new(move |messages, turn_count| {
                    chat.apply_imported(messages, turn_count)
                }),
            }));
            on_cleanup(move || {
                // `conversationRef.current = null` — unless a newer mount already installed its
                // bridge (V1 ran the old cleanup first, so the ref always ended up set).
                if BRIDGE_OWNER.with(|owner| owner.get()) == id {
                    BRIDGE_OWNER.with(|owner| owner.set(0));
                    conversation_ref.try_set_value(None);
                }
            });
        });
    });

    // E14: global keyboard (Ctrl+C cancel, Enter submit / advance), re-registered on every change
    // of V1's dependencies `[chatInput, chatState, menuPanelOpen, miniGameOpen, onAdvance,
    // submitMessage]` with the values of that moment.
    let key_listener = StoredValue::new_local(None::<WindowListener>);
    Effect::new(move |_| {
        let snapshot = KeySnapshot {
            chat_input: chat.chat_input.get(),
            chat_state: chat.chat_state.get(),
            menu_open: menu_open.get(),
            mini_game_open: chat.mini_game_open.get(),
            // D23 (STR-005): Ctrl+C also cancels a streamed reply that is being typed.
            stream_open: chat.stream_open.get(),
        };
        // `onAdvance` ← waitingForInput; `submitMessage` ← conversationHistory, ttsSessionId
        // (prepareLiveTTSSession), turnCount (updateSystemPrompt).
        chat.waiting_for_input.track();
        chat.history_rev.track();
        chat.turn_count.track();
        chat.tts_session.track();
        untrack(move || {
            key_listener.try_update_value(|slot| {
                *slot = None;
                *slot =
                    WindowListener::keydown(move |event| chat.on_global_keydown(event, &snapshot));
            });
        });
    });
    on_cleanup(move || {
        key_listener.try_update_value(|slot| *slot = None);
    });

    // Live2D props (V1 L1865–1870).
    let live2d_emotion = Signal::derive(move || {
        if chat.mini_game_open.get() {
            chat.mini_game_emotion.get()
        } else {
            chat.emotion.get()
        }
    });
    let live2d_lip_sync = Signal::derive(move || {
        if chat.mini_game_open.get() {
            chat.mini_speak.get()
        } else if speaking.get() {
            1.0
        } else {
            0.0
        }
    });
    let live2d_spoken_char = Signal::derive(move || {
        if !chat.mini_game_open.get() && speaking.get() {
            chat.spoken_char.get()
        } else {
            String::new()
        }
    });
    let eye_tracking_enabled = Signal::derive(move || eye_tracking.get() == 1);

    let sleeping = move || chat.emotion.with(|emotion| emotion == SLEEPING);
    let input_panel_class = if show_audio_input {
        "input-panel has-audio-input"
    } else {
        "input-panel "
    };

    // V1's three conditional blocks (L1873–1976) are one branch: a change of block is one swap,
    // after which `blocks::after_swap` re-inserts the new block in place, so that the Tab focus
    // starting point ends up where React's commit left it (KEY-009). It reads the block from the
    // DOM, between `.live2d-shift` and the mini game's root, so these two must stay right before
    // and right after `{chat_blocks}` in the view below.
    let block = Memo::new(move |_| chat_block(chat.chat_state.get(), chat.mini_game_open.get()));
    let live2d_shift_ref = NodeRef::<html::Div>::new();
    let chat_blocks = {
        let mut mounted = false;
        move || {
            let current = block.get();
            if mounted {
                blocks::after_swap(move || {
                    live2d_shift_ref
                        .try_get_untracked()
                        .flatten()
                        .map(Into::into)
                });
            }
            mounted = true;
            match current {
                ChatBlock::Dialogue => EitherOf3::A(view! {
                    <div class="dialogue-panel">
                        <button
                            type="button"
                            class="dialogue-advance-hitbox"
                            on:click=move |_| chat.on_advance()
                            aria-label=move || tr("会話を進める", "Advance dialogue", Some("advance_dialogue"), config_language.get())
                        ></button>
                        <div class="name-text">{move || t("amadeus_kurisu", "アマデウス紅莉栖", config_language.get())}</div>
                        <div class="dialogue-text">{move || chat.dialogue_text.get()}</div>
                        <div class="waiting-indicator">
                            {move || {
                                let state = chat.chat_state.get();
                                if state == ChatState::WaitingApi
                                    || state == ChatState::WaitForAdvance
                                    || chat.waiting_for_input.get()
                                {
                                    chat.waiting_indicator.get()
                                } else {
                                    ""
                                }
                            }}
                        </div>
                        // V1 shows these in `waitingAPI`; D23 keeps them while a streamed reply is
                        // typed with its stream still open (STR-005).
                        {move || {
                            cancel_available(chat.chat_state.get(), chat.stream_open.get())
                                .then(|| {
                                    if is_mobile.get() {
                                        view! {
                                            <button
                                                class="cancel-streaming-button"
                                                on:click=move |event: web_sys::MouseEvent| {
                                                    event.prevent_default();
                                                    chat.cancel_waiting();
                                                }
                                            >
                                                {move || t("cancel_button", "キャンセル", config_language.get())}
                                            </button>
                                        }
                                            .into_any()
                                    } else {
                                        view! {
                                            <div class="pc-cancel-hint">
                                                {move || t("cancel_hint", "CTRL+Cでキャンセル", config_language.get())}
                                            </div>
                                        }
                                            .into_any()
                                    }
                                })
                        }}
                    </div>
                }),
                ChatBlock::Input => EitherOf3::B(view! {
                    <div class=input_panel_class>
                        <input
                            prop:value=move || chat.chat_input.get()
                            on:input=move |event| {
                                put(chat.chat_input, event_target_value(&event));
                                chat.reset_idle_timer();
                            }
                            placeholder=move || {
                                if sleeping() {
                                    t("sleeping_placeholder", "Zzz... (キー入力やマウス移動で起こす)", config_language.get())
                                } else {
                                    t("enter_message", "メッセージを入力", config_language.get())
                                }
                            }
                            class=move || if sleeping() { "chat-input sleeping-input" } else { "chat-input " }
                            on:keydown=move |event: web_sys::KeyboardEvent| {
                                if event.key() == "Enter" {
                                    event.prevent_default();
                                    chat.submit_message(&chat.chat_input.get_untracked());
                                }
                            }
                        />
                        <div class="chat-input-actions">
                            {show_audio_input
                                .then(|| {
                                    view! {
                                        <button
                                            type="button"
                                            class=move || {
                                                if chat.stt_recording.get() { "chat-mic-button recording" } else { "chat-mic-button " }
                                            }
                                            on:click=move |_| chat.toggle_stt()
                                            aria-label=move || tr("音声入力", "Speech input", Some("speech_input"), config_language.get())
                                        >
                                            {move || if chat.stt_recording.get() { "■" } else { "🎤" }}
                                        </button>
                                    }
                                })}
                            <button
                                type="button"
                                class="chat-send-button"
                                on:click=move |_| chat.submit_message(&chat.chat_input.get_untracked())
                                disabled=move || chat.chat_input.with(|input| js_trim(input).is_empty())
                            >
                                {move || tr("送信", "SEND", Some("send"), config_language.get())}
                            </button>
                        </div>
                        <Show when=move || chat.stt_recording.get()>
                            <div class="chat-mic-level">
                                <span style:width=move || {
                                    format!("{}%", js_number_to_string(js_math_round(chat.stt_level.get() * 100.0)))
                                }></span>
                            </div>
                        </Show>
                        <Show when=move || chat.stt_status.with(|status| !status.is_empty())>
                            <div class="chat-stt-hint">{move || chat.stt_status.get()}</div>
                        </Show>
                    </div>
                    <button type="button" class="mini-game-button" on:click=move |_| put(chat.mini_game_open, true)>
                        "MINI GAME"
                    </button>
                    <button
                        type="button"
                        class="mobile-achievements-trigger-button"
                        on:click=move |_| {
                            if let Some(set_overlay) = chat.props.set_active_overlay {
                                let _ = set_overlay.try_run(OverlayPanel::Achievements);
                            }
                        }
                    >
                        "ACHIEVEMENTS"
                    </button>
                }),
                // The mini game is open.
                ChatBlock::Hidden => EitherOf3::C(()),
            }
        }
    };

    view! {
        <div class="chat-panel-root">
            <div
                class=move || if chat.mini_game_open.get() { "live2d-shift live2d-shifted" } else { "live2d-shift" }
                node_ref=live2d_shift_ref
            >
                <Live2DCanvas
                    emotion=live2d_emotion
                    lip_sync_value=live2d_lip_sync
                    spoken_char=live2d_spoken_char
                    eye_tracking_enabled=eye_tracking_enabled
                />
            </div>

            {chat_blocks}

            <MiniGamePanel
                visible=chat.mini_game_open
                config_language=config_language
                on_close=Callback::new(move |()| {
                    put(chat.mini_game_open, false);
                    if let Some(set_overlay) = chat.props.set_active_overlay {
                        let _ = set_overlay.try_run(OverlayPanel::None);
                    }
                })
                on_emotion_change=Callback::new(move |emotion: String| put(chat.mini_game_emotion, emotion))
                on_speak_value_change=Callback::new(move |value: f64| put(chat.mini_speak, value))
            />

            <div
                class=move || if chat.toast.with(|toast| toast.visible) { "achievement-toast visible" } else { "achievement-toast" }
                role="status"
            >
                <div class="achievement-toast-title">{move || chat.toast.with(|toast| format!("★ {}", toast.title))}</div>
                <div class="achievement-toast-desc">{move || chat.toast.with(|toast| toast.description.clone())}</div>
            </div>
        </div>
    }
}
