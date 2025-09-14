use core::{any::Any, time::Duration};

use gilrs::Button;

use self::views::hidden::HiddenView;
use crate::{App, BLUE, command::Actions, gamepad::button_prompt, utils::horizontal_left_right};

pub mod toast;
pub mod views {
    pub mod hidden;
    pub mod home_menu;
    pub mod media_menu;
    pub mod miniseek;
    pub mod seekbar;
    pub mod seeking;
}

pub trait View: Any {
    fn draw(&self, ctx: &egui::Context, app: &mut App);
    fn button_actions(&self) -> Actions;

    fn show_prompts(&self) -> bool {
        true
    }

    fn hide_on_inactive(&self) -> Option<Duration> {
        None
    }
}

impl dyn View {
    pub fn is<T: View>(&self) -> bool {
        Any::type_id(self) == std::any::TypeId::of::<T>()
    }
}

impl Default for Box<dyn View> {
    fn default() -> Self {
        Box::new(HiddenView)
    }
}

pub struct ViewTaken;
#[rustfmt::skip]
impl View for ViewTaken {
    fn draw(&self, _ctx: &egui::Context, _app: &mut App) { unreachable!() }
    fn button_actions(&self) -> Actions { unreachable!() }
    fn show_prompts(&self) -> bool { unreachable!() }
    fn hide_on_inactive(&self) -> Option<Duration> { unreachable!() }
}

pub fn button_prompts(ctx: &egui::Context, app: &App, actions: &Actions) {
    egui::TopBottomPanel::bottom("button prompts")
        .show_separator_line(false)
        .show(ctx, |ui| {
            ui.visuals_mut().override_text_color = Some(BLUE);

            let (left, right) = actions
                .iter()
                .filter(|(_button, cmd)| cmd.show_prompt())
                .partition::<Vec<_>, _>(|(button, _action)| {
                    button_prompt_position(button) == PromptPosition::Left
                });

            horizontal_left_right(
                ui,
                |ui| {
                    for (button, cmd) in left {
                        ui.add(button_prompt(button, cmd.label(app)));
                        ui.add_space(8.);
                    }
                },
                |ui| {
                    for (button, cmd) in right.into_iter().rev() {
                        ui.add_space(8.);
                        ui.add(button_prompt(button, cmd.label(app)));
                    }
                },
            );
        });
}

fn button_prompt_position(button: &Button) -> PromptPosition {
    match button {
        Button::Select
        | Button::Start
        | Button::Mode
        | Button::LeftThumb
        | Button::RightThumb
        | Button::LeftTrigger
        | Button::LeftTrigger2
        | Button::RightTrigger
        | Button::RightTrigger2 => PromptPosition::Left,

        Button::DPadUp
        | Button::DPadDown
        | Button::DPadLeft
        | Button::DPadRight
        | Button::South
        | Button::East
        | Button::North
        | Button::West
        | Button::C
        | Button::Z => PromptPosition::Right,

        Button::Unknown => unreachable!(),
    }
}

#[derive(PartialEq)]
enum PromptPosition {
    Left,
    Right,
}
