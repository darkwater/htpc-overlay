use core::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use egui::{
    Align, Color32, Direction, FontData, FontFamily, Layout, ProgressBar, RichText, Stroke,
    UiBuilder, Widget as _,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
    style::Selection,
};
use egui_wlr_layer::{
    Anchor, InputRegions, KeyboardInteractivity, Layer, LayerAppOpts, LayerSurface,
};
use gilrs::Button;

use self::{
    gamepad::{BUTTON_A, BUTTON_B, BUTTON_X, Gamepad, button_prompt},
    mpv::Mpv,
    toast::{SpawnedToast, Toast},
};

mod gamepad;
mod mpv;
mod toast;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut context = egui_wlr_layer::Context::new();

    let handle = context.new_layer_app(
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
                extreme_bg_color: Color32::from_black_alpha(64),
                panel_fill: Color32::from_black_alpha(128),
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

        let cmd = self.gamepad.update();
        self.handle_command(cmd);

        self.mpv.update().expect("mpv connection broke");

        let view = self.view;

        let actions = view.button_actions();

        let just_pressed = self.gamepad.get_just_pressed();
        for button in just_pressed {
            if let Some(action) = actions.iter().find(|a| a.button == button) {
                self.handle_command((action.command)());
            }
        }

        if actions
            .iter()
            .any(|action| action.position != PromptPosition::None)
        {
            egui::TopBottomPanel::bottom("button prompts")
                .show_separator_line(false)
                .show(ctx, |ui| {
                    ui.scope(|ui| {
                        ui.visuals_mut().override_text_color =
                            Some(Color32::from_rgb(137, 220, 235));

                        let (left, right) = actions
                            .into_iter()
                            .filter(|a| a.position != PromptPosition::None)
                            .partition::<Vec<_>, _>(|a| a.position == PromptPosition::Left);

                        let res = ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            for action in left {
                                ui.add(button_prompt(action.button, &action.label));
                                ui.add_space(8.);
                            }

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                for action in right {
                                    ui.add(button_prompt(action.button, &action.label));
                                    ui.add_space(8.);
                                }
                            });
                        });

                        ui.scope_builder(
                            UiBuilder::new()
                                .max_rect(res.response.rect)
                                .layout(Layout::centered_and_justified(Direction::LeftToRight)),
                            |ui| {
                                let time = chrono::Local::now();
                                let time = time.format("%H:%M").to_string();
                                ui.label(RichText::new(time));
                                ui.add_space(16.);
                            },
                        );
                    });
                });
        }

        let cmd = view.draw(ctx, self);
        self.handle_command(cmd);

        let sub_pos = self.mpv.get_property::<f32>("sub-pos");
        let new_sub_pos =
            (ctx.available_rect().bottom() / ctx.screen_rect().bottom() * 100.).round();
        if sub_pos != new_sub_pos {
            eprintln!("Changing sub-pos from {} to {}", sub_pos, new_sub_pos);
            self.mpv.set_property("sub-pos", new_sub_pos).ok();
        }

        toast::draw(&mut self.toasts, ctx);

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
            Command::MpvCommand(MpvCommand::TogglePause) => {
                self.mpv.cycle_property("pause").unwrap();
            }
            Command::DoneSeeking => {}   // TODO
            Command::CancelSeeking => {} // TODO
            Command::SeekFaster => {
                if let View::SeekBar {
                    seek_speed: Some(seek_speed),
                } = &mut self.view
                {
                    if let Some(new_speed) = seek_speed.longer() {
                        *seek_speed = new_speed;
                    }
                }
            }
            Command::SeekSlower => {
                if let View::SeekBar {
                    seek_speed: Some(seek_speed),
                } = &mut self.view
                {
                    if let Some(new_speed) = seek_speed.shorter() {
                        *seek_speed = new_speed;
                    }
                }
            }
            Command::Quit => {
                EXIT.store(true, Ordering::Relaxed);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
enum View {
    #[default]
    Hidden,
    SeekBar {
        seek_speed: Option<SeekSpeed>,
    },
    Characters,
}

#[derive(Debug, Clone, Copy, Default)]
enum SeekSpeed {
    Second,
    #[default]
    Keyframe,
    TenSeconds,
    Minute,
    TenMinutes,
}

impl SeekSpeed {
    fn duration(self) -> Option<Duration> {
        match self {
            SeekSpeed::Second => Some(Duration::from_secs(1)),
            SeekSpeed::Keyframe => None,
            SeekSpeed::TenSeconds => Some(Duration::from_secs(10)),
            SeekSpeed::Minute => Some(Duration::from_secs(60)),
            SeekSpeed::TenMinutes => Some(Duration::from_secs(600)),
        }
    }

    fn label(self) -> &'static str {
        match self {
            SeekSpeed::Second => "1s",
            SeekSpeed::Keyframe => "Kf",
            SeekSpeed::TenSeconds => "10s",
            SeekSpeed::Minute => "1m",
            SeekSpeed::TenMinutes => "10m",
        }
    }

    fn longer(self) -> Option<Self> {
        match self {
            SeekSpeed::Second => Some(SeekSpeed::Keyframe),
            SeekSpeed::Keyframe => Some(SeekSpeed::TenSeconds),
            SeekSpeed::TenSeconds => Some(SeekSpeed::Minute),
            SeekSpeed::Minute => Some(SeekSpeed::TenMinutes),
            SeekSpeed::TenMinutes => None,
        }
    }

    fn shorter(self) -> Option<Self> {
        match self {
            SeekSpeed::Second => None,
            SeekSpeed::Keyframe => Some(SeekSpeed::Second),
            SeekSpeed::TenSeconds => Some(SeekSpeed::Keyframe),
            SeekSpeed::Minute => Some(SeekSpeed::TenSeconds),
            SeekSpeed::TenMinutes => Some(SeekSpeed::Minute),
        }
    }
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
            View::Hidden => Command::None,
            View::SeekBar { seek_speed } => {
                egui::TopBottomPanel::bottom("seek ui")
                    .show_separator_line(false)
                    .show(ctx, |ui| {
                        ui.add_space(8.);

                        ui.label(app.mpv.get_property::<String>("media-title"));

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(seconds_to_mmss(
                                    app.mpv.get_property::<f32>("time-pos"),
                                ))
                                .size(10.),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(seconds_to_mmss(
                                        app.mpv.get_property::<f32>("duration"),
                                    ))
                                    .size(10.),
                                );
                            });
                        });

                        ui.add_space(-4.);

                        ProgressBar::new(app.mpv.get_property::<f32>("percent-pos") / 100.)
                            .desired_height(4.)
                            .ui(ui);
                    });

                if app.gamepad.inactive_for(Duration::from_secs(5)) {
                    Command::ChangeView(View::Hidden)
                } else {
                    Command::None
                }
            }
            View::Characters => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let chars = available_characters(ui, FontFamily::Proportional);
                        for c in chars {
                            ui.label(RichText::new(c.to_string()).size(50.));
                            ctx.debug_painter().text(
                                ui.cursor().left_top(),
                                egui::Align2::RIGHT_TOP,
                                format!("{:04x}", c as u32),
                                egui::FontId::new(10., FontFamily::Proportional),
                                Color32::WHITE,
                            );
                        }
                    });
                });

                Command::None
            }
        }
    }

    fn on_transition(_from: View, _to: View, _app: &mut App) {
        // match (from, to) {
        //     (View::Hidden, View::SeekBar) => {
        //         app.mpv.set_property("sub-pos", 80).ok();
        //     }
        //     (View::SeekBar, View::Hidden) => {
        //         app.mpv.set_property("sub-pos", 100).ok();
        //     }
        //     _ => {}
        // }
    }

    fn button_actions(&self) -> Actions {
        match self {
            View::Hidden => Actions {
                a: Some(ButtonAction {
                    label: "Show UI".to_string(),
                    position: PromptPosition::None,
                    command: || Command::ChangeView(View::SeekBar { seek_speed: None }),
                }),
                b: Some(ButtonAction {
                    label: "Show UI".to_string(),
                    position: PromptPosition::None,
                    command: || Command::ChangeView(View::SeekBar { seek_speed: None }),
                }),
                x: Some(ButtonAction {
                    label: "Pause".to_string(),
                    position: PromptPosition::None,
                    command: || Command::MpvCommand(MpvCommand::TogglePause),
                }),
                l2: Some(ButtonAction {
                    label: "Characters".to_string(),
                    position: PromptPosition::None,
                    command: || Command::ChangeView(View::Characters),
                }),
                ..Default::default()
            },
            View::SeekBar {
                seek_speed: Some(_),
            } => Actions {
                a: Some(ButtonAction {
                    label: "Done".to_string(),
                    position: PromptPosition::Right,
                    command: || Command::DoneSeeking,
                }),
                b: Some(ButtonAction {
                    label: "Return".to_string(),
                    position: PromptPosition::Right,
                    command: || Command::CancelSeeking,
                }),
                up: Some(ButtonAction {
                    label: "Faster".to_string(),
                    position: PromptPosition::Right,
                    command: || Command::SeekFaster,
                }),
                down: Some(ButtonAction {
                    label: "Slower".to_string(),
                    position: PromptPosition::Right,
                    command: || Command::SeekSlower,
                }),
                ..Default::default()
            },
            View::SeekBar { seek_speed: None } => Actions {
                b: Some(ButtonAction {
                    label: "Hide".to_string(),
                    position: PromptPosition::Right,
                    command: || Command::ChangeView(View::Hidden),
                }),
                x: Some(ButtonAction {
                    label: "Pause".to_string(),
                    position: PromptPosition::Right,
                    command: || Command::MpvCommand(MpvCommand::TogglePause),
                }),
                start: Some(ButtonAction {
                    label: "Quit".to_string(),
                    position: PromptPosition::Left,
                    command: || Command::Quit,
                }),
                left: Some(ButtonAction {
                    label: "Seek back".to_string(),
                    position: PromptPosition::None,
                    command: || {
                        Command::ChangeView(View::SeekBar {
                            seek_speed: Some(SeekSpeed::default()),
                        })
                    },
                }),
                right: Some(ButtonAction {
                    label: "Seek forward".to_string(),
                    position: PromptPosition::None,
                    command: || {
                        Command::ChangeView(View::SeekBar {
                            seek_speed: Some(SeekSpeed::default()),
                        })
                    },
                }),
                ..Default::default()
            },
            View::Characters => Actions {
                b: Some(ButtonAction {
                    label: "Back".to_string(),
                    position: PromptPosition::Right,
                    command: || Command::ChangeView(View::SeekBar { seek_speed: None }),
                }),
                ..Default::default()
            },
        }
    }
}

