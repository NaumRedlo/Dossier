use std::path::{Path, PathBuf};

use iced::widget::stack;
use iced::{Element, Length, Size};
use iced_test::Simulator;

use crate::checks::Outcome;
use crate::first_run::{Check, Fetch, FirstRun, Pairing, Step, CHECKS};
use crate::lang::Lang;
use crate::sources::{Kind, Source};
use crate::{ui, Message, Screen, App};

fn stable() -> Source {
    Source {
        kind: Kind::Stable,
        root: crate::sources::home().join("osu"),
        songs: None,
        skins: None,
        replays: None,
        maps: Some(1342),
        skin_count: 14,
        replay_count: 187,
        scores: 0,
        on: true,
    }
}

fn own() -> Source {
    Source {
        kind: Kind::Own,
        root: crate::sources::home().join(".dossier"),
        songs: None,
        skins: None,
        replays: None,
        maps: Some(0),
        skin_count: 0,
        replay_count: 0,
        scores: 0,
        on: true,
    }
}

fn lazer() -> Source {
    Source {
        kind: Kind::Lazer,
        root: crate::sources::home().join(".local").join("share").join("osu"),
        songs: None,
        skins: None,
        replays: None,
        maps: None,
        skin_count: 3,
        replay_count: 212,
        scores: 0,
        on: true,
    }
}

fn checked(outcomes: &[(Check, Option<Outcome>)]) -> Vec<(Check, Option<Outcome>)> {
    outcomes.to_vec()
}

fn all_passed() -> Vec<(Check, Option<Outcome>)> {
    checked(&[
        (Check::Folder, Some(Outcome::Passed("1,342 maps".into()))),
        (Check::Ffmpeg, Some(Outcome::Passed("7.1".into()))),
        (Check::Engine, Some(Outcome::Passed("0.89.4".into()))),
        (Check::Bot, Some(Outcome::Passed("41 ms".into()))),
    ])
}

fn downloading(done: u64, total: u64) -> crate::ffmpeg::Step {
    crate::ffmpeg::Step::Downloading { from: "martin-riedl.de", done, total: Some(total) }
}

pub fn states(lang: Lang) -> Vec<(String, FirstRun)> {
    let waiting = Pairing::Waiting {
        code: "K7QN-M4XZ".into(),
        link: "https://t.me/onenineeightfour_bot?start=pair-K7QNM4XZ".into(),
        osu: true,
    };
    let none: Vec<(Check, Option<Outcome>)> = Vec::new();
    let mut out = vec![
        ("language", FirstRun::staged(Step::Language, lang, vec![], Pairing::Idle, none.clone())),
        ("folder-stable", FirstRun::staged(Step::Folder, lang, vec![stable()], Pairing::Idle, none.clone())),
        ("folder-lazer", FirstRun::staged(Step::Folder, lang, vec![lazer()], Pairing::Idle, none.clone())),
        ("folder-both", FirstRun::staged(Step::Folder, lang, vec![stable(), lazer()], Pairing::Idle, none.clone())),
        ("folder-missing", FirstRun::staged(Step::Folder, lang, vec![], Pairing::Idle, none.clone())),
        ("folder-own", FirstRun::staged(Step::Folder, lang, vec![own()], Pairing::Idle, none.clone())),
        ("device", FirstRun::staged(Step::Device, lang, vec![stable()], Pairing::Idle, none.clone())),
        ("bot-waiting", FirstRun::staged(Step::Bot, lang, vec![stable()], waiting.clone(), none.clone())),
        ("bot-linked", FirstRun::staged(Step::Bot, lang, vec![stable()], Pairing::Linked { who: "naumredlo".into() }, none.clone())),
        (
            "checks-running",
            FirstRun::staged(
                Step::Checks,
                lang,
                vec![stable()],
                Pairing::Idle,
                checked(&[
                    (Check::Folder, Some(Outcome::Passed("1,342 maps".into()))),
                    (Check::Ffmpeg, Some(Outcome::Passed("7.1".into()))),
                    (Check::Engine, None),
                    (Check::Bot, None),
                ]),
            ),
        ),
        (
            "checks-ffmpeg-missing",
            FirstRun::staged(
                Step::Checks,
                lang,
                vec![stable()],
                Pairing::Idle,
                checked(&[
                    (Check::Folder, Some(Outcome::Passed("1,342 maps".into()))),
                    (Check::Ffmpeg, Some(Outcome::Failed(String::new()))),
                    (Check::Engine, Some(Outcome::Passed("0.89.4".into()))),
                    (Check::Bot, Some(Outcome::Passed("41 ms".into()))),
                ]),
            ),
        ),
        ("checks-ffmpeg-downloading", {
            let mut flow = FirstRun::staged(
                Step::Checks,
                lang,
                vec![stable()],
                Pairing::Idle,
                checked(&[
                    (Check::Folder, Some(Outcome::Passed("1,342 maps".into()))),
                    (Check::Ffmpeg, Some(Outcome::Failed(String::new()))),
                    (Check::Engine, Some(Outcome::Passed("0.89.4".into()))),
                    (Check::Bot, Some(Outcome::Passed("41 ms".into()))),
                ]),
            );
            flow.fetch = Fetch::Going(downloading(12_950_000, 28_832_991));
            flow
        }),
        ("done", FirstRun::staged(Step::Checks, lang, vec![stable()], Pairing::Idle, all_passed())),
        (
            "checks-bot-skipped",
            FirstRun::staged(
                Step::Checks,
                lang,
                vec![own()],
                Pairing::Idle,
                checked(&[
                    (Check::Folder, Some(Outcome::Passed(String::new()))),
                    (Check::Ffmpeg, Some(Outcome::Passed("7.1".into()))),
                    (Check::Engine, Some(Outcome::Passed("0.89.4".into()))),
                    (Check::Bot, Some(Outcome::Skipped(String::new()))),
                ]),
            ),
        ),
    ];
    debug_assert_eq!(CHECKS.len(), 4);
    out.iter_mut().for_each(|_| {});
    out.into_iter().map(|(name, flow)| (name.to_owned(), flow)).collect()
}

pub const SIZES: [(&str, Size); 3] = [
    ("980", Size::new(980.0, 720.0)),
    ("1280", Size::new(1280.0, 800.0)),
    ("1920", Size::new(1920.0, 1080.0)),
];

pub fn frame<'a>(flow: &'a FirstRun, backdrop: &iced::widget::image::Handle) -> Element<'a, Message> {
    stack![
        ui::backdrop(backdrop),
        flow.view().map(Message::FirstRun),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

static FONTS_LOADED: std::sync::Once = std::sync::Once::new();

fn settings_once() -> iced::Settings {
    let mut settings = crate::settings();
    FONTS_LOADED.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().expect("the font system");
        for bytes in crate::theme::FONTS {
            fonts.load_font(std::borrow::Cow::Borrowed(bytes));
        }
    });
    settings.fonts.clear();
    settings
}

pub fn snapshot(flow: &FirstRun, size: Size) -> Result<iced_test::simulator::Snapshot, iced_test::Error> {
    let backdrop = ui::backdrop_handle();
    let mut ui = Simulator::with_size(settings_once(), size, frame(flow, &backdrop));
    ui.snapshot(&crate::theme::theme())
}

pub fn frame_name(name: &str, lang: Lang, label: &str) -> String {
    format!("first-run-{name}-{}-{label}", lang.tag())
}

