use std::path::PathBuf;

use dossier_overlay::live::{unstable_rate, Rests};
use dossier_overlay::{Beatmap, Client, Frame, Gameplay, Message, Meter, Screen, Snapshot};

use crate::beatmaps::{DT, HT, NC};
use crate::stable::{Glance, Map, Mode};

pub struct Publisher {
    session: u64,
    sequence: u64,
    meter: Option<Meter>,
    songs: Option<PathBuf>,
    rests: Option<(String, Option<Rests>)>,
}

pub fn clock_rate(mods: u32) -> f64 {
    if mods & (DT | NC) != 0 {
        1.5
    } else if mods & HT != 0 {
        0.75
    } else {
        1.0
    }
}

impl Publisher {
    pub fn new(session: u64) -> Self {
        Self {
            session,
            sequence: 0,
            meter: None,
            songs: None,
            rests: None,
        }
    }

    pub fn knowing(mut self, meter: Option<Meter>, songs: Option<PathBuf>) -> Self {
        self.meter = meter;
        self.songs = songs;
        self
    }

    fn rests_of(&mut self, map: &Map) -> Option<&Rests> {
        if self.rests.as_ref().is_none_or(|(md5, _)| *md5 != map.md5) {
            let read = self
                .songs
                .as_ref()
                .filter(|_| !map.folder.is_empty() && !map.file.is_empty())
                .and_then(|songs| std::fs::read(songs.join(&map.folder).join(&map.file)).ok())
                .and_then(|bytes| Rests::parse(&String::from_utf8_lossy(&bytes)));
            self.rests = Some((map.md5.clone(), read));
        }
        self.rests.as_ref().and_then(|(_, rests)| rests.as_ref())
    }

    fn next(&mut self, message: Message) -> Frame {
        self.sequence += 1;
        Frame::new(self.session, self.sequence, message)
    }

    pub fn hello(&mut self, pid: u32, build: &str) -> Frame {
        self.next(Message::Hello {
            client: Client::Stable,
            pid,
            build: short(build),
        })
    }

    pub fn snapshot(&mut self, seen: &Glance) -> Frame {
        let resting = seen
            .map
            .as_ref()
            .and_then(|map| self.rests_of(map))
            .map(|rests| rests.resting(f64::from(seen.time_ms)));
        let screen = match seen.mode {
            Some(Mode::Menu) => Screen::Menu,
            Some(Mode::SelectPlay | Mode::SelectMulti) => Screen::Selection,
            Some(Mode::Play) => Screen::Playing,
            Some(Mode::Rank | Mode::RankingVs | Mode::RankingTagCoop | Mode::RankingTeam) => {
                Screen::Results
            }
            _ => Screen::Other,
        };
        let gameplay = seen
            .play
            .as_ref()
            .filter(|_| screen == Screen::Playing)
            .filter(|p| (0..=3).contains(&p.ruleset))
            .map(|play| {
                let c = play.counts;
                let total =
                    u64::from(c.n300) + u64::from(c.n100) + u64::from(c.n50) + u64::from(c.miss);
                let accuracy = (play.ruleset == 0 && total > 0).then(|| {
                    (u64::from(c.n300) * 300 + u64::from(c.n100) * 100 + u64::from(c.n50) * 50)
                        as f64
                        / (total * 300) as f64
                        * 100.0
                });
                Gameplay {
                    ruleset: play.ruleset as u8,
                    time_ms: i64::from(seen.time_ms),
                    score: play.score.max(0) as u64,
                    combo: u32::from(play.combo),
                    max_combo: u32::from(play.max_combo.max(play.combo)),
                    accuracy,
                    misses: u32::from(c.miss),
                    legacy_mods: Some(play.mods),
                    mods: Vec::new(),
                    unstable_rate: unstable_rate(&play.errors, clock_rate(play.mods)),
                    resting,
                }
            });
        self.next(Message::Snapshot {
            snapshot: Snapshot {
                screen,
                beatmap: seen.map.as_ref().map(|map| Beatmap {
                    md5: short(&map.md5),
                    title: short(&map.title),
                    artist: short(&map.artist),
                    difficulty: short(&map.version),
                }),
                gameplay,
                watching_replay: seen.watching,
                meter: self.meter,
            },
        })
    }

    pub fn disconnected(&mut self) -> Frame {
        self.next(Message::Disconnected)
    }
}