#[derive(Default)]
struct Actions {
    a: Option<ButtonAction>,
    b: Option<ButtonAction>,
    x: Option<ButtonAction>,
    y: Option<ButtonAction>,
    l1: Option<ButtonAction>,
    l2: Option<ButtonAction>,
    r1: Option<ButtonAction>,
    r2: Option<ButtonAction>,
    up: Option<ButtonAction>,
    down: Option<ButtonAction>,
    left: Option<ButtonAction>,
    right: Option<ButtonAction>,
    select: Option<ButtonAction>,
    start: Option<ButtonAction>,
    home: Option<ButtonAction>,
}

impl Actions {
    fn get(&self, button: Button) -> Option<&ButtonAction> {
        match button {
            Button::East => self.a.as_ref(),
            Button::South => self.b.as_ref(),
            Button::North => self.x.as_ref(),
            Button::West => self.y.as_ref(),
            Button::LeftTrigger => self.l1.as_ref(),
            Button::LeftTrigger2 => self.l2.as_ref(),
            Button::RightTrigger => self.r1.as_ref(),
            Button::RightTrigger2 => self.r2.as_ref(),
            Button::DPadUp => self.up.as_ref(),
            Button::DPadDown => self.down.as_ref(),
            Button::DPadLeft => self.left.as_ref(),
            Button::DPadRight => self.right.as_ref(),
            Button::Select => self.select.as_ref(),
            Button::Start => self.start.as_ref(),
            Button::Mode => self.home.as_ref(),
            _ => None,
        }
    }

