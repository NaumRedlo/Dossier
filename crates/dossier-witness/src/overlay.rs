use std::path::PathBuf;
use std::time::{Duration, Instant};

use dossier_overlay::live::{unstable_rate, Rests};
use dossier_overlay::{Beatmap, Client, Frame, Gameplay, Message, Meter, Screen, Snapshot};

use crate::beatmaps::{DT, HT, NC};
use crate::pace::{Pace, Reach};
use crate::stable::{Counts, Glance, Map, Mode, Play};

const PACE_GAP: Duration = Duration::from_millis(250);
const CONFIG_GAP: Duration = Duration::from_secs(2);

struct Chart {
    md5: String,
    text: Option<String>,
    rests: Option<Rests>,
    pace: Option<(u32, Option<Pace>)>,
    reach: Option<(Instant, Counts, u32, Reach)>,
    record: Option<Option<f64>>,
}

struct Config {
    file: PathBuf,
    written: Option<std::time::SystemTime>,
    looked: Instant,
}

pub struct Publisher {
    session: u64,
    sequence: u64,
    meter: Option<Meter>,
    songs: Option<PathBuf>,
    chart: Option<Chart>,
    pace_gap: Duration,
    config: Option<Config>,
    config_gap: Duration,
    scores: Option<PathBuf>,
    player: String,
    began: Option<i32>,
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
            chart: None,
            pace_gap: PACE_GAP,
            config: None,
            config_gap: CONFIG_GAP,
            scores: None,
            player: String::new(),
            began: None,
        }
    }

    pub fn knowing(mut self, meter: Option<Meter>, songs: Option<PathBuf>) -> Self {
        self.meter = meter;
        self.songs = songs;
        self
    }

    pub fn watching(mut self, config: Option<PathBuf>, scores: Option<PathBuf>, player: &str) -> Self {
        self.config = config.map(|file| Config { written: std::fs::metadata(&file).and_then(|held| held.modified()).ok(), file, looked: Instant::now() });
        self.scores = scores;
        self.player = player.to_owned();
        self
    }

    fn meter_now(&mut self) -> Option<Meter> {
        let gap = self.config_gap;
        if let Some(config) = self.config.as_mut().filter(|config| config.looked.elapsed() >= gap) {
            config.looked = Instant::now();
            let written = std::fs::metadata(&config.file).and_then(|held| held.modified()).ok();
            if written != config.written {
                config.written = written;
                if let Some(meter) = std::fs::read(&config.file).ok().and_then(|bytes| crate::client::Client::said_in(&String::from_utf8_lossy(&bytes)).meter()) {
                    self.meter = Some(meter);
                }
            }
        }
        self.meter
    }

    fn record_of(&mut self, map: &Map, fresh: bool) -> Option<f64> {
        let (scores, player) = (self.scores.clone()?, self.player.clone());
        let chart = self.chart_of(map);
        if fresh || chart.record.is_none() {
            let held = crate::scores::on_map(&scores, &map.md5, &player);
            chart.record = Some(chart.text.as_deref().filter(|_| !held.is_empty()).and_then(|text| Pace::best(text, &held)));
        }
        chart.record.flatten()
    }

    fn chart_of(&mut self, map: &Map) -> &mut Chart {
        if self.chart.as_ref().is_none_or(|chart| chart.md5 != map.md5) {
            let text = self
                .songs
                .as_ref()
                .filter(|_| !map.folder.is_empty() && !map.file.is_empty())
                .and_then(|songs| std::fs::read(songs.join(&map.folder).join(&map.file)).ok())
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
            self.chart = Some(Chart {
                md5: map.md5.clone(),
                rests: text.as_deref().and_then(Rests::parse),
                text,
                pace: None,
                reach: None,
                record: None,
            });
        }
        self.chart.as_mut().expect("the chart was just read")
    }

    fn reach_of(&mut self, map: &Map, play: &Play) -> Option<Reach> {
        let gap = self.pace_gap;
        let chart = self.chart_of(map);
        if chart.pace.as_ref().is_none_or(|(mods, _)| *mods != play.mods) {
            chart.pace = Some((
                play.mods,
                chart.text.as_deref().and_then(|text| Pace::of(text, play.mods)),
            ));
            chart.reach = None;
        }
        let pace = chart.pace.as_mut()?.1.as_mut()?;
        let combo = u32::from(play.max_combo.max(play.combo));
        let stale = match &chart.reach {
            None => true,
            Some((at, counts, best, _)) => {
                (*counts != play.counts || *best != combo)
                    && (pace.judged(play.counts) == 0 || at.elapsed() >= gap)
            }
        };
        if stale {
            chart.reach = Some((
                Instant::now(),
                play.counts,
                combo,
                pace.reach(play.counts, combo),
            ));
        }
        chart.reach.map(|(.., reach)| reach)
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
            .and_then(|map| self.chart_of(map).rests.as_ref())
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
        let playing = seen
            .play
            .as_ref()
            .filter(|_| screen == Screen::Playing)
            .filter(|p| (0..=3).contains(&p.ruleset));
        let reach = playing
            .filter(|play| play.ruleset == 0)
            .zip(seen.map.as_ref())
            .and_then(|(play, map)| self.reach_of(map, play));
        let starting = seen.time_ms < self.began.unwrap_or(i32::MAX);
        self.began = playing.map(|_| seen.time_ms);
        let record = playing
            .filter(|play| play.ruleset == 0)
            .zip(seen.map.as_ref())
            .and_then(|(_, map)| self.record_of(map, starting));
        let meter = self.meter_now();
        let gameplay = playing
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
                    pp: reach.map(|reach| reach.now),
                    pp_clean: reach.map(|reach| reach.clean),
                    pp_record: record,
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
                meter,
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
            "osu file format v14\n[Events]\n2,20000,28000\n[HitObjects]\n256,192,1500,1,0\n256,192,90000,1,0\n",
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

    #[test]
    fn a_standard_play_tells_the_pp_reached_and_the_pp_of_a_clean_play() {
        let songs = std::env::temp_dir().join(format!("dossier-witness-pace-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&songs);
        std::fs::create_dir_all(songs.join("set")).unwrap();
        let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../dossier-assay/corpus/maps/5114204.osu");
        std::fs::copy(corpus, songs.join("set").join("map.osu")).unwrap();
        let mut publisher = Publisher::new(1).knowing(None, Some(songs.clone()));
        publisher.pace_gap = Duration::ZERO;
        let mut seen = Glance {
            raw_mode: 2,
            mode: Some(Mode::Play),
            time_ms: 1000,
            map: Some(Map {
                md5: "c".repeat(32),
                folder: "set".into(),
                file: "map.osu".into(),
                ..Map::default()
            }),
            watching: Some(false),
            play: Some(Play::default()),
        };
        let told = |publisher: &mut Publisher, seen: &Glance| {
            let frame = publisher.snapshot(seen);
            assert!(frame.encode().is_ok());
            let Message::Snapshot { snapshot } = frame.message else {
                panic!()
            };
            snapshot.gameplay
        };
        let start = told(&mut publisher, &seen).unwrap();
        assert_eq!(start.pp, Some(0.0), "nothing is hit yet");
        let whole = start.pp_clean.unwrap();
        assert!(whole > 10.0, "{whole}");

        let hits = |seen: &mut Glance, n300: u16, miss: u16, combo: u16| {
            let play = seen.play.as_mut().unwrap();
            play.counts = Counts { n300, miss, ..Counts::default() };
            play.combo = combo;
            play.max_combo = combo;
        };
        hits(&mut seen, 120, 0, 150);
        let early = told(&mut publisher, &seen).unwrap();
        hits(&mut seen, 300, 0, 380);
        let later = told(&mut publisher, &seen).unwrap();
        assert!(early.pp.unwrap() > 0.0 && later.pp.unwrap() > early.pp.unwrap(), "{early:?} then {later:?}");
        assert!(later.pp.unwrap() < whole);
        assert_eq!(later.pp_clean, Some(whole), "a clean play keeps its ceiling");

        hits(&mut seen, 294, 6, 90);
        let missed = told(&mut publisher, &seen).unwrap();
        assert!(missed.pp.unwrap() < later.pp.unwrap());

        seen.play.as_mut().unwrap().mods = DT;
        let faster = told(&mut publisher, &seen).unwrap();
        assert!(faster.pp_clean.unwrap() > whole * 1.3, "other mods are another calculation");

        seen.play.as_mut().unwrap().ruleset = 3;
        let mania = told(&mut publisher, &seen).unwrap();
        assert_eq!((mania.pp, mania.pp_clean), (None, None), "only osu!standard is calculated");

        seen.play.as_mut().unwrap().ruleset = 0;
        seen.map.as_mut().unwrap().md5 = "d".repeat(32);
        seen.map.as_mut().unwrap().file = "missing.osu".into();
        let lost = told(&mut publisher, &seen).unwrap();
        assert_eq!((lost.pp, lost.pp_clean), (None, None), "a map that cannot be read has no PP");
        let _ = std::fs::remove_dir_all(&songs);
    }

    #[test]
    fn pp_is_not_recalculated_more_often_than_the_gap_allows() {
        let songs = std::env::temp_dir().join(format!("dossier-witness-gap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&songs);
        std::fs::create_dir_all(songs.join("set")).unwrap();
        let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../dossier-assay/corpus/maps/5114204.osu");
        std::fs::copy(corpus, songs.join("set").join("map.osu")).unwrap();
        let mut publisher = Publisher::new(1).knowing(None, Some(songs.clone()));
        publisher.pace_gap = Duration::from_secs(3600);
        let map = Map {
            md5: "e".repeat(32),
            folder: "set".into(),
            file: "map.osu".into(),
            ..Map::default()
        };
        let play = |n300: u16| Play {
            counts: Counts { n300, ..Counts::default() },
            max_combo: n300,
            ..Play::default()
        };
        let first = publisher.reach_of(&map, &play(100)).unwrap();
        let held = publisher.reach_of(&map, &play(200)).unwrap();
        assert_eq!(held, first, "the value waits for the gap");
        publisher.pace_gap = Duration::ZERO;
        let moved = publisher.reach_of(&map, &play(200)).unwrap();
        assert!(moved.now > first.now);
        let again = publisher.reach_of(&map, &play(0)).unwrap();
        assert_eq!(again.now, 0.0, "a retry starts from nothing at once");
        let _ = std::fs::remove_dir_all(&songs);
    }

    #[test]
    fn the_record_on_the_map_comes_from_the_player_s_local_scores_and_is_read_again_on_a_retry() {
        let songs = std::env::temp_dir().join(format!("dossier-witness-record-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&songs);
        std::fs::create_dir_all(songs.join("set")).unwrap();
        let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../dossier-assay/corpus/maps/5114204.osu");
        std::fs::copy(corpus, songs.join("set").join("map.osu")).unwrap();
        let scores = songs.join("scores.db");
        let held = |n100: u16, miss: u16, mods: u32, player: &str| crate::scores::Held { md5: "f".repeat(32), player: player.to_owned(), counts: Counts { n300: 400, n100, miss, ..Counts::default() }, max_combo: if miss > 0 { 200 } else { 600 }, mods };
        std::fs::write(&scores, crate::scores::written(&[held(30, 5, 0, "Me"), held(2, 0, DT, "Guest")])).unwrap();
        let mut publisher = Publisher::new(1).knowing(None, Some(songs.clone())).watching(None, Some(scores.clone()), "Me");
        let mut seen = Glance {
            raw_mode: 2,
            mode: Some(Mode::Play),
            time_ms: 500,
            map: Some(Map { md5: "f".repeat(32), folder: "set".into(), file: "map.osu".into(), ..Map::default() }),
            watching: Some(false),
            play: Some(Play::default()),
        };
        let told = |publisher: &mut Publisher, seen: &Glance| {
            let Message::Snapshot { snapshot } = publisher.snapshot(seen).message else {
                panic!()
            };
            snapshot.gameplay.unwrap()
        };
        let first = told(&mut publisher, &seen).pp_record.expect("the player has a score here");
        assert!(first > 1.0 && first < told(&mut publisher, &seen).pp_clean.unwrap());
        std::fs::write(&scores, crate::scores::written(&[held(30, 5, 0, "Me"), held(1, 0, 0, "Me")])).unwrap();
        seen.time_ms = 9000;
        assert_eq!(told(&mut publisher, &seen).pp_record, Some(first), "a play in progress does not read the file again");
        seen.time_ms = 300;
        assert!(told(&mut publisher, &seen).pp_record.unwrap() > first, "a retry sees the score just set");
        seen.map.as_mut().unwrap().md5 = "0".repeat(32);
        assert_eq!(told(&mut publisher, &seen).pp_record, None, "a map without a score has no record");
        let _ = std::fs::remove_dir_all(&songs);
    }

    #[test]
    fn a_change_of_the_meter_in_the_client_s_configuration_is_seen_while_it_runs() {
        let file = std::env::temp_dir().join(format!("dossier-witness-config-{}.cfg", std::process::id()));
        std::fs::write(&file, "ScoreMeter = Error\nScoreMeterScale = 1\n").unwrap();
        let mut publisher = Publisher::new(1).knowing(Some(Meter { shown: true, scale: 1.0 }), None).watching(Some(file.clone()), None, "");
        publisher.config_gap = Duration::ZERO;
        let seen = Glance { raw_mode: 0, mode: Some(Mode::Menu), time_ms: 0, map: None, watching: None, play: None };
        let meter = |publisher: &mut Publisher| {
            let Message::Snapshot { snapshot } = publisher.snapshot(&seen).message else {
                panic!()
            };
            snapshot.meter
        };
        assert_eq!(meter(&mut publisher), Some(Meter { shown: true, scale: 1.0 }));
        std::thread::sleep(Duration::from_millis(1100));
        std::fs::write(&file, "ScoreMeter = None\nScoreMeterScale = 1.6\n").unwrap();
        assert_eq!(meter(&mut publisher), Some(Meter { shown: false, scale: 1.6 }));
        let _ = std::fs::remove_file(&file);
    }
}
