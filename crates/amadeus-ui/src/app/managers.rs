//! App-wide singletons of the core managers — V1's module-level `MemoryManager`,
//! `AchievementManager`, `RandomEventService` and `VectorDatabase` objects.
//!
//! V1 constructed each of them when the `/amadeus/app` bundle was evaluated, i.e. before the
//! first render of `RealAmadeusApp`, together with the service singletons. V2 creates them
//! lazily; the root component calls [`init`] when the `/app` route mounts. It builds the managers
//! and the [`services`] singletons in V1's module-evaluation order, so construction time matches
//! V1: launch streak, `AudioService`'s one read of `Config_VoiceVol`, the STT / TTS / RAG
//! constructor health checks, memory/vector `load()`, and the random-event hour bucket. Every
//! accessor returns a `&'static` (page lifetime, thread-local). The managers use interior
//! mutability with `&self` methods and are safe to call back into from their own signal
//! handlers.
//!
//! **Leptos `Callback`s need `Send + Sync` closures:** never capture the returned references in
//! a `Callback`; call the accessor inside the closure instead. Subscribe from components with
//! [`connect_scoped`] (disconnects on cleanup).
//!
//! OWNED BY: integrator. Additive changes only; tell the integrator what you need.

use std::cmp::Ordering;

use amadeus_core::achievements::{AchievementEvent, AchievementManager};
use amadeus_core::memory::{save_failed_warning, LocalDateTime, MemoryManager};
use amadeus_core::random_events::RandomEventService;
use amadeus_core::types::{ConnectionId, Emitter};
use amadeus_core::vector::VectorDatabase;
use wasm_bindgen::JsValue;

use crate::hooks::app_settings;
use crate::i18n::config_language_untracked;
use crate::services;
use crate::storage::{local_store, LocalStorageKv};

/// V1 `MemoryManager`.
pub type Memory = MemoryManager<'static, LocalStorageKv>;
/// V1 `AchievementManager`.
pub type Achievements = AchievementManager<'static, LocalStorageKv>;
/// V1 `VectorDatabase` (store/search logic; HTTP lives in [`mod@services::rag`]).
pub type Vectors = VectorDatabase<'static, LocalStorageKv>;

thread_local! {
    static MEMORY: &'static Memory = {
        let memory: &'static Memory = Box::leak(Box::new(MemoryManager::new(local_store(), &local_now())));
        if let Some(collator) = intl_ja_collator() {
            memory.set_collator(collator);
        }
        // V1 `saveMemory()` warned on every failed write and carried on.
        memory
            .save_failed()
            .connect(|error: &str| log::warn!("{}", save_failed_warning(error)));
        memory
    };
    static ACHIEVEMENTS: &'static Achievements =
        Box::leak(Box::new(AchievementManager::new(local_store(), &local_now())));
    static RANDOM_EVENTS: &'static RandomEventService =
        Box::leak(Box::new(RandomEventService::new(local_now().hour)));
    static VECTORS: &'static Vectors =
        Box::leak(Box::new(VectorDatabase::new(local_store(), app_settings())));
    static WIRED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Creates every manager and service singleton and wires the V1 `AppSettings.settingsChanged`
/// subscriptions (achievements: `Config_Language` → `achievementsChanged`; vector DB:
/// `Config_Experimental_RAG_*` → re-sync) and the RAG health mirror. Idempotent. Call from the
/// `/app` root before anything else.
///
/// The order is V1's module evaluation order (the imports of `RealAmadeusApp.tsx`,
/// spec-lib-core §1.10). With STT, TTS and RAG enabled, the constructor health checks
/// therefore go out as STT, TTS, RAG, before the root's startup checks (TTS, STT, RAG), as in V1.
pub fn init() {
    let _ = memory();
    let _ = achievements();
    let _ = services::ai();
    // Reads `Config_VoiceVol` once: a later change only applies after a reload (QUIRK-14).
    let _ = services::audio();
    let _ = services::stt();
    let _ = services::tts();
    // V1 had ONE VectorDatabase object doing both the store logic and the HTTP health check.
    // V2 splits it: `vectors()` (logic, `load()`) and `services::rag()` (HTTP; its constructor
    // health-checks when RAG is enabled, like V1's constructor tail).
    let _ = vectors();
    let rag = services::rag();
    let _ = random_events();
    if WIRED.with(|w| w.replace(true)) {
        return;
    }
    app_settings().settings_changed().connect(|key: &str| {
        achievements().handle_setting_changed(key);
        vectors().handle_setting_changed(key, app_settings());
    });
    // Mirror the RAG client's availability into the vector store.
    vectors().set_server_available(rag.server_available());
    rag.server_available_changed.connect(|_: &()| {
        vectors().set_server_available(services::rag().server_available());
    });
}

