//! Menus, drop-downs and submenus of the engineering suite.
//!
//! Torno, Vortia and Forja open the same kind of surface from the same places (the title bar's
//! File and app menus, a ribbon group's arrow, a workspace selector, a context menu), so they draw
//! it with the same parts: one floating surface, one row, one separator, one section heading.
//!
//! ```no_run
//! # fn draw(ctx: &egui::Context, ui: &mut egui::Ui, button: &egui::Response) {
//! use nodalia_theme::engineering::{menu, Look};
//! use nodalia_theme::{Accent, Mode};
//!
//! let look = Look::new(Mode::Dark.palette_with(Accent::Torno));
//! menu::popup(egui::Popup::menu(button), &look).show(|ui| {
//!     if menu::item(ui, &look, &menu::Item::new("Open…").icon("folder-open").shortcut("Ctrl+O")).clicked() {
//!         ui.close();
//!     }
//!     menu::separator(ui, &look);
//!     menu::submenu(ui, &look, &menu::Item::new("Open recent"), |ui| {
//!         menu::item(ui, &look, &menu::Item::new("Bracket.dxf"));
//!     });
//! });
//! # }
//! ```
//!
//! The module only draws: the app supplies labels (already translated), icons and shortcut text,
//! and acts on the responses by running its own commands.

use egui::{
    containers::menu::{MenuConfig, SubMenu},
    pos2, vec2, Align, Align2, CornerRadius, Frame, Layout, Margin, Popup, Rect, Response, Role,
    Sense, Stroke, Style, Ui, UiKind, UiStackInfo, WidgetInfo,
};

use super::{body, layout, size, value_font, Look};
use crate::{for_egui::color, for_egui::shadow, icons};

/// The menu's metrics, in points.
pub mod metric {
    /// A menu row.
    pub const ROW: f32 = 28.0;
    /// Narrowest menu.
    pub const MIN_WIDTH: f32 = 208.0;
    /// Padding between the surface's edge and its rows.
    pub const MARGIN: i8 = 6;
    /// Padding inside a row, on both sides.
    pub const ROW_PAD: f32 = 8.0;
    /// Width of the leading column for a check mark or an icon.
    pub const GUTTER: f32 = 20.0;
    /// Icon size.
    pub const ICON: f32 = 16.0;
    /// Space between a label and the shortcut or arrow after it.
    pub const GAP: f32 = 28.0;
    /// Space above and below a separator line, and above a section heading.
    pub const SECTION: f32 = 4.0;
    /// Radius of the floating surface.
    pub const RADIUS: u8 = 12;
    /// Gap between a menu and the submenu it opens.
    pub const SUBMENU_GAP: f32 = 4.0;
}

/// One menu row.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Item<'a> {
    label: &'a str,
    icon: Option<&'a str>,
    shortcut: Option<&'a str>,
    checked: Option<bool>,
    enabled: bool,
    gutter: bool,
    danger: bool,
}

impl<'a> Item<'a> {
    pub fn new(label: &'a str) -> Self {
        Item {
            label,
            enabled: true,
            ..Item::default()
        }
    }
    /// An NDL icon name, drawn in the leading column. An unknown name leaves it empty.
    pub fn icon(mut self, name: &'a str) -> Self {
        self.icon = Some(name);
        self
    }
    /// The shortcut as the platform shows it (`Ctrl+O`, `⌘O`), in the monospaced face.
    pub fn shortcut(mut self, text: &'a str) -> Self {
        self.shortcut = Some(text);
        self
    }
    /// A toggle: a check mark in the leading column when `on`, an empty column when not.
    pub fn checked(mut self, on: bool) -> Self {
        self.checked = Some(on);
        self
    }
    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = on;
        self
    }
    /// Reserves the leading column although this row has no icon or check, so it lines up with
    /// the rows of its menu that do.
    pub fn gutter(mut self) -> Self {
        self.gutter = true;
        self
    }
    /// A destructive command (delete, close without saving): drawn in the danger colour.
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
    fn has_gutter(&self) -> bool {
        self.gutter || self.icon.is_some() || self.checked.is_some()
    }
}

