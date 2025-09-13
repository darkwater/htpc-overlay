use crate::{
    App,
    command::{Actions, Command},
    ui::View,
};

pub struct HiddenView;

impl View for HiddenView {
    fn draw(&self, _ctx: &egui::Context, _app: &mut App) {}

    fn button_actions(&self) -> Actions {
        Actions {
            a: Command::StartSeeking,
            b: Command::ShowUi,
            x: Command::TogglePause,
            y: Command::ShowUi,
            left: Command::SeekBackwardStateless,
            right: Command::SeekForwardStateless,
            up: Command::VolumeUp,
            down: Command::VolumeDown,
            select: Command::ShowMiniSeek,
            start: Command::ShowMenu,
            ..Actions::default()
        }
    }

    fn show_prompts(&self) -> bool {
        false
    }
}