/// V1 `MemoryManager` singleton.
pub fn memory() -> &'static Memory {
    MEMORY.with(|m| *m)
}

/// V1 `AchievementManager` singleton (constructing it updates the launch streak).
pub fn achievements() -> &'static Achievements {
    ACHIEVEMENTS.with(|a| *a)
}

/// V1 `RandomEventService` singleton (hour bucket frozen at construction).
pub fn random_events() -> &'static RandomEventService {
    RANDOM_EVENTS.with(|r| *r)
}

/// V1 `VectorDatabase` singleton (logic half).
pub fn vectors() -> &'static Vectors {
    VECTORS.with(|v| *v)
}

/// V1 `VectorDatabase.checkServerHealth()` (root effect at startup, setters).
pub fn vector_health_check() {
    services::rag().check_server_health();
}

/// V1 CONFIG Apply tail (RealAmadeusApp.tsx L3315–3318):
/// `VectorDatabase.enabled = …; .topK = …; .threshold = …; .serverUrl = …;` — each setter is a
/// no-op when unchanged (Apply has already written the keys, so normally only an out-of-range
/// top-K / threshold acts); `enabled` (turned on) and `serverUrl` (changed) health-check.
pub fn apply_rag_settings(enabled: bool, top_k: f64, threshold: f64, server_url: &str) {
    let settings = app_settings();
    let db = vectors();
    if db.set_enabled(enabled, settings) {
        vector_health_check();
    }
    db.set_top_k(top_k, settings);
    db.set_threshold(threshold, settings);
    if db.set_server_url(server_url, settings) {
        vector_health_check();
    }
}

/// V1 `AchievementManager.notifyEvent(type, data)` with "now" and the stored `Config_Language`
/// (what V1 read internally). Returns the newly unlocked ids in toast order; the
/// `achievement_unlocked` / `achievements_changed` signals fire as in V1.
pub fn notify_achievement(event: &AchievementEvent) -> Vec<&'static str> {
    achievements().notify_event(event, &local_now(), config_language_untracked())
}

/// V1 `AchievementManager.unlock(id)` with "now" and the stored language.
pub fn unlock_achievement(id: &str) -> bool {
    achievements().unlock(id, &local_now(), config_language_untracked())
}

/// Local wall-clock time from the JS `Date` getters (what V1's `new Date()` code read).
pub fn local_now() -> LocalDateTime {
    let d = js_sys::Date::new_0();
    LocalDateTime::new(
        i32::try_from(d.get_full_year()).unwrap_or(1970),
        d.get_month() + 1,
        d.get_date(),
        d.get_hours(),
        d.get_minutes(),
        d.get_seconds(),
    )
}

/// `Math.random` as an [`amadeus_core::random_events::Rng`]: pass `&mut managers::rng()`.
pub fn rng() -> impl FnMut() -> f64 {
    js_sys::Math::random
}

/// Connects `handler` to a manager signal for the lifetime of the current reactive owner
/// (disconnects in `on_cleanup`, like V1's effect cleanups). `emitter` must be a
/// non-capturing closure / fn item, e.g. `|| managers::achievements().achievements_changed()`.
pub fn connect_scoped<T: ?Sized + 'static>(
    emitter: fn() -> &'static Emitter<T>,
    handler: impl Fn(&T) + 'static,
) -> ConnectionId {
    let id = emitter().connect(handler);
    leptos::prelude::on_cleanup(move || {
        emitter().disconnect(id);
    });
    id
}

/// `new Intl.Collator("ja").compare` — exactly what V1's `localeCompare(…, "ja")` used. `None`
/// when `Intl` is unavailable (the core's built-in `JaCollator` approximation stays in place).
fn intl_ja_collator() -> Option<impl Fn(&str, &str) -> Ordering + 'static> {
    let has_intl =
        js_sys::Reflect::has(&js_sys::global(), &JsValue::from_str("Intl")).unwrap_or(false);
    if !has_intl {
        return None;
    }
    let locales = js_sys::Array::of1(&JsValue::from_str("ja"));
    let collator = js_sys::Intl::Collator::new(&locales, &js_sys::Object::new());
    let compare = collator.compare();
    Some(move |a: &str, b: &str| {
        match compare.call2(&JsValue::NULL, &JsValue::from_str(a), &JsValue::from_str(b)) {
            Ok(v) => {
                let n = v.as_f64().unwrap_or(0.0);
                if n < 0.0 {
                    Ordering::Less
                } else if n > 0.0 {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            }
            Err(_) => amadeus_core::memory::ja_compare(a, b),
        }
    })
}
