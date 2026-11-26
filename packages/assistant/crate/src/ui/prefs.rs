//! The Preferences section: provider, model, key, address and approval.

use egui::{vec2, Id, RichText, TextEdit, Ui};
use nodalia_theme::for_egui::color;
use nodalia_theme::office::{self, Look, CONTROL_WIDTH};

use super::{font, text_button, Weight};
use crate::agent::{Assistant, ModelList};
use crate::provider::Provider;
use crate::secrets::KeyState;
use crate::settings::Approval;

/// The "Assistant" section of the app's Preferences page.
pub fn preferences(ui: &mut Ui, look: &Look, a: &mut Assistant) {
    let lang = a.lang;
    office::section(ui, look, lang.t("Assistant"), |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            provider_row(ui, look, a);
            model_row(ui, look, a);
            if a.settings.provider.needs_key() || a.settings.provider == Provider::Custom {
                key_row(ui, look, a);
            }
            if a.settings.provider.custom_address() {
                address_row(ui, look, a);
            }
            row(ui, look, lang.t("Ask before changing the document"), |ui| {
                let mut ask = a.settings.approval == Approval::Ask;
                let check = ui.checkbox(&mut ask, "");
                let name = lang.t("Ask before changing the document");
                check.widget_info(|| {
                    egui::WidgetInfo::selected(egui::Role::CheckBox, true, ask, name)
                });
                if check.changed() {
                    a.settings.approval = if ask { Approval::Ask } else { Approval::Auto };
                    a.save();
                }
            });
            row(ui, look, lang.t("Show the assistant button"), |ui| {
                let check = ui.checkbox(&mut a.settings.enabled, "");
                let name = lang.t("Show the assistant button");
                check.widget_info(|| {
                    egui::WidgetInfo::selected(egui::Role::CheckBox, true, a.settings.enabled, name)
                });
                if check.changed() {
                    a.save();
                }
            });
        });
        ui.add_space(6.0);
        let note = if a.settings.provider.needs_key() {
            lang.t("Your key stays on this computer, in the system keychain, and goes only to the provider you chose. You pay the provider for what you use; Nodalia charges nothing.")
        } else {
            lang.t("No key needed for a model on this computer.")
        };
        office::hint(ui, look, note);
    });
}

/// Width of the label column of a Preferences row.
const LABEL_WIDTH: f32 = 280.0;

/// A Preferences row: the label on the left, the control in a column [`CONTROL_WIDTH`] wide
/// that grows with what is in it, so a key's status lines and a failed connection never overlap
/// the row below. The label stays level with the first line of the control.
fn row<R>(ui: &mut Ui, look: &Look, label: &str, control: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            vec2(LABEL_WIDTH, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(LABEL_WIDTH);
                ui.add_space(5.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(label)
                            .font(font(look, 13.5))
                            .color(color(look.palette.text_primary)),
                    )
                    .wrap(),
                );
            },
        );
        ui.vertical(|ui| {
            ui.set_width(CONTROL_WIDTH);
            ui.spacing_mut().combo_width = CONTROL_WIDTH;
            control(ui)
        })
        .inner
    })
    .inner
}

fn provider_row(ui: &mut Ui, look: &Look, a: &mut Assistant) {
    let lang = a.lang;
    row(ui, look, lang.t("Provider"), |ui| {
        let mut chosen = None;
        let name = lang.t("Provider");
        let combo = egui::ComboBox::from_id_salt("ndl_assistant_provider")
            .selected_text(a.settings.provider.name())
            .show_ui(ui, |ui| {
                for p in Provider::ALL {
                    if ui
                        .selectable_label(p == a.settings.provider, p.name())
                        .clicked()
                    {
                        chosen = Some(p);
                    }
                }
            });
        combo
            .response
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, name));
        if let Some(p) = chosen {
            a.set_provider(p);
        }
    });
}

