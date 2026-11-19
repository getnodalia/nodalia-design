//! Where the window buttons are, and what a title bar has to leave free for them.
//!
//! The buttons are not in the same place everywhere: macOS puts its traffic lights at the left,
//! Windows puts its caption buttons at the right, and on Linux either side is normal (GNOME's
//! `button-layout`, KDE's `ButtonsOnLeft`). Nodalia's own round buttons, drawn on Linux and the
//! BSDs, follow the desktop's side. An app's title bar asks [`Placement`] for its [`Insets`] and
//! lays its content out between them, so the first control never touches a button and the bar
//! reads the same on every system.
//!
//! The module only measures and places. The app still owns the window, its commands and the bar's
//! content.

use egui::{pos2, Rect};

use crate::space;

/// The edge of the title bar that holds the window buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    /// The opposite edge.
    pub fn mirrored(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

/// Who draws the window buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    /// The operating system or the desktop: macOS's traffic lights, Windows's caption buttons or
    /// the desktop's own title bar on Linux.
    System,
    /// The app, with [`crate::window_controls`].
    Nodalia,
}

/// Width macOS's traffic lights take from the window's left edge, with the gap that keeps the
/// first control apart from the green light. Measured on macOS 26, where the lights are
/// 22-23 pt apart and end 70 pt in; the other versions end sooner.
pub const MAC_LIGHTS: f32 = 82.0;

/// Gap between Nodalia's buttons and the window's edge.
pub const EDGE_GAP: f32 = space::SM;

/// Gap between the buttons and the title bar's content.
pub const CONTENT_GAP: f32 = space::MD;

/// How much of each end of the title bar the window buttons take, in logical points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub left: f32,
    pub right: f32,
}

/// The side and owner of the window buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub owner: Owner,
    pub side: Side,
}

impl Placement {
    /// The platform's own buttons, on the side the platform keeps them.
    pub fn system() -> Self {
        Self {
            owner: Owner::System,
            side: system_side(),
        }
    }

    /// Nodalia's buttons on `side`. Linux and the BSDs use [`system_side`] so they sit where the
    /// desktop's own would.
    pub fn nodalia(side: Side) -> Self {
        Self {
            owner: Owner::Nodalia,
            side,
        }
    }

    /// What the title bar leaves free. `lights_in_content` is true on macOS, where the window's
    /// content runs under the title bar and AppKit paints the traffic lights over its left end. A
    /// bar on any other system, or on a macOS window that keeps its native title bar, passes false.
    pub fn insets(&self, lights_in_content: bool) -> Insets {
        let mut insets = Insets::default();
        let side = match (self.owner, self.side) {
            (Owner::System, Side::Left) if lights_in_content => Some((Side::Left, MAC_LIGHTS)),
            (Owner::System, _) => None,
            (Owner::Nodalia, side) => {
                Some((side, crate::window_controls::WIDTH + EDGE_GAP + CONTENT_GAP))
            }
        };
        match side {
            Some((Side::Left, width)) => insets.left = width,
            Some((Side::Right, width)) => insets.right = width,
            None => {}
        }
        insets
    }

    /// The part of `bar` left for the title bar's own content.
    pub fn content_rect(&self, bar: Rect, lights_in_content: bool) -> Rect {
        let insets = self.insets(lights_in_content);
        Rect::from_min_max(
            pos2(bar.left() + insets.left, bar.top()),
            pos2(
                (bar.right() - insets.right).max(bar.left() + insets.left),
                bar.bottom(),
            ),
        )
    }

    /// Where Nodalia draws its three buttons inside `bar`, for [`crate::window_controls::show_on`]:
    /// the buttons' own rectangle, [`EDGE_GAP`] in from the window's edge on the placement's side.
    pub fn controls_rect(&self, bar: Rect) -> Rect {
        let width = crate::window_controls::WIDTH;
        match self.side {
            Side::Left => Rect::from_min_max(
                pos2(bar.left() + EDGE_GAP, bar.top()),
                pos2(bar.left() + EDGE_GAP + width, bar.bottom()),
            ),
            Side::Right => Rect::from_min_max(
                pos2(bar.right() - EDGE_GAP - width, bar.top()),
                pos2(bar.right() - EDGE_GAP, bar.bottom()),
            ),
        }
    }
}

/// The side the platform keeps the window buttons on: the left on macOS, the right on Windows, and
/// what the desktop is set to on Linux and the BSDs (the right when it cannot be told).
///
/// The Linux answer is read once, from the desktop's settings, and kept. `NODALIA_WINDOW_BUTTONS`
/// (`left` or `right`) overrides it, which is how a screenshot of the other side is taken.
pub fn system_side() -> Side {
    if cfg!(target_os = "macos") {
        return Side::Left;
    }
    if cfg!(target_os = "windows") {
        return Side::Right;
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(any(target_os = "macos", target_os = "windows"))
    ))]
    {
        use std::sync::OnceLock;
        static SIDE: OnceLock<Side> = OnceLock::new();
        return *SIDE.get_or_init(detect_desktop_side);
    }
    #[allow(unreachable_code)]
    Side::Right
}

