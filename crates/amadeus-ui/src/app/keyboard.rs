//! The root `window` keydown handler of V1 `RealAmadeusApp` (L4242–4365) as a pure decision
//! function, plus the menu-mode focus helper of effect C (L3999–4015). OWNED BY: app-root
//! engineer.
//!
//! V1 re-registered the listener whenever `[autoMode, executeMenuSelection, isSubPanelOpen,
//! menuOpen, menuSelectedIndex]` changed, so the handler saw the values of the last render.
//! The root reproduces that exactly (see `RealAmadeusApp`): the values below are captured when
//! the listener is (re-)registered, not read live. This keeps V1's interplay with the other
//! `window` keydown listeners (ConfigPanel, overlays, ChatPanel): e.g. the Backspace that closes
//! an overlay never also closes the menu, and the Enter on CLOSE MENU never advances the chat.
//!
//! Escape is not a root key (KEY-011): in V1 the browser leaves DOM fullscreen on Escape and never
//! hands that key press to the page. The desktop build reproduces this for its native window
//! fullscreen in the platform shell ([`crate::platform::escape_leaves_fullscreen`]), whose
//! capture-phase listener runs before this handler and every other `keydown` listener.

use wasm_bindgen::JsCast;

use super::menu::{menu_navigate, MenuDir};

/// Root state a keydown is evaluated against (the V1 effect dependencies).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootKeyState {
    /// `useAppSettingInt("Config_AutoMode", 0) === 1`.
    pub auto_mode: bool,
    /// `activeOverlay !== "none"`.
    pub sub_panel_open: bool,
    /// `menuOpen`.
    pub menu_open: bool,
    /// `menuSelectedIndex`.
    pub menu_selected_index: u8,
}

/// What the root does with a keydown. Every variant except [`RootKeyAction::Ignore`] calls
/// `preventDefault()` first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKeyAction {
    /// Not handled (no `preventDefault`).
    Ignore,
    /// `F11`: `toggleScreenMode()` (any state, any target).
    ToggleScreenMode,
    /// `F3`: `setInt("Config_AutoMode", value)` (any state, any target).
    SetAutoMode(bool),
    /// `Tab` with no overlay open: toggle menu mode (`isTabActive`), also inside text inputs.
    ToggleMenuMode,
    /// `Backspace` with the menu open: `closeMenu()`.
    CloseMenu,
    /// `Enter` with the menu open: `executeMenuSelection(index)`.
    Execute(u8),
    /// Arrows / WASD with the menu open: `setMenuSelectedIndex(index)`.
    Navigate(u8),
}

/// V1 decision logic. `key` is `KeyboardEvent.key`; `text_input_target` is evaluated only for
/// menu keys (V1 `isTextInputTarget(event.target)`, after F11/F3/Tab). Modifiers and `repeat`
/// are not checked (Ctrl+S with the menu open is "down").
pub fn root_key_action(
    key: &str,
    text_input_target: impl FnOnce() -> bool,
    state: RootKeyState,
) -> RootKeyAction {
    match key {
        "F11" => return RootKeyAction::ToggleScreenMode,
        "F3" => return RootKeyAction::SetAutoMode(!state.auto_mode),
        "Tab" => {
            // With an overlay open Tab is NOT prevented: native focus traversal (QUIRK-22).
            return if state.sub_panel_open {
                RootKeyAction::Ignore
            } else {
                RootKeyAction::ToggleMenuMode
            };
        }
        _ => {}
    }
    if !state.menu_open || state.sub_panel_open || text_input_target() {
        return RootKeyAction::Ignore;
    }
    let lower = key.to_lowercase();
    let dir = if key == "ArrowUp" || lower == "w" {
        Some(MenuDir::Up)
    } else if key == "ArrowDown" || lower == "s" {
        Some(MenuDir::Down)
    } else if key == "ArrowLeft" || lower == "a" {
        Some(MenuDir::Left)
    } else if key == "ArrowRight" || lower == "d" {
        Some(MenuDir::Right)
    } else {
        None
    };
    if key == "Backspace" {
        return RootKeyAction::CloseMenu;
    }
    if key == "Enter" {
        return RootKeyAction::Execute(state.menu_selected_index);
    }
    match dir {
        Some(dir) => RootKeyAction::Navigate(menu_navigate(state.menu_selected_index, dir)),
        None => RootKeyAction::Ignore,
    }
}

