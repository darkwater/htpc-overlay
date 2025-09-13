use egui::{Align2, Color32, FontId, ProgressBar, RichText, Widget as _};

use crate::{
    BLUE,
    command::{Actions, Command},
    ui::View,
    utils::horizontal_left_right,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct SeekingView;

impl View for SeekingView {
    fn draw(&self, ctx: &egui::Context, app: &mut crate::App) {
        egui::TopBottomPanel::bottom("seeking ui")
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.add_space(8.);

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

                horizontal_left_right(
                    ui,
                    |ui| {
                        ui.label(
                            RichText::new(
                                app.mpv
                                    .time_pos()
                                    .map(|t| t.mmss())
                                    .unwrap_or_else(|| "--:--".to_string()),
                            )
                            .size(10.),
                        )
                    },
                    |ui| {
                        if let Some(duration) = app.mpv.duration() {
                            ui.label(RichText::new(duration.mmss()).size(10.));
                        }
                    },
                );

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