fn model_row(ui: &mut Ui, look: &Look, a: &mut Assistant) {
    let lang = a.lang;
    let provider = a.settings.provider;
    row(ui, look, lang.t("Model"), |ui| {
        ui.vertical(|ui| {
            let mut options: Vec<String> = match a.models() {
                ModelList::Ready(list) => list.clone(),
                _ => provider
                    .suggested_models()
                    .iter()
                    .map(|m| m.to_string())
                    .collect(),
            };
            let current = a.settings.model();
            if !current.is_empty() && !options.contains(&current) {
                options.insert(0, current.clone());
            }
            if options.is_empty() {
                let id = ui.make_persistent_id(("ndl_assistant_model", provider.id()));
                if let Some(text) = field(ui, id, &current, lang.t("Choose a model"), false) {
                    a.settings.set_model(&text);
                    a.save();
                }
            } else {
                let shown = if current.is_empty() {
                    lang.t("Choose a model").to_string()
                } else {
                    current.clone()
                };
                let mut chosen = None;
                let name = lang.t("Model");
                let combo =
                    egui::ComboBox::from_id_salt(("ndl_assistant_model_list", provider.id()))
                        .selected_text(shown)
                        .width(CONTROL_WIDTH)
                        .show_ui(ui, |ui| {
                            for m in &options {
                                if ui.selectable_label(*m == current, m).clicked() {
                                    chosen = Some(m.clone());
                                }
                            }
                        });
                combo
                    .response
                    .widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, name));
                if let Some(m) = chosen {
                    a.settings.set_model(&m);
                    a.save();
                }
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let loading = matches!(a.models(), ModelList::Loading);
                if text_button(
                    ui,
                    look,
                    if loading {
                        lang.t("Checking…")
                    } else {
                        lang.t("Check connection")
                    },
                    Weight::Secondary,
                )
                .clicked()
                    && !loading
                {
                    a.refresh_models();
                }
            });
            match a.models() {
                ModelList::Ready(list) => office::hint(
                    ui,
                    look,
                    &lang.fill(
                        "Connected. {count} models available.",
                        &[("count", &list.len().to_string())],
                    ),
                ),
                ModelList::Failed(f) => {
                    ui.add(
                        egui::Label::new(
                            RichText::new(f.describe(lang))
                                .font(font(look, 12.5))
                                .color(color(look.palette.status_danger_fg)),
                        )
                        .wrap(),
                    );
                }
                ModelList::Idle | ModelList::Loading => {}
            }
        });
    });
}

fn key_row(ui: &mut Ui, look: &Look, a: &mut Assistant) {
    let lang = a.lang;
    let provider = a.settings.provider;
    row(ui, look, lang.t("API key"), |ui| {
        ui.vertical(|ui| {
            let id = ui.make_persistent_id(("ndl_assistant_key", provider.id()));
            let mut typed: String = ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_default();
            let resp = ui.add(
                TextEdit::singleline(&mut typed)
                    .id(id)
                    .password(true)
                    .hint_text(lang.t("Paste your key"))
                    .desired_width(CONTROL_WIDTH),
            );
            let submitted = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.add_space(4.0);
            let mut save = submitted;
            let mut remove = false;
            ui.horizontal_wrapped(|ui| {
                if text_button(ui, look, lang.t("Save key"), Weight::Primary).clicked() {
                    save = true;
                }
                if a.keys.get(provider).is_some()
                    && text_button(ui, look, lang.t("Remove key"), Weight::Secondary).clicked()
                {
                    remove = true;
                }
            });
            if save && !typed.trim().is_empty() {
                a.keys.set(provider, &typed);
                typed.clear();
                a.refresh_models();
            }
            if remove {
                a.keys.set(provider, "");
            }
            ui.data_mut(|d| d.insert_temp(id, typed));
            let status = match a.keys.state(provider) {
                KeyState::Loading => lang.t("Checking the keychain…").to_string(),
                KeyState::Missing => lang.t("No key saved.").to_string(),
                KeyState::Stored => lang.t("A key is saved in your system keychain.").to_string(),
                KeyState::SessionOnly => lang
                    .t("The system keychain is not available. The key will be kept until you close the app.")
                    .to_string(),
            };
            office::hint(ui, look, &status);
            // The keychain's own words are platform text in English, so the interface says it
            // in its own words and the detail stays out of a translated window.
            if a.keys.problem.is_some() && a.keys.state(provider) != KeyState::SessionOnly {
                office::hint(ui, look, lang.t("The system keychain could not be read."));
            }
            if let Some(url) = provider.key_page() {
                ui.hyperlink_to(lang.t("Get a key"), url);
            }
        });
    });
}

fn address_row(ui: &mut Ui, look: &Look, a: &mut Assistant) {
    let lang = a.lang;
    let provider = a.settings.provider;
    row(ui, look, lang.t("Server address"), |ui| {
        let id = ui.make_persistent_id(("ndl_assistant_address", provider.id()));
        if let Some(text) = field(
            ui,
            id,
            &a.settings.address(),
            provider.default_base_url(),
            false,
        ) {
            a.settings.set_address(&text);
            a.save();
        }
    });
}

/// A one-line field edited through a buffer, so trimming what is stored does not fight the
/// typing. Returns the text when it changed.
fn field(ui: &mut Ui, id: Id, current: &str, hint: &str, password: bool) -> Option<String> {
    let focused = ui.memory(|m| m.has_focus(id));
    let mut text = if focused {
        ui.data_mut(|d| d.get_temp::<String>(id))
            .unwrap_or_else(|| current.to_string())
    } else {
        current.to_string()
    };
    let resp = ui.add(
        TextEdit::singleline(&mut text)
            .id(id)
            .password(password)
            .hint_text(hint)
            .desired_width(CONTROL_WIDTH),
    );
    let changed = resp.changed();
    if resp.has_focus() || changed {
        ui.data_mut(|d| d.insert_temp(id, text.clone()));
    }
    changed.then_some(text)
}
