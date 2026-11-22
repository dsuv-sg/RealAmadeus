//! Weighted startup events: port of V1 `lib/amadeus/services/random-event-service.ts` (itself
//! the TypeScript port of the Qt `RandomEventService`), plus the injected randomness ([`Rng`])
//! shared by the core modules (`minigame`, `memory`).
//!
//! * The time-of-day bucket is chosen **once**, from the local hour passed to
//!   [`RandomEventService::new`]. V1 built the singleton at page load, so the bucket is frozen at
//!   app start (not at login): construct the service once, when the app starts.
//! * [`RandomEventService::try_trigger_event`] consumes exactly one random number:
//!   `roll = floor(random() * total_weight)`, and the first event whose cumulative weight exceeds
//!   `roll` wins. It always succeeds for the V1 pools.
//! * Texts are translated when read, with the caller's current language
//!   (`t(key, default_text, Config_Language)`); the in-code defaults are fallbacks only (13 of
//!   them differ from `ja.json`, which is what Japanese users see).
//!
//! ChatPanel usage (V1 L1286–1312, on **every** ChatPanel mount): `try_trigger_event`, then
//! [`event_text`](RandomEventService::event_text) /
//! [`event_emotion`](RandomEventService::event_emotion); if the text is non-empty, type it out with
//! that emotion and send the achievement event `random_event`. The greeting is logged to the
//! backlog, but it is not added to the LLM history, not spoken and not passed to
//! `MemoryManager::record_emotion`.

mod data;

use std::cell::Cell;
use std::fmt;

use crate::i18n;
use crate::types::{Emitter, Emotion};

pub use data::{
    AFTERNOON_EVENTS, EVENING_EVENTS, FORENOON_EVENTS, LATE_EVENTS, MORNING_EVENTS, NIGHT_EVENTS,
};

// ---------------------------------------------------------------------------------------------
// Injected randomness
// ---------------------------------------------------------------------------------------------

/// Source of uniformly distributed numbers in `[0, 1)`: V1 `Math.random()`.
///
/// `amadeus-core` has no entropy source, so randomness is injected: in the browser pass
/// `&mut js_sys::Math::random` (any `FnMut() -> f64` is an `Rng`), in tests [`Mulberry32`] or
/// [`SequenceRng`]. The number and order of calls per operation match V1 (quiz: 78, startup
/// event: 1, mood hint: 1), so seeded runs reproduce V1 results exactly.
pub trait Rng {
    /// Next value in `[0, 1)`.
    fn next_f64(&mut self) -> f64;
}

impl<F: FnMut() -> f64> Rng for F {
    fn next_f64(&mut self) -> f64 {
        self()
    }
}

/// `Math.floor(random * n)` as an index, `None` when the value is outside `0..n` (only possible
/// with an RNG that breaks the `[0, 1)` contract, or `NaN`).
pub fn random_index(random: f64, n: usize) -> Option<usize> {
    let index = (random * n as f64).floor();
    (index >= 0.0 && index < n as f64).then_some(index as usize)
}

/// The mulberry32 generator (deterministic). It produced the V1 golden vectors of the specs:
/// `Mulberry32::new(42)` yields `0.6011037519201636, 0.44829055899754167, …`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    /// Generator seeded with `seed`.
    pub const fn new(seed: u32) -> Self {
        Self { state: seed }
    }
}

impl Rng for Mulberry32 {
    fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6D2B_79F5);
        let s = self.state;
        let mut t = (s ^ (s >> 15)).wrapping_mul(1 | s);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

/// Scripted values (tests): returns `values` in order, then `fallback` forever; counts calls.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SequenceRng {
    values: Vec<f64>,
    next: usize,
    fallback: f64,
    calls: usize,
}

impl SequenceRng {
    /// Replays `values`, then returns 0.
    pub fn new(values: impl Into<Vec<f64>>) -> Self {
        Self {
            values: values.into(),
            ..Self::default()
        }
    }

