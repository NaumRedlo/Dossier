use crate::draw::{fitted, mix, rgb, Face, Rgba};
use crate::node::{col, row, text, Cross, Node, Shell};
use crate::{Best, Card, Day, Lang, Notice, Place, Play, Sent, Tone};

const INK: Rgba = rgb(0xece7e2, 1.0);
const MUTED: Rgba = rgb(0xc9aaa6, 1.0);
const ACCENT: Rgba = rgb(0xe24848, 1.0);
const GOOD: Rgba = rgb(0xa2b485, 1.0);
const BLUE: Rgba = rgb(0x6ea0f0, 1.0);
const WHITE: Rgba = rgb(0xffffff, 1.0);
const GLASS: Rgba = rgb(0x0d0508, 0.78);
const GLASS_EDGE: Rgba = rgb(0xffffff, 0.10);

fn said(lang: Lang, key: &str) -> &'static str {
    match (lang, key) {
        (Lang::Ru, "connected") => "Witness на связи",
        (Lang::En, "connected") => "Witness is connected",
        (Lang::Ru, "recording") => "Ведётся запись",
        (Lang::En, "recording") => "Recording",
        (Lang::Ru, "saved") => "Игра сохранена",
        (Lang::En, "saved") => "Play saved",
        (Lang::Ru, "day") => "РЕЗУЛЬТАТЫ ЗА ДЕНЬ",
        (Lang::En, "day") => "TODAY'S RESULTS",
        (Lang::Ru, "gain") => "прирост",
        (Lang::En, "gain") => "gained",
        (Lang::Ru, "world") => "в мире",
        (Lang::En, "world") => "in the world",
        (Lang::Ru, "higher") => "в мире, выше на",
        (Lang::En, "higher") => "in the world, up by",
        (Lang::Ru, "lower") => "в мире, ниже на",
        (Lang::En, "lower") => "in the world, down by",
        (Lang::Ru, "best") => "ЛУЧШАЯ ИГРА",
        (Lang::En, "best") => "BEST PLAY",
        (Lang::Ru, "in-game") => "в игре",
        (Lang::En, "in-game") => "in game",
        (Lang::Ru, "chat") => "В БЕСЕДЕ",
        (Lang::En, "chat") => "IN THE CHAT",
        (Lang::Ru, "record") => "ваш рекорд",
        (Lang::En, "record") => "your record",
        (Lang::Ru, "clean") => "без ошибок",
        (Lang::En, "clean") => "without misses",
        (Lang::Ru, "offer") => "Отправить результат в беседу",
        (Lang::En, "offer") => "Send the result to the chat",
        (Lang::Ru, "offer-about") => "бот пришлёт готовую карточку игры",
        (Lang::En, "offer-about") => "the bot will post a ready card of the play",
        (Lang::Ru, "sent") => "Отправлено в беседу",
        (Lang::En, "sent") => "Sent to the chat",
        (Lang::Ru, "hour") => "ч",
        (Lang::En, "hour") => "h",
        (Lang::Ru, "minute") => "мин",
        (Lang::En, "minute") => "min",
        (Lang::Ru, "second") => "с",
        (Lang::En, "second") => "s",
        _ => "",
    }
}

fn counted(
    lang: Lang,
    n: u32,
    forms: [&'static str; 3],
    english: [&'static str; 2],
) -> &'static str {
    match lang {
        Lang::En => english[usize::from(n != 1)],
        Lang::Ru => {
            let (tens, ones) = (n % 100, n % 10);
            if (11..=14).contains(&tens) {
                forms[2]
            } else if ones == 1 {
                forms[0]
            } else if (2..=4).contains(&ones) {
                forms[1]
            } else {
                forms[2]
            }
        }
    }
}

pub fn decimal(value: f64, places: usize, lang: Lang) -> String {
    let made = format!("{value:.places$}");
    match lang {
        Lang::Ru => made.replace('.', ","),
        Lang::En => made,
    }
}

pub fn grouped(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at) % 3 == 0 {
            out.push(' ');
        }
        out.push(digit);
    }
    out
}

pub fn lasted(seconds: u64, lang: Lang) -> String {
    let (hours, minutes, rest) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);
    let (hour, minute, second) = (
        said(lang, "hour"),
        said(lang, "minute"),
        said(lang, "second"),
    );
    match (hours, minutes) {
        (0, 0) => format!("{rest} {second}"),
        (0, _) => format!("{minutes} {minute}"),
        (_, 0) => format!("{hours} {hour}"),
        _ => format!("{hours} {hour} {minutes} {minute}"),
    }
}

