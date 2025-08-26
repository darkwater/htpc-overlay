#![feature(if_let_guard)]

use core::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use egui::{
    Align, Align2, Color32, Direction, FocusDirection, FontData, FontFamily, FontId, Frame, Id,
    Layout, Margin, ProgressBar, Response, RichText, Stroke, UiBuilder, Widget as _,
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

const BLUE: Color32 = Color32::from_rgb(137, 220, 235);

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

        let ev = self.gamepad.update();
        self.handle_event(ev);

        self.mpv.update().expect("mpv connection broke");

        let view = self.view;

        let actions = view.button_actions();

        let just_pressed = self.gamepad.get_just_pressed();
        for button in just_pressed {
            let command = actions.get(button);
            self.handle_command(command, ctx);
        }

        if let Some(limit) = self.view.hide_on_inactive()
            && self.gamepad.inactive_for(limit)
        {
            self.handle_command(Command::HideUi, ctx);
        }

        if self.view.show_prompts() {
            egui::TopBottomPanel::bottom("button prompts")
                .show_separator_line(false)
                .show(ctx, |ui| {
                    ui.scope(|ui| {
                        ui.visuals_mut().override_text_color = Some(BLUE);

                        let (left, right) = actions
                            .iter()
                            .filter(|(_button, cmd)| cmd.show_prompt())
                            .partition::<Vec<_>, _>(|(button, _action)| {
                                button_prompt_position(button) == PromptPosition::Left
                            });

                        let res = ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            for (button, cmd) in left {
                                ui.add(button_prompt(button, cmd.label(self)));
                                ui.add_space(8.);
                            }

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                for (button, cmd) in right {
                                    ui.add(button_prompt(button, cmd.label(self)));
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

        view.draw(ctx, self);

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
    fn handle_command(&mut self, cmd: Command, ctx: &egui::Context) {
        match cmd {
            Command::None => {}

            Command::ShowMiniSeek if self.view == View::MiniSeek => {
                self.change_view(View::Hidden);
            }
            Command::ShowMiniSeek => {
                self.change_view(View::MiniSeek);
            }
            Command::ShowUi => {
                self.change_view(View::SeekBar);
            }
            Command::HideUi => {
                self.change_view(View::Hidden);
            }
            Command::ShowMenu => {
                self.change_view(View::Menu { submenu: None });
            }

            Command::MoveFocus(dir) => {
                ctx.memory_mut(|m| m.move_focus(dir));
            }
            Command::Activate => {
                ctx.memory_mut(|m| m.data.insert_temp(Id::NULL, Activated(true)));
            }

            Command::TogglePause => {
                self.mpv.cycle_property("pause").unwrap();
            }

            Command::StartSeeking => {
                self.mpv.start_seek();
                self.change_view(View::Seeking);
            }
            Command::SeekForward => self.mpv.seek_forward().unwrap(),
            Command::SeekBackward => self.mpv.seek_backward().unwrap(),
            Command::SeekForwardStateless => {
                self.mpv.seek_stateless(10., false).unwrap();
            }
            Command::SeekBackwardStateless => {
                self.mpv.seek_stateless(-10., false).unwrap();
            }
            Command::DoneSeeking => {
                self.change_view(View::SeekBar);
                self.mpv.finish_seek().unwrap();
            }
            Command::CancelSeeking => {
                self.change_view(View::SeekBar);
                self.mpv.cancel_seek().ok();
            }
            Command::SeekFaster => {
                self.mpv.seek_faster();
            }
            Command::SeekSlower => {
                self.mpv.seek_slower();
            }
            Command::SeekExact => {
                self.mpv.toggle_seek_exact();
            }

            Command::CharactersDebug => {
                self.change_view(View::Characters);
            }

            Command::Quit => {
                EXIT.store(true, Ordering::Relaxed);
            }
        }
    }

    fn handle_event(&mut self, ev: Event) {
        match ev {
            Event::None => {}
            Event::Toast(toast) => {
                self.toasts.push(SpawnedToast::new(toast));
            }
            Event::LastGamepadDisconnected => {
                self.change_view(View::Hidden);
            }
        }
    }

    fn change_view(&mut self, new_view: View) {
        if self.view == new_view {
            return;
        }

        // let old_view = self.view;
        self.view = new_view;
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum View {
    #[default]
    Hidden,
    MiniSeek,
    SeekBar,
    Seeking,
    Menu {
        submenu: Option<MenuEntry>,
    },
    Characters,
}

impl View {
    fn show_prompts(self) -> bool {
        !matches!(self, View::Hidden | View::MiniSeek | View::Menu { .. })
    }

    fn hide_on_inactive(self) -> Option<Duration> {
        match self {
            View::MiniSeek => Some(Duration::from_secs(2)),
            View::SeekBar => Some(Duration::from_secs(5)),
            _ => None,
        }
    }

    fn draw(&self, ctx: &egui::Context, app: &mut App) {
        match self {
            View::Hidden => {}
            View::MiniSeek => {
                egui::TopBottomPanel::bottom("mini seek")
                    .show_separator_line(false)
                    .frame(Frame::NONE)
                    .exact_height(4.)
                    .show(ctx, |ui| {
                        ProgressBar::new(app.mpv.get_property::<f32>("percent-pos") / 100.)
                            .desired_height(4.)
                            .ui(ui);
                    });
            }
            View::SeekBar => {
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
            }
            View::Seeking => {
                egui::TopBottomPanel::bottom("seeking ui")
                    .show_separator_line(false)
                    .show(ctx, |ui| {
                        ui.add_space(4.);

                        let pos = app.mpv.get_property::<f32>("percent-pos") / 100.;

                        if let Some(speed) = app.mpv.seek_speed() {
                            let text_pos =
                                ui.cursor().left_top().lerp(ui.cursor().right_top(), pos);

                            ui.painter().text(
                                text_pos,
                                Align2::CENTER_TOP,
                                speed.label(),
                                FontId::proportional(10.),
                                if app.mpv.seek_exact() {
                                    BLUE
                                } else {
                                    Color32::WHITE
                                },
                            );
                        }

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

                        ProgressBar::new(pos).desired_height(4.).ui(ui);
                    });
            }
            View::Menu { submenu: None } => {
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
                        ui.with_layout(
                            Layout::bottom_up(Align::Min).with_cross_justify(true),
                            |ui| {
                                ui.spacing_mut().interact_size.y = 24.;
                                ui.style_mut().visuals.widgets.inactive.weak_bg_fill =
                                    Color32::TRANSPARENT;

                                let id_autofocus = Id::new("menu autofocus");
                                let autofocus = ui
                                    .memory(|m| m.data.get_temp::<MenuEntry>(id_autofocus))
                                    .unwrap_or(MenuEntry::iter().next().unwrap());

                                for entry in MenuEntry::iter() {
                                    let resp = ui.add_enabled(
                                        entry.enabled(app),
                                        egui::Button::new(entry.label()),
                                    );

                                    if entry == autofocus {
                                        resp.autofocus();
                                    }

                                    if resp.activated() {
                                        app.view = View::Menu {
                                            submenu: Some(entry),
                                        };

                                        ui.memory_mut(|m| m.data.insert_temp(id_autofocus, entry));
                                    }
                                }
                            },
                        );
                    });
            }
            View::Menu {
                submenu: Some(submenu),
            } => {
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
                        ui.with_layout(
                            Layout::bottom_up(Align::Min).with_cross_justify(true),
                            |ui| {
                                ui.spacing_mut().interact_size.y = 24.;
                                ui.style_mut().visuals.widgets.inactive.weak_bg_fill =
                                    Color32::TRANSPARENT;

                                submenu.draw(ui, app);
                            },
                        );
                    });
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
            }
        }
    }

    fn button_actions(&self) -> Actions {
        match self {
            View::Hidden | View::MiniSeek => Actions {
                a: Command::StartSeeking,
                b: Command::ShowUi,
                x: Command::TogglePause,
                y: Command::ShowUi,
                left: Command::SeekBackwardStateless,
                right: Command::SeekForwardStateless,
                l2: Command::CharactersDebug,
                select: Command::ShowMiniSeek,
                start: Command::ShowMenu,
                ..Actions::default()
            },
            View::SeekBar => Actions {
                a: Command::StartSeeking,
                b: Command::HideUi,
                x: Command::TogglePause,
                left: Command::SeekBackwardStateless,
                right: Command::SeekForwardStateless,
                start: Command::ShowMenu,
                ..Actions::default()
            },
            View::Seeking => Actions {
                a: Command::DoneSeeking,
                b: Command::CancelSeeking,
                y: Command::SeekExact,
                up: Command::SeekFaster,
                down: Command::SeekSlower,
                left: Command::SeekBackward,
                right: Command::SeekForward,
                ..Actions::default()
            },
            View::Menu { submenu } => Actions {
                a: Command::Activate,
                b: if submenu.is_some() {
                    Command::ShowMenu
                } else {
                    Command::HideUi
                },
                up: Command::MoveFocus(FocusDirection::Up),
                down: Command::MoveFocus(FocusDirection::Down),
                left: Command::MoveFocus(FocusDirection::Left),
                right: Command::MoveFocus(FocusDirection::Right),
                start: Command::HideUi,
                ..Actions::default()
            },
            View::Characters => Actions {
                b: Command::HideUi,
                ..Actions::default()
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
enum SeekSpeed {
    Second,
    #[default]
    FiveSeconds,
    ThirtySeconds,
    Minute,
    TenMinutes,
}

impl SeekSpeed {
    fn duration(self) -> Duration {
        match self {
            SeekSpeed::Second => Duration::from_secs(1),
            SeekSpeed::FiveSeconds => Duration::from_secs(5),
            SeekSpeed::ThirtySeconds => Duration::from_secs(30),
            SeekSpeed::Minute => Duration::from_secs(60),
            SeekSpeed::TenMinutes => Duration::from_secs(600),
        }
    }

    fn label(self) -> &'static str {
        match self {
            SeekSpeed::Second => "1s",
            SeekSpeed::FiveSeconds => "5s",
            SeekSpeed::ThirtySeconds => "30s",
            SeekSpeed::Minute => "1m",
            SeekSpeed::TenMinutes => "10m",
        }
    }

    fn longer(self) -> Option<Self> {
        match self {
            SeekSpeed::Second => Some(SeekSpeed::FiveSeconds),
            SeekSpeed::FiveSeconds => Some(SeekSpeed::ThirtySeconds),
            SeekSpeed::ThirtySeconds => Some(SeekSpeed::Minute),
            SeekSpeed::Minute => Some(SeekSpeed::TenMinutes),
            SeekSpeed::TenMinutes => None,
        }
    }

    fn shorter(self) -> Option<Self> {
        match self {
            SeekSpeed::Second => None,
            SeekSpeed::FiveSeconds => Some(SeekSpeed::Second),
            SeekSpeed::ThirtySeconds => Some(SeekSpeed::FiveSeconds),
            SeekSpeed::Minute => Some(SeekSpeed::ThirtySeconds),
            SeekSpeed::TenMinutes => Some(SeekSpeed::Minute),
        }
    }
}

#[derive(Default)]
struct Actions {
    a: Command,
    b: Command,
    x: Command,
    y: Command,
    l1: Command,
    l2: Command,
    r1: Command,
    r2: Command,
    up: Command,
    down: Command,
    left: Command,
    right: Command,
    select: Command,
    start: Command,
    home: Command,
}

impl Actions {
    fn iter(&self) -> impl Iterator<Item = (Button, Command)> {
        [
            (Button::East, self.a),
            (Button::South, self.b),
            (Button::North, self.x),
            (Button::West, self.y),
            (Button::LeftTrigger, self.l1),
            (Button::LeftTrigger2, self.l2),
            (Button::RightTrigger, self.r1),
            (Button::RightTrigger2, self.r2),
            (Button::DPadUp, self.up),
            (Button::DPadDown, self.down),
            (Button::DPadLeft, self.left),
            (Button::DPadRight, self.right),
            (Button::Select, self.select),
            (Button::Start, self.start),
            (Button::Mode, self.home),
        ]
        .into_iter()
    }

    fn get(&self, button: Button) -> Command {
        self.iter()
            .find(|(b, _action)| *b == button)
            .map(|(_b, action)| action)
            .unwrap_or(Command::None)
    }
}

#[derive(PartialEq)]
enum PromptPosition {
    Left,
    Right,
}

#[derive(Clone, Copy, Default, Debug)]
enum Command {
    #[default]
    None,

    ShowMiniSeek,
    ShowUi,
    HideUi,
    ShowMenu,

    MoveFocus(FocusDirection),
    Activate,

    TogglePause,

    StartSeeking,
    SeekBackward,
    SeekForward,
    SeekBackwardStateless,
    SeekForwardStateless,
    DoneSeeking,
    CancelSeeking,
    SeekFaster,
    SeekSlower,
    SeekExact,

    CharactersDebug,

    Quit,
}

#[derive(Debug)]
enum Event {
    None,
    Toast(Toast),
    LastGamepadDisconnected,
}

impl Command {
    fn label(self, app: &App) -> &'static str {
        match self {
            Command::None => "(none)",

            Command::ShowMiniSeek => "Show position",
            Command::ShowUi => "Show UI",
            Command::HideUi => "Hide UI",
            Command::ShowMenu => "Menu",

            Command::MoveFocus(_) => "Move Focus",
            Command::Activate => "Activate",

            Command::TogglePause if app.mpv.get_property_cached("pause") == Some(true) => "Play",
            Command::TogglePause => "Pause",

            Command::StartSeeking => "Seek",
            Command::SeekBackward => "Seek Backward",
            Command::SeekForward => "Seek Forward",
            Command::SeekBackwardStateless => "Seek Backward",
            Command::SeekForwardStateless => "Seek Forward",
            Command::DoneSeeking => "Done",
            Command::CancelSeeking => "Cancel",
            Command::SeekFaster => "Faster",
            Command::SeekSlower => "Slower",
            Command::SeekExact if app.mpv.seek_exact() => "Keyframes",
            Command::SeekExact => "Exact",

            Command::CharactersDebug => "Characters",

            Command::Quit => "Quit",
        }
    }

    fn show_prompt(self) -> bool {
        !matches!(
            self,
            Command::None
                | Command::ShowUi
                | Command::SeekBackward
                | Command::SeekForward
                | Command::SeekBackwardStateless
                | Command::SeekForwardStateless
                | Command::MoveFocus(_)
                | Command::Activate
        )
    }
}

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

fn button_prompt_position(button: &Button) -> PromptPosition {
    match button {
        Button::Select
        | Button::Start
        | Button::Mode
        | Button::LeftThumb
        | Button::RightThumb
        | Button::LeftTrigger
        | Button::LeftTrigger2
        | Button::RightTrigger
        | Button::RightTrigger2 => PromptPosition::Left,

        Button::DPadUp
        | Button::DPadDown
        | Button::DPadLeft
        | Button::DPadRight
        | Button::South
        | Button::East
        | Button::North
        | Button::West
        | Button::C
        | Button::Z => PromptPosition::Right,

        Button::Unknown => unreachable!(),
    }
}

trait ResponseExt: Sized {
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
struct Activated(bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuEntry {
    Volume,
    Subtitles,
    AudioTrack,
    PlaybackSpeed,
    Playlist,
}

impl MenuEntry {
    fn label(&self) -> &'static str {
        match self {
            MenuEntry::Volume => "Volume",
            MenuEntry::Subtitles => "Subtitles",
            MenuEntry::AudioTrack => "Audio Track",
            MenuEntry::PlaybackSpeed => "Playback Speed",
            MenuEntry::Playlist => "Playlist",
        }
    }

    fn iter() -> impl Iterator<Item = MenuEntry> {
        [
            MenuEntry::Volume,
            MenuEntry::Subtitles,
            MenuEntry::AudioTrack,
            MenuEntry::PlaybackSpeed,
            MenuEntry::Playlist,
        ]
        .into_iter()
    }

    fn enabled(self, app: &App) -> bool {
        match self {
            MenuEntry::Subtitles => !app.mpv.sub_tracks().is_empty(),
            MenuEntry::AudioTrack => app.mpv.audio_tracks().len() > 1,
            _ => true,
        }
    }

    fn width(&self) -> f32 {
        match self {
            MenuEntry::Playlist => 500.,
            _ => 200.,
        }
    }

    fn draw(&self, ui: &mut egui::Ui, app: &mut App) {
        match self {
            MenuEntry::Subtitles => {
                let mut visible = app.mpv.get_property("sub-visibility");
                if ui.checkbox(&mut visible, "Enabled").activated() {
                    app.mpv.set_property("sub-visibility", !visible).ok();
                }

                let mut set_track = None;

                for track in app.mpv.sub_tracks() {
                    let label = match (&track.title, &track.lang, &track.codec) {
                        (Some(title), Some(lang), _) => format!("{title} ({lang})"),
                        (Some(title), None, _) => title.to_string(),
                        (None, Some(lang), _) => lang.to_string(),
                        (None, None, Some(codec)) => format!("({codec})"),
                        (None, None, None) => format!("#{}", track.id),
                    };

                    let res = ui.button(RichText::new(label).color(if track.selected {
                        BLUE
                    } else {
                        Color32::WHITE
                    }));

                    if res.activated() {
                        set_track = Some(track.id);
                    }
                }

                if let Some(id) = set_track {
                    app.mpv.set_property("sid", id).ok();
                }
            }
            MenuEntry::AudioTrack => {
                let mut set_track = None;

                for track in app.mpv.audio_tracks() {
                    let label = match (&track.title, &track.lang, &track.codec) {
                        (Some(title), Some(lang), _) => format!("{title} ({lang})"),
                        (Some(title), None, _) => title.to_string(),
                        (None, Some(lang), _) => lang.to_string(),
                        (None, None, Some(codec)) => format!("({codec})"),
                        (None, None, None) => format!("#{}", track.id),
                    };

                    let res = ui.button(RichText::new(label).color(if track.selected {
                        BLUE
                    } else {
                        Color32::WHITE
                    }));

                    if res.activated() {
                        set_track = Some(track.id);
                    }
                }

                if let Some(id) = set_track {
                    app.mpv.set_property("aid", id).ok();
                }
            }
            MenuEntry::Playlist => {
                let playlist = app.mpv.playlist();

                for entry in playlist {
                    let button =
                        ui.button(RichText::new(entry.display_name()).color(if entry.current {
                            BLUE
                        } else {
                            Color32::WHITE
                        }));

                    if entry.current {
                        button.autofocus();
                    }
                }
            }
            MenuEntry::Volume => {}
            MenuEntry::PlaybackSpeed => {}
        }
    }
}
