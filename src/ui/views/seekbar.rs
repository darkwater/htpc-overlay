use core::time::Duration;

use egui::{ProgressBar, RichText, Widget as _};

use crate::{
    command::{Actions, Command},
    ui::View,
    utils::horizontal_left_right,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct SeekBarView;

impl View for SeekBarView {
    fn draw(&self, ctx: &egui::Context, app: &mut crate::App) {
        egui::TopBottomPanel::bottom("seek ui")
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.add_space(8.);

                ui.label(app.mpv.get_property::<String>("media-title"));

                ui.add_space(4.);

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
                        );

                        if let Some(segment) = app
                            .mpv
                            .sponsorblock_segments()
                            .iter()
                            .find(|s| s.contains(app.mpv.time_pos_fallback()))
                        {
                            ui.label(
                                RichText::new(segment.category.label())
                                    .size(10.)
                                    .color(segment.category.color()),
                            );
                        }
                    },
                    |ui| {
                        if let Some(duration) = app.mpv.duration() {
                            ui.label(RichText::new(duration.mmss()).size(10.));
                        }
                    },
                );

                let rect = ProgressBar::new(app.mpv.get_property::<f32>("percent-pos") / 100.)
                    .desired_height(4.)
                    .ui(ui)
                    .rect;

                let duration = app.mpv.duration_fallback();

                for segment in app.mpv.sponsorblock_segments() {
                    let start = rect.left() + rect.width() * (segment.start() / duration);
                    let end = rect.left() + rect.width() * (segment.end() / duration);

                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(start, rect.top()),
                            egui::pos2(end, rect.bottom()),
                        ),
                        0.,
                        segment.category.color(),
                    );
                }
            });
    }

    fn button_actions(&self) -> Actions {
        Actions {
            a: Command::StartSeeking,
            b: Command::HideUi,
            x: Command::TogglePause,
            left: Command::SeekBackwardStateless,
            right: Command::SeekForwardStateless,
            start: Command::ShowMenu,
            ..Actions::default()
        }
    }

    fn hide_on_inactive(&self) -> Option<std::time::Duration> {
        Some(Duration::from_secs(5))
    }
}
