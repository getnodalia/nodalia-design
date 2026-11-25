//! The chat panel.

use egui::{vec2, Frame, Key, Margin, Modifiers, RichText, ScrollArea, Stroke, TextEdit, Ui};
use nodalia_theme::for_egui::color;
use nodalia_theme::icons;
use nodalia_theme::office::Look;

use super::{badge, font, icon_button, markdown, text_button, Weight};
use crate::agent::{Assistant, Item, Note, ToolItem, ToolState};
use crate::host::Host;
use crate::transport::Failure;

/// What the panel asks of the app.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// The person wants the Preferences page.
    pub open_preferences: bool,
}

enum Act {
    Approve,
    ApproveAll,
    Skip,
    Undo,
    Preferences,
    Send(String),
}

/// Draws the panel in the space `ui` has: an island with the header, the transcript and the
/// box to type in. Returns what the app should do.
pub fn panel(ui: &mut Ui, look: &Look, a: &mut Assistant, host: &mut dyn Host) -> Outcome {
    let p = &look.palette;
    let mut acts: Vec<Act> = Vec::new();
    let mut close = false;
    let mut new_chat = false;
    Frame::new()
        .fill(color(p.surface_panel))
        .stroke(Stroke::new(1.0, color(p.border_subtle)))
        .corner_radius(16)
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_height(ui.available_height());
            ui.set_width(ui.available_width());
            header(ui, look, a, &mut acts, &mut new_chat, &mut close);
            ui.add_space(8.0);
            let composer_height = 118.0;
            let body = (ui.available_height() - composer_height).max(80.0);
            ScrollArea::vertical()
                .id_salt("ndl_assistant_transcript")
                .max_height(body)
                .min_scrolled_height(body)
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| transcript(ui, look, a, host, &mut acts));
            ui.add_space(6.0);
            composer(ui, look, a, &mut acts);
        });
    if close {
        a.open = false;
    }
    if new_chat {
        a.new_chat();
    }
    let mut out = Outcome::default();
    for act in acts {
        match act {
            Act::Approve => a.approve(),
            Act::ApproveAll => a.approve_all(),
            Act::Skip => a.skip(),
            Act::Undo => a.undo_turn(host),
            Act::Preferences => out.open_preferences = true,
            Act::Send(text) => {
                a.draft.clear();
                a.send(host, &text);
            }
        }
    }
    out
}

fn header(
    ui: &mut Ui,
    look: &Look,
    a: &Assistant,
    acts: &mut Vec<Act>,
    new_chat: &mut bool,
    close: &mut bool,
) {
    let p = &look.palette;
    let lang = a.lang;
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(28.0, 28.0), egui::Sense::hover());
        badge(ui.painter(), look, rect, a.settings.provider, true);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.label(
                RichText::new(lang.t("Assistant"))
                    .font(egui::FontId::new(15.0, look.bold.clone()))
                    .color(color(p.text_primary)),
            );
            ui.label(
                RichText::new(format!(
                    "{} · {}",
                    a.settings.provider.name(),
                    a.settings.model()
                ))
                .font(font(look, 11.5))
                .color(color(p.text_secondary)),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button(ui, look, "close", lang.t("Close")).clicked() {
                *close = true;
            }
            if icon_button(ui, look, "settings", lang.t("Preferences")).clicked() {
                acts.push(Act::Preferences);
            }
            if !a.busy()
                && !a.items.is_empty()
                && icon_button(ui, look, "new", lang.t("New chat")).clicked()
            {
                *new_chat = true;
            }
        });
    });
}

fn transcript(ui: &mut Ui, look: &Look, a: &Assistant, host: &mut dyn Host, acts: &mut Vec<Act>) {
    let p = &look.palette;
    let lang = a.lang;
    ui.set_width(ui.available_width());
    if a.items.is_empty() {
        empty(ui, look, a, host, acts);
        return;
    }
    let last_undo = a
        .items
        .iter()
        .rposition(|i| matches!(i, Item::Note(Note::Applied { .. })));
    for (index, item) in a.items.iter().enumerate() {
        match item {
            Item::User(text) => {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    Frame::new()
                        .fill(color(p.accent_subtle))
                        .corner_radius(14)
                        .inner_margin(Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.set_max_width(ui.available_width() * 0.88);
                            ui.label(
                                RichText::new(text)
                                    .font(font(look, 14.0))
                                    .color(color(p.accent_text)),
                            );
                        });
                });
            }
            Item::Assistant(text) => {
                markdown::show(ui, look, lang, &format!("a{index}"), text);
            }
            Item::Tool(tool) => tool_card(ui, look, a, index, tool, acts),
            Item::Error(failure) => error_card(ui, look, a, failure, acts),
            Item::Note(note) => note_row(ui, look, a, note, last_undo == Some(index), acts),
        }
        ui.add_space(8.0);
    }
    if a.busy() && !a.waiting_for_approval() {
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new().size(14.0));
            ui.label(
                RichText::new(lang.t("Thinking…"))
                    .font(font(look, 13.0))
                    .color(color(p.text_secondary)),
            );
        });
    }
}