/// The style a menu's `Ui` takes: Plex body text, no frames on rows, and the surface's colours,
/// radius, margin and shadow, which egui reads from here when it frames a popup.
pub fn style(look: &Look) -> impl Fn(&mut Style) + Clone + Send + Sync + 'static {
    let p = look.palette.clone();
    move |style: &mut Style| {
        let v = &mut style.visuals;
        v.window_fill = color(p.surface_overlay);
        v.window_stroke = Stroke::new(1.0, color(p.border_subtle));
        v.menu_corner_radius = CornerRadius::same(metric::RADIUS);
        v.popup_shadow = shadow(p.elevation_overlay);
        for w in [
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.bg_stroke = Stroke::NONE;
            w.fg_stroke.color = color(p.text_primary);
        }
        v.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
        v.widgets.hovered.weak_bg_fill = color(p.control_bg_hover);
        v.widgets.active.weak_bg_fill = color(p.control_bg_pressed);
        v.widgets.open.weak_bg_fill = color(p.control_bg_hover);
        style.spacing.menu_margin = Margin::same(metric::MARGIN);
        style.spacing.item_spacing = vec2(0.0, 2.0);
        style.spacing.button_padding = vec2(metric::ROW_PAD, 4.0);
        style.spacing.interact_size.y = metric::ROW;
        style.override_font_id = None;
    }
}

/// `popup` dressed as a suite menu: the suite's style on it and on the submenus it opens, and the
/// gap between them. Use it on `Popup::menu`, `Popup::context_menu` or a `Popup::new(..)` of
/// kind `Menu`.
pub fn popup<'a>(popup: Popup<'a>, look: &Look) -> Popup<'a> {
    let config = MenuConfig::new()
        .style(style(look))
        .submenu_gap(metric::SUBMENU_GAP);
    popup
        .layout(Layout::top_down_justified(Align::Min))
        .gap(0.0)
        .style(style(look))
        .info(UiStackInfo::new(UiKind::Menu).with_tag_value(MenuConfig::MENU_CONFIG_TAG, config))
}

/// The frame of a menu surface, for hosts that draw one by hand (a drop-down panel that is not an
/// egui popup).
pub fn surface(look: &Look) -> Frame {
    Frame::NONE
        .fill(color(look.palette.surface_overlay))
        .stroke(Stroke::new(1.0, color(look.palette.border_subtle)))
        .corner_radius(CornerRadius::same(metric::RADIUS))
        .inner_margin(Margin::same(metric::MARGIN))
        .shadow(shadow(look.palette.elevation_overlay))
}

/// How wide `item` wants to be: the leading column, the label, the gap and the shortcut or arrow.
fn wanted_width(ui: &Ui, item: &Item, arrow: bool) -> f32 {
    let measure = |text: &str, font: egui::FontId| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font, egui::Color32::WHITE)
            .size()
            .x
    };
    let mut w = metric::ROW_PAD * 2.0 + measure(item.label, body());
    if item.has_gutter() {
        w += metric::GUTTER;
    }
    if let Some(shortcut) = item.shortcut {
        w += metric::GAP + measure(shortcut, value_font());
    }
    if arrow {
        w += metric::GAP.min(16.0) + 12.0;
    }
    w
}

