use egui::{Color32, RichText};

use super::MediaMenu;
use crate::{BLUE, mpv::TrackType, utils::ResponseExt as _};

pub struct TrackMenu(pub TrackType);

impl MediaMenu for TrackMenu {
    fn label(&self) -> &'static str {
        match self.0 {
            TrackType::Video => "Video Tracks",
            TrackType::Audio => "Audio Tracks",
            TrackType::Sub => "Subtitles",
        }
    }

    fn enabled(&self, app: &crate::App) -> bool {
        match (self.0, app.mpv.tracks_of_type(self.0).len()) {
            (TrackType::Video, 2..) => false,
            (TrackType::Audio, 2..) => true,
            (TrackType::Sub, 1..) => true,
            _ => false,
        }
    }

    fn draw(&self, ui: &mut egui::Ui, app: &mut crate::App) {
        let mut set_track = None;

        let disabled = !app.mpv.tracks_of_type(self.0).iter().any(|t| t.selected);

        let hidden = self.0 == TrackType::Sub && !app.mpv.get_property::<bool>("sub-visibility");

        let res = ui.button(RichText::new("None").color(if disabled || hidden {
            BLUE
        } else {
            Color32::WHITE
        }));

        if disabled {
            res.autofocus();
        }
        if res.activated() {
            set_track = Some(0);
        }

        for track in app.mpv.tracks_of_type(self.0) {
            let label = match (&track.title, &track.lang, &track.codec) {
                (Some(title), Some(lang), _) => format!("{title} ({lang})"),
                (Some(title), None, _) => title.to_string(),
                (None, Some(lang), _) => lang.to_string(),
                (None, None, Some(codec)) => format!("({codec})"),
                (None, None, None) => format!("#{}", track.id),
            };

            let res = ui.button(RichText::new(label).color(if !hidden && track.selected {
                BLUE
            } else {
                Color32::WHITE
            }));

            if track.selected {
                res.autofocus();
            }

            if res.activated() {
                set_track = Some(track.id);
            }
        }

        if let Some(id) = set_track {
            if hidden {
                app.mpv.set_property("sub-visibility", true).ok();
            }

            let prop = match self.0 {
                TrackType::Video => "vid",
                TrackType::Audio => "aid",
                TrackType::Sub => "sid",
            };
            app.mpv.set_property(prop, id).ok();
        }
    }
}
