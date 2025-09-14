use core::time::Duration;

use egui::{Color32, FontFamily, Frame, ProgressBar, RichText, Widget as _};
use gilrs::Button;

use crate::{
    command::{Actions, Command},
    ui::{HiddenView, View},
    utils::available_characters,
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

        if app.gamepad.is_down(Button::LeftTrigger2) {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let chars = available_characters(ui, FontFamily::Proportional);
                    for c in chars {
                        ui.label(RichText::new(c.to_string()).size(50.));
                        ctx.debug_painter().text(
                            ui.cursor().left_top(),
                            egui::Align2::RIGHT_TOP,
                            format!("{:04x}", c as u32),
                            egui::FontId::new(10., FontFamily::Proportional),
                            Color32::WHITE,
                        );
                    }
                });
            });
        }
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
