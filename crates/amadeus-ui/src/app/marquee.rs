//! V1 `MarqueeStatus` (RealAmadeusApp.tsx L3873–3902): the provider/model ticker of the mobile
//! top bar. OWNED BY: app-root engineer.

use std::time::Duration;

use leptos::html;
use leptos::prelude::*;

use super::listener::{clear_listener, replace_listener, ListenerSlot, OwnedWindowListener};

/// `provider && model ? `${provider} / ${model}` : "---"`.
pub fn marquee_text(provider: &str, model: &str) -> String {
    if provider.is_empty() || model.is_empty() {
        "---".to_owned()
    } else {
        format!("{provider} / {model}")
    }
}

/// Mobile top-bar status. Adds `marquee` (12 s scroll, CSS) when the text is wider than the
/// box, checked when the text changes, 100 ms later and on every window resize. Once applied
/// the class never turns off until remount, because its `padding-left: 100%` keeps the inner
/// span wider than the box (V1 QUIRK-17, kept).
#[component]
pub fn MarqueeStatus(
    #[prop(into)] provider: Signal<String>,
    #[prop(into)] model: Signal<String>,
) -> impl IntoView {
    let container: NodeRef<html::Div> = NodeRef::new();
    let inner: NodeRef<html::Span> = NodeRef::new();
    let is_overflowing = RwSignal::new(false);
    let text = Memo::new(move |_| marquee_text(&provider.get(), &model.get()));
    let resize_listener: ListenerSlot = StoredValue::new_local(None);

    // V1 `useEffect(…, [text])`.
    Effect::new(move |_| {
        text.track();
        let check = move || {
            if let (Some(container), Some(inner)) =
                (container.get_untracked(), inner.get_untracked())
            {
                let overflowing = inner.offset_width() > container.client_width();
                if is_overflowing.get_untracked() != overflowing {
                    is_overflowing.set(overflowing);
                }
            }
        };
        check();
        let timer = set_timeout_with_handle(check, Duration::from_millis(100)).ok();
        replace_listener(resize_listener, || {
            OwnedWindowListener::new("resize", move |_| check())
        });
        on_cleanup(move || {
            if let Some(timer) = timer {
                timer.clear();
            }
            clear_listener(resize_listener);
        });
    });

    view! {
        <div
            class=move || if is_overflowing.get() { "mobile-topbar-status marquee" } else { "mobile-topbar-status " }
            node_ref=container
        >
            <span class="mobile-topbar-status-inner" node_ref=inner>
                {move || text.get()}
            </span>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_rules() {
        assert_eq!(marquee_text("FreeAPI", "default"), "FreeAPI / default");
        assert_eq!(marquee_text("", "default"), "---");
        assert_eq!(marquee_text("FreeAPI", ""), "---");
        assert_eq!(
            marquee_text("OpenAI互換(http://x/v1)", "m"),
            "OpenAI互換(http://x/v1) / m"
        );
    }
}
