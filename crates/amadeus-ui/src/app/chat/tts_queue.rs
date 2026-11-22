//! The V1 ChatPanel TTS pipeline (experimental; `RealAmadeusApp.tsx` L1337–1509 and the E4
//! `TTSService` / `AudioService` handlers, spec-app-part2 §8.4–§8.12 and §11).
//!
//! * At most one synthesis is in flight and at most one clip plays; once playback of chunk N has
//!   been requested, synthesis of N+1 starts (pipelining).
//! * `TTSService.synthesize` can answer **synchronously** (cache hit, disabled, empty text) and
//!   `AudioService.playFromFile` can fail synchronously, so these functions never hold the queue
//!   borrow while calling the host: [`TtsHost::with_queue`] closures only mutate the queue.
//! * Session guard (DECISIONS D23, PARITY TTS-006): every reset (`prepareLiveTTSSession` /
//!   `cancelLiveTTS`) starts a new generation, every synthesis is stamped with the generation it
//!   started in, and a result whose stamp is not the current generation is dropped. V1's frozen
//!   closures compared a constant 0, so a stale result was accepted (and played) as soon as the
//!   new session had started a synthesis. `TTSService` reports results without a request id, so a
//!   stamp can only name its result while **one** synthesis is outstanding: a synthesis left over
//!   from before a reset still holds the slot until its (dropped) result arrives, then the new
//!   session's next chunk starts. A result with nothing outstanding is not ours and is dropped.
//! * The streamed tail is always flushed on completion (D23, TTS-003): V1 returned before the
//!   force flush when the completion added no new text, so a last sentence without `。！？!?`
//!   (every English reply that only uses `.`) was never spoken.
//! * Streamed text is captured only once the leading tag is decided (D23,
//!   [`capture_stream_token`]), and what is captured is the typed text
//!   ([`stream_display_prefix`](amadeus_core::chat_utils::stream_display_prefix)): a
//!   half-received tag (`"[SMI"`), a `<think>` block or a memory command is never spoken
//!   (CHAT-028; V1 spoke all of them).

use std::collections::VecDeque;

use amadeus_core::chat_utils::{split_text_for_tts, strip_emotion_tag, TtsStreamCapture};
use amadeus_core::jscompat::js_trim;

/// The TTS refs of V1 ChatPanel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TtsQueue {
    /// `ttsTextQueueRef`.
    text_queue: VecDeque<String>,
    /// `ttsAudioQueueRef`.
    audio_queue: VecDeque<String>,
    /// The TTS session generation (V1 `ttsSessionId`), advanced by every [`reset`](Self::reset).
    generation: u64,
    /// `ttsSynthInFlightRef` + `ttsInFlightSessionIdRef`: the generation of the one outstanding
    /// `TTSService.synthesize` (`None` = none outstanding).
    in_flight: Option<u64>,
    /// `ttsPlaybackBusyRef`.
    playback_busy: bool,
    /// `ttsChunkBufferRef` + `ttsProcessedIndexRef` (streamed replies).
    capture: TtsStreamCapture,
}

impl TtsQueue {
    /// The ref part of `prepareLiveTTSSession` / `cancelLiveTTS`: a new generation, both queues
    /// emptied, playback not busy, the chunk buffer and the processed index cleared. An
    /// outstanding synthesis keeps its (now stale) stamp; see the module docs.
    pub fn reset(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.text_queue.clear();
        self.audio_queue.clear();
        self.playback_busy = false;
        self.capture.reset();
    }

    /// `ttsPlaybackBusyRef.current` (sampled by `isActivelySpeaking`).
    pub fn playback_busy(&self) -> bool {
        self.playback_busy
    }
}

/// What the pipeline needs from the panel and the services.
pub trait TtsHost {
    /// Runs `f` on the queue (`None` once the panel is gone). Must not be re-entered from `f`.
    fn with_queue<R>(&self, f: impl FnOnce(&mut TtsQueue) -> R) -> Option<R>;
    /// `TTSService.enabled`.
    fn tts_enabled(&self) -> bool;
    /// `void TTSService.synthesize(text)` (may call back synchronously).
    fn synthesize(&self, text: &str);
    /// `AudioService.playFromFile(url)` (may call back synchronously).
    fn play(&self, url: &str);
}

