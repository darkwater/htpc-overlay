use egui::{Color32, RichText};

use super::MediaMenu;
use crate::{BLUE, utils::ResponseExt as _};

pub struct PlaylistMenu;

impl MediaMenu for PlaylistMenu {
    fn label(&self) -> &'static str {
        "Playlist"
    }

    fn enabled(&self, app: &crate::App) -> bool {
        app.mpv.playlist().len() > 1
    }

    fn width(&self) -> f32 {
        500.
    }

    fn draw(&self, ui: &mut egui::Ui, app: &mut crate::App) {
        let playlist = app.mpv.playlist();

        let mut goto = None;

        for (index, entry) in playlist.iter().enumerate() {
            let button = ui.button(RichText::new(entry.display_name()).color(if entry.current {
                BLUE
            } else {
                Color32::WHITE
            }));

            if entry.current {
                button.autofocus();
                button.bg_progress_indicator(
                    app.mpv.time_pos_fallback() / app.mpv.duration_fallback(),
                );
            }

            if button.activated() {
                goto = Some(index);
            }

            if button.has_focus() {
                ui.scroll_to_rect(button.rect, None);
            }
        }

        if let Some(entry) = goto {
            app.mpv.set_property("playlist-pos", entry as i64).ok();
        }
    }
}
