use std::{
    io::BufRead as _,
    path::{Path, PathBuf},
    process,
};

use egui::{
    Id,
    cache::{ComputerMut, FrameCache},
};
use gilrs::Button;
use serde::Deserialize;

use super::HomeMenu;
use crate::{
    App,
    command::Command,
    gamepad::button_prompt_raw,
    utils::{ResponseExt as _, youtube_id_from_url},
};

pub struct LibraryMenu;

impl HomeMenu for LibraryMenu {
    fn label(&self) -> &'static str {
        "Library"
    }

    fn enabled(&self, _app: &App) -> bool {
        true
    }

    fn panel(&self, ctx: &egui::Context, app: &mut App) {
        egui::CentralPanel::default()
            .frame(self.frame(ctx))
            .show(ctx, |ui| {
                self.inner(ui, app);
            });
    }

    fn draw(&self, ui: &mut egui::Ui, app: &mut App) {
        let cwd_id = Id::new("library cwd");

        let (contents, cwd) = ui.memory_mut(|mem| {
            let cwd = mem
                .data
                .get_temp::<PathBuf>(cwd_id)
                .unwrap_or_else(|| PathBuf::from("/data/index"));

            let cache = mem.caches.cache::<DirContentsCache<'_>>();
            (cache.get(cwd.as_path()), cwd)
        });

        if cwd != Path::new("/data/index") && cwd.parent().is_some() {
            let button = ui.button(button_prompt_raw(Button::South, "Go up"));

            if button.has_focus() {
                ui.scroll_to_rect(button.rect, None);
            }

            if button.activated()
                && let Some(parent) = cwd.parent()
            {
                ui.memory_mut(|mem| {
                    mem.data.insert_temp(cwd_id, parent.to_path_buf());
                });
            }
        }

        for (idx, entry) in contents.iter().enumerate() {
            let button = ui
                .add_enabled_ui(!entry.is_other_file() || idx == 0, |ui| ui.button(entry.label()))
                .inner;

            if idx == 0 {
                button.autofocus();
            }

            if button.has_focus() {
                ui.scroll_to_rect(button.rect, None);
            }

            if button.activated() {
                match &entry.info {
                    EntryInfo::MediaFile(_media_info) => {
                        app.mpv.load_file(&entry.path.to_string_lossy()).ok();
                        app.mpv.unpause().ok();

                        app.queue_command(Command::HideUi);
                    }
                    EntryInfo::MediaFolder(playlist) => {
                        app.mpv
                            .load_file(&playlist.index_path.to_string_lossy())
                            .ok();
                        app.mpv.unpause().ok();

                        app.queue_command(Command::HideUi);
                    }
                    EntryInfo::OtherFile => {}
                    EntryInfo::RawFolder => {
                        ui.memory_mut(|mem| {
                            mem.data.insert_temp(cwd_id, entry.path.clone());
                        });
                    }
                }
            }
        }
    }
}

type DirContentsCache<'a> = FrameCache<Vec<DirEntry>, DirFetcher>;

#[derive(Default)]
struct DirFetcher;
impl ComputerMut<&Path, Vec<DirEntry>> for DirFetcher {
    fn compute(&mut self, key: &Path) -> Vec<DirEntry> {
        let Ok(read_dir) = std::fs::read_dir(key) else {
            return vec![];
        };

        let mut entries = vec![];
        for entry in read_dir.flatten() {
            entries.push(DirEntry::from_path(entry.path()));
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        entries
    }
}

#[derive(Debug, Clone)]
struct DirEntry {
    path: PathBuf,
    info: EntryInfo,
}

impl DirEntry {
    fn from_path(path: PathBuf) -> Self {
        DirEntry {
            info: EntryInfo::from_path(&path),
            path,
        }
    }

    fn label(&self) -> String {
        let filename = || self.path.file_name().unwrap().to_string_lossy().to_string();

        match &self.info {
            EntryInfo::MediaFile(media_info) => media_info.title.clone().unwrap_or_else(filename),
            EntryInfo::MediaFolder(playlist) => playlist.title.clone().unwrap_or_else(filename),
            EntryInfo::OtherFile | EntryInfo::RawFolder => filename(),
        }
    }

    fn is_other_file(&self) -> bool {
        matches!(self.info, EntryInfo::OtherFile)
    }
}

#[derive(Debug, Clone)]
enum EntryInfo {
    MediaFile(MediaInfo),
    MediaFolder(Playlist),
    OtherFile,
    RawFolder,
}

impl EntryInfo {
    fn from_path(path: &Path) -> Self {
        if path.is_dir() {
            if let Some(playlist) = Playlist::from_path(path) {
                EntryInfo::MediaFolder(playlist)
            } else {
                EntryInfo::RawFolder
            }
        } else if let Some(info) = MediaInfo::from_path(path) {
            EntryInfo::MediaFile(info)
        } else {
            EntryInfo::OtherFile
        }
    }
}

#[derive(Debug, Clone)]
struct MediaInfo {
    title: Option<String>,
    youtube_id: Option<String>,
}

impl MediaInfo {
    fn from_path(path: &Path) -> Option<Self> {
        let output = process::Command::new("ffprobe")
            .arg("-i")
            .arg(path)
            .args(["-show_entries", "format_tags"])
            .args(["-of", "json"])
            .output()
            .unwrap();

        if !output.status.success() {
            return None;
        }

        #[derive(Deserialize)]
        struct Root {
            #[serde(default)]
            format: Format,
        }

        #[derive(Default, Deserialize)]
        struct Format {
            #[serde(default)]
            tags: Tags,
        }

        #[derive(Default, Deserialize)]
        struct Tags {
            title: Option<String>,
            purl: Option<String>,
        }

        let root: Root = serde_json::from_slice(&output.stdout).ok()?;

        Some(Self {
            title: root.format.tags.title,
            youtube_id: root
                .format
                .tags
                .purl
                .as_deref()
                .and_then(youtube_id_from_url)
                .map(|s| s.to_string()),
        })
    }
}

#[derive(Debug, Clone, Default)]
struct Playlist {
    index_path: PathBuf,
    title: Option<String>,
    num_entries: usize,
}

impl Playlist {
    fn from_path(path: &Path) -> Option<Self> {
        let index_path = if path.is_dir() {
            [path.join("index.m3u8"), path.join("index.m3u")]
                .into_iter()
                .find(|p| p.is_file())?
        } else {
            path.to_path_buf()
        };

        let file = std::fs::File::open(&index_path).ok()?;
        let reader = std::io::BufReader::new(file);

        let mut playlist = Playlist { index_path, ..Default::default() };

        for line in reader.lines().map_while(Result::ok) {
            if line == "#EXTM3U" {
                continue;
            }

            match line.chars().next() {
                Some('#') => {
                    if let Some(title) = line.strip_prefix("#PLAYLIST:") {
                        playlist.title = Some(title.trim().to_string());
                    }
                }
                Some(_) => {
                    playlist.num_entries += 1;
                }
                None => {}
            }
        }

        Some(playlist)
    }
}
