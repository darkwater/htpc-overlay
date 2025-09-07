use core::time::Duration;

use egui::{Align, Layout, ProgressBar, RichText, Widget as _};

use crate::{
    command::{Actions, Command},
    ui::View,
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

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(
                            app.mpv
                                .time_pos()
                                .map(|t| t.mmss())
                                .unwrap_or_else(|| "--:--".to_string()),
                        )
                        .size(10.),
                    );
                    if let Some(duration) = app.mpv.duration() {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(duration.mmss()).size(10.));
                        });
                    }
                });

                ui.add_space(-4.);

                ProgressBar::new(app.mpv.get_property::<f32>("percent-pos") / 100.)
                    .desired_height(4.)
                    .ui(ui);
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
