//! The pieces of an integration flow, the same in every enterprise app: Ágora's incoming
//! webhooks and plug-ins, Hito's API tokens and webhooks, and the apps to come (customer
//! management, resources, finance) draw their cards, numbered steps, copyable values and notes
//! with these, so connecting two Nodalia apps looks the same wherever it starts.
//!
//! See `docs/suites/enterprise.md` ("Integraciones"). The module only draws; the app owns what a
//! click does.

use egui::{vec2, Align2, CornerRadius, FontId, Frame, Margin, RichText, Sense, Stroke, Ui};

use crate::creator::Look;
use crate::enterprise::icon;
use crate::for_egui::color;
use crate::{creator as cr, font_size};

/// A card on a dialog's surface: one step darker than the dialog, with a hairline.
pub fn card_frame(look: &Look) -> Frame {
    let p = &look.palette;
    Frame::NONE
        .fill(color(p.surface_window))
        .stroke(Stroke::new(1.0, color(p.border_subtle)))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(12))
}

/// A section heading inside an integrations dialog.
pub fn heading(ui: &mut Ui, look: &Look, text: &str) {
    ui.label(
        RichText::new(text)
            .size(font_size::LARGE)
            .strong()
            .color(color(look.palette.text_primary)),
    );
    ui.add_space(4.0);
}

/// A connected service as a card: its icon, its name, a quiet line under it (what it is, when it
/// last did something) and, below, the actions `add_actions` draws (Send a test, Remove).
pub fn service_card(
    ui: &mut Ui,
    look: &Look,
    glyph: &str,
    title: &str,
    subtitle: &str,
    add_actions: impl FnOnce(&mut Ui),
) {
    let p = &look.palette;
    card_frame(look).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            icon(ui, glyph, 18.0, color(p.text_secondary));
            ui.vertical(|ui| {
                ui.label(RichText::new(title).strong().color(color(p.text_primary)));
                ui.label(
                    RichText::new(subtitle)
                        .size(font_size::CAPTION)
                        .color(color(p.text_tertiary)),
                );
            });
        });
        ui.add_space(8.0);
        ui.horizontal(add_actions);
    });
}

/// "Step n of total · label" in the accent, over a flow's step.
pub fn step_label(ui: &mut Ui, look: &Look, text: &str) {
    ui.label(
        RichText::new(text)
            .size(font_size::CAPTION)
            .color(color(look.palette.accent_default)),
    );
    ui.add_space(2.0);
}

/// A step of the instructions: its number in a disc, then the text beside it.
pub fn numbered(ui: &mut Ui, look: &Look, n: usize, text: &str) {
    let p = &look.palette;
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), 10.0, color(p.accent_default));
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            n.to_string(),
            FontId::proportional(11.0),
            color(p.text_on_accent),
        );
        ui.add(egui::Label::new(RichText::new(text).color(color(p.text_primary))).wrap());
    });
}

/// A value the person copies (an address, a secret shown once), in monospace under its label, with
/// a Copy button. True when Copy was clicked; the app puts `value` on the clipboard.
pub fn copy_row(ui: &mut Ui, look: &Look, label: &str, value: &str, copy: &str) -> bool {
    let p = &look.palette;
    ui.add_space(2.0);
    cr::field_frame(look)
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(label)
                    .size(font_size::CAPTION)
                    .color(color(p.text_tertiary)),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(value)
                        .monospace()
                        .size(font_size::CAPTION)
                        .color(color(p.text_primary)),
                )
                .wrap(),
            );
            ui.add_space(2.0);
            ui.small_button(copy).clicked()
        })
        .inner
}

/// A quiet paragraph of explanation.
pub fn note(ui: &mut Ui, look: &Look, text: &str) {
    ui.label(
        RichText::new(text)
            .size(font_size::SMALL)
            .color(color(look.palette.text_secondary)),
    );
    ui.add_space(6.0);
}

/// Where a connection stands between two apps, for the pill of a service card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Link {
    /// Set up and working.
    Connected,
    /// Set up, waiting for the other app's first message.
    Waiting,
    /// Set up and failing.
    Failing,
    /// Not set up.
    Off,
}

impl Link {
    /// The tone the connection reads in, or none when it is off.
    pub fn tone(self) -> Option<crate::enterprise::Tone> {
        use crate::enterprise::Tone;
        match self {
            Link::Connected => Some(Tone::Success),
            Link::Waiting => Some(Tone::Info),
            Link::Failing => Some(Tone::Danger),
            Link::Off => None,
        }
    }
}

/// Place a connection's pill at the right of the current row: a status pill in the link's tone, or
/// a quiet label when the connection is off.
pub fn link_pill(ui: &mut Ui, look: &Look, link: Link, text: &str) {
    crate::enterprise::trailing(ui, |ui| match link.tone() {
        Some(tone) => {
            crate::enterprise::status_pill(ui, look, tone, text);
        }
        None => {
            ui.label(
                RichText::new(text)
                    .size(font_size::CAPTION)
                    .color(color(look.palette.text_tertiary)),
            );
        }
    });
}

/// The width the numbered steps and the cards keep inside a dialog.
pub const DIALOG_WIDTH: f32 = 520.0;