pub fn written_as(stem: &Path) -> PathBuf {
    let name = stem.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let dir = stem.parent().map(Path::to_path_buf).unwrap_or_default();
    std::fs::read_dir(&dir)
        .ok()
        .and_then(|entries| {
            entries.flatten().map(|e| e.path()).find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with(&format!("{name}-")) && n.ends_with(".png"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or_else(|| dir.join(format!("{name}-unknown.png")))
}

pub fn write_snapshot(shot: &iced_test::simulator::Snapshot, stem: &Path) -> Result<PathBuf, String> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    struct Staging(PathBuf);
    impl Drop for Staging { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
    let parent = stem.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let staged = Staging(parent.join(format!(".dossier-frame-{}-{stamp}-{serial}", std::process::id())));
    std::fs::create_dir(&staged.0).map_err(|e| e.to_string())?;
    let name = stem.file_name().ok_or_else(|| "snapshot needs a file name".to_owned())?;
    shot.matches_image(staged.0.join(name)).map_err(|e| format!("{e:?}"))?;
    let file = written_as(&staged.0.join(name));
    let into = parent.join(file.file_name().ok_or_else(|| "snapshot PNG was not written".to_owned())?);
    std::fs::rename(&file, &into).map_err(|e| e.to_string())?;
    Ok(into)
}

pub fn every_frame() -> Vec<(String, FirstRun, Size)> {
    let mut out = Vec::new();
    for lang in Lang::ALL {
        for (name, flow) in states(lang) {
            for (label, size) in SIZES {
                let wide = label != SIZES[0].0;
                if wide && !matches!(name.as_str(), "folder-stable" | "bot-waiting") {
                    continue;
                }
                out.push((frame_name(&name, lang, label), flow.clone(), size));
            }
        }
    }
    out
}

pub fn write(dir: &Path) -> Result<usize, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut written = 0;
    for (name, flow, size) in every_frame() {
        let stem = dir.join(&name);
        let shot = snapshot(&flow, size).map_err(|e| format!("{e:?}"))?;
        write_snapshot(&shot, &stem)?;
        written += 1;
    }
    let _ = App::boot;
    let _ = Screen::Main;
    Ok(written)
}

pub fn main_frame<'a>(main: &'a crate::main_screen::Main, backdrop: &iced::widget::image::Handle) -> Element<'a, Message> {
    stack![ui::backdrop(backdrop), main.view().map(Message::Main)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub fn snapshot_main(main: &crate::main_screen::Main, size: Size) -> Result<iced_test::simulator::Snapshot, iced_test::Error> {
    let backdrop = ui::backdrop_handle();
    let mut ui = Simulator::with_size(settings_once(), size, main_frame(main, &backdrop));
    ui.snapshot(&crate::theme::theme())
}

pub fn shot_folder(root: &Path, lang: Lang, out: &Path) -> Result<(), String> {
    use crate::main_screen::{decoded, length_of, Main};
    let source = crate::sources::folder_at(root).ok_or_else(|| format!("no replays in {}", root.display()))?;
    let mut settings = crate::settings::Settings::default();
    settings.lang = lang;
    settings.sources = vec![source.clone()];
    let library = crate::library::read(&[source]);
    let mut main = Main::staged(crate::lang::Words::new(lang), settings, library, Some(0));
    let entries: Vec<_> = main.entries().to_vec();
    for entry in &entries {
        if let Some(bg) = entry.map.as_ref().and_then(|m| m.background.as_deref()) {
            if !main.thumbs.contains_key(&entry.map_hash) {
                if let Some(handle) = decoded(bg, 176, Some((176, 100))) {
                    main.thumbs.insert(entry.map_hash.clone(), handle);
                }
            }
        }
    }
    if let Some(first) = entries.first() {
        if let Some(bg) = first.map.as_ref().and_then(|m| m.background.as_deref()) {
            match decoded(bg, 960, None) {
                Some(handle) => {
                    main.scenes.insert(first.map_hash.clone(), handle);
                }
                None => eprintln!("could not decode {}", bg.display()),
            }
        } else {
            eprintln!("no background for the first replay");
        }
        main.lengths.insert(first.path.clone(), length_of(&first.path));
    }
    let shot = snapshot_main(&main, Size::new(980.0, 720.0)).map_err(|e| format!("{e:?}"))?;
    let stem = out.with_extension("");
    let _ = std::fs::remove_file(written_as(&stem));
    shot.matches_image(&stem).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

fn mock_map(key: &str, artist: &str, title: &str, version: &str) -> crate::library::Map {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("docs").join("mockups").join("main");
    crate::library::Map {
        file: docs.join(format!("{key}.osu")),
        artist: artist.to_owned(),
        title: title.to_owned(),
        version: version.to_owned(),
        background: Some(docs.join(format!("bg-{key}.jpg"))),
    }
}

const NOON: i64 = 1_789_560_000;
const DAY: i64 = 86_400;

fn mock_entry(
    player: &str,
    map: Option<crate::library::Map>,
    named: Option<crate::library::Named>,
    mods: &[&str],
    combo: u16,
    counts: [u16; 4],
    days_ago: i64,
    kind: Kind,
) -> crate::library::Entry {
    use crate::library::{Grade, Outcome};
    let hash = map.as_ref().map_or_else(|| format!("none-{player}-{days_ago}"), |m| format!("hash-{}", m.title));
    let miss = counts[3];
    let outcome = if miss > 0 { Outcome::Misses(miss) } else if combo % 2 == 0 { Outcome::FullCombo } else { Outcome::SliderBreak };
    let total = counts.iter().map(|c| *c as f64).sum::<f64>().max(1.0);
    let accuracy = (300.0 * counts[0] as f64 + 100.0 * counts[1] as f64 + 50.0 * counts[2] as f64) / (300.0 * total) * 100.0;
    crate::library::Entry {
        path: PathBuf::from(format!("{player}-{days_ago}.osr")),
        kind,
        client: if kind == Kind::Lazer { crate::library::Client::Lazer } else { crate::library::Client::Stable },
        player: player.to_owned(),
        replay_hash: format!("{player}-{days_ago}"),
        map_hash: hash,
        mods: mods.iter().map(|m| (*m).to_owned()).collect(),
        combo,
        counts,
        accuracy,
        grade: Grade::of(counts, false),
        outcome,
        played_at: NOON - days_ago * DAY - 3600,
        named,
        map,
    }
}

pub fn mock_library() -> crate::library::Library {
    let astral = mock_map("astral", "Dj Grimoire", "Astral Quantization", "Nattu VN0TH3R");
    let zenith = mock_map("zenith", "xi", "Blue Zenith", "FOUR DIMENSIONS");
    let freedom = mock_map("freedom", "xi", "FREEDOM DiVE", "Extra");
    let sink = mock_map("sink", "Chroma", "sink to the deep sea world", "Ascension");
    let nevermind = mock_map("nevermind", "Phoneboy", "Nevermind", "Insane");
    let galactic = mock_map("galactic", "DragonForce", "Galactic Astro Domination", "Extreme");
    let lucky = crate::library::named("Deeo_XD - Chocofan - LUCKY CAT [_] (2026-08-03) Osu");
    let entries = vec![
        mock_entry("NaumRedlo", Some(astral), None, &["HD", "DT", "HR"], 604, [1184, 77, 15, 0], 0, Kind::Lazer),
        mock_entry("Guest", Some(zenith.clone()), None, &["EZ"], 401, [1900, 40, 3, 0], 33, Kind::Stable),
        mock_entry("-legusshhka-", Some(freedom), None, &[], 227, [1700, 140, 20, 27], 33, Kind::Stable),
        mock_entry("Deeo_XD", None, lucky, &["DT"], 312, [520, 40, 6, 4], 44, Kind::Stable),
        mock_entry("kazak1865", Some(sink), None, &["HR"], 180, [900, 200, 40, 60], 49, Kind::Stable),
        mock_entry("kazak1865", Some(nevermind), None, &[], 421, [880, 60, 4, 3], 49, Kind::Stable),
        mock_entry("kazak1865", Some(galactic), None, &["HD"], 690, [1480, 90, 10, 1], 49, Kind::Stable),
        mock_entry("Guest", Some(zenith), None, &[], 233, [1600, 250, 30, 9], 494, Kind::Stable),
    ];
    crate::library::Library { entries, maps: 6 }
}

pub const SHOWN_BUILD: &str = "0.92.3";

pub fn main_states(lang: Lang) -> Vec<(String, crate::main_screen::Main)> {
    use crate::main_screen::{decoded, Main, Overlay};
    let mut settings = crate::settings::Settings::default();
    settings.lang = lang;
    settings.device = "Dossier device".to_owned();
    let library = mock_library();
    let staged = |chosen: Option<usize>| {
        let mut main = Main::staged(crate::lang::Words::new(lang).in_zone(3 * 3600), settings.clone(), library.clone(), chosen);
        main.now_unix = NOON;
        for entry in &library.entries {
            if let Some(bg) = entry.map.as_ref().and_then(|m| m.background.as_deref()) {
                if let Some(handle) = decoded(bg, 176, Some((176, 100))) {
                    main.thumbs.insert(entry.map_hash.clone(), handle);
                }
            }
        }
        if let Some(entry) = chosen.and_then(|c| library.entries.get(c)) {
            if let Some(bg) = entry.map.as_ref().and_then(|m| m.background.as_deref()) {
                if let Some(handle) = decoded(bg, 640, None) {
                    main.scenes.insert(entry.map_hash.clone(), handle);
                }
            }
            main.lengths.insert(entry.path.clone(), 231_400);
        }
        main
    };
    let mut worker = staged(Some(0));
    worker.overlay = Overlay::Settings;
    worker.overlay_drawn = Overlay::Settings;
    worker.overlay_fade = iced::Animation::new(true);
    worker.side = crate::settings_screen::Side::Bot;
    worker.settings.worker_on = true;
    worker.settings.token = "staged".into();
    worker.ffmpeg_version = Some("7.1".to_owned());
    worker.worker_step = Some(crate::worker::Step::Drawing { title: "NaumRedlo — yaseta - Bluenation [Grace]".into(), done: 9_120, of: 19_200, left_seconds: 96.0, fps: 142.0 });
    worker.worker_done = 3;
    worker.worker_back = 1;
    worker.worker_last = Some(crate::worker::Step::Delivered { title: "kotofey — xi - FREEDOM DiVE [Extra]".into() });
    worker.farm = Some(crate::bot::Farm {
        waiting: 2,
        workers: vec![
            crate::bot::FarmWorker { name: "MacBook Pro Наума".into(), state: "rendering".into(), delivered: 3, handed_back: 1, mine: true },
            crate::bot::FarmWorker { name: "ssnowy-pc".into(), state: "ready".into(), delivered: 12, handed_back: 0, mine: false },
            crate::bot::FarmWorker { name: "kotofey studio".into(), state: "resting".into(), delivered: 7, handed_back: 2, mine: false },
        ],
    });
    worker.ground_fade = iced::Animation::new(true);
    let mut rendering = staged(Some(0));
    rendering.ffmpeg = Some(PathBuf::from("ffmpeg"));
    rendering.rendering = Some(crate::main_screen::Rendering {
        path: library.entries[0].path.clone(),
        reached: vec![
            crate::render::Step::ReplayRead,
            crate::render::Step::MapOnDisk,
            crate::render::Step::Judged,
            crate::render::Step::Drawing { frames: 4512, of: 7280, left_seconds: 18.0 },
        ],
        out: None,
    });
    rendering.progress_shown = rendering.progress_target().unwrap_or(0.0);
    let mut queued = staged(Some(1));
    queued.ffmpeg = Some(PathBuf::from("ffmpeg"));
    queued.rendering = rendering.rendering.clone();
    queued.progress_shown = rendering.progress_shown;
    queued.menu_open = iced::Animation::new(true);
    queued.menu = Some(crate::main_screen::Tab::Feed);
    queued.queued = [1, 2]
        .into_iter()
        .map(|at| crate::main_screen::Queued {
            path: library.entries[at].path.clone(),
            ask: crate::render::Ask {
                replay: library.entries[at].path.clone(),
                map: PathBuf::from("map.osu"),
                map_hash: library.entries[at].map_hash.clone(),
                ffmpeg: PathBuf::from("ffmpeg"),
                out: PathBuf::from("out.mp4"),
                size: crate::render::SIZE,
                fps: 60,
                crf: 20,
                skin: None,
                music_level: 1.0,
                hitsound_level: 1.0,
                play: crate::render::Play::default(),
            },
        })
        .collect();
    let mut fetching = staged(Some(3));
    fetching.fetching = vec![crate::main_screen::Fetching::new(
        1,
        library.entries[3].map_hash.clone(),
        vec![
            crate::maps::Step::Looking,
            crate::maps::Step::Found(crate::maps::Found { from: "osu.direct", set: 2190769, artist: "Chocofan".into(), title: "LUCKY CAT".into(), version: String::new() }),
            crate::maps::Step::Downloading { from: "osu.direct", done: 14_890_000, total: Some(22_860_000) },
        ],
    )];
    let mut hovering = staged(Some(0));
    hovering.hover = Some(1);
    hovering.hover_bounds = Some(iced::Rectangle::new(iced::Point::new(40.0 + 116.0 + 22.0, 720.0 - 10.0 - 61.0), iced::Size::new(108.0, 61.0)));
    hovering.lifts.insert(1, iced::Animation::new(true));
    hovering.combos.insert(library.entries[1].path.clone(), Some(1224));
    let mut rendered = staged(Some(0));
    rendered.ffmpeg = Some(PathBuf::from("ffmpeg"));
    rendered.rendering = Some(crate::main_screen::Rendering {
        path: library.entries[0].path.clone(),
        reached: vec![crate::render::Step::Encoded, crate::render::Step::Saved(PathBuf::from("out.mp4"), crate::videos::Probe { length_ms: 120_000, width: 1920, height: 1080, fps: 60.0 })],
        out: Some(PathBuf::from("out.mp4")),
    });
    rendered.announce(
        crate::notices::Mark::Done,
        rendered.words.t("rendered-notice"),
        "NaumRedlo — Dj Grimoire — Astral Quantization [Nattu VN0TH3R]".to_owned(),
        "3:51  84,2 МБ".to_owned(),
        library.entries[0].map_hash.clone(),
        crate::notices::Link::OpenVideo(PathBuf::from("out.mp4")),
    );
    for notice in &mut rendered.notices.notices { notice.at = NOON; }
    for toast in &mut rendered.toasts {
        toast.shown = iced::Animation::new(true);
        toast.born = rendered.now - std::time::Duration::from_secs(2);
    }
    if let Some(bg) = library.entries[0].map.as_ref().and_then(|m| m.background.clone()) {
        if let Some(handle) = crate::main_screen::decoded(&bg, 640, None) {
            rendered.scenes.insert(library.entries[0].map_hash.clone(), handle);
        }
    }
    let mut empty = Main::staged(crate::lang::Words::new(lang).in_zone(3 * 3600), settings.clone(), crate::library::Library::default(), None);
    empty.now_unix = NOON;
    let mut looking = Main::staged(crate::lang::Words::new(lang).in_zone(3 * 3600), settings.clone(), crate::library::Library::default(), None);
    looking.now_unix = NOON;
    looking.looking = Some(crate::scan::Step::Looking { files: 84_120, found: 37, seconds: 12 });
    let mut with_videos = staged(Some(0));
    with_videos.overlay = crate::main_screen::Overlay::Videos;
    with_videos.overlay_drawn = crate::main_screen::Overlay::Videos;
    with_videos.overlay_fade = iced::Animation::new(true);
    with_videos.ground_fade = iced::Animation::new(true);
    let mock_video = |at: usize, made_at: i64, length_ms: i64, size: u64, mods: &[&str]| {
        let entry = &library.entries[at];
        crate::videos::Video {
            path: std::path::PathBuf::from(format!("/renders/{}.mp4", entry.player)),
            replay: entry.path.clone(),
            replay_hash: entry.replay_hash.clone(),
            map_hash: entry.map_hash.clone(),
            player: entry.player.clone(),
            song: entry.song().unwrap_or_default(),
            version: entry.map.as_ref().map(|m| m.version.clone()).unwrap_or_default(),
            mods: mods.iter().map(|m| (*m).to_owned()).collect(),
            length_ms,
            width: 1920,
            height: 1080,
            fps: 60.0,
            size,
            made_at,
            sent_at: None,
            background: None,
            remote: None,
            look: None,
        }
    };
    with_videos.store.videos = vec![
        mock_video(0, NOON - 1800, 231_000, 84_200_000, &["HD", "DT", "HR"]),
        mock_video(1, NOON - 38 * 3600, 134_000, 51_000_000, &["EZ"]),
        mock_video(2, NOON - 39 * 3600, 242_000, 97_700_000, &[]),
    ];
    rendered.store.videos = with_videos.store.videos.clone();
    let mut playing = with_videos.clone();
    playing.open_video = Some(0);
    playing.player = Some(std::rc::Rc::new(std::cell::RefCell::new(crate::player::Player::still(
        std::path::Path::new("/renders/NaumRedlo.mp4"),
        231_000,
        67_000,
    ))));
    playing.cinema = iced::Animation::new(true);
    playing.stage_open = iced::Animation::new(true);
    playing.settings.player_level = 0.8;
    let got = |id: u64, from: (i64, &str), at: usize, mods: &[&str], duration: u32, size: u64, ago: i64, seen: bool| {
        let entry = &library.entries[at];
        crate::inbox::Received {
            id,
            from: Some(crate::inbox::Face { player: from.0, name: from.1.to_owned(), country: "RU".into(), avatar: String::new(), shared: true }),
            player: from.1.to_owned(),
            song: entry.song().unwrap_or_default(),
            version: entry.map.as_ref().map(|m| m.version.clone()).unwrap_or_default(),
            mods: mods.iter().map(|m| (*m).to_owned()).collect(),
            map_hash: entry.map_hash.clone(),
            duration,
            size,
            width: 1920,
            height: 1080,
            thumb: true,
            replay: true,
            settings: None,
            sent_at: NOON - ago,
            seen,
        }
    };
    let mut received = with_videos.clone();
    received.settings.token = "staged".to_owned();
    received.settings.linked_as = "@naumredlo".to_owned();
    received.account = Some(crate::bot::Me { telegram_id: 7, name: "Naum Redlo".into(), username: "naumredlo".into(), avatar: false, telegram: true, player: None });
    received.sharing.tab = crate::main_screen::sharing::Tab::Received;
    received.sharing.loaded = true;
    received.sharing.registered = true;
    received.sharing.videos = vec![
        got(3, (2, "kotofey"), 1, &["HD", "HR"], 134, 51_000_000, 1200, false),
        got(2, (3, "ssnowy"), 2, &[], 242, 97_700_000, 26 * 3600, true),
        got(1, (2, "kotofey"), 0, &["DT"], 231, 84_200_000, 4 * 86_400, true),
    ];
    let mut received_open = received.clone();
    received_open.sharing.open = Some(3);
    received_open.sharing.videos[0].seen = true;
    received_open.cinema = iced::Animation::new(true);
    received_open.stage_open = iced::Animation::new(true);
    let mut received_getting = received_open.clone();
    received_getting.sharing.getting = Some(crate::main_screen::sharing::Getting { id: 3, done: 19_400_000, total: 51_000_000, stop: Default::default() });
    let mut share = playing.clone();
    share.settings.token = "staged".to_owned();
    share.settings.linked_as = "@naumredlo".to_owned();
    share.account = Some(crate::bot::Me { telegram_id: 7, name: "Naum Redlo".into(), username: "naumredlo".into(), avatar: false, telegram: true, player: None });
    share.sharing.loaded = true;
    share.sharing.registered = true;
    share.sharing.receivers = Some(vec![
        crate::inbox::Face { player: 2, name: "kotofey".into(), country: "RU".into(), avatar: String::new(), shared: true },
        crate::inbox::Face { player: 3, name: "ssnowy".into(), country: "RU".into(), avatar: String::new(), shared: true },
        crate::inbox::Face { player: 4, name: "Mirrorwave".into(), country: "KZ".into(), avatar: String::new(), shared: false },
    ]);
    share.sharing.picker = Some(crate::main_screen::sharing::Picker { path: std::path::PathBuf::from("/renders/NaumRedlo.mp4"), picked: vec![2], busy: false });
    let mut playing_mini = playing.clone();
    playing_mini.mini_player = true;
    playing_mini.cinema = iced::Animation::new(false);
    playing_mini.stage_open = iced::Animation::new(false);
    let mut playing_wide = playing.clone();
    playing_wide.widened = iced::Animation::new(true);
    let mut menu_guest = staged(Some(0));
    menu_guest.menu_open = iced::Animation::new(true);
    menu_guest.menu = Some(crate::main_screen::Tab::Account);
    let mut menu_feed = staged(Some(0));
    menu_feed.now_unix = NOON;
    menu_feed.settings.token = "staged".to_owned();
    menu_feed.settings.linked_as = "Naum Redlo".to_owned();
    menu_feed.account = Some(crate::bot::Me { telegram_id: 7, name: "Naum Redlo".into(), username: "naumredlo".into(), avatar: false, telegram: true, player: None });
    menu_feed.menu_open = iced::Animation::new(true);
    menu_feed.menu = Some(crate::main_screen::Tab::Feed);
    menu_feed.rendering = Some(crate::main_screen::Rendering {
        path: library.entries[0].path.clone(),
        reached: vec![crate::render::Step::Judged, crate::render::Step::Drawing { frames: 8_400, of: 13_860, left_seconds: 18.0 }],
        out: None,
    });
    menu_feed.progress_shown = menu_feed.progress_target().unwrap_or(0.0);
    for (mark, words, detail, note, at, link) in [
        (crate::notices::Mark::Bad, "Не удалось завершить рендер", "Guest — xi — Blue Zenith", "ffmpeg завершился с кодом 1", 1, crate::notices::Link::RenderAgain(library.entries[1].path.clone())),
        (crate::notices::Mark::Done, "Карта скачана", "xi — Blue Zenith", "[FOUR DIMENSIONS]", 1, crate::notices::Link::None),
        (crate::notices::Mark::Done, "Отправлено в Telegram", "-legusshhka- — xi — FREEDOM DiVE [Extra]", "@naumredlo  97,7 МБ", 2, crate::notices::Link::None),
        (crate::notices::Mark::Plain, "Открыто", "сборка 0.89.4", "", 0, crate::notices::Link::None),
    ]
    .into_iter()
    .rev()
    {
        let hash = if note.is_empty() { String::new() } else { library.entries[at].map_hash.clone() };
        menu_feed.notices.push(mark, words.to_owned(), detail.to_owned(), note.to_owned(), hash, link);
    }
    for (i, notice) in menu_feed.notices.notices.iter_mut().enumerate() {
        notice.at = NOON - 60 * (i as i64 + 1) * 17;
    }
    for entry in &library.entries {
        if let Some(bg) = entry.map.as_ref().and_then(|m| m.background.clone()) {
            if let Some(handle) = crate::main_screen::decoded(&bg, 640, None) {
                menu_feed.scenes.insert(entry.map_hash.clone(), handle);
            }
        }
    }
    let mut menu_account = menu_feed.clone();
    menu_account.menu_open = iced::Animation::new(true);
    menu_account.menu = Some(crate::main_screen::Tab::Account);
    menu_account.rendering = None;
    let mut menu_stats = menu_account.clone();
    menu_stats.menu_open = iced::Animation::new(true);
    menu_stats.menu = Some(crate::main_screen::Tab::Stats);
    menu_stats.store.videos = with_videos.store.videos.clone();
    menu_stats.store.videos[1].sent_at = Some(NOON);
    let mut failing = menu_feed.clone();
    failing.menu = None;
    failing.menu_open = iced::Animation::new(false);
    failing.error_shown = failing.notices.notices.iter().find(|n| n.mark == crate::notices::Mark::Bad).map(|n| n.id);
    let mut prefs_app = staged(Some(0));
    prefs_app.now_unix = NOON;
    prefs_app.overlay = crate::main_screen::Overlay::Settings;
    prefs_app.overlay_drawn = crate::main_screen::Overlay::Settings;
    prefs_app.overlay_fade = iced::Animation::new(true);
    prefs_app.ground_fade = iced::Animation::new(true);
    prefs_app.ffmpeg_version = Some("7.1".to_owned());
    prefs_app.update = crate::updates::State::Latest { at: NOON };
    prefs_app.sizes = (596_000_000, 1_180_000_000, 146_800_000);
    prefs_app.storage = crate::settings_screen::Storage { app: 48_200_000, data: 2_140_000_000, videos: 596_000_000, skins: 277_000_000, maps: 1_180_000_000, cache: 14_600_000 };
    prefs_app.store.videos = with_videos.store.videos.clone();
    prefs_app.marks_now = std::collections::HashMap::new();
    let mut prefs_bot = prefs_app.clone();
    prefs_bot.side = crate::settings_screen::Side::Bot;
    prefs_bot.settings.token = "staged".to_owned();
    prefs_bot.settings.linked_as = "@naumredlo".to_owned();
    prefs_bot.account = Some(crate::bot::Me { telegram_id: 7, name: "Naum Redlo".into(), username: "naumredlo".into(), avatar: false, telegram: true, player: None });
    prefs_app.skins = vec![std::path::PathBuf::from("/skins/- # Seoul v11"), std::path::PathBuf::from("/skins/rafis 2019")];
    prefs_bot.chats = vec![
        crate::bot::Chat { id: 7, title: "Личный чат".into(), private: true, photo: false },
        crate::bot::Chat { id: -100, title: "osu! RU  lounge".into(), private: false, photo: false },
        crate::bot::Chat { id: -101, title: "1984 crew".into(), private: false, photo: false },
    ];
    prefs_bot.now_unix = NOON;
    prefs_bot.sharing.loaded = true;
    prefs_bot.sharing.registered = true;
    prefs_bot.sharing.replays = Some(crate::mixed::State { on: true, name: "NaumRedlo".into(), count: 128, most: 300 });
    let mut prefs_osu = prefs_bot.clone();
    prefs_osu.account = Some(crate::bot::Me { telegram_id: 0, name: "NaumRedlo".into(), username: String::new(), avatar: false, telegram: false, player: Some(1) });
    prefs_osu.settings.linked_as = "NaumRedlo".to_owned();
    prefs_osu.chats = Vec::new();
    prefs_osu.pin = None;
    let mut linking = prefs_osu.clone();
    linking.pairing = crate::main_screen::Pairing::Linking { code: "K7QN-M4XZ".into(), link: "https://t.me/bot?start=pair-K7QNM4XZ".into() };
    linking.qr = crate::first_run::qr_for("https://t.me/bot?start=pair-K7QNM4XZ");
    let mut prefs_witness = prefs_app.clone();
    prefs_witness.settings.tiles_app = vec!["witness".to_owned()];
    prefs_witness.settings.witness_keep = true;
    prefs_witness.witness = crate::witness::Seen {
        status: crate::witness::Status::Playing,
        state: Some(crate::witness::State { mode: "Play".into(), artist: "xi".into(), title: "FREEDOM DiVE".into(), version: "FOUR DIMENSIONS".into(), ..crate::witness::State::default() }),
        playing: Some(crate::witness::Progress { frames: 3886, score: 92_242, ..crate::witness::Progress::default() }),
        kept: 5,
        written: 3,
        told: 2,
        untold: false,
    };
    let mut prefs_shared = prefs_app.clone();
    prefs_shared.settings.tiles_app = vec!["sources".to_owned()];
    prefs_shared.settings.sources.push(Source { replay_count: 37, ..crate::sources::shared(true) });
    prefs_bot.pin = Some(crate::community::wire::Pin { chat: Some(-100), since: Some(NOON - 5 * DAY), free_at: Some(NOON + 25 * DAY), error: String::new() });
    let community = |section: crate::community_screen::Section, person: Option<usize>| {
        let mut main = staged(Some(0));
        main.now_unix = NOON;
        main.overlay = Overlay::Community;
        main.overlay_drawn = Overlay::Community;
        main.overlay_fade = iced::Animation::new(true);
        main.ground_fade = iced::Animation::new(true);
        main.community = Some(main.staged_community());
        main.news = sample_news();
        main.community_section = section;
        main.community_person = person;
        main.person_fade = iced::Animation::new(person.is_some());
        main.spot_held = Some(main.spot_due);
        main.rank_held = Some(main.rank_due);
        main
    };
    let scored = |scale: bool| {
        use crate::community::wire;
        let mut main = community(crate::community_screen::Section::Feed, None);
        let staged = main.community.as_ref().and_then(|catalog| {
            let play = catalog.live.iter().filter(|play| play.passed && catalog.people.get(play.who).is_some_and(|person| !person.you)).max_by(|a, b| a.pp.total_cmp(&b.pp))?;
            let mut scored = crate::community_screen::Scored::of(catalog, play.who, &play.more, true)?;
            scored.map.beatmap = Some(1);
            scored.map.stars = Some(5.84);
            let others = [(0usize, 412.6f32, 98.34f32, 1_204u32, "S", &["HD"][..], 34i64), (2, 611.0, 99.21, 1_480, "SS", &["HD", "HR"][..], 18), (3, 402.2, 97.1, 1_101, "A", &[][..], 29), (4, 356.9, 95.12, 902, "A", &["DT"][..], 55)];
            let mut rows: Vec<wire::BoardRow> = others
                .into_iter()
                .filter(|(who, ..)| *who != play.who)
                .enumerate()
                .filter_map(|(at, (who, pp, accuracy, combo, grade, mods, days))| {
                    let person = catalog.people.get(who)?;
                    Some(wire::BoardRow {
                        who: person.id,
                        name: person.name.clone(),
                        country: person.country.clone(),
                        you: person.you,
                        play: wire::Play { id: Some(100 + at as u64), pp, accuracy, combo: Some(combo), grade: grade.to_owned(), mods: mods.iter().map(|m| (*m).to_owned()).collect(), score: (pp * 2_000.0) as u64, at: Some(NOON - days * 86_400), ..wire::Play::default() },
                        ..wire::BoardRow::default()
                    })
                })
                .collect();
            rows.push(wire::BoardRow {
                who: scored.who,
                name: scored.name.clone(),
                country: catalog.people[play.who].country.clone(),
                play: wire::Play { id: play.more.id, pp: play.pp, accuracy: play.accuracy, combo: play.more.combo, grade: play.grade.clone(), mods: play.mods.clone(), score: play.more.score, at: Some(play.at), ..wire::Play::default() },
                ..wire::BoardRow::default()
            });
            rows.sort_by(|a, b| b.play.pp.total_cmp(&a.play.pp));
            for (at, row) in rows.iter_mut().enumerate() {
                row.place = at as u32 + 1;
            }
            let about = wire::MapAbout { bpm: Some(172.0), length: Some(192), max_combo: Some(1_480), status: "ranked".into(), ..wire::MapAbout::default() };
            Some((scored, wire::MapBoard { beatmap: 1, metric: "pp".into(), map: Some(about), plays: 23, players: 5, rows, ..wire::MapBoard::default() }))
        });
        if let Some((scored, board)) = staged {
            main.map_boards.insert(1, board);
            main.community_reading = Some(crate::community_screen::Reading::Score(scored));
            main.read_fade = iced::Animation::new(true);
            main.score_scale = scale;
        }
        main
    };
    let titled = |code: &str, picked: &str| {
        let mut main = community(crate::community_screen::Section::Titles, None);
        let who = main.community.as_ref().and_then(|catalog| catalog.people.iter().find(|person| person.name == picked).map(|person| person.id));
        if let Some(catalog) = main.community.as_mut() {
            for (at, person) in catalog.people.iter_mut().enumerate() {
                if person.titles.iter().any(|held| held == code) {
                    person.title_dates.insert(code.to_owned(), NOON - (20 + 31 * at as i64) * 86_400);
                }
            }
        }
        main.community_reading = Some(crate::community_screen::Reading::Title { code: code.to_owned(), who });
        main.read_fade = iced::Animation::new(true);
        main
    };
    let mut signing = staged(Some(0));
    signing.pairing = crate::main_screen::Pairing::Waiting { code: "K7QN-M4XZ".into(), link: "https://t.me/bot?start=pair-K7QNM4XZ".into(), osu: true };
    signing.qr = crate::first_run::qr_for("https://t.me/bot?start=pair-K7QNM4XZ");
    let mut idle = staged(Some(0));
    idle.resting = iced::Animation::new(true);
    let mut notifications = staged(Some(0));
    notifications.now_unix = NOON;
    for (mark, heading, detail, note, link) in [
        (crate::notices::Mark::Done, notifications.words.t("rendered-notice"), "NaumRedlo — Dj Grimoire — Astral Quantization [Nattu VN0TH3R]", "3:51  84,2 МБ", crate::notices::Link::OpenVideo(PathBuf::from("out.mp4"))),
        (crate::notices::Mark::Plain, notifications.words.t("whats-new"), &format!("Dossier {SHOWN_BUILD}"), "", crate::notices::Link::Page("https://example.com/changes".into())),
        (crate::notices::Mark::Bad, notifications.words.t("render-failed"), "A very long map name with several words that should wrap naturally inside the notification without covering its actions or closing button", "ffmpeg exited with code 1", crate::notices::Link::RenderAgain(library.entries[0].path.clone())),
    ] {
        notifications.announce(mark, heading, detail.into(), note.into(), String::new(), link);
    }
    for notice in &mut notifications.notices.notices { notice.at = NOON; }
    for toast in &mut notifications.toasts {
        toast.shown = iced::Animation::new(true);
        toast.born = notifications.now - std::time::Duration::from_secs(2);
    }
    vec![
        ("main-rest".to_owned(), staged(Some(0))),
        ("main-idle".to_owned(), idle),
        ("main-notifications".to_owned(), notifications),
        ("main-menu-guest".to_owned(), menu_guest),
        ("main-menu-account".to_owned(), menu_account),
        ("main-menu-feed".to_owned(), menu_feed),
        ("main-menu-stats".to_owned(), menu_stats),
        ("main-signing".to_owned(), signing),
        ("main-prefs".to_owned(), prefs_app),
        ("main-prefs-bot".to_owned(), prefs_bot),
        ("main-prefs-shared".to_owned(), prefs_shared),
        ("main-prefs-witness".to_owned(), prefs_witness),
        ("main-prefs-bot-osu".to_owned(), prefs_osu),
        ("main-linking".to_owned(), linking),
        ("main-failure".to_owned(), failing),
        ("main-videos".to_owned(), with_videos),
        ("main-videos-received".to_owned(), received),
        ("main-videos-received-open".to_owned(), received_open),
        ("main-videos-received-getting".to_owned(), received_getting),
        ("main-videos-share".to_owned(), share),
        ("main-player".to_owned(), playing),
        ("main-player-mini".to_owned(), playing_mini),
        ("main-player-wide".to_owned(), playing_wide),
        ("main-nomap".to_owned(), staged(Some(3))),
        ("main-worker".to_owned(), worker),
        ("main-community-feed".to_owned(), community(crate::community_screen::Section::Feed, None)),
        ("main-community-people".to_owned(), community(crate::community_screen::Section::People, None)),
        ("main-community-sidebar".to_owned(), {
            let mut main = community(crate::community_screen::Section::Feed, None);
            main.side_open = iced::Animation::new(true);
            main
        }),
        ("main-community-compare".to_owned(), {
            let mut main = community(crate::community_screen::Section::Compare, None);
            main.pool_players();
            let pick = |name: &str| main.compare_pool.iter().find(|person| person.name == name).map(|person| person.id);
            main.compare = main.compare_pool.iter().filter(|person| person.you).map(|person| person.id).chain(pick("kotofey")).chain(pick("Mirrorwave")).collect();
            main
        }),
        ("main-community-everyone".to_owned(), {
            let mut main = community(crate::community_screen::Section::People, None);
            main.settings.people_everyone = true;
            if let Some(catalog) = main.community.as_mut() {
                let outsiders: Vec<crate::community::Person> = [("Mirrorwave", "KZ", 11_215, 5_127), ("ssnowy", "BY", 8_402, 18_966)]
                    .into_iter()
                    .enumerate()
                    .map(|(at, (name, country, pp, rank))| crate::community::Person {
                        id: 900 + at as i64,
                        player: Some(90 + at as i64),
                        name: name.to_owned(),
                        country: country.to_owned(),
                        pp,
                        rank,
                        accuracy: 97.4,
                        plays: 31_000,
                        hours: 1_100,
                        outside: true,
                        ..crate::community::Person::default()
                    })
                    .collect();
                catalog.people.retain(|person| person.name != "Mirrorwave" && person.name != "ssnowy");
                catalog.people.extend(outsiders);
            }
            main
        }),
        ("main-community-person".to_owned(), community(crate::community_screen::Section::People, Some(1))),
        ("main-community-boards".to_owned(), community(crate::community_screen::Section::Boards, None)),
        ("main-community-adaptive".to_owned(), {
            let mut main = community(crate::community_screen::Section::Boards, None);
            main.community_standing = crate::community_screen::Standing::Adaptive;
            main
        }),
        ("main-community-titles".to_owned(), community(crate::community_screen::Section::Titles, None)),
        ("main-community-profile".to_owned(), community(crate::community_screen::Section::Profile, None)),
        ("main-community-reading".to_owned(), {
            let mut main = community(crate::community_screen::Section::Feed, None);
            main.community_reading = main.news.stories.first().cloned().map(crate::community_screen::Reading::Story);
            main.read_fade = iced::Animation::new(true);
            main
        }),
        ("main-community-score".to_owned(), scored(false)),
        ("main-community-score-scale".to_owned(), scored(true)),
        ("main-community-score-witnessed".to_owned(), {
            let mut main = scored(false);
            if let Some(crate::community_screen::Reading::Score(scored)) = main.community_reading.as_mut() {
                scored.play = crate::community::Play { pp: 0.0, pp_if: Some(scored.play.pp), id: None, witnessed: true, ..scored.play.clone() };
            }
            main
        }),
        ("main-community-title".to_owned(), titled("ss_100", "ssnowy")),
        ("main-community-title-earned".to_owned(), titled("combo_2000", "Mirrorwave")),
        ("main-community-clip".to_owned(), {
            let mut main = community(crate::community_screen::Section::Feed, None);
            main.clip = Some(crate::main_screen::Clip {
                path: std::path::PathBuf::from("/clips/osunewsru.mp4"),
                link: "https://t.me/osunewsru/4242".into(),
                from: "осу!новостник".into(),
                said: "kotofey поставил первое HDDT FC на карте из пула мирового кубка".into(),
                thumb: None,
            });
            main.player = Some(std::rc::Rc::new(std::cell::RefCell::new(crate::player::Player::still(std::path::Path::new("/clips/osunewsru.mp4"), 42_000, 12_000))));
            main.cinema = iced::Animation::new(true);
            main.stage_open = iced::Animation::new(true);
            main
        }),
        ("main-community-clip-mini".to_owned(), {
            let mut main = community(crate::community_screen::Section::Feed, None);
            main.clip = Some(crate::main_screen::Clip {
                path: std::path::PathBuf::from("/clips/osunewsru.mp4"),
                link: "https://t.me/osunewsru/4242".into(),
                from: "осу!новостник".into(),
                said: "kotofey поставил первое HDDT FC на карте из пула мирового кубка".into(),
                thumb: None,
            });
            main.player = Some(std::rc::Rc::new(std::cell::RefCell::new(crate::player::Player::still(std::path::Path::new("/clips/osunewsru.mp4"), 42_000, 12_000))));
            main.mini_player = true;
            main
        }),
        ("main-rendering".to_owned(), rendering),
        ("main-queued".to_owned(), queued),
        ("main-rendered".to_owned(), rendered),
        ("main-hover".to_owned(), hovering),
        ("main-fetching".to_owned(), {
            for job in &mut fetching.fetching {
                job.shown = job.target().unwrap_or(0.0);
            }
            fetching
        }),
        ("main-empty".to_owned(), empty),
        ("main-looking".to_owned(), looking),
    ]
}

fn sample_news() -> crate::news::News {
    use crate::news::{Block, Build, Change, News, Post, Story};
    let change = |title: &str, major: bool| Change { category: "Gameplay".into(), title: title.into(), major, kind: "fix".into() };
    let mut news = News {
        builds: vec![
            Build { stream: "Lazer".into(), version: "2026.921.0".into(), at: NOON - 30 * 3600, url: String::new(), changes: vec![change("Fix startup crashes for some windows users", true)] },
            Build {
                stream: "Lazer".into(),
                version: "2026.920.0".into(),
                at: NOON - 52 * 3600,
                url: String::new(),
                changes: vec![change("Add legacy style hit error bar", true), change("Use colour hit error meter by default in osu!catch HUD", false), change("Fix beatmap carousel flicker", false)],
            },
            Build { stream: "Tachyon".into(), version: "2026.918.0".into(), at: NOON - 100 * 3600, url: String::new(), changes: vec![change("Improve performance when loading chat", false)] },
        ],
        stories: vec![
            Story {
                title: "osu!mania 4K World Cup 2026: Semifinals Recap".into(),
                url: "https://osu.ppy.sh/home/news/2026-09-22-mwc4k-semifinals".into(),
                at: NOON - 11 * 3600,
                lead: "A recap of the second half of the tournament.".into(),
                image: None,
                body: crate::news::blocks_of(
                    "<p>Four teams came into the last weekend before the finals, and two of them leave with a place on the <strong>final stage</strong>.</p>\
                     <h2>Winners bracket</h2>\
                     <p>The first semifinal went the full distance: the tiebreaker was decided by a single miss in its closing stream. Every match is on the <a href=\"/wiki/en/Tournaments/MWC\">tournament's wiki page</a>.</p>\
                     <ul><li>South Korea 7 : 6 China</li><li>United States 7 : 3 Indonesia</li></ul>\
                     <blockquote><p>We practised the tiebreaker more than any other map in the pool, and it still nearly got away from us.</p></blockquote>\
                     <h2>Looking ahead</h2><p>The grand finals are played next weekend, with the mappool shown on Thursday.</p>",
                    "https://osu.ppy.sh/home/news/2026-09-22-mwc4k-semifinals",
                    crate::news::Flow::Article,
                ),
            },
            Story { title: "New Featured Artist: Exsy".into(), url: String::new(), at: NOON - 70 * 3600, lead: "A new artist joins the Featured Artist library.".into(), image: None, body: vec![Block::Text(vec![crate::news::Span::plain("A new artist joins the Featured Artist library.")])] },
            Story { title: "Project Loved: September 2026".into(), url: String::new(), at: NOON - 96 * 3600, lead: "This month's picks for Project Loved.".into(), image: None, body: vec![Block::Text(vec![crate::news::Span::plain("This month's picks for Project Loved.")])] },
        ],
        threads: Vec::new(),
        posts: vec![{
            let markup = "<a href=\"https://osu.ppy.sh/users/2\"><b>kotofey</b></a> поставил <b>первое HDDT FC</b> на карте из пула мирового кубка<br/>сыграв в 99.21% аккураси<br/><br/><a href=\"?q=%23скор\"><b>#скор</b></a>";
            Post {
                channel: "osunewsru".into(),
                name: "осу!новостник".into(),
                text: crate::news::plain(markup),
                url: "https://t.me/osunewsru/1".into(),
                at: NOON - 3 * 3600,
                image: None,
                body: crate::news::blocks_of(markup, "https://t.me/s/osunewsru", crate::news::Flow::Post),
                images: Vec::new(),
                videos: vec![crate::news::Video { thumb: None, src: Some("https://cdn4.telesco.pe/file/clip.mp4".into()), duration: "0:42".into(), link: "https://t.me/osunewsru/1".into() }],
            }
        }],
        fetched: std::collections::HashMap::new(),
    };
    for source in [crate::news::UPDATES, crate::news::STORIES] {
        news.mark(source, NOON - 600);
    }
    news.mark(&crate::news::channel_source("osunewsru"), NOON - 600);
    news
}

pub fn every_main_frame() -> Vec<(String, crate::main_screen::Main, Size)> {
    let mut out = Vec::new();
    for lang in Lang::ALL {
        for (name, main) in main_states(lang) {
            for (label, size) in SIZES {
                if label != SIZES[0].0 && name != "main-rest" && name != "main-idle" && name != "main-notifications" && name != "main-player-mini" && name != "main-community-clip-mini" {
                    continue;
                }
                let mut frame = main.clone();
                if name == "main-idle" || name == "main-notifications" || name == "main-player-mini" || name == "main-community-clip-mini" { frame.width = size.width; frame.height = size.height; }
                out.push((format!("{name}-{}-{label}", lang.tag()), frame, size));
            }
        }
    }
    out
}

pub fn write_main(dir: &Path) -> Result<usize, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut written = 0;
    for (name, main, size) in every_main_frame() {
        let stem = dir.join(&name);
        let shot = snapshot_main(&main, size).map_err(|e| format!("{e:?}"))?;
        write_snapshot(&shot, &stem)?;
        written += 1;
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::mouse;
    use iced::window;
    use std::time::{Duration, Instant};

    type Screen<'a> = Simulator<'a, Message>;

    fn still(ui: &mut Screen<'_>, dir: &Path, name: &str) {
        let stem = dir.join(name);
        let _ = std::fs::remove_file(written_as(&stem));
        let shot = ui.snapshot(&crate::theme::theme()).expect("a snapshot");
        let _ = shot.matches_image(&stem);
    }

    #[test]
    #[ignore]
    fn a_wheel_notch_glides_the_journal() {
        use crate::main_screen::Main;
        let Some(root) = std::env::var_os("DOSSIER_CORPUS").map(PathBuf::from) else {
            return;
        };
        let source = crate::sources::folder_at(&root).expect("replays");
        let mut settings = crate::settings::Settings::default();
        settings.sources = vec![source.clone()];
        let library = crate::library::read(&[source]);
        let main = Main::staged(crate::lang::Words::new(Lang::Ru), settings, library, Some(0));
        let backdrop = ui::backdrop_handle();
        let size = Size::new(980.0, 720.0);
        let mut screen = Simulator::with_size(settings_once(), size, main_frame(&main, &backdrop));
        screen.point_at(iced::Point::new(490.0, 690.0));
        let mut at = Instant::now() + Duration::from_secs(3600);
        let _ = screen.simulate([iced::Event::Mouse(mouse::Event::WheelScrolled { delta: mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 } })]);
        for _ in 0..40 {
            at += Duration::from_micros(8_333);
            let _ = screen.simulate([iced::Event::Window(window::Event::RedrawRequested(at))]);
        }
        let offsets: Vec<f32> = screen
            .into_messages()
            .filter_map(|message| match message {
                Message::Main(crate::main_screen::Message::Strip(viewport)) => Some(viewport.absolute_offset().x),
                _ => None,
            })
            .collect();
        assert!(offsets.len() > 10, "the journal jumped instead of gliding: {offsets:?}");
        assert!(offsets.windows(2).all(|pair| pair[1] >= pair[0]), "the journal went back and forth");
    }

    #[test]
    #[ignore]
    fn a_tile_carried_to_the_edge_scrolls_the_page_in_frames() {
        let Some(dir) = std::env::var_os("DOSSIER_DRAG_FRAMES").map(PathBuf::from) else {
            return;
        };
        std::fs::create_dir_all(&dir).expect("a folder");
        let (_, main, size) = every_main_frame().into_iter().find(|(name, _, _)| name.starts_with("main-prefs-en")).expect("the settings are staged");
        let backdrop = ui::backdrop_handle();
        let mut screen = Simulator::with_size(settings_once(), size, main_frame(&main, &backdrop));
        let mut at = Instant::now() + Duration::from_secs(3600);
        let tick = Duration::from_micros(16_667);
        let redraw = |screen: &mut Screen<'_>, at: &mut Instant| {
            *at += tick;
            let _ = screen.simulate([iced::Event::Window(window::Event::RedrawRequested(*at))]);
        };
        screen.point_at(iced::Point::new(490.0, 500.0));
        let _ = screen.simulate([iced::Event::Mouse(mouse::Event::WheelScrolled { delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -400.0 } })]);
        redraw(&mut screen, &mut at);
        still(&mut screen, &dir, "edge-000-scrolled");
        let tile = screen.find("Maps and cache").expect("the maps tile").visible_bounds().expect("the maps tile is on screen");
        let grip = iced::Point::new(tile.x + tile.width + 30.0, tile.center_y());
        screen.point_at(grip);
        let _ = screen.simulate([iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))]);
        let top = iced::Point::new(200.0, 150.0);
        for step in 1..=30 {
            let k = step as f32 / 30.0;
            let p = grip + (top - grip) * (k * k * (3.0 - 2.0 * k));
            screen.point_at(p);
            let _ = screen.simulate([iced::Event::Mouse(mouse::Event::CursorMoved { position: p })]);
            redraw(&mut screen, &mut at);
        }
        still(&mut screen, &dir, "edge-030-at-the-top");
        for step in 1..=90 {
            redraw(&mut screen, &mut at);
            if step % 30 == 0 {
                still(&mut screen, &dir, &format!("edge-{:03}-scrolling", 30 + step));
            }
        }
        let render = iced::Point::new(120.0, 230.0);
        for step in 1..=12 {
            let p = top + (render - top) * (step as f32 / 12.0);
            screen.point_at(p);
            let _ = screen.simulate([iced::Event::Mouse(mouse::Event::CursorMoved { position: p })]);
            redraw(&mut screen, &mut at);
        }
        for _ in 0..12 {
            redraw(&mut screen, &mut at);
        }
        still(&mut screen, &dir, "edge-150-over-render");
        let _ = screen.simulate([iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))]);
        for _ in 0..50 {
            redraw(&mut screen, &mut at);
        }
        still(&mut screen, &dir, "edge-200-landed");
        let moved: Vec<_> = screen
            .into_messages()
            .filter_map(|message| match message {
                Message::Main(crate::main_screen::Message::Prefs(crate::settings_screen::Message::Moved(what, before))) => Some((what, before)),
                _ => None,
            })
            .collect();
        assert_eq!(moved.first().map(|(what, _)| *what), Some(crate::settings_screen::Tile::Maps));
    }

    #[test]
    #[ignore]
    fn free_space_shows_the_places_in_frames() {
        let Some(dir) = std::env::var_os("DOSSIER_DRAG_FRAMES").map(PathBuf::from) else {
            return;
        };
        std::fs::create_dir_all(&dir).expect("a folder");
        let (_, main, size) = every_main_frame().into_iter().find(|(name, _, _)| name.starts_with("main-prefs-en")).expect("the settings are staged");
        let backdrop = ui::backdrop_handle();
        let mut screen = Simulator::with_size(settings_once(), size, main_frame(&main, &backdrop));
        let language = screen.find("Language").expect("the language tile").bounds();
        let empty = iced::Point::new(language.x + 380.0, language.y + 40.0);
        let mut at = Instant::now() + Duration::from_secs(3600);
        let tick = Duration::from_micros(16_667);
        screen.point_at(empty);
        let _ = screen.simulate([iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))]);
        for step in 1..=24 {
            at += tick;
            let _ = screen.simulate([iced::Event::Window(window::Event::RedrawRequested(at))]);
            if step % 8 == 0 {
                still(&mut screen, &dir, &format!("free-{step:03}-pressed"));
            }
        }
        let _ = screen.simulate([iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))]);
        for step in 1..=90 {
            at += tick;
            let _ = screen.simulate([iced::Event::Window(window::Event::RedrawRequested(at))]);
            if step % 15 == 0 {
                still(&mut screen, &dir, &format!("free-{:03}-released", 24 + step));
            }
        }
    }

    #[test]
    #[ignore]
    fn a_dragged_tile_in_frames() {
        let Some(dir) = std::env::var_os("DOSSIER_DRAG_FRAMES").map(PathBuf::from) else {
            return;
        };
        std::fs::create_dir_all(&dir).expect("a folder");
        let (_, main, size) = every_main_frame().into_iter().find(|(name, _, _)| name.starts_with("main-prefs-en")).expect("the settings are staged");
        let backdrop = ui::backdrop_handle();
        let mut screen = Simulator::with_size(settings_once(), size, main_frame(&main, &backdrop));
        let heading = screen.find("Gameplay").expect("the gameplay tile").bounds();
        let grip = iced::Point::new(heading.x + heading.width + 30.0, heading.center_y());
        let reach = iced::Vector::new(std::env::var("DOSSIER_DRAG_X").ok().and_then(|v| v.parse().ok()).unwrap_or(-518.0), std::env::var("DOSSIER_DRAG_Y").ok().and_then(|v| v.parse().ok()).unwrap_or(-124.0));
        let mut at = Instant::now() + Duration::from_secs(3600);
        let tick = Duration::from_micros(16_667);
        let redraw = |screen: &mut Screen<'_>, at: &mut Instant| {
            *at += tick;
            let _ = screen.simulate([iced::Event::Window(window::Event::RedrawRequested(*at))]);
        };
        still(&mut screen, &dir, "drag-000-rest");
        screen.point_at(grip);
        let _ = screen.simulate([iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))]);
        let steps = 48;
        for step in 1..=steps {
            let k = step as f32 / steps as f32;
            let eased = k * k * (3.0 - 2.0 * k);
            let p = grip + reach * eased;
            screen.point_at(p);
            let _ = screen.simulate([iced::Event::Mouse(mouse::Event::CursorMoved { position: p })]);
            redraw(&mut screen, &mut at);
            if step % 6 == 0 || step == 2 {
                still(&mut screen, &dir, &format!("drag-{step:03}-carry"));
            }
        }
        for step in 1..=30 {
            redraw(&mut screen, &mut at);
            if step % 10 == 0 {
                still(&mut screen, &dir, &format!("drag-{:03}-held", steps + step));
            }
        }
        let _ = screen.simulate([iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))]);
        for step in 1..=40 {
            redraw(&mut screen, &mut at);
            if step % 5 == 0 {
                still(&mut screen, &dir, &format!("drag-{:03}-land", steps + 30 + step));
            }
        }
        let moved = screen
            .into_messages()
            .any(|message| matches!(message, Message::Main(crate::main_screen::Message::Prefs(crate::settings_screen::Message::Moved(..)))));
        assert!(moved, "dropping a tile elsewhere did not move it");
    }
}