fn row(ui: &mut Ui, look: &Look, item: &Item, arrow: bool, open: bool) -> Response {
    let p = &look.palette;
    let wanted =
        wanted_width(ui, item, arrow).max(metric::MIN_WIDTH - f32::from(metric::MARGIN) * 2.0);
    let sense = if item.enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_at_least(vec2(wanted, metric::ROW), sense);
    resp.widget_info(|| match item.checked {
        Some(on) => WidgetInfo::selected(Role::MenuItemCheckBox, item.enabled, on, item.label),
        None => WidgetInfo::labeled(Role::MenuItem, item.enabled, item.label),
    });
    let painter = ui.painter();
    let lit = item.enabled && (resp.hovered() || resp.has_focus() || open);
    if lit {
        painter.rect_filled(rect, layout::ROW_RADIUS, color(p.accent_subtle));
    }
    let ink = if !item.enabled {
        color(p.text_disabled)
    } else if item.danger {
        color(p.status_danger_fg)
    } else if lit {
        color(p.accent_text)
    } else {
        color(p.text_primary)
    };
    let mut x = rect.left() + metric::ROW_PAD;
    if item.has_gutter() {
        let slot = Rect::from_min_size(
            pos2(x, rect.center().y - metric::ICON / 2.0),
            vec2(metric::ICON, metric::ICON),
        );
        let (name, tint) = match (item.checked, item.icon) {
            (Some(true), _) => (Some("check"), color(p.accent_text)),
            (Some(false), _) => (None, ink),
            (None, name) => (
                name,
                if item.enabled {
                    color(p.text_secondary)
                } else {
                    ink
                },
            ),
        };
        if let Some(glyph) = name.and_then(icons::get) {
            let style = if item.checked == Some(true) {
                icons::Style::bold(tint)
            } else {
                icons::Style::regular(tint)
            };
            icons::paint(painter, slot, glyph, style);
        }
        x += metric::GUTTER;
    }
    painter.text(
        pos2(x, rect.center().y),
        Align2::LEFT_CENTER,
        item.label,
        body(),
        ink,
    );
    let mut right = rect.right() - metric::ROW_PAD;
    if arrow {
        if let Some(glyph) = icons::get("chevron-right") {
            let slot = Rect::from_center_size(pos2(right - 6.0, rect.center().y), vec2(12.0, 12.0));
            let tint = if item.enabled {
                color(p.text_secondary)
            } else {
                color(p.text_disabled)
            };
            icons::paint(painter, slot, glyph, icons::Style::bold(tint));
        }
        right -= 14.0;
    }
    if let Some(shortcut) = item.shortcut {
        let tint = if item.enabled {
            color(p.text_tertiary)
        } else {
            color(p.text_disabled)
        };
        painter.text(
            pos2(right, rect.center().y),
            Align2::RIGHT_CENTER,
            shortcut,
            value_font(),
            tint,
        );
    }
    if lit {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// A command row. The response is clicked when the command was chosen; the caller closes the
/// menu with `ui.close()`.
pub fn item(ui: &mut Ui, look: &Look, item: &Item) -> Response {
    row(ui, look, item, false, false)
}

/// A row that opens `contents` beside it. Hovering opens it, a click or the right arrow does too.
pub fn submenu<R>(
    ui: &mut Ui,
    look: &Look,
    item: &Item,
    contents: impl FnOnce(&mut Ui) -> R,
) -> Response {
    let id = ui.next_auto_id();
    let open = ui
        .ctx()
        .memory(|m| m.data.get_temp::<bool>(id.with("open")))
        .unwrap_or(false);
    let resp = row(ui, look, item, true, open);
    let shown = SubMenu::new().show(ui, &resp, |ui| {
        ui.set_min_width(metric::MIN_WIDTH);
        contents(ui)
    });
    ui.ctx()
        .memory_mut(|m| m.data.insert_temp(id.with("open"), shown.is_some()));
    resp
}

/// A hairline between groups of commands, with room around it.
pub fn separator(ui: &mut Ui, look: &Look) {
    let (rect, _) = ui.allocate_at_least(vec2(1.0, metric::SECTION * 2.0 + 1.0), Sense::hover());
    let y = rect.center().y.round();
    ui.painter().line_segment(
        [
            pos2(rect.left() + metric::ROW_PAD, y),
            pos2(rect.right() - metric::ROW_PAD, y),
        ],
        Stroke::new(1.0, color(look.palette.border_subtle)),
    );
}

/// A small section title over a group of rows (a ribbon group's name, "Recent").
pub fn heading(ui: &mut Ui, look: &Look, text: &str) {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::FontId::new(
            size::CAPTION,
            egui::FontFamily::Name(super::SEMIBOLD.into()),
        ),
        color(look.palette.text_tertiary),
    );
    let height = metric::ROW - 6.0 + metric::SECTION;
    let (rect, resp) = ui.allocate_at_least(
        vec2(galley.size().x + metric::ROW_PAD * 2.0, height),
        Sense::hover(),
    );
    resp.widget_info(|| WidgetInfo::labeled(Role::Heading, true, text));
    ui.painter().galley(
        pos2(
            rect.left() + metric::ROW_PAD,
            rect.bottom() - galley.size().y - 3.0,
        ),
        galley,
        color(look.palette.text_tertiary),
    );
}