fn short(text: &str) -> String {
    text.chars().take(256).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stable::{Counts, Play};

    #[test]
    fn stable_snapshots_have_accuracy_and_do_not_keep_gameplay_in_menus() {
        let mut publisher = Publisher::new(1);
        let hello = publisher.hello(7, "stable");
        let mut seen = Glance {
            raw_mode: 2,
            mode: Some(Mode::Play),
            time_ms: 123,
            map: None,
            watching: Some(false),
            play: Some(Play {
                ruleset: 0,
                combo: 2,
                max_combo: 3,
                counts: Counts {
                    n300: 2,
                    miss: 1,
                    ..Counts::default()
                },
                ..Play::default()
            }),
        };
        let frame = publisher.snapshot(&seen);
        let Message::Snapshot { snapshot } = &frame.message else {
            panic!()
        };
        assert!(
            (snapshot.gameplay.as_ref().unwrap().accuracy.unwrap() - 66.6666666667).abs() < 0.0001
        );
        assert!(frame.sequence > hello.sequence);
        assert!(frame.encode().is_ok());
        seen.mode = Some(Mode::Menu);
        let Message::Snapshot { snapshot } = publisher.snapshot(&seen).message else {
            panic!()
        };
        assert!(snapshot.gameplay.is_none());
    }

    #[test]
    fn non_standard_accuracy_is_not_guessed_and_long_titles_are_bounded() {
        let mut publisher = Publisher::new(1);
        let seen = Glance {
            raw_mode: 2,
            mode: Some(Mode::Play),
            time_ms: -500,
            map: Some(crate::stable::Map {
                title: "я".repeat(20_000),
                ..Default::default()
            }),
            watching: None,
            play: Some(Play {
                ruleset: 3,
                ..Default::default()
            }),
        };
        let frame = publisher.snapshot(&seen);
        assert!(frame.encode().is_ok());
        let Message::Snapshot { snapshot } = frame.message else {
            panic!()
        };
        assert!(snapshot.gameplay.unwrap().accuracy.is_none());
        assert_eq!(snapshot.beatmap.unwrap().title.chars().count(), 256);
    }

    #[test]
    fn a_play_tells_its_unstable_rate_its_rests_and_the_meter_the_client_shows() {
        let songs = std::env::temp_dir().join(format!("dossier-witness-rests-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&songs);
        std::fs::create_dir_all(songs.join("set")).unwrap();
        std::fs::write(
            songs.join("set").join("map.osu"),
            "[Events]\n2,20000,28000\n[HitObjects]\n256,192,1500,1,0\n256,192,90000,1,0\n",
        )
        .unwrap();
        let meter = Meter {
            shown: true,
            scale: 1.5,
        };
        let mut publisher = Publisher::new(1).knowing(Some(meter), Some(songs.clone()));
        let mut seen = Glance {
            raw_mode: 2,
            mode: Some(Mode::Play),
            time_ms: 24_000,
            map: Some(Map {
                md5: "a".repeat(32),
                folder: "set".into(),
                file: "map.osu".into(),
                ..Map::default()
            }),
            watching: Some(false),
            play: Some(Play {
                mods: DT,
                errors: vec![-10, 10, -10, 10],
                ..Play::default()
            }),
        };
        let told = |publisher: &mut Publisher, seen: &Glance| {
            let frame = publisher.snapshot(seen);
            assert!(frame.encode().is_ok());
            let Message::Snapshot { snapshot } = frame.message else {
                panic!()
            };
            snapshot
        };
        let snapshot = told(&mut publisher, &seen);
        assert_eq!(snapshot.meter, Some(meter));
        let game = snapshot.gameplay.unwrap();
        assert!((game.unstable_rate.unwrap() - 100.0 / 1.5).abs() < 1e-9);
        assert_eq!(game.resting, Some(true), "the map is in its break");
        seen.time_ms = 30_000;
        assert_eq!(told(&mut publisher, &seen).gameplay.unwrap().resting, Some(false));
        seen.map.as_mut().unwrap().md5 = "b".repeat(32);
        seen.map.as_mut().unwrap().file = "missing.osu".into();
        let lost = told(&mut publisher, &seen).gameplay.unwrap();
        assert_eq!(lost.resting, None, "a map that cannot be read leaves the rest unknown");
        seen.play.as_mut().unwrap().errors = vec![3];
        assert_eq!(told(&mut publisher, &seen).gameplay.unwrap().unstable_rate, None);
        let _ = std::fs::remove_dir_all(&songs);
    }
}
