use egui::{Id, Response};

pub trait ResponseExt: Sized {
    fn autofocus(&self);
    fn activated(&self) -> bool;
}

impl ResponseExt for Response {
    fn autofocus(&self) {
        if self.ctx.memory(|m| m.focused().is_none()) {
            self.request_focus();
        }
    }

    fn activated(&self) -> bool {
        self.has_focus()
            && self
                .ctx
                .memory(|m| m.data.get_temp::<Activated>(Id::NULL).unwrap_or_default())
                .0
    }
}

#[derive(Clone, Copy, Default)]
pub struct Activated(pub bool);

pub fn seconds_to_mmss(seconds: f32) -> String {
    let minutes = (seconds / 60.0).floor() as u32;
    let seconds = (seconds % 60.0).floor() as u32;
    format!("{}:{:02}", minutes, seconds)
}

#[expect(dead_code)]
pub fn available_characters(ui: &egui::Ui, family: egui::FontFamily) -> Vec<char> {
    ui.fonts(|f| {
        f.lock()
            .fonts
            .font(&egui::FontId::new(10.0, family)) // size is arbitrary for getting the characters
            .characters()
            .iter()
            .filter(|(chr, _fonts)| {
                !chr.is_whitespace()
                    && !chr.is_ascii_control()
                    && _fonts.iter().any(|f| f == "kenney_input_nintendo_switch")
            })
            .map(|(chr, _fonts)| *chr)
            .collect()
    })

    // egui::CentralPanel::default().show(ctx, |ui| {
    //     ui.horizontal_wrapped(|ui| {
    //         let chars = available_characters(ui, FontFamily::Proportional);
    //         for c in chars {
    //             ui.label(RichText::new(c.to_string()).size(50.));
    //             ctx.debug_painter().text(
    //                 ui.cursor().left_top(),
    //                 egui::Align2::RIGHT_TOP,
    //                 format!("{:04x}", c as u32),
    //                 egui::FontId::new(10., FontFamily::Proportional),
    //                 Color32::WHITE,
    //             );
    //         }
    //     });
    // });
}
