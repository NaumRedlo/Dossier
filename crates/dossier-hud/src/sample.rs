use dossier_overlay::{Client, Gameplay, Screen, Snapshot, View};

use crate::{Best, Card, Context, Day, Lang, Meter, Notice, Outcome, Place, Play, Sent, Tone};

fn view(screen: Screen) -> View {
    View {
        client: Client::Stable,
        pid: 1,
        snapshot: Snapshot {
            screen,
            beatmap: None,
            gameplay: (screen == Screen::Playing).then(|| Gameplay {
                ruleset: 0,
                time_ms: 61_000,
                score: 482_310,
                combo: 512,
                max_combo: 512,
                accuracy: Some(98.61),
                misses: 0,
                legacy_mods: Some(8),
                mods: Vec::new(),
                unstable_rate: None,
                resting: None,
                pp: None,
                pp_clean: None,
            }),
            watching_replay: Some(false),
            meter: None,
        },
    }
}

fn play(resting: bool) -> Play {
    Play {
        pp: Some(312.0),
        record: Some(398.0),
        clean: Some(421.0),
        resting,
        unstable_rate: Some(84.2),
        meter: Some(Meter { scale: 1.0 }),
    }
}

fn sent() -> Sent {
    Sent {
        chat: "osu! друзья".into(),
        title: "Towa no Maigo".into(),
        player: "NaumRedlo".into(),
        mods: "HD".into(),
        pp: 412.0,
        accuracy: 98.61,
        grade: "A".into(),
    }
}

pub fn frames(lang: Lang) -> Vec<(&'static str, View, Context)> {
    let base = Context {
        lang,
        ..Context::default()
    };
    vec![
        (
            "menu",
            view(Screen::Menu),
            Context {
                day: Some(Day {
                    pp_gain: 38.0,
                    rank: 11_188,
                    climbed: 214,
                    best: Some(Best {
                        title: "Towa no Maigo".into(),
                        pp: 412.0,
                        accuracy: 98.61,
                        mods: "HD".into(),
                        stars: 6.88,
                    }),
                    plays: 14,
                    seconds: 4_320,
                    records: 2,
                }),
                notices: vec![
                    Notice {
                        title: "Видео готово".into(),
                        body: "Sendan Life, 3 мин 51 с".into(),
                        sign: "✓".into(),
                        tone: Tone::Good,
                    },
                    Notice {
                        title: "kotofey сыграл вашу карту".into(),
                        body: "Glass Orchard, 98,61%".into(),
                        sign: "K".into(),
                        tone: Tone::Blue,
                    },
                ],
                ..base.clone()
            },
        ),
        (
            "select",
            view(Screen::Selection),
            Context {
                card: Some(Card {
                    pool: Some("Winter BlyatCup 2026".into()),
                    stars: Some(6.25),
                    places: vec![
                        Place {
                            place: 1,
                            name: "kotofey".into(),
                            accuracy: 98.61,
                            you: false,
                        },
                        Place {
                            place: 2,
                            name: "ssnowy".into(),
                            accuracy: 98.10,
                            you: false,
                        },
                        Place {
                            place: 3,
                            name: "NaumRedlo".into(),
                            accuracy: 97.84,
                            you: true,
                        },
                    ],
                }),
                ..base.clone()
            },
        ),
        (
            "play",
            view(Screen::Playing),
            Context {
                play: Some(play(false)),
                ..base.clone()
            },
        ),
        (
            "rest",
            view(Screen::Playing),
            Context {
                play: Some(play(true)),
                ..base.clone()
            },
        ),
        (
            "results",
            view(Screen::Results),
            Context {
                outcome: Some(Outcome {
                    saved: true,
                    offer: Some("Tab".into()),
                    holding: 0.0,
                    sent: None,
                }),
                ..base.clone()
            },
        ),
        (
            "hold",
            view(Screen::Results),
            Context {
                outcome: Some(Outcome {
                    saved: true,
                    offer: Some("Tab".into()),
                    holding: 0.6,
                    sent: None,
                }),
                ..base.clone()
            },
        ),
        (
            "sent",
            view(Screen::Results),
            Context {
                outcome: Some(Outcome {
                    saved: true,
                    offer: Some("Tab".into()),
                    holding: 0.0,
                    sent: Some(sent()),
                }),
                ..base
            },
        ),
    ]
}

pub fn backdrop(width: u32, height: u32) -> Vec<u8> {
    let blobs: [(f32, f32, f32, [f32; 3]); 3] = [
        (0.28, 0.38, 0.42, [72.0, 120.0, 214.0]),
        (0.72, 0.44, 0.40, [214.0, 92.0, 150.0]),
        (0.5, 0.9, 0.5, [40.0, 170.0, 160.0]),
    ];
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let (u, v) = (x as f32 / width as f32, y as f32 / height as f32);
            let mut colour = [11.0f32, 15.0, 26.0];
            for (cx, cy, reach, tint) in blobs {
                let far = (((u - cx) * 16.0 / 9.0).powi(2) + (v - cy).powi(2)).sqrt() / reach;
                let share = (1.0 - far).clamp(0.0, 1.0).powi(2) * 0.8;
                for channel in 0..3 {
                    colour[channel] += (tint[channel] - colour[channel]) * share;
                }
            }
            for channel in colour {
                pixels.push((channel * 0.5).round().clamp(0.0, 255.0) as u8);
            }
            pixels.push(255);
        }
    }
    pixels
}

pub fn png(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("a PNG header is written");
        writer
            .write_image_data(pixels)
            .expect("PNG pixels are written");
    }
    bytes
}
