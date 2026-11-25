use crate::draw::{fitted, rgb, Face, Rgba};
use crate::node::{col, row, text, Cross, Node, Shell};
use crate::{Card, Lang, Notice, Place, Play, Profile, Tone};

const INK: Rgba = rgb(0xece7e2, 1.0);
const MUTED: Rgba = rgb(0xc9aaa6, 1.0);
const ACCENT: Rgba = rgb(0xe24848, 1.0);
const GOOD: Rgba = rgb(0xa2b485, 1.0);
const QUIET: Rgba = rgb(0xb9aeb0, 1.0);
const PANEL: Rgba = rgb(0x100a0c, 0.88);
const RADIUS: f32 = 8.0;
pub const HIDE: &str = "Ctrl+Shift+H";
pub const BOARD: &str = "Ctrl+Shift+D";
pub const BIGGER: &str = "Ctrl+Shift+=";
pub const SMALLER: &str = "Ctrl+Shift+−";

fn said(lang: Lang, key: &str) -> &'static str {
    match (lang, key) {
        (Lang::Ru, "connected") => "Witness подключился",
        (Lang::En, "connected") => "Witness is on",
        (Lang::Ru, "chat") => "Топ беседы",
        (Lang::En, "chat") => "Chat top",
        (Lang::Ru, "record") => "рекорд",
        (Lang::En, "record") => "record",
        (Lang::Ru, "clean") => "FC",
        (Lang::En, "clean") => "FC",
        (Lang::Ru, "offer") => "Отправить в беседу",
        (Lang::En, "offer") => "Send to the chat",
        (Lang::Ru, "offer-about") => "зажмите клавишу",
        (Lang::En, "offer-about") => "hold the key",
        (Lang::Ru, "tuning") => "Оверлей",
        (Lang::En, "tuning") => "Overlay",
        (Lang::Ru, "scale") => "Размер",
        (Lang::En, "scale") => "Size",
        (Lang::Ru, "bigger") => "больше",
        (Lang::En, "bigger") => "larger",
        (Lang::Ru, "smaller") => "меньше",
        (Lang::En, "smaller") => "smaller",
        (Lang::Ru, "hide") => "спрятать оверлей",
        (Lang::En, "hide") => "hide the overlay",
        (Lang::Ru, "board") => "Dossier",
        (Lang::En, "board") => "Dossier",
        (Lang::Ru, "board-close") => "закрыть",
        (Lang::En, "board-close") => "close",
        (Lang::Ru, "board-empty") => "Пока тихо",
        (Lang::En, "board-empty") => "All quiet",
        (Lang::Ru, "board-empty-about") => "Здесь появятся события Dossier",
        (Lang::En, "board-empty-about") => "Dossier's events will show up here",
        (Lang::Ru, "max-pp") => "Максимум PP",
        (Lang::En, "max-pp") => "Max PP",
        (Lang::Ru, "max-pp-about") => "SS, без модов",
        (Lang::En, "max-pp-about") => "SS, no mods",
        (Lang::Ru, "rank") => "Мировой ранг",
        (Lang::En, "rank") => "Global rank",
        (Lang::Ru, "country") => "В стране",
        (Lang::En, "country") => "Country",
        (Lang::Ru, "accuracy") => "Точность",
        (Lang::En, "accuracy") => "Accuracy",
        (Lang::Ru, "plays") => "Игр",
        (Lang::En, "plays") => "Plays",
        (Lang::Ru, "level") => "Уровень",
        (Lang::En, "level") => "Level",
        (Lang::Ru, "max-combo") => "Максимум комбо",
        (Lang::En, "max-combo") => "Max combo",
        (Lang::Ru, "max-combo-about") => "на карте",
        (Lang::En, "max-combo-about") => "on the map",
        (Lang::Ru, "duel") => "Поиск дуэли",
        (Lang::En, "duel") => "Searching for a duel",
        (Lang::Ru, "duel-about") => "в очереди",
        (Lang::En, "duel-about") => "in the queue",
        (Lang::Ru, "hidden") => "Оверлей спрятан",
        (Lang::En, "hidden") => "Overlay hidden",
        (Lang::Ru, "hidden-about") => "Ctrl+Shift+H, чтобы вернуть",
        (Lang::En, "hidden-about") => "Ctrl+Shift+H to bring it back",
        _ => "",
    }
}

pub fn decimal(value: f64, places: usize, lang: Lang) -> String {
    let made = format!("{value:.places$}");
    match lang {
        Lang::Ru => made.replace('.', ","),
        Lang::En => made,
    }
}

