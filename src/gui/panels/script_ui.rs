// gui/panels/script_ui.rs — The controls the loaded script declares
//
// Sliders, numbers, toggles, choices and buttons from the script's `ui`
// table (see lua_interface/ui.rs). Every change goes to the engine at once,
// so a running script reads it on its next tick; a loaded script that has
// not been started yet starts with them.

use iced::widget::{button, checkbox, column, container, pick_list, row, slider, text, text_input, Space};
use iced::{alignment, Alignment, Element, Length};
use std::collections::HashMap;

use crate::gui::theme::{self, btn, Btn};
use crate::lua_interface::{ScriptUiView, UiEvent, UiKind, UiValue, UiWidget};

#[derive(Debug, Clone)]
pub enum ScriptUiMessage {
    Slide(String, f64),
    /// Text typed in a number field.
    Type(String, String),
    /// Enter in a number field.
    Submit(String),
    Toggle(String, bool),
    Choose(String, String),
    Press(String),
}

#[derive(Default)]
pub struct ScriptUiPanel {
    view: ScriptUiView,
    /// Number fields being typed in, by control name.
    typed: HashMap<String, String>,
}

impl ScriptUiPanel {
    /// The controls as the engine has them now.
    pub fn set_view(&mut self, view: ScriptUiView) {
        self.typed.retain(|name, _| view.widgets.iter().any(|w| &w.name == name && w.kind == UiKind::Number));
        self.view = view;
    }

    /// The script listens to field clicks.
    pub fn on_click(&self) -> bool {
        self.view.on_click
    }

    /// What to send to the engine, if anything.
    pub fn update(&mut self, message: ScriptUiMessage) -> Option<UiEvent> {
        let (name, value) = match message {
            ScriptUiMessage::Press(name) => return Some(UiEvent::Press { name }),
            ScriptUiMessage::Type(name, typed) => {
                self.typed.insert(name, typed);
                return None;
            }
            ScriptUiMessage::Submit(name) => {
                let typed = self.typed.remove(&name)?;
                let v = typed.trim().parse::<f64>().ok().filter(|v| v.is_finite())?;
                (name, UiValue::Number(v))
            }
            ScriptUiMessage::Slide(name, v) => {
                let step = match self.widget(&name).map(|w| &w.kind) {
                    Some(UiKind::Slider { step, .. }) => *step,
                    _ => return None,
                };
                (name, UiValue::Number(snap(v, step)))
            }
            ScriptUiMessage::Toggle(name, on) => (name, UiValue::Bool(on)),
            ScriptUiMessage::Choose(name, option) => (name, UiValue::Text(option)),
        };
        // Shown at once; the engine's next view confirms it.
        let widget = self.view.widgets.iter_mut().find(|w| w.name == name)?;
        widget.value = value.clone();
        Some(UiEvent::Set { name, value })
    }

    fn widget(&self, name: &str) -> Option<&UiWidget> {
        self.view.widgets.iter().find(|w| w.name == name)
    }

    pub fn view(&self) -> Element<'_, ScriptUiMessage> {
        let mut rows = column![].spacing(6);

        if self.view.widgets.is_empty() {
            rows = rows.push(
                text("The loaded script declares no controls. Scripts add them with ui.slider, ui.number, ui.toggle, ui.choice and ui.button (see lua/demo/ui_demo.lua).")
                    .size(12)
                    .color(theme::MUTED),
            );
        }

        let params: Vec<&UiWidget> = self.view.widgets.iter().filter(|w| w.kind != UiKind::Button).collect();
        if !params.is_empty() {
            let mut group = column![section_title("PARAMETERS")].spacing(4);
            for widget in params {
                group = group.push(self.param_row(widget));
            }
            rows = rows.push(container(group).padding([8, 10]).width(Length::Fill).style(theme::group));
        }

        let buttons: Vec<&UiWidget> = self.view.widgets.iter().filter(|w| w.kind == UiKind::Button).collect();
        if !buttons.is_empty() {
            let mut line = row![].spacing(6);
            for widget in buttons {
                line = line.push(
                    button(text(widget.name.as_str()).size(12))
                        .padding([4, 10])
                        .style(btn(Btn::Neutral))
                        .on_press(ScriptUiMessage::Press(widget.name.clone())),
                );
            }
            let group = column![section_title("ACTIONS"), line.wrap()].spacing(6);
            rows = rows.push(container(group).padding([8, 10]).width(Length::Fill).style(theme::group));
        }

        if self.view.on_click {
            rows = rows.push(
                text("This script listens to the field: Ctrl+click (left or right) sends the point to it.")
                    .size(11)
                    .color(theme::ACCENT_FG),
            );
        }

