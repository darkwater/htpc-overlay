use egui::{Align, Align2, Color32, FontId, Layout, ProgressBar, RichText, Widget as _};

use crate::{
    BLUE,
    command::{Actions, Command},
    ui::View,
    utils::seconds_to_mmss,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct SeekingView;

impl View for SeekingView {
    fn draw(&self, ctx: &egui::Context, app: &mut crate::App) {
        egui::TopBottomPanel::bottom("seeking ui")
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.add_space(4.);

                let pos = app.mpv.get_property::<f32>("percent-pos") / 100.;

                if let Some(speed) = app.mpv.seek_speed() {
                    let text_pos = ui.cursor().left_top().lerp(ui.cursor().right_top(), pos);

                    ui.painter().text(
                        text_pos,
                        Align2::CENTER_TOP,
                        speed.label(),
                        FontId::proportional(10.),
                        if app.mpv.seek_exact() {
                            BLUE
                        } else {
                            Color32::WHITE
                        },
                    );
                }

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(seconds_to_mmss(app.mpv.get_property::<f32>("time-pos")))
                            .size(10.),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(seconds_to_mmss(app.mpv.get_property::<f32>("duration")))
                                .size(10.),
                        );
                    });
                });

                ProgressBar::new(pos).desired_height(4.).ui(ui);
            });
    }

    fn button_actions(&self) -> Actions {
        Actions {
            a: Command::DoneSeeking,
            b: Command::CancelSeeking,
            y: Command::SeekExact,
            up: Command::SeekFaster,
            down: Command::SeekSlower,
            left: Command::SeekBackward,
            right: Command::SeekForward,
            ..Actions::default()
        }
    }
}