fn glass(side: f32, top: f32, bottom: f32, radius: f32) -> Shell {
    Shell {
        side,
        top,
        bottom,
        radius,
        fill: GLASS,
        edge: Some(GLASS_EDGE),
    }
}

fn plate(shell: Shell, child: Node) -> Node {
    Node::Plate {
        shell,
        wide: None,
        high: None,
        child: Box::new(child),
    }
}

fn cap(words: &str) -> Node {
    text(words, Face::Mono, 11.0, MUTED)
}

fn fact(value: String, label: String, colour: Rgba, size: f32) -> Node {
    col(
        0.0,
        vec![
            text(value, Face::MonoBold, size, colour),
            text(label, Face::Sans, 12.0, MUTED),
        ],
    )
}

fn small(value: String, label: &str) -> Node {
    row(
        6.0,
        vec![
            text(value, Face::MonoBold, 13.0, INK),
            text(label, Face::Sans, 12.0, MUTED),
        ],
    )
}

fn mod_colour(code: &str) -> Rgba {
    match code.get(..2).unwrap_or("").to_ascii_uppercase().as_str() {
        "NM" => rgb(0x5ec2d0, 1.0),
        "HD" => rgb(0xc9873f, 1.0),
        "HR" => rgb(0xe0484c, 1.0),
        "DT" => rgb(0x8c5ad0, 1.0),
        "FM" => rgb(0xc0487a, 1.0),
        "TB" => rgb(0xa05a5c, 1.0),
        _ => rgb(0x9a8f92, 1.0),
    }
}

fn chip(code: &str) -> Node {
    let base = mod_colour(code);
    plate(
        Shell {
            side: 8.0,
            top: 3.0,
            bottom: 3.0,
            radius: 6.0,
            fill: [base[0], base[1], base[2], 0.2],
            edge: None,
        },
        text(code, Face::MonoBold, 12.0, mix(base, WHITE, 0.45)),
    )
}

pub fn star_colour(stars: f64) -> Rgba {
    const STOPS: [(f64, u32); 10] = [
        (0.1, 0x4290fb),
        (1.25, 0x4fc0ff),
        (2.0, 0x4fffd5),
        (2.5, 0x7cff4f),
        (3.3, 0xf6f05c),
        (4.2, 0xff8068),
        (4.9, 0xff4e6f),
        (5.8, 0xc645b8),
        (6.7, 0x6563de),
        (7.7, 0x18158e),
    ];
    if stars <= STOPS[0].0 {
        return rgb(STOPS[0].1, 1.0);
    }
    for pair in STOPS.windows(2) {
        if stars <= pair[1].0 {
            let share = ((stars - pair[0].0) / (pair[1].0 - pair[0].0)) as f32;
            return mix(rgb(pair[0].1, 1.0), rgb(pair[1].1, 1.0), share);
        }
    }
    rgb(STOPS[9].1, 1.0)
}

fn stars(value: f64, lang: Lang) -> Node {
    let fill = star_colour(value);
    let bright = fill[0] * 0.2126 + fill[1] * 0.7152 + fill[2] * 0.0722 > 0.62;
    let ink = if bright { rgb(0x1a1016, 1.0) } else { WHITE };
    plate(
        Shell {
            side: 9.0,
            top: 3.0,
            bottom: 3.0,
            radius: 11.0,
            fill,
            edge: None,
        },
        row(
            4.0,
            vec![
                Node::Star {
                    side: 10.0,
                    colour: ink,
                },
                text(decimal(value, 2, lang), Face::MonoBold, 12.0, ink),
            ],
        ),
    )
}

fn tile(lead: Node, words: &str) -> Node {
    plate(
        glass(11.0, 7.0, 7.0, 17.0),
        row(
            9.0,
            vec![
                Node::Logo {
                    side: 16.0,
                    colour: ACCENT,
                },
                lead,
                text(words, Face::Semi, 13.0, INK),
            ],
        ),
    )
}

pub fn tile_connected(lang: Lang) -> Node {
    tile(
        Node::Dot {
            side: 7.0,
            colour: GOOD,
            glow: 1.0,
        },
        said(lang, "connected"),
    )
}

pub fn tile_saved(lang: Lang) -> Node {
    tile(
        Node::Tick {
            side: 16.0,
            colour: GOOD,
        },
        said(lang, "saved"),
    )
}