fn empty(ui: &mut Ui, look: &Look, a: &Assistant, host: &mut dyn Host, acts: &mut Vec<Act>) {
    let p = &look.palette;
    let lang = a.lang;
    ui.add_space(24.0);
    if let Err(f) = a.ready() {
        if matches!(f, Failure::NoKey | Failure::NoModel) {
            ui.label(
                RichText::new(lang.t("Set up the assistant"))
                    .font(egui::FontId::new(17.0, look.bold.clone()))
                    .color(color(p.text_primary)),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(lang.t("Choose a provider and add your own API key. Nodalia has no servers in between: your questions go straight to the provider you choose."))
                    .font(font(look, 13.5))
                    .color(color(p.text_secondary)),
            );
            ui.add_space(12.0);
            if text_button(ui, look, lang.t("Open Preferences"), Weight::Primary).clicked() {
                acts.push(Act::Preferences);
            }
            return;
        }
    }
    ui.label(
        RichText::new(lang.fill("Ask {app} anything", &[("app", host.app_name())]))
            .font(egui::FontId::new(17.0, look.bold.clone()))
            .color(color(p.text_primary)),
    );
    ui.add_space(6.0);
    ui.label(
        RichText::new(lang.t("It can read your document and, with your approval, change it."))
            .font(font(look, 13.5))
            .color(color(p.text_secondary)),
    );
    ui.add_space(14.0);
    for suggestion in host.suggestions(lang) {
        let (rect, resp) =
            ui.allocate_exact_size(vec2(ui.available_width(), 38.0), egui::Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &suggestion));
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            10.0,
            color(if resp.hovered() {
                p.control_bg_hover
            } else {
                p.control_bg
            }),
        );
        painter.text(
            egui::pos2(rect.left() + 12.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &suggestion,
            font(look, 13.5),
            color(p.text_primary),
        );
        if resp.clicked() {
            acts.push(Act::Send(suggestion));
        }
        ui.add_space(6.0);
    }
}

fn tool_card(
    ui: &mut Ui,
    look: &Look,
    a: &Assistant,
    index: usize,
    tool: &ToolItem,
    acts: &mut Vec<Act>,
) {
    let p = &look.palette;
    let lang = a.lang;
    let (icon, ink) = match &tool.state {
        ToolState::Waiting => ("pause", p.status_warning_fg),
        ToolState::Done if tool.changes => ("check", p.status_success_fg),
        ToolState::Done => ("view", p.text_secondary),
        ToolState::Failed(_) => ("warning", p.status_danger_fg),
        ToolState::Skipped => ("close", p.text_secondary),
    };
    Frame::new()
        .fill(color(p.surface_sunken))
        .corner_radius(12)
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::hover());
                if let Some(i) = icons::get(icon) {
                    icons::paint(ui.painter(), rect, i, icons::Style::bold(color(ink)));
                }
                ui.vertical(|ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(&tool.summary)
                                .font(font(look, 13.0))
                                .color(color(p.text_primary)),
                        )
                        .wrap(),
                    );
                    match &tool.state {
                        ToolState::Waiting => {
                            ui.label(
                                RichText::new(lang.t("Waiting for your approval"))
                                    .font(font(look, 12.0))
                                    .color(color(p.status_warning_fg)),
                            );
                        }
                        ToolState::Skipped => {
                            ui.label(
                                RichText::new(lang.t("Skipped"))
                                    .font(font(look, 12.0))
                                    .color(color(p.text_secondary)),
                            );
                        }
                        ToolState::Failed(why) => {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(format!("{}: {why}", lang.t("Failed")))
                                        .font(font(look, 12.0))
                                        .color(color(p.status_danger_fg)),
                                )
                                .wrap(),
                            );
                        }
                        ToolState::Done => {}
                    }
                });
            });
            if !tool.detail.is_empty() && tool.detail != "{}" {
                egui::CollapsingHeader::new(
                    RichText::new(lang.t("Details"))
                        .font(font(look, 12.0))
                        .color(color(p.text_secondary)),
                )
                .id_salt(("ndl_assistant_detail", index))
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(&tool.detail)
                                .font(egui::FontId::new(11.5, egui::FontFamily::Monospace))
                                .color(color(p.text_secondary)),
                        )
                        .wrap(),
                    );
                });
            }
            if tool.state == ToolState::Waiting && a.waiting_for_approval() {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    if text_button(ui, look, lang.t("Apply"), Weight::Primary).clicked() {
                        acts.push(Act::Approve);
                    }
                    if text_button(ui, look, lang.t("Skip"), Weight::Secondary).clicked() {
                        acts.push(Act::Skip);
                    }
                    if text_button(ui, look, lang.t("Apply all"), Weight::Secondary).clicked() {
                        acts.push(Act::ApproveAll);
                    }
                });
            }
        });
}

