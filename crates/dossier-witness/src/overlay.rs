use dossier_overlay::{Beatmap, Client, Frame, Gameplay, Message, Screen, Snapshot};

use crate::stable::{Glance, Mode};

pub struct Publisher {
    session: u64,
    sequence: u64,
}

impl Publisher {
    pub fn new(session: u64) -> Self {
        Self {
            session,
            sequence: 0,
        }
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
}