        rows.padding(10).into()
    }

    fn param_row<'a>(&'a self, widget: &'a UiWidget) -> Element<'a, ScriptUiMessage> {
        let name = widget.name.clone();
        let label = text(widget.name.as_str()).size(12).color(theme::TEXT_2).width(Length::Fixed(110.0));

        let control: Element<'a, ScriptUiMessage> = match (&widget.kind, &widget.value) {
            (UiKind::Slider { min, max, step }, UiValue::Number(v)) => row![
                slider(*min..=*max, *v, move |v| ScriptUiMessage::Slide(name.clone(), v))
                    .step(*step)
                    .width(Length::Fill),
                text(format_number(*v)).size(12).font(theme::MONO).width(Length::Fixed(56.0)).align_x(alignment::Horizontal::Right),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into(),
            (UiKind::Number, UiValue::Number(v)) => {
                let typed = self.typed.get(&widget.name);
                let shown = typed.cloned().unwrap_or_else(|| format_number(*v));
                let submit = ScriptUiMessage::Submit(name.clone());
                let mut style_invalid = false;
                if let Some(t) = typed {
                    style_invalid = t.trim().parse::<f64>().map_or(true, |v| !v.is_finite());
                }
                row![
                    text_input("", &shown)
                        .on_input(move |t| ScriptUiMessage::Type(name.clone(), t))
                        .on_submit(submit)
                        .font(theme::MONO)
                        .size(12)
                        .padding([2, 6])
                        .align_x(alignment::Horizontal::Right)
                        .width(Length::Fixed(90.0))
                        .style(move |t, s| {
                            let mut style = theme::input(t, s);
                            if typed.is_some() {
                                style.border.color = if style_invalid { theme::RED } else { theme::ACCENT };
                            }
                            style
                        }),
                    text(if typed.is_some() { "Enter to apply" } else { "" }).size(10).color(theme::MUTED),
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .into()
            }
            (UiKind::Toggle, UiValue::Bool(on)) => checkbox(*on)
                .on_toggle(move |on| ScriptUiMessage::Toggle(name.clone(), on))
                .style(theme::check)
                .size(14)
                .into(),
            (UiKind::Choice { options }, UiValue::Text(selected)) => pick_list(
                options.as_slice(),
                Some(selected.clone()),
                move |option| ScriptUiMessage::Choose(name.clone(), option),
            )
            .text_size(12)
            .into(),
            _ => Space::new().into(),
        };

        row![label, control].spacing(6).align_y(Alignment::Center).into()
    }
}

fn section_title<'a>(title: &'a str) -> Element<'a, ScriptUiMessage> {
    text(title).size(11).font(theme::MONO_BOLD).color(theme::MUTED).into()
}

/// `v` rounded to the decimals of `step` (0.45, not 0.45000000000000007).
fn snap(v: f64, step: f64) -> f64 {
    let decimals = (-step.log10()).ceil().clamp(0.0, 9.0) as usize;
    format!("{v:.decimals$}").parse().unwrap_or(v)
}

fn format_number(v: f64) -> String {
    let s = format!("{:.4}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".to_string() } else { s.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> ScriptUiPanel {
        let mut panel = ScriptUiPanel::default();
        panel.set_view(ScriptUiView {
            widgets: vec![
                UiWidget { name: "speed".into(), kind: UiKind::Slider { min: 0.0, max: 4.0, step: 0.05 }, value: UiValue::Number(1.0) },
                UiWidget { name: "id".into(), kind: UiKind::Number, value: UiValue::Number(0.0) },
                UiWidget { name: "go".into(), kind: UiKind::Button, value: UiValue::Bool(false) },
            ],
            on_click: true,
        });
        panel
    }

    #[test]
    fn slides_are_sent_rounded_to_the_step() {
        let mut panel = panel();
        let event = panel.update(ScriptUiMessage::Slide("speed".into(), 0.35 + 0.1));
        assert_eq!(event, Some(UiEvent::Set { name: "speed".into(), value: UiValue::Number(0.45) }));
        assert_eq!(panel.widget("speed").unwrap().value, UiValue::Number(0.45));
    }

    #[test]
    fn numbers_are_sent_on_enter_when_valid() {
        let mut panel = panel();
        assert_eq!(panel.update(ScriptUiMessage::Type("id".into(), "3".into())), None);
        let event = panel.update(ScriptUiMessage::Submit("id".into()));
        assert_eq!(event, Some(UiEvent::Set { name: "id".into(), value: UiValue::Number(3.0) }));

        panel.update(ScriptUiMessage::Type("id".into(), "three".into()));
        assert_eq!(panel.update(ScriptUiMessage::Submit("id".into())), None);
    }

    #[test]
    fn presses_are_sent() {
        let mut panel = panel();
        assert_eq!(panel.update(ScriptUiMessage::Press("go".into())), Some(UiEvent::Press { name: "go".into() }));
    }

    #[test]
    fn numbers_read_cleanly() {
        assert_eq!(format_number(1.0), "1");
        assert_eq!(format_number(0.45), "0.45");
        assert_eq!(format_number(-0.00001), "0");
    }
}
