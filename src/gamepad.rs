use std::{
    collections::hash_map::Entry,
    time::{Duration, Instant},
};

use egui::{
    Align, FontSelection, RichText, Style,
    ahash::{HashMap, HashMapExt as _},
    text::LayoutJob,
};
use gilrs::{
    Axis, Button, EventType, Filter, GamepadId, Gilrs, GilrsBuilder, PowerInfo,
    ev::filter::{FilterFn, Repeat, axis_dpad_to_button},
};

use crate::{command::Event, ui::toast::Toast};

pub struct Gamepad {
    gilrs: Gilrs,
    just_pressed: Vec<Button>,
    last_input: Instant,
    used_gamepads: Vec<GamepadId>,
    power_states: HashMap<GamepadId, (PowerInfo, Instant)>,
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
            power_states: HashMap::new(),
        }
    }

    pub fn update(&mut self, events: &mut Vec<Event>) {
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

            self.update_power_state(id, events);

            match event {
                EventType::ButtonPressed(button, _) | EventType::ButtonRepeated(button, _)
                    if button != Button::Mode =>
                {
                    self.just_pressed.push(button)
                }
                EventType::ButtonReleased(button, _) if button == Button::Mode => {
                    self.just_pressed.push(button)
                }
                EventType::Connected => {
                    events.push(Event::Toast(Toast::GamepadConnected {
                        name: self.gilrs.gamepad(id).name().to_string(),
                    }));
                }
                EventType::Disconnected => {
                    if self.used_gamepads.is_empty() {
                        continue;
                    }

                    self.used_gamepads.retain(|&g| g != id);

                    if self.used_gamepads.is_empty() {
                        events.push(Event::LastGamepadDisconnected);
                    } else {
                        events.push(Event::Toast(Toast::GamepadDisconnected {
                            name: self.gilrs.gamepad(id).name().to_string(),
                        }));
                    }
                }
                _ => {}
            }
        }
    }

    fn update_power_state(&mut self, id: GamepadId, events: &mut Vec<Event>) {
        match self.power_states.entry(id) {
            Entry::Occupied(mut entry) => {
                if entry.get().1.elapsed() < Duration::from_secs(60) {
                    return;
                }

                let info = self.gilrs.gamepad(id).power_info();
                if entry.get().0 != info {
                    let prev = entry.insert((info, Instant::now()));
                    self.on_power_info_changed(id, Some(prev.0), info, events);
                } else {
                    entry.get_mut().1 = Instant::now();
                }
            }
            Entry::Vacant(entry) => {
                let info = self.gilrs.gamepad(id).power_info();
                entry.insert((info, Instant::now()));
                self.on_power_info_changed(id, None, info, events);
            }
        }
    }

    fn on_power_info_changed(
        &mut self,
        id: GamepadId,
        prev: Option<PowerInfo>,
        info: PowerInfo,
        events: &mut Vec<Event>,
    ) {
        match (prev, info) {
            (
                Some(PowerInfo::Discharging(prev) | PowerInfo::Charging(prev)),
                PowerInfo::Discharging(lvl),
            ) if lvl <= 15 && lvl != prev => events.push(Event::Toast(Toast::GamepadLowBattery {
                name: self.gilrs.gamepad(id).name().to_string(),
                level: lvl,
            })),
            (None, PowerInfo::Discharging(lvl)) if lvl <= 15 => {
                events.push(Event::Toast(Toast::GamepadLowBattery {
                    name: self.gilrs.gamepad(id).name().to_string(),
                    level: lvl,
                }))
            }
            _ => {}
        }
    }

    pub fn power_info(&self, id: GamepadId) -> PowerInfo {
        self.power_states
            .get(&id)
            .map(|(p, _)| *p)
            .unwrap_or(PowerInfo::Unknown)
    }

    pub fn is_down(&self, button: Button) -> bool {
        self.gilrs.gamepads().any(|(_, g)| g.is_pressed(button))
    }

    pub fn get_just_pressed(&self) -> Vec<Button> {
        self.just_pressed.clone()
    }

    pub fn take_just_pressed(&mut self, button: Button) -> bool {
        if let Some(idx) = self.just_pressed.iter().position(|&b| b == button) {
            self.just_pressed.remove(idx);
            true
        } else {
            false
        }
    }

    pub fn inactive_for(&self, duration: Duration) -> bool {
        self.last_input.elapsed() > duration
    }

    pub fn get(&self, id: GamepadId) -> gilrs::Gamepad<'_> {
        self.gilrs.gamepad(id)
    }

    pub fn gamepads(&self) -> &[GamepadId] {
        &self.used_gamepads
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
        Button::Mode => "\u{e009}",

        Button::LeftThumb => "L3",
        Button::RightThumb => "R3",

        Button::DPadUp => "⏶",
        Button::DPadDown => "⏷",
        Button::DPadLeft => "⏴",
        Button::DPadRight => "⏵",

        Button::Unknown => "?",
    }
}

pub fn button_prompt_raw(button: Button, label: &str) -> LayoutJob {
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

    job
}

pub fn button_prompt(button: Button, label: &str) -> egui::Label {
    egui::Label::new(button_prompt_raw(button, label))
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
