use core::sync::atomic::{AtomicBool, Ordering};

use egui::{
    Color32, FontData, FontFamily, Id, Stroke,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
    style::Selection,
};
use egui_wlr_layer::{
    Anchor, InputRegions, KeyboardInteractivity, Layer, LayerAppOpts, LayerSurface,
};

use self::{
    command::Command,
    gamepad::Gamepad,
    mpv::Mpv,
    ui::{View, toast::SpawnedToast},
    utils::Activated,
};

mod command;
mod gamepad;
mod mpv;
mod ui;
mod utils;

const BLUE: Color32 = Color32::from_rgb(137, 220, 235);

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut context = egui_wlr_layer::Context::new();

    let handle = context.new_layer_app(Box::new(App::default()), LayerAppOpts {
        layer: Layer::Overlay,
        namespace: Some("htpc-overlay"),
        output: None,
        input_regions: InputRegions::None,
    });

    loop {
        context.blocking_dispatch().unwrap();

        if EXIT.swap(false, Ordering::Relaxed) {
            handle.exit();
        }

        if EXITED.load(Ordering::Relaxed) {
            break Ok(());
        }
    }
}

static EXIT: AtomicBool = AtomicBool::new(false);
static EXITED: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
pub struct App {
    initialized: bool,
    gamepad: Gamepad,
    view: Box<dyn ui::View>,
    mpv: Mpv,
    toasts: Vec<SpawnedToast>,
}

impl App {
    fn take_view(&mut self) -> Box<dyn ui::View> {
        std::mem::replace(&mut self.view, Box::new(ui::ViewTaken))
    }

    fn restore_view(&mut self, view: Box<dyn ui::View>) {
        if self.view.is::<ui::ViewTaken>() {
            self.view = view;
        }
    }

    fn change_view(&mut self, new_view: impl View) {
        self.view = Box::new(new_view);
    }
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
                extreme_bg_color: Color32::from_black_alpha(128),
                panel_fill: Color32::from_black_alpha(192),
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

            ctx.options_mut(|o| o.max_passes = 3.try_into().unwrap());

            ctx.request_discard("init");
            return;
        }

        ctx.memory_mut(|m| m.data.insert_temp(Id::NULL, Activated(false)));

        self.gamepad.update().execute(self);

        self.mpv.update().expect("mpv connection broke");

        let view = self.take_view();

        let actions = view.button_actions();

        let just_pressed = self.gamepad.get_just_pressed();
        for button in just_pressed {
            actions.get(button).execute(self, ctx);
        }

        if let Some(limit) = view.hide_on_inactive()
            && self.gamepad.inactive_for(limit)
        {
            Command::HideUi.execute(self, ctx);
        }

        if view.show_prompts() {
            ui::button_prompts(ctx, self, &actions);
        }

        view.draw(ctx, self);

        let sub_pos = self.mpv.get_property::<f32>("sub-pos");
        let new_sub_pos =
            (ctx.available_rect().bottom() / ctx.screen_rect().bottom() * 100.).round();
        if sub_pos != new_sub_pos {
            eprintln!("Changing sub-pos from {} to {}", sub_pos, new_sub_pos);
            self.mpv.set_property("sub-pos", new_sub_pos).ok();
        }

        ui::toast::draw(&mut self.toasts, ctx);

        self.restore_view(view);

        ctx.request_repaint();
    }

    fn on_init(&mut self, layer: &LayerSurface) {
        layer.set_anchor(Anchor::all());
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    }

    fn on_exit(&mut self) {
        self.mpv.set_property("sub-pos", 100).ok();
        EXITED.store(true, Ordering::Relaxed);
    }
}
