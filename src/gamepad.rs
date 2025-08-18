use std::time::{Duration, Instant};

use egui::{Align, FontSelection, RichText, Style, Widget, text::LayoutJob};
use gilrs::{Axis, Button, EventType, Gilrs};

use crate::{Command, toast::Toast};

pub const BUTTON_A: Button = Button::East;
pub const BUTTON_B: Button = Button::South;
pub const BUTTON_X: Button = Button::North;
pub const BUTTON_Y: Button = Button::West;

pub struct Gamepad {
    gilrs: Gilrs,
    just_pressed: Vec<Button>,
    last_input: Instant,
}

impl Gamepad {
    pub fn new() -> Self {
        Self {
            gilrs: Gilrs::new().expect("Failed to initialize Gilrs"),
            just_pressed: Vec::new(),
            last_input: Instant::now(),
        }
    }

    pub fn update(&mut self) -> Command {
        self.just_pressed.clear();

        while let Some(gilrs::Event { id, event, .. }) = self.gilrs.next_event() {
            println!("New event from {}: {:?}", id, event);

            self.last_input = Instant::now();

            match event {
                EventType::ButtonPressed(button, _) => self.just_pressed.push(button),
                EventType::AxisChanged(Axis::LeftStickX, value, _) if value < 0. => {
                    self.just_pressed.push(Button::DPadLeft);
                }
                EventType::AxisChanged(Axis::LeftStickX, value, _) if value > 0. => {
                    self.just_pressed.push(Button::DPadRight);
                }
                EventType::AxisChanged(Axis::LeftStickY, value, _) if value < 0. => {
                    // self.just_pressed.push(Button::DPadDown);
                    return Command::Toast(Toast::GamepadConnected {
                        name: self.gilrs.gamepad(id).name().to_string(),
                    });
                }
                EventType::AxisChanged(Axis::LeftStickY, value, _) if value > 0. => {
                    self.just_pressed.push(Button::DPadUp);
                }
                EventType::Connected => {
                    return Command::Toast(Toast::GamepadConnected {
                        name: self.gilrs.gamepad(id).name().to_string(),
                    });
                }
                _ => {}
            }
        }

        Command::None
    }

    pub fn get_just_pressed(&self) -> Vec<Button> {
        self.just_pressed.clone()
    }

    pub fn just_pressed(&self, button: Button) -> bool {
        self.just_pressed.contains(&button)
    }

    pub fn just_pressed_any(&self) -> bool {
        !self.just_pressed.is_empty()
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
