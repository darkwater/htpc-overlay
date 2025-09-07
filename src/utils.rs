use egui::{Id, Response};

pub trait ResponseExt: Sized {
    fn autofocus(&self);
    fn activated(&self) -> bool;
    fn bg_progress_indicator(&self, progress: f32);
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

    fn bg_progress_indicator(&self, progress: f32) {
        let mut rect = self.rect;
        rect.set_width(rect.width() * progress.clamp(0.0, 1.0));

        self.ctx.layer_painter(self.layer_id).rect_filled(
            rect,
            2.,
            egui::Color32::from_white_alpha(8),
        );
    }
}

#[derive(Clone, Copy, Default)]
pub struct Activated(pub bool);

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
