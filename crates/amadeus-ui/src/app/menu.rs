//! The desktop side menu: V1 `MenuItem` / `SmallMenuLabel` (RealAmadeusApp.tsx L4710–4778), the
//! `.menu-panel` block of the root (L4523–4636) and the keyboard navigation table of the root
//! keydown handler (L4303–4359). OWNED BY: app-root engineer.

use leptos::prelude::*;

use crate::env::asset_url;

/// Menu entry indices (V1 `executeMenuSelection(index)`).
pub const MENU_BACKLOG: u8 = 0;
/// CONFIG.
pub const MENU_CONFIG: u8 = 1;
/// STATUS.
pub const MENU_STATUS: u8 = 2;
/// WINDOW / FULL SCREEN toggle.
pub const MENU_SCREEN_MODE: u8 = 3;
/// CHANGE LOG.
pub const MENU_CHANGE_LOG: u8 = 4;
/// LOGOUT (confirmation).
pub const MENU_LOGOUT: u8 = 5;
/// HELP.
pub const MENU_HELP: u8 = 6;
/// SHUT DOWN (confirmation).
pub const MENU_SHUTDOWN: u8 = 7;
/// CLOSE MENU.
pub const MENU_CLOSE: u8 = 8;

/// Direction of a menu navigation key (arrows / WASD).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuDir {
    /// `ArrowUp` / `w`.
    Up,
    /// `ArrowDown` / `s`.
    Down,
    /// `ArrowLeft` / `a`.
    Left,
    /// `ArrowRight` / `d`.
    Right,
}

/// V1 menu navigation (`MENU_NAV`, spec-app-part4 §7.4): the entry selected after pressing
/// `dir` on entry `index`. Unlisted moves (and unknown indices) keep the selection.
pub fn menu_navigate(index: u8, dir: MenuDir) -> u8 {
    use MenuDir::{Down, Left, Right, Up};
    match (index, dir) {
        (0, Down) => 1,
        (0, Up) => 6,
        (0, Right) => 8,
        (1, Down) => 2,
        (1, Up) => 0,
        (1, Right) => 8,
        (2, Down) => 4,
        (2, Up) => 1,
        (2, Left) => 3,
        (2, Right) => 8,
        (3, Up) => 7,
        (3, Left) => 8,
        (3, Down) => 5,
        (3, Right) => 2,
        (4, Down) => 6,
        (4, Up) => 2,
        (4, Left) => 5,
        (4, Right) => 8,
        (5, Up) => 3,
        (5, Down) => 7,
        (5, Left) => 8,
        (5, Right) => 4,
        (6, Down) => 0,
        (6, Up) => 4,
        (6, Left) => 7,
        (6, Right) => 8,
        (7, Up) => 5,
        (7, Down) => 3,
        (7, Left) => 8,
        (7, Right) => 6,
        (8, Left) => 6,
        (8, Right) => 7,
        (other, _) => other,
    }
}

/// One big icon entry of the menu (V1 `<MenuItem icon label iconPos labelPos>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuIconEntry {
    /// Menu index executed on click.
    pub index: u8,
    /// Image base name under `resources/images/` (`{icon}.png`, `{icon}Selected.png`).
    pub icon: &'static str,
    /// Label (contains `\n`; `.menu-label` has `white-space: pre-line`).
    pub label: &'static str,
    /// Icon centre in stage pixels (the button is 90×90).
    pub icon_pos: (i32, i32),
    /// Label centre in stage pixels (the button is 200×50).
    pub label_pos: (i32, i32),
}

/// The five icon entries in V1 DOM order.
pub const MENU_ICON_ENTRIES: [MenuIconEntry; 5] = [
    MenuIconEntry {
        index: MENU_BACKLOG,
        icon: "Backlog",
        label: "BACK\nLOG",
        icon_pos: (270, 146),
        label_pos: (441, 147),
    },
    MenuIconEntry {
        index: MENU_CONFIG,
        icon: "Config",
        label: "CONFIG",
        icon_pos: (270, 320),
        label_pos: (441, 321),
    },
    MenuIconEntry {
        index: MENU_STATUS,
        icon: "Status",
        label: "STATUS",
        icon_pos: (270, 494),
        label_pos: (441, 495),
    },
    MenuIconEntry {
        index: MENU_CHANGE_LOG,
        icon: "ChangeLog",
        label: "CHANGE\nLOG",
        icon_pos: (270, 674),
        label_pos: (441, 669),
    },
    MenuIconEntry {
        index: MENU_HELP,
        icon: "Help",
        label: "HELP",
        icon_pos: (270, 848),
        label_pos: (441, 843),
    },
];

