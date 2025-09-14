use core::sync::atomic::Ordering;

use egui::{FocusDirection, Id};
use gilrs::Button;

use crate::{
    App, EXIT,
    mpv::time::Time,
    ui::{
        toast::{SpawnedToast, Toast},
        views::{
            hidden::HiddenView, home_menu::HomeMenuView, media_menu::MediaMenuView,
            miniseek::MiniSeekView, seekbar::SeekBarView, seeking::SeekingView,
        },
    },
    utils::Activated,
};

#[derive(Clone, Copy, Default, Debug)]
pub enum Command {
    #[default]
    None,

    ShowMiniSeek,
    ShowUi,
    HideUi,
    ShowMediaMenu,
    ShowHomeMenu,

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

    VolumeUp,
    VolumeDown,

    Quit,
}

#[derive(Debug)]
pub enum Event {
    Toast(Toast),
    LastGamepadDisconnected,
}

#[derive(Default)]
pub struct Actions {
    pub a: Command,
    pub b: Command,
    pub x: Command,
    pub y: Command,
    pub l1: Command,
    pub l2: Command,
    pub r1: Command,
    pub r2: Command,
    pub up: Command,
    pub down: Command,
    pub left: Command,
    pub right: Command,
    pub select: Command,
    pub start: Command,
    pub home: Command,
}

impl Command {
    pub fn label(self, app: &App) -> &'static str {
        match self {
            Command::None => "(none)",

            Command::ShowMiniSeek => "Show position",
            Command::ShowUi => "Show UI",
            Command::HideUi => "Hide UI",
            Command::ShowMediaMenu => "Media Menu",
            Command::ShowHomeMenu => "Home Menu",

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

            Command::VolumeUp => "Volume Up",
            Command::VolumeDown => "Volume Down",

            Command::Quit => "Quit",
        }
    }

    pub fn show_prompt(self) -> bool {
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

    pub fn execute(self, app: &mut App, ctx: &egui::Context) {
        match self {
            Command::None => {}

            Command::ShowMiniSeek => {
                app.change_view(MiniSeekView);
            }
            Command::ShowUi => {
                app.change_view(SeekBarView);
            }
            Command::HideUi => {
                app.change_view(HiddenView);
            }
            Command::ShowMediaMenu => {
                app.change_view(MediaMenuView::main());
            }
            Command::ShowHomeMenu => {
                app.change_view(HomeMenuView::main());
            }

            Command::MoveFocus(dir) => {
                ctx.memory_mut(|m| m.move_focus(dir));
            }
            Command::Activate => {
                ctx.memory_mut(|m| m.data.insert_temp(Id::NULL, Activated(true)));
            }

            Command::TogglePause => {
                app.mpv.cycle_property("pause").unwrap();
            }

            Command::StartSeeking => {
                app.mpv.start_seek();
                app.change_view(SeekingView);
            }
            Command::SeekForward => app.mpv.seek_forward().unwrap(),
            Command::SeekBackward => app.mpv.seek_backward().unwrap(),
            Command::SeekForwardStateless => {
                app.mpv.seek_stateless(Time::seconds(5), false).unwrap();
            }
            Command::SeekBackwardStateless => {
                app.mpv.seek_stateless(Time::seconds(-5), false).unwrap();
            }
            Command::DoneSeeking => {
                app.change_view(SeekBarView);
                app.mpv.finish_seek().unwrap();
            }
            Command::CancelSeeking => {
                app.change_view(SeekBarView);
                app.mpv.cancel_seek().ok();
            }
            Command::SeekFaster => {
                app.mpv.seek_faster();
            }
            Command::SeekSlower => {
                app.mpv.seek_slower();
            }
            Command::SeekExact => {
                app.mpv.toggle_seek_exact();
            }

            Command::VolumeUp => {
                if let Some(device) = app.dlna.devices().get_mut(0) {
                    device.set_volume((device.volume() as f32 + 5.) as u8);
                }
            }
            Command::VolumeDown => {
                if let Some(device) = app.dlna.devices().get_mut(0) {
                    device.set_volume((device.volume() as f32 - 5.) as u8);
                }
            }

            Command::Quit => {
                EXIT.store(true, Ordering::Relaxed);
            }
        }
    }
}

impl Event {
    pub fn execute(self, app: &mut App) {
        match self {
            Event::Toast(toast) => {
                app.toasts.push(SpawnedToast::new(toast));
            }
            Event::LastGamepadDisconnected => {
                if !app.view.is::<HiddenView>() {
                    app.toasts
                        .push(SpawnedToast::new(Toast::LastGamepadDisconnected));

                    app.change_view(HiddenView);
                }
            }
        }
    }
}

impl Actions {
    pub fn iter(&self) -> impl Iterator<Item = (Button, Command)> {
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

    pub fn get(&self, button: Button) -> Command {
        self.iter()
            .find(|(b, _action)| *b == button)
            .map(|(_b, action)| action)
            .unwrap_or(Command::None)
    }
}