/// V1 `processNextTTSChunkIfPossible()`.
pub fn process_next(host: &impl TtsHost) {
    if !host.tts_enabled() {
        return;
    }
    let next = host
        .with_queue(|q| {
            if q.in_flight.is_some() || q.text_queue.is_empty() {
                return None;
            }
            let next = q.text_queue.pop_front()?;
            if js_trim(&next).is_empty() {
                return None;
            }
            q.in_flight = Some(q.generation);
            Some(next)
        })
        .flatten();
    if let Some(text) = next {
        host.synthesize(&text);
    }
}

/// V1 `playNextTTSChunkIfPossible()`.
pub fn play_next(host: &impl TtsHost) {
    enum Action {
        Nothing,
        Process,
        Play(String),
    }
    let action = host
        .with_queue(|q| {
            if q.playback_busy {
                return Action::Nothing;
            }
            match q.audio_queue.pop_front() {
                None => Action::Process,
                Some(url) if url.is_empty() => Action::Nothing,
                Some(url) => {
                    q.playback_busy = true;
                    Action::Play(url)
                }
            }
        })
        .unwrap_or(Action::Nothing);
    match action {
        Action::Nothing => {}
        Action::Process => process_next(host),
        Action::Play(url) => {
            host.play(&url);
            process_next(host);
        }
    }
}

/// V1 `enqueueTTSChunk(text)`.
pub fn enqueue(host: &impl TtsHost, text: &str) {
    let cleaned = js_trim(text);
    if cleaned.is_empty() {
        return;
    }
    host.with_queue(|q| q.text_queue.push_back(cleaned.to_owned()));
    process_next(host);
}

/// V1 `triggerTTSIfEnabled(text, true)` (non-streamed replies): strip tags and think blocks,
/// split with `splitTextForTTS`, enqueue every piece.
pub fn speak_reply(host: &impl TtsHost, text: &str) {
    if !host.tts_enabled() {
        return;
    }
    let cleaned = strip_emotion_tag(text);
    if cleaned.is_empty() {
        return;
    }
    for piece in split_text_for_tts(&cleaned) {
        enqueue(host, &piece);
    }
}

/// V1 `captureStreamTextForTTS(sourceText, complete)` + `flushTTSChunkBuffer`, except that the
/// completion always force-flushes the buffered tail (D23, TTS-003).
pub fn capture_stream_text(host: &impl TtsHost, source_text: &str, complete: bool) {
    if !host.tts_enabled() {
        return;
    }
    let chunks = host
        .with_queue(|q| {
            let mut chunks = q.capture.capture(source_text, complete);
            if complete {
                // A no-op when `capture` already force-flushed (new text in the completion).
                chunks.extend(q.capture.flush(true));
            }
            chunks
        })
        .unwrap_or_default();
    for chunk in chunks {
        enqueue(host, &chunk);
    }
}

/// V1 `handleStreamToken`'s `captureStreamTextForTTS(display, false)`, behind the gate the
/// typing switch uses (D23): nothing is captured until the leading tag is decided (`parsed`).
/// V1's UTF-16 delta spoke a half-received tag once it was consumed (`"[SMIOkay…"`). The first
/// capture after the decision starts at index 0, so no displayable text is lost, and a reply
/// that is never decided while streaming is spoken by the completion flush. `display` is the
/// typed text (CHAT-028), which only grows while the stream is open, so the delta never
/// contains text that is later removed (think blocks, memory commands).
pub fn capture_stream_token(host: &impl TtsHost, display: &str, parsed: bool) {
    if parsed {
        capture_stream_text(host, display, false);
    }
}

/// Frees the synthesis slot for a result and tells whether it belongs to the current session.
fn take_result(q: &mut TtsQueue) -> bool {
    q.in_flight.take() == Some(q.generation)
}

/// `TTSService.synthesisComplete(url)` handler. A stale result is dropped and the current
/// session's next chunk (queued while the slot was held) starts.
pub fn on_synthesis_complete(host: &impl TtsHost, url: &str) {
    let current = host.with_queue(|q| {
        let current = take_result(q);
        if current {
            q.audio_queue.push_back(url.to_owned());
        }
        current
    });
    match current {
        Some(true) => play_next(host),
        Some(false) => process_next(host),
        None => {}
    }
}

/// `TTSService.synthesisError()` handler (the failed chunk is skipped).
pub fn on_synthesis_error(host: &impl TtsHost) {
    if host.with_queue(take_result).is_some() {
        process_next(host);
    }
}