    /// Always returns `value` (`random() ≡ value`).
    pub fn constant(value: f64) -> Self {
        Self::new(Vec::new()).with_fallback(value)
    }

    /// Value returned once the scripted values are used up.
    pub fn with_fallback(mut self, fallback: f64) -> Self {
        self.fallback = fallback;
        self
    }

    /// Number of values drawn so far.
    pub fn calls(&self) -> usize {
        self.calls
    }
}

impl Rng for SequenceRng {
    fn next_f64(&mut self) -> f64 {
        self.calls += 1;
        match self.values.get(self.next) {
            Some(value) => {
                self.next += 1;
                *value
            }
            None => self.fallback,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------------------------

/// One weighted startup event (V1 `RandomEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RandomEvent {
    /// i18n key (`startup_<bucket>_<n>`); every key exists in all 13 locales.
    pub key: &'static str,
    /// In-code Japanese text (fallback of `t`).
    pub default_text: &'static str,
    /// Emotion shown with the text.
    pub emotion: Emotion,
    /// Relative weight (> 0).
    pub weight: u32,
}

impl RandomEvent {
    /// Localized text: `t(key, default_text, lang)`.
    pub fn text(&self, lang: i64) -> &'static str {
        i18n::t(self.key, self.default_text, lang)
    }
}

/// Local-hour bucket of the startup events (frozen at construction, V1 `initEvents`).
///
/// The boundaries differ from `memory::time_context` and from the achievement hours on purpose
/// (V1 parity): 0–4 / 5–9 / 10–13 / 14–17 / 18–21 / 22–23.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeBucket {
    /// 00:00–04:59 (`startup_night_*`, 16 events, Σ133).
    Night,
    /// 05:00–09:59 (`startup_morning_*`, 16 events, Σ135).
    Morning,
    /// 10:00–13:59 (`startup_forenoon_*`, 14 events, Σ119).
    Forenoon,
    /// 14:00–17:59 (`startup_afternoon_*`, 14 events, Σ118).
    Afternoon,
    /// 18:00–21:59 (`startup_evening_*`, 14 events, Σ120).
    Evening,
    /// Everything else, i.e. 22:00–23:59 (`startup_late_*`, 13 events, Σ116).
    Late,
}

impl TimeBucket {
    /// All buckets in hour order.
    pub const ALL: [TimeBucket; 6] = [
        TimeBucket::Night,
        TimeBucket::Morning,
        TimeBucket::Forenoon,
        TimeBucket::Afternoon,
        TimeBucket::Evening,
        TimeBucket::Late,
    ];

    /// Bucket of a local hour (`Date#getHours()`).
    pub fn from_hour(hour: u32) -> Self {
        match hour {
            0..=4 => TimeBucket::Night,
            5..=9 => TimeBucket::Morning,
            10..=13 => TimeBucket::Forenoon,
            14..=17 => TimeBucket::Afternoon,
            18..=21 => TimeBucket::Evening,
            _ => TimeBucket::Late,
        }
    }

    /// The bucket's events, in V1 order.
    pub fn events(self) -> &'static [RandomEvent] {
        match self {
            TimeBucket::Night => &NIGHT_EVENTS,
            TimeBucket::Morning => &MORNING_EVENTS,
            TimeBucket::Forenoon => &FORENOON_EVENTS,
            TimeBucket::Afternoon => &AFTERNOON_EVENTS,
            TimeBucket::Evening => &EVENING_EVENTS,
            TimeBucket::Late => &LATE_EVENTS,
        }
    }

    /// Common key prefix (`"startup_night_"`, …).
    pub fn key_prefix(self) -> &'static str {
        match self {
            TimeBucket::Night => "startup_night_",
            TimeBucket::Morning => "startup_morning_",
            TimeBucket::Forenoon => "startup_forenoon_",
            TimeBucket::Afternoon => "startup_afternoon_",
            TimeBucket::Evening => "startup_evening_",
            TimeBucket::Late => "startup_late_",
        }
    }

    /// Sum of the weights.
    pub fn total_weight(self) -> u32 {
        total_weight(self.events())
    }
}