fn panel(side: f32, top: f32, bottom: f32) -> Shell {
    Shell {
        side,
        top,
        bottom,
        radius: RADIUS,
        fill: PANEL,
        edge: None,
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

fn boxed(child: Node) -> Node {
    plate(panel(12.0, 10.0, 11.0), child)
}

fn title(words: impl Into<String>) -> Node {
    text(words, Face::Semi, 13.0, INK)
}

fn label(words: impl Into<String>) -> Node {
    text(words, Face::Sans, 12.0, MUTED)
}

fn fitted_title(words: &str, most: f32) -> Node {
    title(fitted(words, Face::Semi, 13.0, most))
}

fn small(value: String, before: &str) -> Node {
    row(
        5.0,
        vec![label(before), text(value, Face::Semi, 13.0, INK)],
    )
}

fn lead(tick: Option<f32>) -> Node {
    match tick {
        Some(drawn) => Node::Tick {
            side: 16.0,
            colour: GOOD,
            drawn,
        },
        None => Node::Logo {
            side: 16.0,
            colour: ACCENT,
        },
    }
}

pub fn tile(lang: Lang, greeted: f32, tick: Option<f32>) -> Node {
    Node::Morph {
        shell: panel(8.0, 6.0, 6.0),
        from: Box::new(lead(tick)),
        to: Box::new(row(
            8.0,
            vec![lead(tick), text(said(lang, "connected"), Face::Semi, 12.0, INK)],
        )),
        shown: greeted,
    }
}

pub fn reach(play: &Play, pp: f64, lang: Lang, shown: f32) -> Node {
    let now = decimal(pp.max(0.0).round(), 0, lang);
    let counter = row(
        5.0,
        vec![
            text(now.clone(), Face::MonoBold, 20.0, INK),
            text("PP", Face::Semi, 12.0, MUTED),
        ],
    )
    .crossed(Cross::End);
    let top = play
        .record
        .into_iter()
        .chain(play.clean)
        .fold(pp, f64::max)
        .max(1.0);
    let head = vec![
        text(now, Face::MonoBold, 24.0, INK),
        text("PP", Face::Semi, 13.0, MUTED),
    ];
    let mut lines = vec![
        row(6.0, head).crossed(Cross::End),
        Node::Track {
            share: (pp / top) as f32,
            mark: play.record.map(|record| (record / top) as f32),
            fill: ACCENT,
        },
    ];
    let mut facts = Vec::new();
    if let Some(record) = play.record {
        facts.push(small(decimal(record.round(), 0, lang), said(lang, "record")));
    }
    facts.push(Node::Grow);
    if let Some(clean) = play.clean {
        facts.push(small(decimal(clean.round(), 0, lang), said(lang, "clean")));
    }
    if facts.len() > 1 {
        lines.push(row(8.0, facts));
    }
    Node::Morph {
        shell: panel(12.0, 7.0, 9.0),
        from: Box::new(counter),
        to: Box::new(col(8.0, lines).wide(210.0)),
        shown,
    }
}

pub fn rate(value: f32, lang: Lang, scale: f32) -> Node {
    let k = scale.clamp(0.75, 2.0);
    plate(
        panel(8.0 * k, 2.0 * k, 2.0 * k),
        row(
            6.0 * k,
            vec![
                text("UR", Face::Sans, 11.0 * k, MUTED),
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

pub fn notice(notice: &Notice) -> Node {
    let (colour, sign) = match notice.tone {
        Tone::Good => (GOOD, "✓"),
        Tone::Accent => (ACCENT, "!"),
        Tone::Blue => (QUIET, "i"),
    };
    let mut lines = vec![fitted_title(&notice.title, 220.0)];
    if !notice.body.is_empty() {
        lines.push(label(fitted(&notice.body, Face::Sans, 12.0, 220.0)));
    }
    plate(
        panel(12.0, 10.0, 10.0),
        row(
            10.0,
            vec![
                Node::Badge {
                    side: 28.0,
                    radius: 14.0,
                    colour,
                    sign: sign.to_owned(),
                },
                col(2.0, lines),
            ],
        )
        .wide(262.0),
    )
}

const MEDALS: [u32; 3] = [0xe6b84f, 0xc3c8d0, 0xd08a5c];

fn place(place: &Place, lang: Lang) -> Node {
    let medal = MEDALS
        .get(place.place.saturating_sub(1) as usize)
        .map_or(MUTED, |hex| rgb(*hex, 1.0));
    let face = if place.you { Face::Semi } else { Face::Sans };
    Node::Plate {
        shell: Shell {
            side: 6.0,
            top: 4.0,
            bottom: 4.0,
            radius: 6.0,
            fill: if place.you {
                [ACCENT[0], ACCENT[1], ACCENT[2], 0.16]
            } else {
                [0.0, 0.0, 0.0, 0.0]
            },
            edge: None,
        },
        wide: Some(262.0),
        high: None,
        child: Box::new(row(
            10.0,
            vec![
                Node::Badge {
                    side: 20.0,
                    radius: 6.0,
                    colour: medal,
                    sign: place.place.to_string(),
                },
                text(fitted(&place.name, face, 13.0, 150.0), face, 13.0, INK),
                Node::Grow,
                text(
                    format!("{}%", decimal(place.accuracy, 2, lang)),
                    Face::Semi,
                    13.0,
                    INK,
                ),
            ],
        )),
    }
}

/// The height of the chat's top with all its three places, which is the room reserved for it so that
/// the plates above it stand still whether it has arrived or not.
pub fn card_height() -> f32 {
    let full = Card { places: vec![Place::default(); 3], ..Card::default() };
    card(&full, Lang::Ru).map_or(0.0, |node| node.measure().1)
}

pub fn card(card: &Card, lang: Lang) -> Option<Node> {
    if card.places.is_empty() {
        return None;
    }
    let mut lines = vec![title(said(lang, "chat"))];
    lines.extend(card.places.iter().take(3).map(|shown| place(shown, lang)));
    Some(plate(panel(10.0, 10.0, 8.0), col(4.0, lines)))
}

fn key_cap(key: &str) -> Node {
    plate(
        Shell {
            side: 7.0,
            top: 3.0,
            bottom: 3.0,
            radius: 5.0,
            fill: rgb(0xffffff, 0.08),
            edge: Some(rgb(0xffffff, 0.24)),
        },
        text(key, Face::Semi, 12.0, INK),
    )
}

pub fn offer(key: &str, holding: f32, lang: Lang) -> Node {
    let words = row(
        10.0,
        vec![
            key_cap(key),
            col(
                0.0,
                vec![title(said(lang, "offer")), label(said(lang, "offer-about"))],
            ),
        ],
    );
    let wide = words.measure().0.max(200.0);
    let bar = Node::Track {
        share: holding,
        mark: None,
        fill: ACCENT,
    };
    boxed(col(6.0, vec![words, bar]).wide(wide))
}

fn hint(keys: &str, words: &str) -> Node {
    row(
        12.0,
        vec![text(keys, Face::Semi, 12.0, INK), Node::Grow, label(words)],
    )
}

pub fn tuning(scale: u16, lang: Lang) -> Node {
    boxed(
        col(
            6.0,
            vec![
                row(
                    10.0,
                    vec![
                        title(said(lang, "tuning")),
                        Node::Grow,
                        label(said(lang, "scale")),
                        title(format!("{scale} %")),
                    ],
                ),
                Node::Rule,
                hint(BIGGER, said(lang, "bigger")),
                hint(SMALLER, said(lang, "smaller")),
                hint(HIDE, said(lang, "hide")),
            ],
        )
        .wide(250.0),
    )
}

pub fn board(notices: &[Notice], lang: Lang) -> Node {
    let mut lines = vec![
        row(
            10.0,
            vec![
                title(said(lang, "board")),
                Node::Grow,
                label(BOARD),
                label(said(lang, "board-close")),
            ],
        ),
        Node::Rule,
    ];
    if notices.is_empty() {
        lines.push(title(said(lang, "board-empty")));
        lines.push(label(said(lang, "board-empty-about")));
    }
    lines.extend(notices.iter().take(5).map(notice));
    plate(panel(14.0, 12.0, 14.0), col(8.0, lines).wide(286.0))
}

const MOD_BITS: [(u32, &str); 14] = [
    (1 << 1, "EZ"),
    (1, "NF"),
    (1 << 8, "HT"),
    (1 << 4, "HR"),
    (1 << 14, "PF"),
    (1 << 5, "SD"),
    (1 << 9, "NC"),
    (1 << 6, "DT"),
    (1 << 3, "HD"),
    (1 << 10, "FL"),
    (1 << 7, "RX"),
    (1 << 13, "AP"),
    (1 << 12, "SO"),
    (1 << 11, "AT"),
];

/// The acronyms of a legacy mod set, as the game lists them: a Nightcore is not also a Double Time,
/// and a Perfect is not also a Sudden Death.
pub fn mod_names(mods: u32) -> String {
    let nightcore = mods & (1 << 9) != 0;
    let perfect = mods & (1 << 14) != 0;
    MOD_BITS
        .iter()
        .filter(|(bit, name)| {
            mods & bit != 0 && !(nightcore && *name == "DT") && !(perfect && *name == "SD")
        })
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join(" ")
}

pub const MAX_PP_WIDE: f32 = 210.0 + 24.0;

/// The plate is as wide as its words and its number need, because a combo may have two digits or five.
pub fn max_combo(combo: u32, lang: Lang) -> Node {
    boxed(row(
        18.0,
        vec![
            col(2.0, vec![title(said(lang, "max-combo")), label(said(lang, "max-combo-about"))]),
            text(format!("{}x", grouped(u64::from(combo), lang)), Face::Semi, 20.0, INK),
        ],
    ))
}

pub fn max_pp(pp: f64, mods: u32, lang: Lang) -> Node {
    let about = if mods == 0 {
        said(lang, "max-pp-about").to_owned()
    } else {
        format!("SS · {}", mod_names(mods))
    };
    boxed(row(
        14.0,
        vec![
            col(2.0, vec![title(said(lang, "max-pp")), label(about)]),
            Node::Grow,
            text(format!("{} pp", decimal(pp, 0, lang)), Face::Semi, 20.0, ACCENT),
        ],
    )
    .wide(210.0))
}

fn grouped(value: u64, lang: Lang) -> String {
    let digits = value.to_string();
    let gap = if lang == Lang::Ru { '\u{a0}' } else { ',' };
    let mut out = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at) % 3 == 0 {
            out.push(gap);
        }
        out.push(digit);
    }
    out
}

fn stat(name: &str, value: String) -> Node {
    row(10.0, vec![label(name), Node::Grow, text(value, Face::Semi, 13.0, INK)])
}

/// The player's own osu! profile in one plate: who, the rank, and the numbers below.
pub fn profile(profile: &Profile, lang: Lang) -> Node {
    let initial: String = profile.name.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
    let rank = if profile.rank > 0 { format!("#{}", grouped(profile.rank, lang)) } else { "—".to_owned() };
    let country = match (profile.country.is_empty(), profile.country_rank) {
        (false, 1..) => format!("{} #{}", profile.country, grouped(profile.country_rank, lang)),
        (false, 0) => profile.country.clone(),
        _ => String::new(),
    };
    let mut lines = vec![fitted_title(&profile.name, 170.0), text(rank, Face::Semi, 12.0, ACCENT)];
    if !country.is_empty() {
        lines.push(label(country));
    }
    boxed(
        col(
            8.0,
            vec![
                row(
                    12.0,
                    vec![
                        Node::Badge {
                            side: 36.0,
                            radius: 18.0,
                            colour: ACCENT,
                            sign: initial,
                        },
                        col(2.0, lines),
                    ],
                ),
                Node::Rule,
                col(
                    5.0,
                    vec![
                        stat("pp", decimal(profile.pp, 0, lang)),
                        stat(said(lang, "accuracy"), format!("{}%", decimal(profile.accuracy, 2, lang))),
                        stat(said(lang, "plays"), grouped(profile.plays, lang)),
                        stat(said(lang, "level"), decimal(profile.level, 0, lang)),
                    ],
                ),
            ],
        )
        .wide(206.0),
    )
}

/// How long a duel has been searched for: a dot, the words, and the time as minutes and seconds.
pub fn duel(seconds: u64, lang: Lang) -> Node {
    boxed(row(
        12.0,
        vec![
            Node::Dot { side: 10.0, colour: ACCENT },
            col(2.0, vec![title(said(lang, "duel")), label(said(lang, "duel-about"))]),
            Node::Grow,
            text(format!("{}:{:02}", seconds / 60, seconds % 60), Face::Semi, 20.0, INK),
        ],
    )
    .wide(240.0))
}

pub fn hidden(lang: Lang) -> Node {
    boxed(col(
        2.0,
        vec![title(said(lang, "hidden")), label(said(lang, "hidden-about"))],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_worded_the_way_each_language_writes_them() {
        assert_eq!(decimal(98.614, 2, Lang::Ru), "98,61");
        assert_eq!(decimal(98.614, 2, Lang::En), "98.61");
    }

    #[test]
    fn mods_are_listed_as_the_game_lists_them() {
        assert_eq!(mod_names(0), "");
        assert_eq!(mod_names((1 << 3) | (1 << 6)), "DT HD");
        assert_eq!(mod_names((1 << 6) | (1 << 9) | (1 << 3)), "NC HD", "a Nightcore is not also a Double Time");
        assert_eq!(mod_names((1 << 5) | (1 << 14)), "PF", "a Perfect is not also a Sudden Death");
    }

    #[test]
    fn the_max_combo_plate_grows_with_its_number_instead_of_overflowing() {
        let (short, long) = (max_combo(99, Lang::En).measure().0, max_combo(12_843, Lang::En).measure().0);
        assert!(long > short + 20.0, "{short} {long}");
        let digits = crate::draw::text_width("12,843x", Face::Semi, 20.0);
        assert!(long >= digits + 40.0, "the number {digits} fits in a plate of {long}");
    }
}