pub fn tile_recording(lang: Lang, shown: f32, glow: f32) -> Node {
    let lead = || {
        vec![
            Node::Logo {
                side: 16.0,
                colour: ACCENT,
            },
            Node::Dot {
                side: 7.0,
                colour: ACCENT,
                glow,
            },
        ]
    };
    let mut full = lead();
    full.push(text(said(lang, "recording"), Face::Semi, 13.0, INK));
    Node::Morph {
        shell: glass(11.0, 7.0, 7.0, 17.0),
        from: Box::new(row(9.0, lead())),
        to: Box::new(row(9.0, full)),
        shown,
    }
}

pub fn reach(play: &Play, lang: Lang, shown: f32) -> Node {
    let now = decimal(play.pp.max(0.0).round(), 0, lang);
    let counter = row(
        5.0,
        vec![
            text(now.clone(), Face::MonoBold, 22.0, INK),
            text("PP", Face::Semi, 12.0, MUTED),
        ],
    )
    .crossed(Cross::End);
    let top = play
        .record
        .into_iter()
        .chain(play.clean)
        .fold(play.pp, f64::max)
        .max(1.0);
    let head = vec![
        text(now, Face::MonoBold, 26.0, INK),
        text("PP", Face::Semi, 13.0, MUTED),
    ];
    let mut lines = vec![
        row(6.0, head).crossed(Cross::End),
        Node::Track {
            share: (play.pp / top) as f32,
            mark: play.record.map(|record| (record / top) as f32),
            fill: ACCENT,
        },
    ];
    let mut facts = Vec::new();
    if let Some(record) = play.record {
        facts.push(small(
            decimal(record.round(), 0, lang),
            said(lang, "record"),
        ));
    }
    facts.push(Node::Grow);
    if let Some(clean) = play.clean {
        facts.push(small(decimal(clean.round(), 0, lang), said(lang, "clean")));
    }
    if facts.len() > 1 {
        lines.push(row(8.0, facts));
    }
    Node::Morph {
        shell: glass(13.0, 7.0, 9.0, 12.0),
        from: Box::new(counter),
        to: Box::new(col(8.0, lines).wide(210.0)),
        shown,
    }
}

pub fn rate(value: f32, lang: Lang, scale: f32) -> Node {
    let k = scale.clamp(0.75, 2.0);
    plate(
        glass(9.0 * k, 2.0 * k, 2.0 * k, 10.0 * k),
        row(
            6.0 * k,
            vec![
                text("UR", Face::Mono, 11.0 * k, MUTED),
                text(
                    decimal(f64::from(value), 1, lang),
                    Face::MonoBold,
                    14.0 * k,
                    INK,
                ),
            ],
        ),
    )
}

fn best(best: &Best, lang: Lang) -> Node {
    col(
        6.0,
        vec![
            cap(said(lang, "best")),
            row(
                10.0,
                vec![
                    text(
                        fitted(&best.title, Face::Semi, 14.0, 190.0),
                        Face::Semi,
                        14.0,
                        INK,
                    ),
                    Node::Grow,
                    text(
                        format!("{} PP", decimal(best.pp.round(), 0, lang)),
                        Face::MonoBold,
                        14.0,
                        INK,
                    ),
                ],
            ),
            row(
                8.0,
                vec![
                    chip(if best.mods.is_empty() {
                        "NM"
                    } else {
                        &best.mods
                    }),
                    text(
                        format!("{}%", decimal(best.accuracy, 2, lang)),
                        Face::MonoBold,
                        12.0,
                        MUTED,
                    ),
                    stars(best.stars, lang),
                ],
            ),
        ],
    )
}

pub fn day(day: &Day, lang: Lang) -> Node {
    let gain = format!(
        "{}{} PP",
        if day.pp_gain >= 0.0 { "+" } else { "−" },
        decimal(day.pp_gain.abs().round(), 0, lang)
    );
    let moved = match day.climbed {
        0 => said(lang, "world").to_owned(),
        up if up > 0 => format!("{} {}", said(lang, "higher"), grouped(up as u64)),
        down => format!("{} {}", said(lang, "lower"), grouped(down.unsigned_abs())),
    };
    let mut lines = vec![
        cap(said(lang, "day")),
        row(
            26.0,
            vec![
                fact(
                    gain,
                    said(lang, "gain").to_owned(),
                    if day.pp_gain >= 0.0 { GOOD } else { MUTED },
                    22.0,
                ),
                fact(
                    format!("#{}", grouped(u64::from(day.rank))),
                    moved,
                    INK,
                    22.0,
                ),
            ],
        )
        .crossed(Cross::Start),
    ];
    if let Some(shown) = &day.best {
        lines.push(Node::Rule);
        lines.push(best(shown, lang));
    }
    lines.push(Node::Rule);
    let mut tail = vec![
        small(
            day.plays.to_string(),
            counted(lang, day.plays, ["игра", "игры", "игр"], ["play", "plays"]),
        ),
        small(lasted(day.seconds, lang), said(lang, "in-game")),
    ];
    if day.records > 0 {
        tail.push(small(
            day.records.to_string(),
            counted(
                lang,
                day.records,
                ["рекорд", "рекорда", "рекордов"],
                ["record", "records"],
            ),
        ));
    }
    lines.push(row(16.0, tail));
    plate(glass(16.0, 14.0, 14.0, 12.0), col(11.0, lines).wide(286.0))
}