/// Label of menu entry 3: the CURRENT mode (`screenMode === 0 ? "FULL\nSCREEN" : "WINDOW"`).
pub fn screen_mode_label(screen_mode: i64) -> &'static str {
    if screen_mode == 0 {
        "FULL\nSCREEN"
    } else {
        "WINDOW"
    }
}

/// The four small text entries in V1 DOM order: `(index, fixed label, centre)`; entry 3's label
/// comes from [`screen_mode_label`].
pub const SMALL_MENU_ENTRIES: [(u8, Option<&str>, (i32, i32)); 4] = [
    (MENU_SCREEN_MODE, None, (135, 584)),
    (MENU_LOGOUT, Some("LOGOUT"), (135, 758)),
    (MENU_SHUTDOWN, Some("SHUT\nDOWN"), (135, 950)),
    (MENU_CLOSE, Some("CLOSE\nMENU"), (405, 950)),
];

/// React's `style={{ left: x, top: y }}` serialisation (numbers get `px`).
fn position_style(left: i32, top: i32) -> String {
    format!("left: {left}px; top: {top}px;")
}

/// `${base} ${selected ? "selected" : ""}` — V1 keeps the trailing space when not selected.
fn selectable_class(base: &'static str, selected: bool) -> String {
    if selected {
        format!("{base} selected")
    } else {
        format!("{base} ")
    }
}

/// V1 `.menu-panel` inline transform:
/// `scaleX(${width / (1920 * scale)}) scaleY(${height / (1080 * scale)})` (always
/// `scaleX(1) scaleY(1)`, `NaN` for a 0×0 window).
pub fn menu_panel_transform(width: f64, height: f64, scale: f64) -> String {
    use amadeus_core::settings::jsnum::js_number_to_string as num;
    format!(
        "scaleX({}) scaleY({})",
        num(width / (1920.0 * scale)),
        num(height / (1080.0 * scale))
    )
}

/// V1 `MenuItem`: two sibling buttons (icon with normal/selected images, then the label).
#[component]
pub fn MenuItem(
    icon: &'static str,
    label: &'static str,
    #[prop(into)] selected: Signal<bool>,
    #[prop(into)] on_click: Callback<()>,
    icon_pos: (i32, i32),
    label_pos: (i32, i32),
) -> impl IntoView {
    let normal = asset_url(&format!("resources/images/{icon}.png"));
    let highlighted = asset_url(&format!("resources/images/{icon}Selected.png"));
    view! {
        <button
            type="button"
            class=move || selectable_class("menu-icon", selected.get())
            style=position_style(icon_pos.0 - 45, icon_pos.1 - 45)
            on:click=move |_| on_click.run(())
        >
            <img src=normal alt="" class="menu-icon-normal"/>
            <img src=highlighted alt="" class="menu-icon-selected"/>
        </button>
        <button
            type="button"
            class=move || selectable_class("menu-label", selected.get())
            style=position_style(label_pos.0 - 100, label_pos.1 - 25)
            on:click=move |_| on_click.run(())
        >
            {label}
        </button>
    }
}

/// V1 `SmallMenuLabel`: one text-only button.
#[component]
pub fn SmallMenuLabel(
    #[prop(into)] selected: Signal<bool>,
    #[prop(into)] label: Signal<&'static str>,
    pos: (i32, i32),
    #[prop(into)] on_click: Callback<()>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class=move || selectable_class("small-menu-label", selected.get())
            style=position_style(pos.0 - 100, pos.1 - 25)
            on:click=move |_| on_click.run(())
        >
            {move || label.get()}
        </button>
    }
}