/// `AudioService.playbackStarted` handler.
pub fn on_playback_started(host: &impl TtsHost) {
    host.with_queue(|q| q.playback_busy = true);
}

/// `AudioService.playbackFinished` / `AudioService.errorOccurred` handler.
pub fn on_playback_ended(host: &impl TtsHost) {
    host.with_queue(|q| q.playback_busy = false);
    play_next(host);
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::HashSet;

    use amadeus_core::chat_utils::{
        extract_memory_commands, stream_display_prefix, StreamEmotionDetector,
    };

    use super::*;

    /// Records the service calls; `cached` texts answer synchronously like V1's cache hits and
    /// `fail_play` makes `play` emit `errorOccurred` synchronously.
    #[derive(Default)]
    struct Host {
        queue: RefCell<TtsQueue>,
        enabled: Cell<bool>,
        cached: HashSet<String>,
        fail_play: Cell<bool>,
        calls: RefCell<Vec<String>>,
    }

    impl TtsHost for Host {
        fn with_queue<R>(&self, f: impl FnOnce(&mut TtsQueue) -> R) -> Option<R> {
            Some(f(&mut self.queue.borrow_mut()))
        }
        fn tts_enabled(&self) -> bool {
            self.enabled.get()
        }
        fn synthesize(&self, text: &str) {
            self.calls.borrow_mut().push(format!("synth {text}"));
            if self.cached.contains(text) {
                on_synthesis_complete(self, &format!("blob:{text}"));
            }
        }
        fn play(&self, url: &str) {
            self.calls.borrow_mut().push(format!("play {url}"));
            if self.fail_play.get() {
                on_playback_ended(self);
            }
        }
    }

    fn host() -> Host {
        let host = Host::default();
        host.enabled.set(true);
        host
    }

    fn calls(host: &Host) -> Vec<String> {
        std::mem::take(&mut *host.calls.borrow_mut())
    }

    #[test]
    fn pipelines_one_synthesis_and_one_playback() {
        let host = host();
        speak_reply(&host, "[SMILE] こんにちは。元気？  ええ!");
        assert_eq!(
            calls(&host),
            vec!["synth こんにちは。"],
            "one synthesis in flight"
        );
        on_synthesis_complete(&host, "blob:1");
        assert_eq!(
            calls(&host),
            vec!["play blob:1", "synth 元気？"],
            "next synthesis starts with playback"
        );
        on_playback_started(&host);
        on_synthesis_complete(&host, "blob:2");
        assert!(calls(&host).is_empty(), "audio waits while the clip plays");
        on_playback_ended(&host);
        assert_eq!(calls(&host), vec!["play blob:2", "synth ええ!"]);
        on_synthesis_error(&host);
        on_playback_ended(&host);
        assert!(
            calls(&host).is_empty(),
            "the failed chunk is skipped, nothing left"
        );
        assert!(!host.queue.borrow().playback_busy());
    }

    #[test]
    fn cache_hits_answer_re_entrantly() {
        let mut host = host();
        host.cached.insert("一。".into());
        host.cached.insert("二。".into());
        speak_reply(&host, "一。二。三。");
        // synth 一 → (sync) play 一; synth 二 → (sync) complete while busy → queued, which frees
        // the synthesis slot, so 三 starts right away.
        assert_eq!(
            calls(&host),
            vec!["synth 一。", "play blob:一。", "synth 二。", "synth 三。"]
        );
        on_playback_started(&host);
        on_playback_ended(&host);
        assert_eq!(calls(&host), vec!["play blob:二。"]);
    }

    #[test]
    fn disabled_tts_does_nothing() {
        let host = Host::default();
        speak_reply(&host, "こんにちは。");
        capture_stream_text(&host, "こんにちは。", true);
        assert!(calls(&host).is_empty());
        assert!(
            host.queue.borrow().text_queue.is_empty(),
            "V1 returns before touching the queue"
        );
    }

    #[test]
    fn stale_result_after_a_reset_is_dropped() {
        let host = host();
        speak_reply(&host, "古い。");
        assert_eq!(calls(&host), vec!["synth 古い。"]);
        // New message: prepareLiveTTSSession — the old result is dropped.
        host.queue.borrow_mut().reset();
        on_synthesis_complete(&host, "blob:old");
        assert!(calls(&host).is_empty());
        assert!(host.queue.borrow().audio_queue.is_empty());
        // The slot is free again: the new session synthesizes at once.
        speak_reply(&host, "新しい。");
        assert_eq!(calls(&host), vec!["synth 新しい。"]);
        on_synthesis_complete(&host, "blob:new");
        assert_eq!(calls(&host), vec!["play blob:new"]);
    }

    #[test]
    fn generation_guard_closes_the_v1_race() {
        let host = host();
        speak_reply(&host, "古い。");
        assert_eq!(calls(&host), vec!["synth 古い。"]);
        host.queue.borrow_mut().reset();
        // V1 race: the new session started its synthesis right away (stamp 0 again), so the
        // stale result was accepted and played, and two syntheses overlapped. Now the new chunks
        // wait while the stale synthesis is outstanding ...
        speak_reply(&host, "新しい。次。");
        assert!(calls(&host).is_empty());
        // ... its result is dropped and the new session's first chunk starts.
        on_synthesis_complete(&host, "blob:stale");
        assert_eq!(calls(&host), vec!["synth 新しい。"]);
        on_synthesis_complete(&host, "blob:new");
        assert_eq!(calls(&host), vec!["play blob:new", "synth 次。"]);
        on_synthesis_complete(&host, "blob:next");
        on_playback_ended(&host);
        assert_eq!(calls(&host), vec!["play blob:next"]);

        // A stale error frees the slot the same way (several resets in a row).
        speak_reply(&host, "一。");
        assert_eq!(calls(&host), vec!["synth 一。"]);
        host.queue.borrow_mut().reset();
        host.queue.borrow_mut().reset();
        speak_reply(&host, "二。");
        assert!(calls(&host).is_empty());
        on_synthesis_error(&host);
        assert_eq!(calls(&host), vec!["synth 二。"]);
        on_synthesis_complete(&host, "blob:二");
        assert_eq!(calls(&host), vec!["play blob:二"]);
    }

    #[test]
    fn result_with_nothing_outstanding_is_ignored() {
        let host = host();
        // E.g. a synthesis started by a previous ChatPanel mount.
        on_synthesis_complete(&host, "blob:other");
        on_synthesis_error(&host);
        assert!(calls(&host).is_empty());
        assert!(host.queue.borrow().audio_queue.is_empty());
    }

    #[test]
    fn double_audio_error_starts_the_next_clip_early() {
        let host = host();
        speak_reply(&host, "一。二。");
        on_synthesis_complete(&host, "blob:1");
        on_synthesis_complete(&host, "blob:2");
        assert_eq!(
            calls(&host),
            vec!["synth 一。", "play blob:1", "synth 二。"]
        );
        // Q-A5: one failure emits errorOccurred twice → the second one plays clip 2 at once.
        on_playback_ended(&host);
        assert_eq!(calls(&host), vec!["play blob:2"]);
        on_playback_ended(&host);
        assert!(calls(&host).is_empty());
    }

    #[test]
    fn synchronous_play_failure_moves_on() {
        let host = host();
        host.fail_play.set(true);
        speak_reply(&host, "一。二。");
        on_synthesis_complete(&host, "blob:1");
        assert_eq!(
            calls(&host),
            vec!["synth 一。", "play blob:1", "synth 二。"]
        );
    }

    #[test]
    fn streamed_text_is_cut_by_sentence_and_the_tail_is_flushed_on_completion() {
        let host = host();
        capture_stream_text(&host, "「そうね。」それで", false);
        assert_eq!(calls(&host), vec!["synth 「そうね。」"]);
        on_synthesis_error(&host);
        // Completion with the same text: no delta (V1 returned before the force flush and never
        // spoke "それで"); D23 flushes the tail anyway.
        capture_stream_text(&host, "「そうね。」それで", true);
        assert_eq!(calls(&host), vec!["synth それで"]);
        assert_eq!(host.queue.borrow().capture.pending(), "");
        on_synthesis_error(&host);

        // An English reply with only `.`: nothing during the stream, all of it at completion.
        host.queue.borrow_mut().reset();
        capture_stream_text(&host, "Okay. This is fine. Really fine", false);
        assert!(calls(&host).is_empty());
        capture_stream_text(&host, "Okay. This is fine. Really fine", true);
        assert_eq!(calls(&host), vec!["synth Okay. This is fine. Really fine"]);
        on_synthesis_error(&host);

        // New text in the completion: one force flush, no duplicate.
        host.queue.borrow_mut().reset();
        capture_stream_text(&host, "それで", false);
        capture_stream_text(&host, "それでね", true);
        assert_eq!(calls(&host), vec!["synth それでね"]);
    }

    /// Feeds `tokens` the way `Chat::on_stream_token` / `on_stream_complete` do (the typed
    /// text of the raw stream, behind the `parsed` gate; the completion display at the end) and
    /// returns every synthesized text (each result fails, so the queue moves on).
    fn stream_and_drain(host: &Host, tokens: &[&str]) -> Vec<String> {
        let mut detector = StreamEmotionDetector::new();
        let mut raw = String::new();
        for token in tokens {
            detector.push_token(token);
            raw.push_str(token);
            capture_stream_token(host, &stream_display_prefix(&raw), detector.is_parsed());
        }
        let raw = detector.completion_raw(&tokens.concat());
        let cleaned = strip_emotion_tag(&extract_memory_commands(&raw).cleaned_text);
        capture_stream_text(host, &cleaned, true);
        let mut spoken = Vec::new();
        loop {
            let batch = calls(host);
            if batch.is_empty() {
                return spoken;
            }
            spoken.extend(batch);
            on_synthesis_error(host);
        }
    }

    #[test]
    fn streamed_text_waits_for_the_leading_tag() {
        // A tag split across tokens: V1 (and V2 before the gate) captured "[SMI" and then the
        // post-tag text from index 4, so it spoke "[SMIOkay…".
        let host = host();
        let spoken = stream_and_drain(
            &host,
            &[
                "[SMI", "LE] ", "Okay", ". Th", "is i", "s fi", "ne. ", "Real", "ly f", "ine",
            ],
        );
        assert_eq!(spoken, vec!["synth Okay. This is fine. Really fine"]);

        // An open `<think>` block before the tag is never spoken.
        host.queue.borrow_mut().reset();
        let spoken = stream_and_drain(
            &host,
            &[
                "<think>",
                "secret plan",
                "</think>",
                "[SUR",
                "PRISED] え",
                "っ！本当",
                "に？",
            ],
        );
        assert_eq!(spoken, vec!["synth えっ！", "synth 本当に？"]);

        // A reply too short to be decided while streaming is spoken by the completion flush.
        host.queue.borrow_mut().reset();
        assert_eq!(stream_and_drain(&host, &["H", "i"]), vec!["synth Hi"]);
    }

    /// The reply in `n`-character chunks (the audit mock streams 4).
    fn chunks(reply: &str, n: usize) -> Vec<String> {
        let chars: Vec<char> = reply.chars().collect();
        chars.chunks(n).map(|c| c.iter().collect()).collect()
    }

    #[test]
    fn streamed_memory_commands_and_think_text_are_never_spoken() {
        // CHAT-028 / converge OPEN: V1 (and V2 before the fix) synthesized both commands and the
        // think text after the leading tag.
        let cases = [
            (
                "[SMILE] 覚えておくわ。[SAVE_FACT: ユーザーの名前は岡部]\n\n\n\
                 [SAVE_EVENT: {\"summary\": \"ラボで実験した\", \"tags\": [\"ラボ\", \"実験\"]}]",
                vec!["synth 覚えておくわ。"],
            ),
            (
                "[SMILE] <think>ここは内部の思考メモで表示されてはいけない</think>本文です。",
                vec!["synth 本文です。"],
            ),
            (
                "<think>internal [SAVE_FACT: 好物はプリン]</think>[BLUSH] な、何よ。",
                vec!["synth な、何よ。"],
            ),
            // The streamed chunker keeps inner spaces (V1 `captureStreamTextForTTS`).
            ("前置き [WINK] 後ろ", vec!["synth 前置き  後ろ"]),
        ];
        for (reply, expected) in cases {
            for n in [1, 2, 3, 4, 7] {
                let host = host();
                let tokens = chunks(reply, n);
                let tokens: Vec<&str> = tokens.iter().map(String::as_str).collect();
                assert_eq!(
                    stream_and_drain(&host, &tokens),
                    expected,
                    "{reply:?} by {n}"
                );
            }
        }
    }
}
