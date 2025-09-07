use core::time::Duration;

use egui::{Frame, ProgressBar, Widget as _};

use crate::{
    command::{Actions, Command},
    ui::{HiddenView, View},
};

pub struct MiniSeekView;

impl View for MiniSeekView {
    fn draw(&self, ctx: &egui::Context, app: &mut crate::App) {
        egui::TopBottomPanel::bottom("mini seek")
            .show_separator_line(false)
            .frame(Frame::NONE)
            .exact_height(4.)
            .show(ctx, |ui| {
                ProgressBar::new(app.mpv.get_property::<f32>("percent-pos") / 100.)
                    .desired_height(4.)
                    .ui(ui);
            });
    }

    fn button_actions(&self) -> Actions {
        Actions {
            select: Command::HideUi,
            ..HiddenView.button_actions()
        }
    }

    fn show_prompts(&self) -> bool {
        false
    }

    fn hide_on_inactive(&self) -> Option<std::time::Duration> {
        Some(Duration::from_secs(2))
    }
}
