//! Reactive hooks (settings, viewport). OWNED BY: ui-foundation.
//!
//! * [`settings`]: V1 `AppSettings` singleton, `useAppSettingInt` / `useAppSettingFloat`.
//! * [`viewport`]: V1 `useIsMobileViewport` / `subscribeViewportState`.
//!
//! The language helpers (app `t()` and the site locale) live in [`crate::i18n`].

pub mod settings;
pub mod viewport;

pub use settings::{
    app_settings, set_app_setting_bool, set_app_setting_float, set_app_setting_int,
    set_app_setting_string, use_app_setting_bool, use_app_setting_float, use_app_setting_int,
    use_app_setting_string, use_setting, use_settings, SettingDescriptor, Settings, SettingsCtx,
};
pub use viewport::{
    is_mobile_viewport_now, is_mobile_viewport_snapshot, subscribe_viewport_state,
    use_is_mobile_viewport, ViewportSubscription,
};
