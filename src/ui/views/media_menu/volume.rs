use gilrs::Button;

use super::MediaMenu;
use crate::{App, utils::ResponseExt};

pub struct VolumeMenu;

impl VolumeMenu {
    fn draw_impl<V: VolumeImpl>(&self, ui: &mut egui::Ui, app: &mut App, mut v: V) {
        let volume = v.current_volume(app);

        let button = ui.button(v.label());

        button.ralign_overlay(ui, |ui| {
            ui.add_space(8.);
            ui.label(format!("{volume:.0}%"));
        });

        button.autofocus();

        button.bg_progress_indicator(volume / 100.0);

        if button.has_focus() && app.gamepad.take_just_pressed(Button::DPadLeft) {
            v.change_volume(app, -5.0);
        }

        if button.has_focus() && app.gamepad.take_just_pressed(Button::DPadRight) {
            v.change_volume(app, 5.0);
        }
    }
}

impl MediaMenu for VolumeMenu {
    fn label(&self) -> &'static str {
        "Volume"
    }

    fn enabled(&self, _app: &App) -> bool {
        true
    }

    fn draw(&self, ui: &mut egui::Ui, app: &mut App) {
        self.draw_impl(ui, app, Mpv);
    }

    fn catch_left_right(&self) -> bool {
        true
    }
}

trait VolumeImpl {
    fn label(&self) -> &'static str;
    fn current_volume(&mut self, app: &mut App) -> f32;
    fn change_volume(&mut self, app: &mut App, delta: f32);
}

struct Mpv;
impl VolumeImpl for Mpv {
    fn label(&self) -> &'static str {
        "mpv"
    }

    fn current_volume(&mut self, app: &mut App) -> f32 {
        app.mpv.get_property::<f32>("volume")
    }

    fn change_volume(&mut self, app: &mut App, delta: f32) {
        app.mpv.change_volume(delta).ok();
    }
}

// struct Tv;
// impl VolumeImpl for Tv {
//     fn label(&self) -> &'static str {
//         "TV"
//     }

//     fn current_volume(&mut self, app: &mut App) -> f32 {
//         app.mpv.get_property::<f32>("volume")
//     }

//     fn change_volume(&mut self, app: &mut App, delta: f32) {
//         app.mpv.change_volume(delta).ok();
//     }
// }