fn error_card(ui: &mut Ui, look: &Look, a: &Assistant, failure: &Failure, acts: &mut Vec<Act>) {
    let p = &look.palette;
    let lang = a.lang;
    Frame::new()
        .fill(color(p.status_danger_bg))
        .corner_radius(12)
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    RichText::new(failure.describe(lang))
                        .font(font(look, 13.0))
                        .color(color(p.status_danger_fg)),
                )
                .wrap(),
            );
            if matches!(
                failure,
                Failure::NoKey | Failure::NoModel | Failure::InsecureAddress | Failure::Rejected(_)
            ) {
                ui.add_space(6.0);
                if text_button(ui, look, lang.t("Open Preferences"), Weight::Secondary).clicked() {
                    acts.push(Act::Preferences);
                }
            }
        });
}

fn note_row(
    ui: &mut Ui,
    look: &Look,
    a: &Assistant,
    note: &Note,
    newest_applied: bool,
    acts: &mut Vec<Act>,
) {
    let p = &look.palette;
    let lang = a.lang;
    let say = |ui: &mut Ui, text: &str| {
        ui.add(
            egui::Label::new(
                RichText::new(text)
                    .font(font(look, 12.5))
                    .color(color(p.text_secondary)),
            )
            .wrap(),
        );
    };
    match note {
        Note::Stopped => say(ui, lang.t("Stopped.")),
        Note::Truncated => say(
            ui,
            lang.t("The answer was cut because it reached the model's length limit."),
        ),
        Note::Refused => say(ui, lang.t("The model declined to answer.")),
        Note::Empty => say(ui, lang.t("The model sent an empty answer.")),
        Note::Applied { steps, undone } => {
            ui.horizontal(|ui| {
                let text = if *steps == 1 {
                    lang.t("1 change made").to_string()
                } else {
                    lang.fill("{count} changes made", &[("count", &steps.to_string())])
                };
                say(ui, &text);
                if *undone {
                    say(ui, lang.t("Undone"));
                } else if newest_applied
                    && !a.busy()
                    && text_button(ui, look, lang.t("Undo"), Weight::Secondary).clicked()
                {
                    acts.push(Act::Undo);
                }
            });
        }
    }
}

fn composer(ui: &mut Ui, look: &Look, a: &mut Assistant, acts: &mut Vec<Act>) {
    let p = &look.palette;
    let lang = a.lang;
    let id = ui.make_persistent_id("ndl_assistant_input");
    // Enter sends, Shift+Enter breaks the line.
    let enter = ui.memory(|m| m.has_focus(id))
        && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
    Frame::new()
        .fill(color(p.control_field_bg))
        .stroke(Stroke::new(1.0, color(p.border_default)))
        .corner_radius(12)
        .inner_margin(Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                TextEdit::multiline(&mut a.draft)
                    .id(id)
                    .hint_text(lang.t("Ask something…"))
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .frame(Frame::NONE)
                    .font(font(look, 14.0)),
            );
            ui.horizontal(|ui| {
                let status = match (a.keys.state(a.settings.provider), a.busy()) {
                    (crate::secrets::KeyState::Loading, true) => {
                        lang.t("Waiting for the keychain…")
                    }
                    _ => "",
                };
                ui.label(
                    RichText::new(status)
                        .font(font(look, 12.0))
                        .color(color(p.text_secondary)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if a.busy() {
                        if text_button(ui, look, lang.t("Stop"), Weight::Secondary).clicked() {
                            a.stop();
                        }
                    } else {
                        let can = !a.draft.trim().is_empty();
                        let resp = ui.add_enabled_ui(can, |ui| {
                            text_button(ui, look, lang.t("Send"), Weight::Primary)
                        });
                        if (resp.inner.clicked() && can) || (enter && can) {
                            acts.push(Act::Send(a.draft.clone()));
                        }
                    }
                });
            });
        });
    ui.add_space(4.0);
    ui.label(
        RichText::new(lang.t("AI can make mistakes. Check what matters."))
            .font(font(look, 11.5))
            .color(color(p.text_tertiary)),
    );
}