pub fn notice(notice: &Notice) -> Node {
    let colour = match notice.tone {
        Tone::Good => GOOD,
        Tone::Blue => BLUE,
        Tone::Accent => ACCENT,
    };
    plate(
        glass(14.0, 12.0, 12.0, 12.0),
        row(
            12.0,
            vec![
                Node::Badge {
                    side: 30.0,
                    radius: 9.0,
                    colour,
                    sign: notice.sign.clone(),
                },
                col(
                    1.0,
                    vec![
                        text(
                            fitted(&notice.title, Face::Semi, 14.0, 250.0),
                            Face::Semi,
                            14.0,
                            INK,
                        ),
                        text(
                            fitted(&notice.body, Face::Sans, 13.0, 250.0),
                            Face::Sans,
                            13.0,
                            MUTED,
                        ),
                    ],
                ),
            ],
        )
        .wide(302.0),
    )
}

fn face_colour(name: &str) -> Rgba {
    const SHADES: [u32; 6] = [0x6ea0f0, 0xe0a84a, 0xc44fc0, 0x58b889, 0xd0705a, 0x8c7ad8];
    let sum = name.bytes().fold(0usize, |sum, byte| {
        sum.wrapping_mul(31).wrapping_add(usize::from(byte))
    });
    rgb(SHADES[sum % SHADES.len()], 1.0)
}

fn place(place: &Place, lang: Lang) -> Node {
    let sign: String = place
        .name
        .chars()
        .next()
        .map(|sign| sign.to_uppercase().collect())
        .unwrap_or_default();
    row(
        7.0,
        vec![
            text(
                place.place.to_string(),
                Face::MonoBold,
                12.0,
                if place.you { ACCENT } else { MUTED },
            ),
            Node::Disc {
                side: 22.0,
                colour: if place.you {
                    ACCENT
                } else {
                    face_colour(&place.name)
                },
                sign,
                ring: place.you.then_some([ACCENT[0], ACCENT[1], ACCENT[2], 0.45]),
            },
            text(
                format!("{}%", decimal(place.accuracy, 2, lang)),
                Face::MonoBold,
                13.0,
                INK,
            ),
        ],
    )
}

pub fn card(card: &Card, lang: Lang) -> Option<Node> {
    if card.pool.is_none() && card.places.is_empty() {
        return None;
    }
    let mut head = vec![Node::Logo {
        side: 16.0,
        colour: ACCENT,
    }];
    if let Some(pool) = &card.pool {
        head.push(text(
            fitted(pool, Face::Semi, 13.0, 300.0),
            Face::Semi,
            13.0,
            INK,
        ));
    }
    head.push(Node::Grow);
    if let Some(value) = card.stars {
        head.push(stars(value, lang));
    }
    let mut lines = vec![row(9.0, head)];
    if !card.places.is_empty() {
        let mut places = vec![cap(said(lang, "chat"))];
        places.extend(card.places.iter().take(3).map(|shown| place(shown, lang)));
        lines.push(row(14.0, places));
    }
    Some(plate(
        glass(14.0, 11.0, 11.0, 12.0),
        col(10.0, lines).wide(442.0),
    ))
}

fn key_cap(key: &str) -> Node {
    plate(
        Shell {
            side: 7.0,
            top: 5.0,
            bottom: 5.0,
            radius: 7.0,
            fill: rgb(0xffffff, 0.06),
            edge: Some(rgb(0xffffff, 0.28)),
        },
        text(key, Face::MonoBold, 12.0, INK),
    )
}

