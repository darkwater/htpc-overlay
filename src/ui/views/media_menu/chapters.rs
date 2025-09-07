use egui::{Color32, RichText};

use super::MediaMenu;
use crate::{BLUE, utils::ResponseExt as _};

pub struct ChaptersMenu;

impl MediaMenu for ChaptersMenu {
    fn label(&self) -> &'static str {
        "Chapters"
    }

    fn enabled(&self, app: &crate::App) -> bool {
        !app.mpv.chapters().is_empty()
    }

    fn draw(&self, ui: &mut egui::Ui, app: &mut crate::App) {
        let chapters = app.mpv.chapters();

        if chapters.is_empty() {
            return;
        }

        let mut goto = None;

        for chapter in chapters {
            let button = ui.button(RichText::new(chapter.title.unwrap_or("<no title>")).color(
                if chapter.current {
                    BLUE
                } else {
                    Color32::WHITE
                },
            ));

            if chapter.current {
                button.autofocus();
                button.bg_progress_indicator(
                    (app.mpv.time_pos_fallback() - chapter.start) / chapter.duration,
                );
            }

            if button.activated() {
                goto = Some(chapter);
            }

            if button.has_focus() {
                ui.scroll_to_rect(button.rect, None);
            }
        }

        if let Some(entry) = goto {
            app.mpv.set_property("time-pos", entry.start).ok();
        }
    }
}