    fn iter(&self) -> impl Iterator<Item = (Button, &ButtonAction)> {
        [
            (Button::East, self.a.as_ref()),
            (Button::South, self.b.as_ref()),
            (Button::North, self.x.as_ref()),
            (Button::West, self.y.as_ref()),
            (Button::LeftTrigger, self.l1.as_ref()),
            (Button::LeftTrigger2, self.l2.as_ref()),
            (Button::RightTrigger, self.r1.as_ref()),
            (Button::RightTrigger2, self.r2.as_ref()),
            (Button::DPadUp, self.up.as_ref()),
            (Button::DPadDown, self.down.as_ref()),
            (Button::DPadLeft, self.left.as_ref()),
            (Button::DPadRight, self.right.as_ref()),
            (Button::Select, self.select.as_ref()),
            (Button::Start, self.start.as_ref()),
            (Button::Mode, self.home.as_ref()),
        ]
        .into_iter()
        .filter_map(|(button, action)| action.map(|a| (button, a)))
    }
}

struct ButtonAction {
    label: String,
    position: PromptPosition,
    command: fn() -> Command,
}

#[derive(PartialEq)]
enum PromptPosition {
    None,
    Left,
    Right,
}

#[derive(Debug)]
enum Command {
    None,
    ChangeView(View),
    Toast(Toast),
    MpvCommand(MpvCommand),
    DoneSeeking,
    CancelSeeking,
    SeekFaster,
    SeekSlower,
    Quit,
}

#[derive(Debug)]
enum MpvCommand {
    TogglePause,
}

#[expect(dead_code)]
fn available_characters(ui: &egui::Ui, family: egui::FontFamily) -> Vec<char> {
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
}

fn seconds_to_mmss(seconds: f32) -> String {
    let minutes = (seconds / 60.0).floor() as u32;
    let seconds = (seconds % 60.0).floor() as u32;
    format!("{}:{:02}", minutes, seconds)
}
