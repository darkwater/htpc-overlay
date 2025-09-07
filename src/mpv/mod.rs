use core::time::Duration;
use std::{
    io::{self, BufRead, BufReader, ErrorKind, Write as _},
    os::unix::net::UnixStream,
    time::Instant,
};

use egui::ahash::{HashMap, HashMapExt as _};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use self::{
    command::{Command, Event, EventOrResponse, Response},
    seek_speed::SeekSpeed,
    time::Time,
};

mod command;
pub mod seek_speed;
pub mod time;

pub struct Mpv {
    socket: BufReader<UnixStream>,
    observed_properties: HashMap<String, Value>,
    next_observe_id: i32,
    event_buffer: Vec<Event>,
    seek_state: Option<SeekState>,
    tracks: Vec<Track>,
    chapters: Vec<ChapterRaw>,
    playlist: Vec<PlaylistEntry>,
}

struct SeekState {
    speed: SeekSpeed,
    exact: bool,
    ended: Option<Instant>,

    // from before seek
    pos: f32,
    paused: bool,
}

impl Mpv {
    pub fn new() -> Self {
        let stream = UnixStream::connect("/run/user/1000/mpv.sock")
            .expect("Failed to connect to mpv socket");
        stream
            .set_nonblocking(true)
            .expect("Failed to set non-blocking mode");

        let mut this = Self {
            socket: BufReader::new(stream),
            observed_properties: HashMap::new(),
            next_observe_id: 0,
            event_buffer: Vec::new(),
            seek_state: None,
            tracks: Vec::new(),
            chapters: Vec::new(),
            playlist: Vec::new(),
        };

        this.observe_property("time-pos").unwrap();
        this.observe_property("duration").unwrap();
        this.observe_property("playlist").unwrap();
        this.observe_property("track-list").unwrap();
        this.observe_property("chapter-list").unwrap();

        this
    }

    fn blocking<T>(&mut self, f: impl FnOnce(&mut Self) -> io::Result<T>) -> io::Result<T> {
        self.socket.get_mut().set_nonblocking(false)?;
        let result = f(self);
        self.socket.get_mut().set_nonblocking(true)?;
        result
    }

    fn read_line<T: DeserializeOwned>(&mut self) -> io::Result<Option<T>> {
        let mut buf = String::new();
        match self.socket.read_line(&mut buf) {
            Ok(0) => Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "EOF reached while reading from mpv socket",
            )),
            Ok(_) => {
                // eprintln!("< {}", buf.trim());

                let event: T = serde_json::from_str(&buf).map_err(|e| {
                    io::Error::other(format!("Failed to deserialize mpv event: {}", e))
                })?;

                Ok(Some(event))
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn read_response<T: DeserializeOwned>(&mut self) -> io::Result<Response<T>> {
        self.blocking(|this| {
            loop {
                let v = this.read_line::<EventOrResponse<T>>()?.unwrap();

                match v {
                    EventOrResponse::Event(event) => {
                        this.event_buffer.push(event);
                    }
                    EventOrResponse::Response(response) => {
                        return Ok(response);
                    }
                }
            }
        })
    }

    fn read_events(&mut self) -> io::Result<()> {
        while let Some(event) = self.read_line::<Event>()? {
            self.event_buffer.push(event);
        }
        Ok(())
    }

    pub fn command<T: DeserializeOwned>(&mut self, cmd: Command) -> io::Result<Option<T>> {
        let cmd_str = serde_json::to_string(&cmd).expect("Failed to serialize command");
        // eprintln!("> {}", cmd_str);
        writeln!(self.socket.get_mut(), "{}", cmd_str)?;
        self.socket.get_mut().flush()?;

        let response = self.blocking(|mpv| mpv.read_response::<T>())?;

        if response.error == "success" {
            Ok(response.data)
        } else {
            Err(io::Error::other(format!("mpv command error: {}", response.error)))
        }
    }

    pub fn update(&mut self) -> io::Result<()> {
        self.read_events()?;
        for ev in std::mem::take(&mut self.event_buffer) {
            self.handle_event(ev);
        }
        Ok(())
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::PropertyChange { data, name } => match name.as_str() {
                "playlist" => {
                    Self::store_deserialized_property(&name, data, &mut self.playlist);
                }
                "track-list" => {
                    Self::store_deserialized_property(&name, data, &mut self.tracks);
                }
                "chapter-list" => {
                    Self::store_deserialized_property(&name, data, &mut self.chapters);
                }
                _ => {
                    self.observed_properties.insert(name, data);
                }
            },
            Event::Seek => {}
            Event::Unknown => {
                eprintln!("Unknown event received");
            }
        }
    }

