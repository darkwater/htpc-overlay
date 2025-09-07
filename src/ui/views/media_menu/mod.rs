use core::fmt::Debug;

use egui::{Align, Color32, FocusDirection, Frame, Id, Layout, Margin, ScrollArea};

use crate::{
    command::{Actions, Command},
    mpv::TrackType,
    ui::View,
    utils::ResponseExt as _,
};

mod chapters;
mod playlist;
mod tracks;

fn entries() -> [Box<dyn MediaMenu>; 5] {
    [
        Box::new(playlist::PlaylistMenu),
        Box::new(chapters::ChaptersMenu),
        Box::new(tracks::TrackMenu(TrackType::Video)),
        Box::new(tracks::TrackMenu(TrackType::Audio)),
        Box::new(tracks::TrackMenu(TrackType::Sub)),
    ]
}

#[derive(Debug, Default)]
pub struct MediaMenuView {
    pub submenu: Option<Box<dyn MediaMenu>>,
}

impl MediaMenuView {
    pub fn main() -> Self {
        Self { submenu: None }
    }

    pub fn sub(menu: Box<dyn MediaMenu>) -> Self {
        Self { submenu: Some(menu) }
    }
}

impl View for MediaMenuView {
    fn draw(&self, ctx: &egui::Context, app: &mut crate::App) {
        if let Some(submenu) = &self.submenu {
            egui::SidePanel::left("submenu")
                .show_separator_line(false)
                .resizable(false)
                .frame({
                    Frame::new()
                        .inner_margin(Margin::symmetric(2, 2))
                        .fill(ctx.style().visuals.panel_fill)
                })
                .exact_width(submenu.width())
                .show(ctx, |ui| {
                    ScrollArea::vertical()
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                        )
                        .show(ui, |ui| {
                            ui.with_layout(
                                Layout::top_down(Align::Min).with_cross_justify(true),
                                |ui| {
                                    ui.spacing_mut().interact_size.y = 24.;
                                    ui.style_mut().visuals.widgets.inactive.weak_bg_fill =
                                        Color32::TRANSPARENT;

                                    submenu.draw(ui, app);
                                },
                            );
                        });
                });
        } else {
            egui::SidePanel::left("menu")
                .show_separator_line(false)
                .resizable(false)
                .frame({
                    Frame::new()
                        .inner_margin(Margin::symmetric(2, 2))
                        .fill(ctx.style().visuals.panel_fill)
                })
                .exact_width(200.)
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
                                app.change_view(MediaMenuView::sub(entry));
                            }
                        }
                    });
                });
        }
    }

    fn button_actions(&self) -> Actions {
        Actions {
            a: Command::Activate,
            b: if self.submenu.is_some() {
                Command::ShowMenu
            } else {
                Command::HideUi
            },
            x: Command::TogglePause,
            up: Command::MoveFocus(FocusDirection::Up),
            down: Command::MoveFocus(FocusDirection::Down),
            // left: Command::MoveFocus(FocusDirection::Left),
            // right: Command::MoveFocus(FocusDirection::Right),
            left: Command::SeekBackwardStateless,
            right: Command::SeekForwardStateless,
            start: Command::HideUi,
            ..Actions::default()
        }
    }
}

pub trait MediaMenu: 'static {
    fn label(&self) -> &'static str;
    fn enabled(&self, app: &crate::App) -> bool;
    fn width(&self) -> f32 {
        300.
    }

    fn draw(&self, ui: &mut egui::Ui, app: &mut crate::App);
}

impl Debug for dyn MediaMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.label()).finish_non_exhaustive()
    }
}
