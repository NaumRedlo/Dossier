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
        (Check::Engine, Some(Outcome::Passed("0.12.0".into()))),
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
                    (Check::Engine, Some(Outcome::Passed("0.12.0".into()))),
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
                    (Check::Engine, Some(Outcome::Passed("0.12.0".into()))),
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
                    (Check::Engine, Some(Outcome::Passed("0.12.0".into()))),
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
    let mut first = false;
    FONTS_LOADED.call_once(|| first = true);
    if !first {
        settings.fonts.clear();
    }
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
        let _ = std::fs::remove_file(written_as(&stem));
        let _ = std::fs::remove_file(written_as(&stem));
        let shot = snapshot(&flow, size).map_err(|e| format!("{e:?}"))?;
        shot.matches_image(&stem).map_err(|e| format!("{e:?}"))?;
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

pub fn main_states(lang: Lang) -> Vec<(String, crate::main_screen::Main)> {
    use crate::main_screen::{decoded, Main, Overlay};
    let mut settings = crate::settings::Settings::default();
    settings.lang = lang;
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
    worker.overlay = Overlay::Worker;
    worker.overlay_drawn = Overlay::Worker;
    worker.overlay_fade = iced::Animation::new(true);
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
    let mut fetching = staged(Some(3));
    fetching.fetching = Some(crate::main_screen::Fetching {
        hash: library.entries[3].map_hash.clone(),
        reached: vec![
            crate::maps::Step::Looking,
            crate::maps::Step::Found(crate::maps::Found { from: "osu.direct", set: 2190769, artist: "Chocofan".into(), title: "LUCKY CAT".into() }),
            crate::maps::Step::Downloading { from: "osu.direct", done: 14_890_000, total: Some(22_860_000) },
        ],
    });
    let mut hovering = staged(Some(0));
    hovering.hover = Some(1);
    hovering.hover_bounds = Some(iced::Rectangle::new(iced::Point::new(40.0 + 116.0 + 22.0, 720.0 - 10.0 - 61.0), iced::Size::new(108.0, 61.0)));
    hovering.lifts.insert(1, iced::Animation::new(true));
    hovering.combos.insert(library.entries[1].path.clone(), Some(1224));
    let mut rendered = staged(Some(0));
    rendered.ffmpeg = Some(PathBuf::from("ffmpeg"));
    rendered.rendering = Some(crate::main_screen::Rendering {
        path: library.entries[0].path.clone(),
        reached: vec![crate::render::Step::Encoded, crate::render::Step::Saved(PathBuf::from("out.mp4"))],
        out: Some(PathBuf::from("out.mp4")),
    });
    rendered.announce(
        crate::notices::Mark::Done,
        rendered.words.t("rendered-notice"),
        "NaumRedlo — Dj Grimoire — Astral Quantization [Nattu VN0TH3R]".to_owned(),
        "3:51 · 84,2 МБ".to_owned(),
        library.entries[0].map_hash.clone(),
        crate::notices::Link::OpenVideo(PathBuf::from("out.mp4")),
    );
    for toast in &mut rendered.toasts {
        toast.shown = iced::Animation::new(true);
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
            fps: 60,
            size,
            made_at,
            sent_at: None,
            background: None,
        }
    };
    with_videos.store.videos = vec![
        mock_video(0, NOON - 1800, 231_000, 84_200_000, &["HD", "DT", "HR"]),
        mock_video(1, NOON - 38 * 3600, 134_000, 51_000_000, &["EZ"]),
        mock_video(2, NOON - 39 * 3600, 242_000, 97_700_000, &[]),
    ];
    let mut playing = with_videos.clone();
    playing.open_video = Some(0);
    playing.player = Some(std::rc::Rc::new(std::cell::RefCell::new(crate::player::Player::still(
        std::path::Path::new("/renders/NaumRedlo.mp4"),
        231_000,
        67_000,
    ))));
    let mut menu_guest = staged(Some(0));
    menu_guest.menu_open = iced::Animation::new(true);
    menu_guest.menu = Some(crate::main_screen::Tab::Account);
    let mut menu_feed = staged(Some(0));
    menu_feed.now_unix = NOON;
    menu_feed.settings.token = "staged".to_owned();
    menu_feed.settings.linked_as = "Naum Redlo".to_owned();
    menu_feed.account = Some(crate::bot::Me { telegram_id: 7, name: "Naum Redlo".into(), username: "naumredlo".into(), avatar: false });
    menu_feed.menu_open = iced::Animation::new(true);
    menu_feed.menu = Some(crate::main_screen::Tab::Feed);
    menu_feed.rendering = Some(crate::main_screen::Rendering {
        path: library.entries[0].path.clone(),
        reached: vec![crate::render::Step::Judged, crate::render::Step::Drawing { frames: 8_400, of: 13_860, left_seconds: 18.0 }],
        out: None,
    });
    menu_feed.progress_shown = menu_feed.progress_target().unwrap_or(0.0);
    for (mark, words, detail, note, at, link) in [
        (crate::notices::Mark::Bad, "Рендер не завершился", "Guest — xi — Blue Zenith", "ffmpeg завершился с кодом 1", 1, crate::notices::Link::RenderAgain(library.entries[1].path.clone())),
        (crate::notices::Mark::Done, "Карта скачана", "xi — Blue Zenith", "[FOUR DIMENSIONS]", 1, crate::notices::Link::None),
        (crate::notices::Mark::Done, "Ушло в Telegram", "-legusshhka- — xi — FREEDOM DiVE [Extra]", "@naumredlo · 97,7 МБ", 2, crate::notices::Link::None),
        (crate::notices::Mark::Plain, "Открыто", "сборка 0.12.0", "", 0, crate::notices::Link::None),
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
    let mut signing = staged(Some(0));
    signing.pairing = crate::main_screen::Pairing::Waiting { code: "K7QN-M4XZ".into(), link: "https://t.me/bot?start=pair-K7QNM4XZ".into() };
    signing.qr = crate::first_run::qr_for("https://t.me/bot?start=pair-K7QNM4XZ");
    vec![
        ("main-rest".to_owned(), staged(Some(0))),
        ("main-menu-guest".to_owned(), menu_guest),
        ("main-menu-account".to_owned(), menu_account),
        ("main-menu-feed".to_owned(), menu_feed),
        ("main-menu-stats".to_owned(), menu_stats),
        ("main-signing".to_owned(), signing),
        ("main-failure".to_owned(), failing),
        ("main-videos".to_owned(), with_videos),
        ("main-player".to_owned(), playing),
        ("main-nomap".to_owned(), staged(Some(3))),
        ("main-worker".to_owned(), worker),
        ("main-rendering".to_owned(), rendering),
        ("main-rendered".to_owned(), rendered),
        ("main-hover".to_owned(), hovering),
        ("main-fetching".to_owned(), {
            fetching.progress_shown = fetching.progress_target().unwrap_or(0.0);
            fetching
        }),
        ("main-empty".to_owned(), empty),
        ("main-looking".to_owned(), looking),
    ]
}

pub fn every_main_frame() -> Vec<(String, crate::main_screen::Main, Size)> {
    let mut out = Vec::new();
    for lang in Lang::ALL {
        for (name, main) in main_states(lang) {
            for (label, size) in SIZES {
                if label != SIZES[0].0 && name != "main-rest" {
                    continue;
                }
                out.push((format!("{name}-{}-{label}", lang.tag()), main.clone(), size));
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
        let _ = std::fs::remove_file(written_as(&stem));
        let shot = snapshot_main(&main, size).map_err(|e| format!("{e:?}"))?;
        shot.matches_image(&stem).map_err(|e| format!("{e:?}"))?;
        written += 1;
    }
    Ok(written)
}
