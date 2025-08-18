use core::time::Duration;

use egui::{
    Color32, FontData, FontFamily, ProgressBar, Stroke, Widget as _,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
    style::Selection,
};
use egui_wlr_layer::{
    Anchor, InputRegions, KeyboardInteractivity, Layer, LayerAppOpts, LayerSurface,
};
use gilrs::Button;

use self::{
    gamepad::{Gamepad, button_prompt},
    mpv::Mpv,
    toast::{SpawnedToast, Toast},
};

mod gamepad;
mod mpv;
mod toast;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut context = egui_wlr_layer::Context::new();

    context.new_layer_app(
        Box::new(App::default()),
        LayerAppOpts {
            layer: Layer::Overlay,
            namespace: Some("htpc-overlay"),
            output: None,
            input_regions: InputRegions::None,
        },
    );

    loop {
        context.blocking_dispatch().unwrap();
    }
}

#[derive(Default)]
struct App {
    initialized: bool,
    gamepad: Gamepad,
    view: View,
    mpv: Mpv,
    toasts: Vec<SpawnedToast>,
}

impl egui_wlr_layer::App for App {
    fn update(&mut self, ctx: &egui::Context) {
        if !self.initialized {
            self.initialized = true;

            ctx.set_visuals(egui::Visuals {
                dark_mode: true,
                override_text_color: Some(Color32::WHITE),
                selection: Selection {
                    bg_fill: Color32::WHITE,
                    stroke: Stroke::new(1.0, Color32::RED),
                },
                extreme_bg_color: Color32::TRANSPARENT,
                panel_fill: Color32::from_black_alpha(32),
                ..Default::default()
            });

            ctx.add_font(FontInsert::new(
                "kenney_input_nintendo_switch",
                FontData::from_static(include_bytes!("../assets/kenney_input_nintendo_switch.ttf")),
                vec![InsertFontFamily {
                    family: FontFamily::Proportional,
                    priority: FontPriority::Highest,
                }],
            ));

            ctx.set_zoom_factor(1.5);

            ctx.request_discard("init");
            return;
        }

        let cmd = self.gamepad.update();
        self.handle_command(cmd);

        self.mpv.update().expect("mpv connection broke");

        let view = self.view;
        let cmd = view.draw(ctx, self);
        self.handle_command(cmd);

        toast::draw(&mut self.toasts, ctx);

        ctx.request_repaint();
    }

    fn on_init(&mut self, layer: &LayerSurface) {
        layer.set_anchor(Anchor::all());
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    }
}

impl App {
    fn handle_command(&mut self, cmd: Command) {
        if !matches!(cmd, Command::None) {
            eprintln!("Handling command: {:?}", cmd);
        }

        match cmd {
            Command::None => {}
            Command::ChangeView(view) => {
                let old_view = self.view;
                self.view = view;
                View::on_transition(old_view, self.view, self);
            }
            Command::Toast(toast) => {
                self.toasts.push(SpawnedToast::new(toast));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
enum View {
    #[default]
    Hidden,
    SeekBar,
}

impl View {
    fn draw(&self, ctx: &egui::Context, app: &mut App) -> Command {
        // let painter = ctx.debug_painter();
        // let rect = ctx.screen_rect();

        // painter.line_segment(
        //     [rect.left_top(), rect.right_bottom()],
        //     Stroke::new(1.0, Color32::WHITE),
        // );
        // painter.line_segment(
        //     [rect.left_bottom(), rect.right_top()],
        //     Stroke::new(1.0, Color32::WHITE),
        // );

        match self {
            View::Hidden => {
                if app.gamepad.just_pressed_any() {
                    Command::ChangeView(View::SeekBar)
                } else {
                    Command::None
                }
            }
            View::SeekBar => {
                egui::TopBottomPanel::bottom("seek ui")
                    .show_separator_line(false)
                    .show(ctx, |ui| {
                        ui.add_space(8.);
                        ProgressBar::new(app.mpv.get_property::<f32>("percent-pos") / 100.)
                            .desired_height(4.)
                            .ui(ui);
                    });

                if app.gamepad.inactive_for(Duration::from_secs(2)) {
                    Command::ChangeView(View::Hidden)
                } else {
                    Command::None
                }
            }
        }
    }

    fn on_transition(from: View, to: View, app: &mut App) {
        match (from, to) {
            (View::Hidden, View::SeekBar) => {
                app.mpv.set_property("sub-pos", 90).ok();
            }
            (View::SeekBar, View::Hidden) => {
                app.mpv.set_property("sub-pos", 100).ok();
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
enum Command {
    None,
    ChangeView(View),
    Toast(Toast),
}
