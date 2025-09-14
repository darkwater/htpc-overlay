use core::fmt::Debug;

use egui::{Align, Color32, FocusDirection, Frame, Id, Layout, Margin, ScrollArea};
use gilrs::PowerInfo;

use crate::{
    command::{Actions, Command},
    ui::View,
    utils::ResponseExt as _,
};

mod library;

fn entries() -> [Box<dyn HomeMenu>; 1] {
    [Box::new(library::LibraryMenu)]
}

#[derive(Debug, Default)]
pub struct HomeMenuView {
    pub submenu: Option<Box<dyn HomeMenu>>,
}

impl HomeMenuView {
    pub fn main() -> Self {
        Self { submenu: None }
    }

    pub fn sub(menu: Box<dyn HomeMenu>) -> Self {
        Self { submenu: Some(menu) }
    }
}

impl View for HomeMenuView {
    fn draw(&self, ctx: &egui::Context, app: &mut crate::App) {
        if let Some(submenu) = &self.submenu {
            submenu.panel(ctx, app);
        } else {
            egui::SidePanel::right("home menu")
                .show_separator_line(false)
                .resizable(false)
                .frame({
                    Frame::new()
                        .inner_margin(Margin::symmetric(2, 2))
                        .fill(ctx.style().visuals.panel_fill)
                })
                .exact_width(150.)
                .show(ctx, |ui| {
                    ui.with_layout(Layout::bottom_up(Align::Min).with_cross_justify(true), |ui| {
                        ui.spacing_mut().interact_size.y = 24.;
                        ui.style_mut().visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;

                        let id_autofocus = Id::new("menu autofocus");
                        let autofocus = ui
                            .memory(|m| m.data.get_temp::<&'static str>(id_autofocus))
                            .unwrap_or(entries()[0].label());

                        for entry in entries() {
                            let resp = ui
                                .add_enabled(entry.enabled(app), egui::Button::new(entry.label()));

                            if entry.label() == autofocus {
                                resp.autofocus();
                            }

                            if resp.activated() {
                                ui.memory_mut(|m| m.data.insert_temp(id_autofocus, entry.label()));
                                app.change_view(HomeMenuView::sub(entry));
                            }
                        }

                        ui.with_layout(
                            Layout::top_down(Align::Min).with_cross_justify(true),
                            |ui| {
                                ui.add_space(8.);

                                for &id in app.gamepad.gamepads() {
                                    let gamepad = app.gamepad.get(id);

                                    match app.gamepad.power_info(id) {
                                        PowerInfo::Charging(level)
                                        | PowerInfo::Discharging(level) => {
                                            ui.label(gamepad.name()).ralign_overlay(ui, |ui| {
                                                ui.label(format!("{}%", level));
                                            });
                                        }
                                        _ => {}
                                    }
                                }
                            },
                        );
                    });
                });
        }
    }

    fn button_actions(&self) -> Actions {
        let left_right = if self.submenu.as_ref().is_some_and(|m| m.catch_left_right()) {
            Actions::default()
        } else {
            Actions {
                left: Command::SeekBackwardStateless,
                right: Command::SeekForwardStateless,
                ..Actions::default()
            }
        };

        Actions {
            a: Command::Activate,
            b: if self.submenu.is_some() {
                Command::ShowHomeMenu
            } else {
                Command::HideUi
            },
            x: Command::TogglePause,
            up: Command::MoveFocus(FocusDirection::Up),
            down: Command::MoveFocus(FocusDirection::Down),
            // left: Command::MoveFocus(FocusDirection::Left),
            // right: Command::MoveFocus(FocusDirection::Right),
            home: Command::HideUi,
            ..left_right
        }
    }
}

pub trait HomeMenu: 'static {
    fn label(&self) -> &'static str;
    fn enabled(&self, app: &crate::App) -> bool;
    fn width(&self) -> f32 {
        300.
    }
    fn frame(&self, ctx: &egui::Context) -> Frame {
        Frame::new()
            .inner_margin(Margin::symmetric(2, 2))
            .fill(ctx.style().visuals.panel_fill)
    }

    fn panel(&self, ctx: &egui::Context, app: &mut crate::App) {
        egui::SidePanel::right("home submenu")
            .show_separator_line(false)
            .resizable(false)
            .frame(self.frame(ctx))
            .exact_width(self.width())
            .show(ctx, |ui| {
                self.inner(ui, app);
            });
    }

    fn inner(&self, ui: &mut egui::Ui, app: &mut crate::App) {
        ScrollArea::vertical()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                ui.with_layout(Layout::top_down(Align::Min).with_cross_justify(true), |ui| {
                    ui.spacing_mut().interact_size.y = 24.;
                    ui.style_mut().visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;

                    self.draw(ui, app);
                });
            });
    }

    fn draw(&self, ui: &mut egui::Ui, app: &mut crate::App);

    fn catch_left_right(&self) -> bool {
        false
    }
}

impl Debug for dyn HomeMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.label()).finish_non_exhaustive()
    }
}
