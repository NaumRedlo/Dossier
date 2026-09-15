use std::path::{Path, PathBuf};

use iced::widget::stack;
use iced::{Element, Length, Size};
use iced_test::Simulator;

use crate::checks::Outcome;
use crate::first_run::{Check, FirstRun, Pairing, Step, CHECKS};
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

pub fn states(lang: Lang) -> Vec<(String, FirstRun)> {
    let waiting = Pairing::Waiting {
        code: "K7QN-M4XZ".into(),
        link: "https://t.me/onenineeightfour_bot?start=pair-K7QNM4XZ".into(),
    };
    let none: Vec<(Check, Option<Outcome>)> = Vec::new();
    let mut out = vec![
        ("language", FirstRun::staged(Step::Language, lang, vec![], false, Pairing::Idle, none.clone())),
        ("folder-stable", FirstRun::staged(Step::Folder, lang, vec![stable()], false, Pairing::Idle, none.clone())),
        ("folder-lazer", FirstRun::staged(Step::Folder, lang, vec![lazer()], false, Pairing::Idle, none.clone())),
        ("folder-both", FirstRun::staged(Step::Folder, lang, vec![stable(), lazer()], false, Pairing::Idle, none.clone())),
        ("folder-missing", FirstRun::staged(Step::Folder, lang, vec![], false, Pairing::Idle, none.clone())),
        ("device", FirstRun::staged(Step::Device, lang, vec![stable()], false, Pairing::Idle, none.clone())),
        ("bot-waiting", FirstRun::staged(Step::Bot, lang, vec![stable()], false, waiting.clone(), none.clone())),
        ("bot-linked", FirstRun::staged(Step::Bot, lang, vec![stable()], false, Pairing::Linked { who: "naumredlo".into() }, none.clone())),
        ("bot-code", FirstRun::staged(Step::Bot, lang, vec![stable()], false, Pairing::Manual { code: "K7QN-M4".into(), busy: false, wrong: false }, none.clone())),
        ("bot-only", FirstRun::staged(Step::Bot, lang, vec![], true, waiting, none.clone())),
        (
            "checks-running",
            FirstRun::staged(
                Step::Checks,
                lang,
                vec![stable()],
                false,
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
                false,
                Pairing::Idle,
                checked(&[
                    (Check::Folder, Some(Outcome::Passed("1,342 maps".into()))),
                    (Check::Ffmpeg, Some(Outcome::Failed(String::new()))),
                    (Check::Engine, Some(Outcome::Passed("0.12.0".into()))),
                    (Check::Bot, Some(Outcome::Passed("41 ms".into()))),
                ]),
            ),
        ),
        (
            "checks-bot-only-ffmpeg-missing",
            FirstRun::staged(
                Step::Checks,
                lang,
                vec![],
                true,
                Pairing::Idle,
                checked(&[
                    (Check::Folder, Some(Outcome::Passed("~/.dossier/Songs".into()))),
                    (Check::Ffmpeg, Some(Outcome::Failed(String::new()))),
                    (Check::Engine, Some(Outcome::Passed("0.12.0".into()))),
                    (Check::Bot, Some(Outcome::Passed("41 ms".into()))),
                ]),
            ),
        ),
        ("done", FirstRun::staged(Step::Checks, lang, vec![stable()], false, Pairing::Idle, all_passed())),
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