/// Effect C: entering menu mode blurs a focused `INPUT` / `TEXTAREA` (not `SELECT` or
/// contentEditable).
pub fn blur_focused_text_field() {
    let Some(active) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element())
    else {
        return;
    };
    if let Ok(element) = active.dyn_into::<web_sys::HtmlElement>() {
        let tag = element.tag_name();
        if tag == "INPUT" || tag == "TEXTAREA" {
            let _ = element.blur();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLOSED: RootKeyState = RootKeyState {
        auto_mode: false,
        sub_panel_open: false,
        menu_open: false,
        menu_selected_index: 0,
    };
    const OPEN: RootKeyState = RootKeyState {
        menu_open: true,
        ..CLOSED
    };

    fn act(key: &str, state: RootKeyState) -> RootKeyAction {
        root_key_action(key, || false, state)
    }

    #[test]
    fn function_keys_work_in_every_state() {
        for state in [
            CLOSED,
            OPEN,
            RootKeyState {
                sub_panel_open: true,
                ..OPEN
            },
        ] {
            assert_eq!(
                root_key_action("F11", || true, state),
                RootKeyAction::ToggleScreenMode
            );
            assert_eq!(
                root_key_action("F3", || true, state),
                RootKeyAction::SetAutoMode(true)
            );
        }
        assert_eq!(
            act(
                "F3",
                RootKeyState {
                    auto_mode: true,
                    ..CLOSED
                }
            ),
            RootKeyAction::SetAutoMode(false)
        );
    }

    #[test]
    fn tab_toggles_unless_an_overlay_is_open() {
        assert_eq!(
            root_key_action("Tab", || true, CLOSED),
            RootKeyAction::ToggleMenuMode
        );
        assert_eq!(act("Tab", OPEN), RootKeyAction::ToggleMenuMode);
        assert_eq!(
            act(
                "Tab",
                RootKeyState {
                    sub_panel_open: true,
                    ..OPEN
                }
            ),
            RootKeyAction::Ignore
        );
    }

    #[test]
    fn menu_keys_need_open_menu_without_overlay_or_text_input() {
        assert_eq!(act("Enter", CLOSED), RootKeyAction::Ignore);
        assert_eq!(act("Backspace", CLOSED), RootKeyAction::Ignore);
        assert_eq!(
            act(
                "Enter",
                RootKeyState {
                    sub_panel_open: true,
                    ..OPEN
                }
            ),
            RootKeyAction::Ignore
        );
        assert_eq!(
            root_key_action("Enter", || true, OPEN),
            RootKeyAction::Ignore
        );
        assert_eq!(
            act(
                "Enter",
                RootKeyState {
                    menu_selected_index: 5,
                    ..OPEN
                }
            ),
            RootKeyAction::Execute(5)
        );
        assert_eq!(act("Backspace", OPEN), RootKeyAction::CloseMenu);
    }

    #[test]
    fn navigation_keys() {
        assert_eq!(act("ArrowDown", OPEN), RootKeyAction::Navigate(1));
        assert_eq!(act("s", OPEN), RootKeyAction::Navigate(1));
        assert_eq!(act("S", OPEN), RootKeyAction::Navigate(1));
        assert_eq!(act("w", OPEN), RootKeyAction::Navigate(6));
        assert_eq!(act("ArrowRight", OPEN), RootKeyAction::Navigate(8));
        assert_eq!(act("D", OPEN), RootKeyAction::Navigate(8));
        // "=" moves still prevent default (V1 sets the same index).
        assert_eq!(act("a", OPEN), RootKeyAction::Navigate(0));
        assert_eq!(
            act(
                "ArrowLeft",
                RootKeyState {
                    menu_selected_index: 2,
                    ..OPEN
                }
            ),
            RootKeyAction::Navigate(3)
        );
        assert_eq!(act("x", OPEN), RootKeyAction::Ignore);
        assert_eq!(act("Escape", OPEN), RootKeyAction::Ignore);
    }

    #[test]
    fn escape_is_never_a_root_key() {
        // KEY-011: leaving fullscreen is the browser's (desktop: the platform shell's) business.
        for state in [
            CLOSED,
            OPEN,
            RootKeyState {
                sub_panel_open: true,
                ..OPEN
            },
            RootKeyState {
                auto_mode: true,
                menu_selected_index: 8,
                ..OPEN
            },
        ] {
            assert_eq!(act("Escape", state), RootKeyAction::Ignore);
            assert_eq!(
                root_key_action("Escape", || true, state),
                RootKeyAction::Ignore
            );
        }
    }
}