pub fn offer(key: &str, lang: Lang) -> Node {
    plate(
        glass(14.0, 10.0, 10.0, 12.0),
        row(
            12.0,
            vec![
                key_cap(key),
                col(
                    1.0,
                    vec![
                        text(said(lang, "offer"), Face::Semi, 14.0, INK),
                        text(said(lang, "offer-about"), Face::Sans, 12.0, MUTED),
                    ],
                ),
            ],
        ),
    )
}

pub fn sent(sent: &Sent, lang: Lang) -> Node {
    let grade_colour = match sent.grade.as_str() {
        "SS" | "X" | "XH" | "S" | "SH" => rgb(0xf0c95a, 1.0),
        "A" => rgb(0x8fd9a0, 1.0),
        "B" => rgb(0x6ea0f0, 1.0),
        "C" => rgb(0xc47ae0, 1.0),
        _ => rgb(0xe07070, 1.0),
    };
    let reply = Node::Plate {
        shell: Shell {
            side: 10.0,
            top: 10.0,
            bottom: 10.0,
            radius: 10.0,
            fill: rgb(0x12070a, 1.0),
            edge: Some(rgb(0x2a161b, 1.0)),
        },
        wide: Some(320.0),
        high: None,
        child: Box::new(
            row(
                10.0,
                vec![
                    Node::Cover {
                        wide: 74.0,
                        high: 50.0,
                        radius: 8.0,
                        from: rgb(0xe8aa54, 1.0),
                        to: rgb(0x5a3a8a, 1.0),
                    },
                    col(
                        2.0,
                        vec![
                            text(
                                fitted(&sent.title, Face::Semi, 13.0, 150.0),
                                Face::Semi,
                                13.0,
                                INK,
                            ),
                            row(
                                6.0,
                                vec![
                                    text(
                                        fitted(&sent.player, Face::Sans, 12.0, 96.0),
                                        Face::Sans,
                                        12.0,
                                        MUTED,
                                    ),
                                    chip(if sent.mods.is_empty() {
                                        "NM"
                                    } else {
                                        &sent.mods
                                    }),
                                ],
                            ),
                            row(
                                8.0,
                                vec![
                                    text(
                                        format!("{} PP", decimal(sent.pp.round(), 0, lang)),
                                        Face::MonoBold,
                                        12.0,
                                        INK,
                                    ),
                                    text(
                                        format!("{}%", decimal(sent.accuracy, 2, lang)),
                                        Face::MonoBold,
                                        12.0,
                                        MUTED,
                                    ),
                                ],
                            ),
                        ],
                    ),
                    Node::Grow,
                    Node::Badge {
                        side: 34.0,
                        radius: 9.0,
                        colour: grade_colour,
                        sign: sent.grade.clone(),
                    },
                ],
            )
            .wide(300.0),
        ),
    };
    plate(
        glass(12.0, 11.0, 12.0, 12.0),
        col(
            9.0,
            vec![
                row(
                    9.0,
                    vec![
                        Node::Tick {
                            side: 18.0,
                            colour: GOOD,
                        },
                        text(said(lang, "sent"), Face::Semi, 14.0, INK),
                        Node::Grow,
                        text(
                            fitted(&sent.chat, Face::Sans, 12.0, 120.0),
                            Face::Sans,
                            12.0,
                            MUTED,
                        ),
                    ],
                ),
                reply,
            ],
        )
        .wide(320.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_worded_the_way_each_language_writes_them() {
        assert_eq!(decimal(98.614, 2, Lang::Ru), "98,61");
        assert_eq!(decimal(98.614, 2, Lang::En), "98.61");
        assert_eq!(grouped(11_188), "11 188");
        assert_eq!(grouped(214), "214");
        assert_eq!(lasted(4_320, Lang::Ru), "1 ч 12 мин");
        assert_eq!(lasted(231, Lang::En), "3 min");
        assert_eq!(lasted(51, Lang::Ru), "51 с");
        let plays = ["игра", "игры", "игр"];
        assert_eq!(
            [1, 2, 5, 11, 21, 104].map(|n| counted(Lang::Ru, n, plays, ["play", "plays"])),
            ["игра", "игры", "игр", "игр", "игра", "игры"]
        );
        assert_eq!(counted(Lang::En, 1, plays, ["play", "plays"]), "play");
    }

    #[test]
    fn a_star_pill_takes_its_colour_from_the_difficulty() {
        assert_ne!(star_colour(2.0), star_colour(6.5));
        assert_eq!(star_colour(9.5), star_colour(8.0));
        assert!(star_colour(0.0)[2] > 0.9);
    }
}