/// The side a GNOME `button-layout` string puts the close button on. The layout is `left:right`,
/// each half a comma-separated list: `appmenu:minimize,maximize,close` is the right,
/// `close,minimize,maximize:` the left. A layout with no close button has no side: the right.
pub fn side_from_button_layout(layout: &str) -> Side {
    let layout = layout.trim().trim_matches('\'');
    let (left, right) = layout.split_once(':').unwrap_or((layout, ""));
    let has_close = |part: &str| part.split(',').any(|b| b.trim() == "close");
    if has_close(left) && !has_close(right) {
        Side::Left
    } else {
        Side::Right
    }
}

/// The side KDE's `kwinrc` `[org.kde.kdecoration2]` puts the buttons on: left only when the
/// buttons are listed for the left and none for the right (`ButtonsOnLeft=XIA`).
pub fn side_from_kwinrc(text: &str) -> Option<Side> {
    let (mut left, mut right) = (None, None);
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("ButtonsOnLeft=") {
            left = Some(value.trim().to_owned());
        } else if let Some(value) = line.strip_prefix("ButtonsOnRight=") {
            right = Some(value.trim().to_owned());
        }
    }
    match (left, right) {
        (Some(l), ref r) if l.contains('X') && !r.as_deref().is_some_and(|r| r.contains('X')) => {
            Some(Side::Left)
        }
        (None, None) => None,
        _ => Some(Side::Right),
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(any(target_os = "macos", target_os = "windows"))
))]
fn detect_desktop_side() -> Side {
    if let Ok(value) = std::env::var("NODALIA_WINDOW_BUTTONS") {
        match value.trim().to_ascii_lowercase().as_str() {
            "left" => return Side::Left,
            "right" => return Side::Right,
            _ => {}
        }
    }
    if let Ok(out) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.wm.preferences", "button-layout"])
        .output()
    {
        if out.status.success() {
            return side_from_button_layout(&String::from_utf8_lossy(&out.stdout));
        }
    }
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")));
    config
        .and_then(|dir| std::fs::read_to_string(dir.join("kwinrc")).ok())
        .and_then(|text| side_from_kwinrc(&text))
        .unwrap_or(Side::Right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::vec2;

    #[test]
    fn a_gnome_layout_puts_the_buttons_where_close_is() {
        for (layout, side) in [
            ("appmenu:close", Side::Right),
            ("'appmenu:minimize,maximize,close'", Side::Right),
            ("close,minimize,maximize:", Side::Left),
            ("'close:maximize'", Side::Left),
            ("close:close", Side::Right),
            (":minimize,maximize", Side::Right),
            ("", Side::Right),
        ] {
            assert_eq!(side_from_button_layout(layout), side, "{layout}");
        }
    }

    #[test]
    fn kwin_buttons_on_the_left_are_left_only_when_the_right_has_no_close() {
        assert_eq!(
            side_from_kwinrc("ButtonsOnLeft=XIA\nButtonsOnRight=\n"),
            Some(Side::Left)
        );
        assert_eq!(
            side_from_kwinrc("ButtonsOnLeft=M\nButtonsOnRight=HIAX\n"),
            Some(Side::Right)
        );
        assert_eq!(side_from_kwinrc("[General]\n"), None);
    }

    #[test]
    fn the_title_bar_leaves_room_on_the_side_of_the_buttons_and_only_there() {
        let bar = Rect::from_min_size(pos2(0.0, 0.0), vec2(900.0, 44.0));
        let nodalia_right = Placement::nodalia(Side::Right);
        let nodalia_left = Placement::nodalia(Side::Left);
        let w = crate::window_controls::WIDTH + EDGE_GAP + CONTENT_GAP;
        assert_eq!(
            nodalia_right.insets(false),
            Insets {
                left: 0.0,
                right: w
            }
        );
        assert_eq!(
            nodalia_left.insets(false),
            Insets {
                left: w,
                right: 0.0
            }
        );
        assert_eq!(nodalia_left.content_rect(bar, false).min.x, w);
        assert_eq!(nodalia_right.content_rect(bar, false).max.x, 900.0 - w);
        // The strips for the buttons and for the content never overlap, and leave the gap.
        let strip = nodalia_left.controls_rect(bar);
        assert!(strip.right() + CONTENT_GAP <= nodalia_left.content_rect(bar, false).left());
        let strip = nodalia_right.controls_rect(bar);
        assert!(strip.left() >= nodalia_right.content_rect(bar, false).right() + CONTENT_GAP);
    }

    #[test]
    fn the_system_lights_take_room_only_when_the_content_runs_under_them() {
        let lights = Placement {
            owner: Owner::System,
            side: Side::Left,
        };
        assert_eq!(
            lights.insets(true),
            Insets {
                left: MAC_LIGHTS,
                right: 0.0
            }
        );
        assert_eq!(lights.insets(false), Insets::default());
        let caption = Placement {
            owner: Owner::System,
            side: Side::Right,
        };
        assert_eq!(caption.insets(true), Insets::default());
    }

    #[test]
    fn a_bar_narrower_than_its_insets_keeps_a_valid_rect() {
        let bar = Rect::from_min_size(pos2(0.0, 0.0), vec2(40.0, 44.0));
        let r = Placement::nodalia(Side::Left).content_rect(bar, false);
        assert!(r.width() >= 0.0);
    }
}
