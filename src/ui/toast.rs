use core::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use egui::{Align, Align2, Area, Color32, Frame, Id, Layout, RichText, vec2};

#[derive(Debug)]
pub struct SpawnedToast {
    id: Id,
    timestamp: Instant,
    toast: Toast,
}

impl SpawnedToast {
    pub fn new(toast: Toast) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        Self {
            id: Id::new("toast").with(COUNTER.fetch_add(1, Ordering::Relaxed)),
            timestamp: Instant::now(),
            toast,
        }
    }
}

pub fn draw(toasts: &mut Vec<SpawnedToast>, ctx: &egui::Context) {
    let margin = 6.;
    let mut cursor = margin;

    toasts.retain_mut(|toast| {
        let slide_in = 1. - (toast.timestamp.elapsed().as_secs_f32() * 2.).clamp(0., 1.);
        let slide_out = 1. - (toast.timestamp.elapsed().as_secs_f32() - 4.).clamp(0., 1.);

        // easing
        let slide_in = slide_in * slide_in * slide_in;
        let slide_out = slide_out * slide_out * slide_out;

        let height = Area::new(toast.id)
            .anchor(
                Align2::RIGHT_TOP,
                vec2(-margin + slide_in * 200., cursor - (1. - slide_out) * 100.),
            )
            .constrain(false)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Max), |ui| {
                    Frame::new()
                        .fill(Color32::from_black_alpha(192))
                        .corner_radius(8.)
                        .inner_margin(6.)
                        .show(ui, |ui| {
                            toast.toast.ui(ui);
                        });
                });
            })
            .response
            .rect
            .height();

        cursor += (margin + height) * slide_out;

        toast.timestamp.elapsed().as_secs() < 5
    });
}

#[derive(Debug)]
pub enum Toast {
    GamepadConnected { name: String },
    GamepadDisconnected { name: String },
    LastGamepadDisconnected,
    DlnaDeviceDiscovered { name: String },
}

impl Toast {
    pub fn ui(&self, ui: &mut egui::Ui) {
        match self {
            Toast::GamepadConnected { name } => {
                ui.label("Gamepad connected");
                ui.label(RichText::new(name).size(10.));
            }
            Toast::GamepadDisconnected { name } => {
                ui.label("Gamepad disconnected");
                ui.label(RichText::new(name).size(10.));
            }
            Toast::LastGamepadDisconnected => {
                ui.label("Last gamepad disconnected");
            }
            Toast::DlnaDeviceDiscovered { name } => {
                ui.label("DLNA device discovered");
                ui.label(RichText::new(name).size(10.));
            }
        }
    }
}