    fn store_deserialized_property<T: DeserializeOwned>(name: &str, data: Value, field: &mut T) {
        match serde_json::from_value::<T>(data.clone()) {
            Ok(value) => {
                *field = value;
            }
            Err(e) => {
                eprintln!("Failed to parse {}: {e}\nData: {data}", name);
            }
        }
    }

    pub fn observe_property(&mut self, property: &str) -> io::Result<()> {
        let cmd = Command::observe_property(self.next_observe_id, property);
        self.next_observe_id += 1;

        self.command::<()>(cmd)?;
        Ok(())
    }

    pub fn get_property_cached<T: DeserializeOwned>(&self, name: &str) -> Option<T> {
        if let Some(value) = self.observed_properties.get(name) {
            serde_json::from_value(value.clone()).ok()
        } else {
            None
        }
    }

    pub fn get_property<T: DeserializeOwned>(&mut self, name: &str) -> T {
        if let Some(value) = self.get_property_cached(name) {
            value
        } else {
            self.observe_property(name)
                .expect("Failed to observe property");

            loop {
                self.read_events().expect("Failed to read events");

                for ev in &self.event_buffer {
                    if let Event::PropertyChange { data, name: prop_name } = ev
                        && prop_name == name
                    {
                        return serde_json::from_value(data.clone())
                            .unwrap_or_else(|_| panic!("Failed to parse property {}", name));
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }

    pub fn set_property(&mut self, name: &str, value: impl Serialize) -> io::Result<()> {
        self.command::<()>(Command::set_property(name, value))?;
        Ok(())
    }

    pub fn cycle_property(&mut self, name: &str) -> io::Result<()> {
        self.command::<()>(Command::cycle_property(name))?;
        Ok(())
    }

    pub fn time_pos(&self) -> Option<Time> {
        self.get_property_cached("time-pos")
    }

    pub fn time_pos_fallback(&self) -> Time {
        self.time_pos().unwrap_or(Time::ZERO)
    }

    pub fn duration(&self) -> Option<Time> {
        self.get_property_cached("duration")
    }

    pub fn duration_fallback(&self) -> Time {
        self.duration().unwrap_or(self.time_pos_fallback())
    }

    pub fn pause(&mut self) -> io::Result<()> {
        self.set_property("pause", true)
    }

    pub fn unpause(&mut self) -> io::Result<()> {
        self.set_property("pause", false)
    }

    fn seek_state(&mut self) -> &mut SeekState {
        match self.seek_state {
            Some(SeekState { ended: Some(ended), .. })
                if ended.elapsed() < Duration::from_secs(60) =>
            {
                let pos = self.get_property("percent-pos");
                let paused = self.get_property("pause");

                self.pause().ok();

                let state = self.seek_state.as_mut().unwrap();
                state.pos = pos;
                state.paused = paused;
                state.ended = None;

                state
            }

            Some(ref mut state @ SeekState { ended: None, .. }) => state,

            Some(SeekState { ended: Some(_), .. }) | None => {
                self.seek_state = Some(SeekState {
                    speed: Default::default(),
                    exact: false,
                    ended: None,

                    pos: self.get_property("percent-pos"),
                    paused: self.get_property("pause"),
                });

                self.pause().ok();

                self.seek_state.as_mut().unwrap()
            }
        }
    }

    fn seconds_left(&self) -> Option<Time> {
        let duration: Time = self.duration()?;
        let position: Time = self.time_pos()?;

        if duration > Time::ZERO && position >= Time::ZERO {
            Some(duration - position)
        } else {
            None
        }
    }

    pub fn start_seek(&mut self) {
        self.seek_state();
    }

    fn seek_inner(&mut self, forward: bool) -> io::Result<()> {
        let seconds_left = self.seconds_left();

        let state = self.seek_state();

        let mut seconds = state.speed.time();
        if !forward {
            seconds = -seconds;
        }

        let would_seek_past_end = forward && seconds_left.is_some_and(|left| left < seconds);
        let exact = state.exact || would_seek_past_end;
        self.command::<()>(Command::seek(seconds, exact))?;

        Ok(())
    }

    pub fn seek_forward(&mut self) -> io::Result<()> {
        self.seek_inner(true)
    }

    pub fn seek_backward(&mut self) -> io::Result<()> {
        self.seek_inner(false)
    }

    pub fn seek_stateless(&mut self, seconds: Time, exact: bool) -> io::Result<()> {
        self.command::<()>(Command::seek(seconds, exact))?;
        Ok(())
    }

    pub fn seek_faster(&mut self) {
        if let Some(SeekState { speed: ref mut seek_speed, .. }) = self.seek_state
            && let Some(new_speed) = seek_speed.longer()
        {
            *seek_speed = new_speed;
        }
    }

    pub fn seek_slower(&mut self) {
        if let Some(SeekState { speed: ref mut seek_speed, .. }) = self.seek_state
            && let Some(new_speed) = seek_speed.shorter()
        {
            *seek_speed = new_speed;
        }
    }

    pub fn seek_exact(&self) -> bool {
        self.seek_state.as_ref().is_some_and(|s| s.exact)
    }

    pub fn toggle_seek_exact(&mut self) {
        if let Some(SeekState { ref mut exact, .. }) = self.seek_state {
            *exact = !*exact;
        }
    }

    pub fn seek_speed(&self) -> Option<SeekSpeed> {
        self.seek_state.as_ref().map(|s| s.speed)
    }

    pub fn finish_seek(&mut self) -> io::Result<()> {
        if let Some(SeekState { paused, ref mut ended, .. }) = self.seek_state {
            *ended = Some(Instant::now());

            if !paused {
                self.unpause()?;
            }
        }
        Ok(())
    }

    pub fn cancel_seek(&mut self) -> io::Result<()> {
        if let Some(SeekState { pos, paused, .. }) = self.seek_state.take() {
            self.command::<()>(Command::set_property("percent-pos", pos))?;
            if !paused {
                self.unpause()?;
            }
        }
        Ok(())
    }

    pub fn tracks_of_type(&self, ty: TrackType) -> &[Track] {
        let first = self.tracks.iter().position(|t| t.ty == ty);
        let last = self.tracks.iter().rposition(|t| t.ty == ty);

        if let (Some(first), Some(last)) = (first, last) {
            &self.tracks[first..=last]
        } else {
            &[]
        }
    }

    pub fn chapters(&self) -> Vec<Chapter<'_>> {
        if self.chapters.is_empty() {
            return vec![];
        }

        let current_chapter_index = self
            .time_pos()
            .and_then(|time_pos| self.chapters.iter().rposition(|c| c.time <= time_pos));

        let starts = self.chapters.iter().map(|c| c.time);
        let ends = self
            .chapters
            .iter()
            .skip(1)
            .map(|c| c.time)
            .chain(std::iter::once(self.duration_fallback()));

        let durations = starts.zip(ends).map(|(start, end)| end - start);

        self.chapters
            .iter()
            .zip(durations)
            .enumerate()
            .map(|(index, (raw, duration))| Chapter {
                title: raw.title.as_deref(),
                start: raw.time,
                current: current_chapter_index == Some(index),
                duration,
            })
            .collect()
    }

    pub fn playlist(&self) -> &[PlaylistEntry] {
        &self.playlist
    }
}

impl Default for Mpv {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[expect(dead_code)]
pub struct Track {
    #[serde(rename = "type")]
    pub ty: TrackType,
    /// The ID as it's used for --sid/--aid/--vid. This is unique within tracks of the same type
    /// (sub/audio/video), but otherwise not.
    pub id: i32,
    /// Track title as it is stored in the file. Not always available.
    pub title: Option<String>,
    /// Track language as identified by the file. Not always available.
    pub lang: Option<String>,
    /// The codec name used by this track, for example h264. Unavailable in some rare cases.
    pub codec: Option<String>,
    /// The filename if the track is from an external file, unavailable otherwise.
    pub external_filename: Option<String>,
    /// yes/true if the track is currently decoded, no/false or unavailable otherwise.
    #[serde(default)]
    pub selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackType {
    Video,
    Audio,
    Sub,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[expect(dead_code)]
pub struct PlaylistEntry {
    pub filename: String,
    /// true if the playlist-playing-pos property points to this entry
    #[serde(default)]
    pub playing: bool,
    /// true if the playlist-current-pos property points to this entry
    #[serde(default)]
    pub current: bool,
    /// Name of the Nth entry. Available if the playlist file contains such fields and mpv's parser
    /// supports it for the given playlist format, or if the playlist entry has been opened before
    /// and a media-title other than filename has been acquired.
    pub title: Option<String>,
    /// Unique ID for this entry. This is an automatically assigned integer ID that is unique for
    /// the entire life time of the current mpv core instance. Other commands, events, etc. use
    /// this as playlist_entry_id fields.
    pub id: i32,
    /// The original path of the playlist for this entry before mpv ex- panded it. Unavailable if
    /// the file was not originally associated with a playlist in some way.
    pub playlist_path: Option<String>,
}

impl PlaylistEntry {
    pub fn display_name(&self) -> &str {
        match self {
            Self { title: Some(t), .. } => t,
            Self { filename, .. } => filename,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ChapterRaw {
    pub title: Option<String>,
    pub time: Time,
}

#[derive(Debug)]
pub struct Chapter<'a> {
    pub title: Option<&'a str>,
    pub start: Time,
    pub current: bool,
    pub duration: Time,
}
