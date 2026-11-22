//! V1 `AchievementPanel` (`RealAmadeusApp.tsx` 2432–2511). OWNED BY: panels engineer.
//!
//! * Always mounted (0.25 s fade, z 2500 — above the status panel it is opened from).
//! * Re-renders on `achievementsChanged` (unlock, reset, `Config_Language` change), subscribed
//!   for the panel's lifetime, and — like a V1 render — when it opens or the language prop
//!   changes.
//! * Rows come from `AchievementManager::all_achievements(lang)` in definition order with V1's
//!   `lang` = the **stored** `Config_Language` (not the prop); locked secrets show `???` and the
//!   secret hint. The progress is the raw stored unlock count / 28 (no star).
//! * While visible, Backspace or Escape closes (`preventDefault`).

use leptos::prelude::*;

use amadeus_core::achievements::AchievementView;

use super::common::{
    close_label, fade_style, fire, image_url, use_keydown_while_visible, visible_memo, OVERLAY_BG,
};
use crate::app::managers;
use crate::i18n::config_language_untracked;

#[component]
pub fn AchievementPanel(
    #[prop(into)] visible: Signal<bool>,
    #[prop(into)] config_language: Signal<i32>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let visible = visible_memo(visible);
    let revision = RwSignal::new(0_u64);

    managers::connect_scoped(
        || managers::achievements().achievements_changed(),
        move |_: &()| revision.update(|r| *r = r.wrapping_add(1)),
    );

    use_keydown_while_visible(visible, move |event| {
        let key = event.key();
        if key == "Backspace" || key == "Escape" {
            event.prevent_default();
            fire(on_close, ());
        }
    });

    // V1 re-read the manager on every render (parent render, revision bump); the texts use the
    // stored language either way.
    let views = move || {
        revision.track();
        visible.track();
        config_language.track();
        managers::achievements().all_achievements(config_language_untracked())
    };
    let progress = move || {
        revision.track();
        visible.track();
        let achievements = managers::achievements();
        progress_text(achievements.unlocked_count(), achievements.total_count())
    };

    view! {
        <div style=move || fade_style(visible.get(), "0.25s", 2500)>
            <div class="overlay-panel achievement-panel">
                <img src=image_url(OVERLAY_BG) alt="" class="overlay-bg"/>
                <div class="achievement-panel-title">"ACHIEVEMENTS"</div>
                <div class="achievement-panel-progress">{progress}</div>
                <div class="achievement-list">
                    // React key = id; the key here also carries the rendered values so a changed
                    // row (unlock, language) is re-rendered.
                    <For each=views key=row_key children=achievement_row/>
                </div>
                <button type="button" class="overlay-close-button" on:click=move |_| fire(on_close, ())>
                    {move || close_label(config_language.get())}
                </button>
            </div>
        </div>
    }
}

/// `${unlockedCount} / ${totalCount}` (no star; the status panel adds one).
pub fn progress_text(unlocked: usize, total: usize) -> String {
    format!("{unlocked} / {total}")
}

fn row_key(row: &AchievementView) -> (&'static str, bool, String, String, String) {
    (
        row.id,
        row.unlocked,
        row.title.clone(),
        row.description.clone(),
        row.unlocked_at.clone(),
    )
}

fn achievement_row(row: AchievementView) -> impl IntoView {
    let row_class = row.row_class();
    let icon_class = row.icon_class();
    let glyph = row.icon_glyph();
    let shows_date = row.shows_date();
    let AchievementView {
        title,
        description,
        unlocked_at,
        ..
    } = row;
    let date = shows_date.then(|| view! { <div class="achievement-date">{unlocked_at}</div> });
    view! {
        <div class=row_class>
            <div class=icon_class>{glyph}</div>
            <div class="achievement-text">
                <div class="achievement-row-title">{title}</div>
                <div class="achievement-row-desc">{description}</div>
            </div>
            {date}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use amadeus_core::achievements::AchievementManager;
    use amadeus_core::memory::LocalDateTime;
    use amadeus_core::storage::MemoryStore;

    use super::*;

    fn now() -> LocalDateTime {
        LocalDateTime::new(2026, 11, 18, 7, 30, 0)
    }

    #[test]
    fn rows_follow_v1_format_rules() {
        let store = MemoryStore::new();
        let manager = AchievementManager::new(&store, &now());
        let rows = manager.all_achievements(0);
        assert_eq!(rows.len(), 28);
        assert_eq!(
            progress_text(manager.unlocked_count(), manager.total_count()),
            "0 / 28"
        );
        let first = &rows[0];
        assert_eq!(
            (first.id, first.title.as_str()),
            ("first_chat", "初めまして")
        );
        assert_eq!(
            (first.row_class(), first.icon_class(), first.icon_glyph()),
            ("achievement-row locked", "achievement-icon locked", "☆")
        );
        assert!(!first.shows_date());
        let secret = &rows[27];
        assert_eq!(secret.id, "secret_reading_steiner");
        assert_eq!(
            (
                secret.title.as_str(),
                secret.description.as_str(),
                secret.icon_glyph()
            ),
            ("???", "シークレットアチーブメント", "?")
        );

        assert!(manager.unlock("secret_reading_steiner", &now(), 1));
        let rows = manager.all_achievements(1);
        let secret = &rows[27];
        assert_eq!(secret.title, "Reading Steiner");
        assert_eq!(
            secret.description,
            "Discovered a secret that transcends world lines"
        );
        assert_eq!(
            (secret.row_class(), secret.icon_class(), secret.icon_glyph()),
            ("achievement-row unlocked", "achievement-icon unlocked", "★")
        );
        assert_eq!(secret.unlocked_at, "2026-11-18 07:30");
        assert!(secret.shows_date());
        assert_eq!(
            progress_text(manager.unlocked_count(), manager.total_count()),
            "1 / 28"
        );
    }

    #[test]
    fn secret_hint_in_all_languages() {
        // spec-app-part3 Appendix C.1 `achievement_secret_hint`.
        let hints = [
            "シークレットアチーブメント",
            "Secret Achievement",
            "隐藏成就",
            "비밀 업적",
            "Logro secreto",
            "Succès secret",
            "Geheimer Erfolg",
            "Секретное достижение",
            "Таємне досягнення",
            "Conquista secreta",
            "Gizli Başarım",
            "הישג סודי",
            "إنجاز سري",
        ];
        let store = MemoryStore::new();
        let manager = AchievementManager::new(&store, &now());
        for (lang, hint) in hints.iter().enumerate() {
            let rows = manager.all_achievements(lang as i64);
            assert_eq!(rows[27].description, *hint, "lang {lang}");
            assert_eq!(rows[27].title, "???");
        }
    }
}