/// V1 `.menu-panel`: fades with `menuOpen`, hosts the nine entries. A click selects the entry
/// and executes it (`setMenuSelectedIndex(i); executeMenuSelection(i)`); hovering does nothing.
#[component]
pub fn MenuPanel(
    #[prop(into)] menu_open: Signal<bool>,
    #[prop(into)] selected_index: Signal<u8>,
    #[prop(into)] screen_mode: Signal<i64>,
    /// `(width, height, scale)` of the window frame.
    #[prop(into)]
    dimensions: Signal<(f64, f64, f64)>,
    /// Click on entry `i`.
    #[prop(into)]
    on_select: Callback<u8>,
) -> impl IntoView {
    let menu_bg = asset_url("resources/images/Menu.png");
    let icons = MENU_ICON_ENTRIES
        .iter()
        .map(|entry| {
            let index = entry.index;
            view! {
                <MenuItem
                    icon=entry.icon
                    label=entry.label
                    selected=Signal::derive(move || selected_index.get() == index)
                    on_click=move |()| on_select.run(index)
                    icon_pos=entry.icon_pos
                    label_pos=entry.label_pos
                />
            }
        })
        .collect_view();
    let small = SMALL_MENU_ENTRIES
        .iter()
        .map(|&(index, fixed, pos)| {
            let label = Signal::derive(move || {
                fixed.unwrap_or_else(|| screen_mode_label(screen_mode.get()))
            });
            view! {
                <SmallMenuLabel
                    selected=Signal::derive(move || selected_index.get() == index)
                    label=label
                    pos=pos
                    on_click=move |()| on_select.run(index)
                />
            }
        })
        .collect_view();
    view! {
        <div
            class="menu-panel"
            style:opacity=move || if menu_open.get() { "1" } else { "0" }
            style:pointer-events=move || if menu_open.get() { "auto" } else { "none" }
            style:transition="opacity 0.3s ease-in-out"
            style:transform=move || {
                let (width, height, scale) = dimensions.get();
                menu_panel_transform(width, height, scale)
            }
            style:transform-origin="top left"
        >
            <img src=menu_bg alt="" class="menu-bg"/>
            <div class="menu-content">{icons} {small}</div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_table_matches_v1() {
        use MenuDir::{Down, Left, Right, Up};
        // (from, [up, down, left, right]) — spec-app-part4 §7.4.
        let table: [(u8, [u8; 4]); 10] = [
            (0, [6, 1, 0, 8]),
            (1, [0, 2, 1, 8]),
            (2, [1, 4, 3, 8]),
            (3, [7, 5, 8, 2]),
            (4, [2, 6, 5, 8]),
            (5, [3, 7, 8, 4]),
            (6, [4, 0, 7, 8]),
            (7, [5, 3, 8, 6]),
            (8, [8, 8, 6, 7]),
            (9, [9, 9, 9, 9]),
        ];
        for (from, [up, down, left, right]) in table {
            assert_eq!(menu_navigate(from, Up), up, "{from} up");
            assert_eq!(menu_navigate(from, Down), down, "{from} down");
            assert_eq!(menu_navigate(from, Left), left, "{from} left");
            assert_eq!(menu_navigate(from, Right), right, "{from} right");
        }
    }

    #[test]
    fn layout_positions_match_v1() {
        let lefts: Vec<_> = MENU_ICON_ENTRIES
            .iter()
            .map(|e| {
                (
                    e.icon_pos.0 - 45,
                    e.icon_pos.1 - 45,
                    e.label_pos.0 - 100,
                    e.label_pos.1 - 25,
                )
            })
            .collect();
        assert_eq!(
            lefts,
            vec![
                (225, 101, 341, 122),
                (225, 275, 341, 296),
                (225, 449, 341, 470),
                (225, 629, 341, 644),
                (225, 803, 341, 818)
            ]
        );
        let small: Vec<_> = SMALL_MENU_ENTRIES
            .iter()
            .map(|(i, _, (x, y))| (*i, x - 100, y - 25))
            .collect();
        assert_eq!(
            small,
            vec![(3, 35, 559), (5, 35, 733), (7, 35, 925), (8, 305, 925)]
        );
        assert_eq!(position_style(225, 101), "left: 225px; top: 101px;");
    }

    #[test]
    fn labels_and_classes() {
        assert_eq!(screen_mode_label(0), "FULL\nSCREEN");
        assert_eq!(screen_mode_label(1), "WINDOW");
        assert_eq!(screen_mode_label(7), "WINDOW");
        assert_eq!(selectable_class("menu-icon", true), "menu-icon selected");
        assert_eq!(selectable_class("menu-icon", false), "menu-icon ");
    }

    #[test]
    fn menu_panel_transform_uses_js_numbers() {
        let s = 1366.0 / 1920.0;
        assert_eq!(
            menu_panel_transform(1920.0 * s, 1080.0 * s, s),
            "scaleX(1) scaleY(1)"
        );
        assert_eq!(
            menu_panel_transform(0.0, 0.0, 0.0),
            "scaleX(NaN) scaleY(NaN)"
        );
    }
}
