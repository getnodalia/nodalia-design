//! The assistant's interface, the same in every Nodalia app: a button with the provider's mark,
//! a chat panel (an island on the right) and a section for Preferences.
//!
//! The app owns the layout. It puts [`button`] in its title bar, [`panel`] in a right-hand
//! container [`PANEL_WIDTH`] wide while [`Assistant::open`] is true, and [`preferences`] in its
//! Preferences page, and it calls [`Assistant::poll`] once per frame.
//!
//! ## Provider marks
//!
//! Anthropic, OpenAI and the other makers allow their logos only with permission, so the crate
//! ships none. A provider is shown as a neutral badge with its initial. An app that has the
//! right to use an official mark registers it with [`set_mark`] and it replaces the badge
//! everywhere.

mod chat;
pub mod markdown;
mod prefs;

use std::sync::RwLock;

use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Rect, Response, Sense, Ui};
use nodalia_theme::for_egui::color;
use nodalia_theme::office::Look;

use crate::agent::Assistant;
use crate::provider::Provider;

pub use chat::{panel, Outcome};
pub use prefs::preferences;

/// The width of the chat panel's content.
pub const PANEL_WIDTH: f32 = 400.0;
/// The size of the title-bar button.
pub const BUTTON_SIZE: f32 = 34.0;

/// Draws an official mark inside `rect` in `ink`.
pub type Mark = fn(&Painter, Rect, Color32);

static MARKS: RwLock<Vec<(Provider, Mark)>> = RwLock::new(Vec::new());

/// Replaces `provider`'s badge with `mark`. Only for marks the app is allowed to use.
pub fn set_mark(provider: Provider, mark: Mark) {
    if let Ok(mut marks) = MARKS.write() {
        marks.retain(|(p, _)| *p != provider);
        marks.push((provider, mark));
    }
}

fn mark_of(provider: Provider) -> Option<Mark> {
    MARKS
        .read()
        .ok()
        .and_then(|m| m.iter().find(|(p, _)| *p == provider).map(|(_, f)| *f))
}

/// The letter a provider's badge shows.
pub fn monogram(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => "C",
        Provider::OpenAi => "O",
        Provider::Gemini => "G",
        Provider::Mistral => "M",
        Provider::OpenRouter => "R",
        Provider::Ollama => "L",
        Provider::Custom => "+",
    }
}

/// Draws `provider`'s badge in `rect`. `active` tints it with the app's accent.
pub fn badge(painter: &Painter, look: &Look, rect: Rect, provider: Provider, active: bool) {
    let p = &look.palette;
    let (fill, ink) = if active {
        (color(p.accent_subtle), color(p.accent_text))
    } else {
        (color(p.control_bg), color(p.text_primary))
    };
    painter.rect_filled(rect, rect.height() * 0.3, fill);
    if let Some(mark) = mark_of(provider) {
        mark(painter, rect.shrink(rect.height() * 0.22), ink);
    } else {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            monogram(provider),
            FontId::new(rect.height() * 0.5, look.bold.clone()),
            ink,
        );
    }
}

/// The button for the app's title bar: the selected provider's badge. Click it to open or close
/// the panel. A dot says the assistant is working, or waiting for you.
pub fn button(ui: &mut Ui, look: &Look, assistant: &mut Assistant) -> Response {
    let p = &look.palette;
    let (rect, resp) = ui.allocate_exact_size(vec2(BUTTON_SIZE, BUTTON_SIZE), Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::Role::Button,
            true,
            assistant.open,
            assistant.lang.t("Assistant"),
        )
    });
    let painter = ui.painter();
    if assistant.open {
        painter.rect_filled(rect, 10.0, color(p.accent_subtle));
    } else if resp.hovered() {
        painter.rect_filled(rect, 10.0, color(p.control_bg_hover));
    }
    badge(
        painter,
        look,
        Rect::from_center_size(rect.center(), vec2(24.0, 24.0)),
        assistant.settings.provider,
        assistant.open,
    );
    if assistant.waiting_for_approval() || assistant.busy() {
        let dot = if assistant.waiting_for_approval() {
            p.status_warning_fg
        } else {
            p.accent_default
        };
        painter.circle_filled(
            rect.right_top() + vec2(-6.0, 6.0),
            4.0,
            color(p.surface_window),
        );
        painter.circle_filled(rect.right_top() + vec2(-6.0, 6.0), 3.0, color(dot));
    }
    if resp.clicked() {
        assistant.open = !assistant.open;
    }
    resp
}

