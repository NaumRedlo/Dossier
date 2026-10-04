use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Accept {
    #[default]
    Shared,
    Everyone,
    Nobody,
}

impl Accept {
    pub const ALL: [Accept; 3] = [Accept::Shared, Accept::Everyone, Accept::Nobody];

    pub fn tag(self) -> &'static str {
        match self {
            Accept::Shared => "shared",
            Accept::Everyone => "everyone",
            Accept::Nobody => "nobody",
        }
    }

    pub fn of(tag: &str) -> Accept {
        Accept::ALL.into_iter().find(|accept| accept.tag() == tag).unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct Face {
    pub player: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub shared: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Look {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub music: f32,
    pub hitsounds: f32,
    pub play: Option<crate::render::Play>,
}

impl Look {
    pub fn of(ask: &crate::render::Ask) -> Look {
        Look { width: ask.size.0, height: ask.size.1, fps: ask.fps, music: ask.music_level, hitsounds: ask.hitsound_level, play: Some(ask.play) }
    }

    pub fn dress(&self, ask: &mut crate::render::Ask) {
        if self.width >= 640 && self.height >= 360 {
            ask.size = (self.width.min(3840), self.height.min(2160));
        }
        if self.fps > 0 {
            ask.fps = self.fps.clamp(24, 240);
        }
        if self.music > 0.0 || self.hitsounds > 0.0 {
            ask.music_level = self.music.clamp(0.0, 1.0);
            ask.hitsound_level = self.hitsounds.clamp(0.0, 1.0);
        }
        if let Some(play) = self.play {
            ask.play = play;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct Received {
    pub id: u64,
    #[serde(default)]
    pub from: Option<Face>,
    #[serde(default)]
    pub player: String,
    #[serde(default)]
    pub song: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub mods: Vec<String>,
    #[serde(default)]
    pub map_hash: String,
    #[serde(default)]
    pub duration: u32,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub thumb: bool,
    #[serde(default)]
    pub replay: bool,
    #[serde(default)]
    pub storage: String,
    #[serde(default = "yes")]
    pub available: bool,
    #[serde(default)]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub settings: Option<Look>,
    #[serde(default)]
    pub sent_at: i64,
    #[serde(default)]
    pub seen: bool,
}

fn yes() -> bool { true }

impl Received {
    pub fn map_line(&self) -> String {
        if self.version.is_empty() {
            self.song.clone()
        } else {
            format!("{} [{}]", self.song, self.version)
        }
    }

    pub fn sender(&self) -> &str {
        self.from.as_ref().map_or("", |face| face.name.as_str())
    }

    pub fn cached(&self) -> Option<PathBuf> {
        Some(video_path(self.id)).filter(|path| path.is_file())
    }

    pub fn replay_name(&self) -> String {
        let said = format!("{} - {} ({})", self.player, self.map_line(), self.id);
        let clean: String = said.chars().map(|c| if c.is_alphanumeric() || " -_[]().,!'".contains(c) { c } else { '_' }).collect();
        format!("{}.osr", clean.trim().chars().take(120).collect::<String>())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct Inbox {
    #[serde(default)]
    pub registered: bool,
    #[serde(default)]
    pub accept: String,
    #[serde(default)]
    pub videos: Vec<Received>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct Refusal {
    pub player: i64,
    #[serde(default)]
    pub why: String,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct Shared {
    #[serde(default)]
    pub sent: Vec<i64>,
    #[serde(default)]
    pub refused: Vec<Refusal>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct Receivers {
    #[serde(default)]
    pub people: Vec<Face>,
}

pub fn folder() -> PathBuf {
    crate::sources::own_root().join("cache").join("received")
}

pub fn video_path(id: u64) -> PathBuf {
    folder().join(format!("{id}.mp4"))
}

pub fn fresh(heard: i64, videos: &[Received]) -> Vec<&Received> {
    videos.iter().filter(|video| !video.seen && video.sent_at > heard).collect()
}

pub fn unseen(videos: &[Received]) -> usize {
    videos.iter().filter(|video| !video.seen).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video(id: u64, sent_at: i64, seen: bool) -> Received {
        Received { id, sent_at, seen, player: "NaumRedlo".into(), song: "xi — FREEDOM DiVE".into(), version: "FOUR DIMENSIONS".into(), ..Received::default() }
    }

    #[test]
    fn the_setting_reads_what_the_server_says_and_falls_back_to_chat_mates() {
        assert_eq!(Accept::of("everyone"), Accept::Everyone);
        assert_eq!(Accept::of("nobody"), Accept::Nobody);
        assert_eq!(Accept::of("whatever"), Accept::Shared);
        assert!(Accept::ALL.iter().all(|accept| Accept::of(accept.tag()) == *accept));
    }

    #[test]
    fn only_unseen_videos_newer_than_the_last_heard_are_news() {
        let videos = [video(3, 300, false), video(2, 200, true), video(1, 100, false)];
        assert_eq!(fresh(150, &videos).iter().map(|v| v.id).collect::<Vec<_>>(), vec![3]);
        assert_eq!(fresh(0, &videos).len(), 2);
        assert_eq!(unseen(&videos), 2);
    }

    #[test]
    fn the_inbox_reads_what_the_server_sends() {
        let body = r#"{"registered": true, "accept": "everyone", "at": 1, "videos": [{"id": 5, "from": {"player": 2, "name": "kotofey", "country": "RU", "avatar": "https://a.ppy.sh/80"},
            "player": "kotofey", "song": "xi - Blue Zenith", "version": "FOUR DIMENSIONS", "mods": ["HD", "HR"], "map_hash": "abc", "duration": 134, "size": 51000000,
            "width": 1920, "height": 1080, "thumb": true, "replay": true, "settings": {"width": 1920, "height": 1080, "fps": 60, "play": {"hud": false, "dim": 90}}, "sent_at": 1790000000, "seen": false}]}"#;
        let inbox: Inbox = serde_json::from_str(body).unwrap();
        let got = &inbox.videos[0];
        assert_eq!((got.id, got.sender(), got.duration, got.thumb, got.replay), (5, "kotofey", 134, true, true));
        assert_eq!(got.map_line(), "xi - Blue Zenith [FOUR DIMENSIONS]");
        let look = got.settings.clone().unwrap();
        let play = look.play.unwrap();
        assert!(!play.hud && play.dim == 90 && play.key_overlay);
        assert_eq!(Accept::of(&inbox.accept), Accept::Everyone);
    }

    #[test]
    fn a_borrowed_look_changes_only_what_it_knows() {
        let mut ask = crate::render::Ask {
            replay: PathBuf::new(),
            map: PathBuf::new(),
            map_hash: String::new(),
            ffmpeg: PathBuf::new(),
            out: PathBuf::new(),
            size: (1280, 720),
            fps: 30,
            crf: 20,
            skin: None,
            music_level: 0.5,
            hitsound_level: 0.5,
            play: crate::render::Play::default(),
        };
        Look::default().dress(&mut ask);
        assert_eq!((ask.size, ask.fps, ask.music_level), ((1280, 720), 30, 0.5));
        let look = Look { width: 1920, height: 1080, fps: 60, music: 1.0, hitsounds: 0.8, play: Some(crate::render::Play { hud: false, ..crate::render::Play::default() }) };
        look.dress(&mut ask);
        assert_eq!((ask.size, ask.fps, ask.music_level, ask.hitsound_level, ask.play.hud), ((1920, 1080), 60, 1.0, 0.8, false));
        assert_eq!(Look::of(&ask), look);
    }

    #[test]
    fn a_replay_gets_a_name_a_disk_takes() {
        let got = Received { id: 7, player: "a/b".into(), song: "x: y?".into(), version: "z*".into(), ..Received::default() };
        assert_eq!(got.replay_name(), "a_b - x_ y_ [z_] (7).osr");
    }
}
