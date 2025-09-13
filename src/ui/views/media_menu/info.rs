use egui::{Margin, RichText};

use super::MediaMenu;

pub struct InfoMenu;

impl MediaMenu for InfoMenu {
    fn label(&self) -> &'static str {
        "Info"
    }

    fn enabled(&self, app: &crate::App) -> bool {
        app.mpv.metadata().has_anything_interesting()
    }

    fn width(&self) -> f32 {
        400.
    }

    fn frame(&self, ctx: &egui::Context) -> egui::Frame {
        egui::Frame::new()
            .inner_margin(Margin::symmetric(8, 8))
            .fill(ctx.style().visuals.panel_fill)
    }

    fn draw(&self, ui: &mut egui::Ui, app: &mut crate::App) {
        if let Some(ref title) = app.mpv.metadata().title {
            ui.label(RichText::new(title).heading());
        }

        ui.horizontal(|ui| {
            if let Some(ref artist) = app.mpv.metadata().artist {
                ui.label(artist);
            }

            if let Some(ref date) = app.mpv.metadata().date {
                ui.label(date.format("%Y-%m-%d").to_string());
            }
        });

        if let Some(ref description) = app.mpv.metadata().description {
            ui.add_space(16.);
            ui.label(description);
        }
    }
}
