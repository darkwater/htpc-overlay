use gilrs::Button;

use super::MediaMenu;
use crate::{App, utils::ResponseExt};

pub struct VolumeMenu;

impl MediaMenu for VolumeMenu {
    fn label(&self) -> &'static str {
        "Volume"
    }

    fn enabled(&self, _app: &App) -> bool {
        true
    }

    fn draw(&self, ui: &mut egui::Ui, app: &mut App) {
        self.draw_impl(ui, app, Mpv);
        for idx in 0..app.dlna.devices().len() {
            self.draw_impl(ui, app, Dlna(idx));
        }
    }

    fn catch_left_right(&self) -> bool {
        true
    }
}

impl VolumeMenu {
    fn draw_impl<V: VolumeImpl>(&self, ui: &mut egui::Ui, app: &mut App, mut v: V) {
        let volume = v.current_volume(app);

        let button = ui.button(v.label(app));

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

trait VolumeImpl {
    fn label(&self, app: &mut App) -> String;
    fn current_volume(&mut self, app: &mut App) -> f32;
    fn change_volume(&mut self, app: &mut App, delta: f32);
}

struct Mpv;
impl VolumeImpl for Mpv {
    fn label(&self, _app: &mut App) -> String {
        "mpv".to_string()
    }

    fn current_volume(&mut self, app: &mut App) -> f32 {
        app.mpv.get_property::<f32>("volume")
    }

    fn change_volume(&mut self, app: &mut App, delta: f32) {
        app.mpv.change_volume(delta).ok();
    }
}

struct Dlna(usize);
impl VolumeImpl for Dlna {
    fn label(&self, app: &mut App) -> String {
        app.dlna
            .devices()
            .get(self.0)
            .map_or("(unknown)", |d| d.friendly_name())
            .to_string()
    }

    fn current_volume(&mut self, app: &mut App) -> f32 {
        app.dlna
            .devices()
            .get(self.0)
            .map_or(0., |d| d.volume() as f32)
    }

    fn change_volume(&mut self, app: &mut App, delta: f32) {
        if let Some(device) = app.dlna.devices().get_mut(self.0) {
            device.set_volume((device.volume() as f32 + delta) as u8);
        }
    }
}