fn total_weight(events: &[RandomEvent]) -> u32 {
    events
        .iter()
        .fold(0_u32, |sum, event| sum.saturating_add(event.weight))
}

/// V1 weighted pick for one random number in `[0, 1)`: `roll = floor(random * total)`, then the
/// first event whose cumulative weight exceeds `roll`; `None` only for an empty pool (or an
/// out-of-contract random value ≥ 1 / `NaN`, exactly like V1).
pub fn pick_weighted(events: &'static [RandomEvent], random: f64) -> Option<&'static RandomEvent> {
    let roll = (random * f64::from(total_weight(events))).floor();
    let mut cumulative = 0_u32;
    for event in events {
        cumulative = cumulative.saturating_add(event.weight);
        if roll < f64::from(cumulative) {
            return Some(event);
        }
    }
    None
}

/// Payload of [`RandomEventService::event_triggered`] (V1 `eventTriggered(text, emotion)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventTriggered {
    /// Localized text (language passed to `try_trigger_event`).
    pub text: String,
    /// The event's emotion.
    pub emotion: Emotion,
}

/// V1 `RandomEventService` singleton.
pub struct RandomEventService {
    bucket: TimeBucket,
    last: Cell<Option<&'static RandomEvent>>,
    event_triggered: Emitter<EventTriggered>,
}

impl fmt::Debug for RandomEventService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RandomEventService")
            .field("bucket", &self.bucket)
            .field("last", &self.last.get().map(|e| e.key))
            .finish_non_exhaustive()
    }
}

impl RandomEventService {
    /// V1 constructor (`initEvents`): selects the bucket of the local `hour` once.
    pub fn new(hour: u32) -> Self {
        Self {
            bucket: TimeBucket::from_hour(hour),
            last: Cell::new(None),
            event_triggered: Emitter::new(),
        }
    }

    /// The frozen time-of-day bucket.
    pub fn bucket(&self) -> TimeBucket {
        self.bucket
    }

    /// The bucket's events (V1 `events`).
    pub fn events(&self) -> &'static [RandomEvent] {
        self.bucket.events()
    }

    /// V1 `eventTriggered` (never used by the V1 UI).
    pub fn event_triggered(&self) -> &Emitter<EventTriggered> {
        &self.event_triggered
    }

    /// V1 `tryTriggerEvent()`: draws exactly one random number, stores the picked event and
    /// emits [`event_triggered`](Self::event_triggered) with its text in `lang`. Returns `false`
    /// only when nothing was picked (impossible with a conforming RNG).
    pub fn try_trigger_event(&self, rng: &mut dyn Rng, lang: i64) -> bool {
        match pick_weighted(self.events(), rng.next_f64()) {
            Some(event) => {
                self.last.set(Some(event));
                self.event_triggered.emit(&EventTriggered {
                    text: self.event_text(lang).to_owned(),
                    emotion: event.emotion,
                });
                true
            }
            None => false,
        }
    }

    /// The last picked event, if any.
    pub fn last_event(&self) -> Option<&'static RandomEvent> {
        self.last.get()
    }

    /// V1 `getEventKey()`: key of the last event, `""` before the first trigger.
    pub fn event_key(&self) -> &'static str {
        self.last.get().map_or("", |event| event.key)
    }

    /// V1 `getEventText()`: the last event's text translated with the **current** `lang` on every
    /// call; `""` before the first trigger.
    pub fn event_text(&self, lang: i64) -> &'static str {
        self.last.get().map_or("", |event| event.text(lang))
    }

    /// V1 `getEventEmotion()`: upper-case tag of the last event, `""` before the first trigger.
    pub fn event_emotion(&self) -> &'static str {
        self.last.get().map_or("", |event| event.emotion.as_str())
    }
}

#[cfg(test)]
mod tests;
