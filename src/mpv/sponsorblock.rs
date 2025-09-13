use egui::Color32;
use ehttp::Request;
use serde::{Deserialize, Serialize};
use url::Url;

use super::time::Time;

pub fn fetch_skip_segments(video_id: &str) -> Option<Vec<SkipSegment>> {
    let mut url = Url::parse("https://sponsor.ajay.app/api/skipSegments").unwrap();

    url.query_pairs_mut()
        .append_pair("videoID", video_id)
        .append_pair(
            "categories",
            &serde_json::to_string(&[
                Category::Sponsor,
                Category::Selfpromo,
                Category::Intro,
                Category::Outro,
            ])
            .unwrap(),
        );

    let res = ehttp::fetch_blocking(&Request::get(url.as_str())).ok()?;

    serde_json::from_slice(&res.bytes)
        .map_err(|e| eprintln!("Failed to parse skip segments: {}", e))
        .ok()
}

#[derive(Debug, Deserialize)]
pub struct SkipSegment {
    pub segment: (Time, Time),
    #[expect(dead_code)]
    #[serde(rename = "UUID")]
    pub uuid: String,
    pub category: Category,
}

impl SkipSegment {
    pub fn start(&self) -> Time {
        self.segment.0
    }

    pub fn end(&self) -> Time {
        self.segment.1
    }

    pub fn contains(&self, time: Time) -> bool {
        time >= self.start() && time < self.end()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Sponsor,
    Selfpromo,
    Interaction,
    Intro,
    Outro,
    Preview,
    Hook,
    Filler,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::Sponsor => "Sponsor",
            Category::Selfpromo => "Self-Promo",
            Category::Interaction => "Interaction",
            Category::Intro => "Intro",
            Category::Outro => "Outro",
            Category::Preview => "Preview",
            Category::Hook => "Hook",
            Category::Filler => "Filler",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Category::Sponsor => Color32::from_rgb(255, 215, 0),
            Category::Selfpromo => Color32::from_rgb(135, 206, 250),
            Category::Interaction => Color32::from_rgb(144, 238, 144),
            Category::Intro => Color32::from_rgb(255, 182, 193),
            Category::Outro => Color32::from_rgb(255, 160, 122),
            Category::Preview => Color32::from_rgb(221, 160, 221),
            Category::Hook => Color32::from_rgb(255, 105, 180),
            Category::Filler => Color32::from_rgb(211, 211, 211),
        }
    }
}
