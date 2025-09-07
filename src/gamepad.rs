use std::time::{Duration, Instant};

use egui::{Align, FontSelection, RichText, Style, Widget, text::LayoutJob};
use gilrs::{
    Axis, Button, EventType, Filter, GamepadId, Gilrs, GilrsBuilder,
    ev::filter::{FilterFn, Repeat, axis_dpad_to_button},
};

use crate::{Event, toast::Toast};

pub struct Gamepad {
    gilrs: Gilrs,
    just_pressed: Vec<Button>,
    last_input: Instant,
    used_gamepads: Vec<GamepadId>,
}

impl Gamepad {
    pub fn new() -> Self {
        Self {
            gilrs: GilrsBuilder::new()
                .with_default_filters(false)
                .build()
                .expect("Failed to initialize Gilrs"),
            just_pressed: Vec::new(),
            last_input: Instant::now(),
            used_gamepads: Vec::new(),
        }
    }

    pub fn update(&mut self) -> Event {
        self.just_pressed.clear();

        while let Some(ev @ gilrs::Event { id, event, .. }) = self
            .gilrs
            .next_event()
            .filter_ev(&LeftStickToDPad { threshold: 0.3 }, &mut self.gilrs)
            .filter_ev(&axis_dpad_to_button, &mut self.gilrs)
            .filter_ev(
                &Repeat {
                    after: Duration::from_millis(300),
                    every: Duration::from_secs(1) / 30,
                },
                &mut self.gilrs,
            )
        {
            self.gilrs.update(&ev);

            if !self.used_gamepads.contains(&id) {
                self.used_gamepads.push(id);
            }

            self.last_input = Instant::now();

            match event {
                EventType::ButtonPressed(button, _) | EventType::ButtonRepeated(button, _) => {
                    self.just_pressed.push(button)
                }
                EventType::Connected => {
                    return Event::Toast(Toast::GamepadConnected {
                        name: self.gilrs.gamepad(id).name().to_string(),
                    });
                }
                EventType::Disconnected => {
                    if self.used_gamepads.is_empty() {
                        continue;
                    }

                    self.used_gamepads.retain(|&g| g != id);

                    if self.used_gamepads.is_empty() {
                        return Event::LastGamepadDisconnected;
                    } else {
                        return Event::Toast(Toast::GamepadDisconnected {
                            name: self.gilrs.gamepad(id).name().to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        Event::None
    }

    pub fn get_just_pressed(&self) -> Vec<Button> {
        self.just_pressed.clone()
    }

    pub fn inactive_for(&self, duration: Duration) -> bool {
        self.last_input.elapsed() > duration
    }
}

impl Default for Gamepad {
    fn default() -> Self {
        Self::new()
    }
}

pub fn button_label(button: Button) -> &'static str {
    match button {
        Button::East => "\u{e005}",
        Button::South => "\u{e007}",
        Button::North => "\u{e019}",
        Button::West => "\u{e01b}",
        Button::C => "🇨",
        Button::Z => "🇿",

        Button::LeftTrigger => "L1",
        Button::LeftTrigger2 => "L2",
        Button::RightTrigger => "R1",
        Button::RightTrigger2 => "R2",

        Button::Select => "\u{e00d}",
        Button::Start => "\u{e00f}",
        Button::Mode => "⭐",

        Button::LeftThumb => "L3",
        Button::RightThumb => "R3",

        Button::DPadUp => "⏶",
        Button::DPadDown => "⏷",
        Button::DPadLeft => "⏴",
        Button::DPadRight => "⏵",

        Button::Unknown => "?",
    }
}

pub fn button_prompt(button: Button, label: &str) -> impl Widget {
    let s = button_label(button);

    let mut job = LayoutJob::default();
    let style = Style::default();

    RichText::new(s)
        .size(24.)
        .append_to(&mut job, &style, FontSelection::Default, Align::Center);

    RichText::new(format!("  {label}")).append_to(
        &mut job,
        &style,
        FontSelection::Default,
        Align::Center,
    );

    egui::Label::new(job)
}

struct LeftStickToDPad {
    threshold: f32,
}

impl FilterFn for LeftStickToDPad {
    fn filter(&self, ev: Option<gilrs::Event>, _gilrs: &mut Gilrs) -> Option<gilrs::Event> {
        let mut ev = ev?;

        match &mut ev.event {
            EventType::AxisChanged(axis @ Axis::LeftStickX, value, _code) => {
                *axis = Axis::DPadX;

                if *value < -self.threshold {
                    *value = -1.0;
                } else if *value > self.threshold {
                    *value = 1.0;
                } else {
                    *value = 0.0;
                }
            }
            EventType::AxisChanged(axis @ Axis::LeftStickY, value, _code) => {
                *axis = Axis::DPadY;

                if *value < -self.threshold {
                    *value = -1.0;
                } else if *value > self.threshold {
                    *value = 1.0;
                } else {
                    *value = 0.0;
                }
            }
            _ => {}
        }

        Some(ev)
    }
}