/// A square icon button. Returns the response; `label` is for screen readers.
pub(crate) fn icon_button(ui: &mut Ui, look: &Look, icon: &str, label: &str) -> Response {
    let p = &look.palette;
    let (rect, resp) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    let painter = ui.painter();
    if resp.hovered() {
        painter.rect_filled(rect, 8.0, color(p.control_bg_hover));
    }
    if let Some(icon) = nodalia_theme::icons::get(icon) {
        nodalia_theme::icons::paint(
            painter,
            Rect::from_center_size(rect.center(), vec2(18.0, 18.0)),
            icon,
            nodalia_theme::icons::Style::bold(color(p.text_secondary)),
        );
    }
    resp
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Weight {
    /// The accent.
    Primary,
    Secondary,
}

/// A text button, as tall as the NDL controls.
pub(crate) fn text_button(ui: &mut Ui, look: &Look, label: &str, weight: Weight) -> Response {
    let p = &look.palette;
    let font = FontId::new(13.0, look.bold.clone());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, Color32::WHITE);
    let size = vec2(galley.size().x + 24.0, 30.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    let (fill, ink) = match (weight, resp.hovered()) {
        (Weight::Primary, false) => (p.accent_default, p.text_on_accent),
        (Weight::Primary, true) => (p.accent_hover, p.text_on_accent),
        (Weight::Secondary, false) => (p.control_bg, p.text_primary),
        (Weight::Secondary, true) => (p.control_bg_hover, p.text_primary),
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 9.0, color(fill));
    painter.text(
        pos2(rect.center().x, rect.center().y),
        Align2::CENTER_CENTER,
        label,
        FontId::new(13.0, look.bold.clone()),
        color(ink),
    );
    resp
}

pub(crate) fn font(look: &Look, size: f32) -> FontId {
    FontId::new(size, look.regular.clone())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nodalia_theme::{Accent, Mode};
    use serde_json::{json, Value};

    use super::*;
    use crate::agent::{Item, Note, ToolItem, ToolState};
    use crate::host::{Host, ToolSpec};
    use crate::lang::Lang;
    use crate::settings::Settings;
    use crate::transport::Failure;

    struct Nothing;

    impl Host for Nothing {
        fn app_name(&self) -> &str {
            "Celdas"
        }
        fn guide(&self) -> String {
            String::new()
        }
        fn context(&mut self) -> Result<Value, String> {
            Ok(json!({}))
        }
        fn tools(&self) -> Vec<ToolSpec> {
            Vec::new()
        }
        fn call(&mut self, _: &str, _: &Value) -> Result<Value, String> {
            Ok(Value::Null)
        }
        fn suggestions(&self, _: Lang) -> Vec<String> {
            vec!["Sum column B".into()]
        }
        fn undo_depth(&mut self) -> usize {
            0
        }
        fn undo(&mut self, _: usize) -> Result<(), String> {
            Ok(())
        }
    }

    fn assistant(provider: Provider, lang: Lang) -> Assistant {
        let mut a = Assistant::with_settings(
            Settings {
                provider,
                ..Settings::default()
            },
            Arc::new(|| {}),
        );
        a.lang = lang;
        a.open = true;
        a
    }

    fn frames(a: &mut Assistant, mode: Mode) {
        let ctx = egui::Context::default();
        let look = Look::new(mode.palette_with(Accent::Celdas));
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 800.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                button(ui, &look, a);
                panel(ui, &look, a, &mut Nothing);
                preferences(ui, &look, a);
            });
            out.textures_delta.clear();
        }
    }

    #[test]
    fn every_state_of_the_panel_and_preferences_draws() {
        for lang in [Lang::English, Lang::Spanish] {
            for mode in [Mode::Light, Mode::Dark] {
                // Empty chat, with and without a model to ask.
                for provider in Provider::ALL {
                    frames(&mut assistant(provider, lang), mode);
                }
                let mut a = assistant(Provider::Ollama, lang);
                a.settings.set_model("llama3");
                frames(&mut a, mode);
                a.items = vec![
                    Item::User("Summarise **this**".into()),
                    Item::Assistant("# Title\n\n- one\n- two\n\n```\ncode\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |".into()),
                    Item::Tool(ToolItem {
                        name: "write".into(),
                        summary: "write(cell=\"A1\")".into(),
                        detail: "{\"cell\":\"A1\"}".into(),
                        changes: true,
                        state: ToolState::Waiting,
                    }),
                    Item::Tool(ToolItem {
                        name: "read".into(),
                        summary: "read()".into(),
                        detail: "{}".into(),
                        changes: false,
                        state: ToolState::Failed("bad range".into()),
                    }),
                    Item::Error(Failure::NoKey),
                    Item::Error(Failure::Network("down".into())),
                    Item::Note(Note::Stopped),
                    Item::Note(Note::Truncated),
                    Item::Note(Note::Applied {
                        steps: 3,
                        undone: false,
                    }),
                    Item::Note(Note::Applied {
                        steps: 1,
                        undone: true,
                    }),
                ];
                frames(&mut a, mode);
            }
        }
    }

    #[test]
    fn a_registered_mark_replaces_the_badge() {
        fn mark(_: &Painter, _: Rect, _: Color32) {}
        assert!(mark_of(Provider::Mistral).is_none());
        set_mark(Provider::Mistral, mark);
        assert!(mark_of(Provider::Mistral).is_some());
        assert!(mark_of(Provider::Gemini).is_none());
        // Every provider has a badge letter of its own.
        let mut letters: Vec<_> = Provider::ALL.iter().map(|p| monogram(*p)).collect();
        letters.sort_unstable();
        letters.dedup();
        assert_eq!(letters.len(), Provider::ALL.len());
    }
}
