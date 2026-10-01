use iced::widget::{button, column, container, row, text, Space};
use iced::{Background, Border, Color, Element, Length};

use crate::community::{placed, wire, Person, Placed, Rarity};
use crate::community_screen::{self as screen, Ground, Message, Scored};
use crate::glyphs::{glyph, Icon};
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

const GREEN: Color = theme::HIT_100;

pub(crate) struct Sheet<'a> {
    pub source: String,
    pub tint: Color,
    pub title: String,
    pub meta: String,
    pub body: Vec<Element<'a, Message>>,
    pub outside: Option<(String, String)>,
}

fn well<'a>(inside: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside)
        .padding([16, 18])
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.24 * k))),
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.06 * k), width: 1.0, radius: 14.0.into() },
            ..container::Style::default()
        })
        .into()
}

fn said<'a>(words: String, colour: Color) -> Element<'a, Message> {
    text(words).font(theme::SANS).size(theme::BODY).color(ui::faded(colour)).into()
}

fn worth(w: &crate::lang::Words, play: &wire::Play, by_score: bool) -> String {
    match (by_score, play.pp > 0.0, play.pp_if) {
        (true, _, _) => w.lang().group(play.score),
        (false, true, _) => format!("{} pp", screen::decimal(w, play.pp, 0)),
        (false, false, Some(guess)) => format!("≈{} pp", screen::decimal(w, guess, 0)),
        (false, false, None) => "—".to_owned(),
    }
}

fn board_row<'a>(ground: &Ground<'a>, row_said: &'a wire::BoardRow, by_score: bool, chosen: bool) -> Element<'a, Message> {
    let w = ground.words;
    let known = ground.catalog.people.iter().position(|person| person.id == row_said.who);
    let face: Element<'a, Message> = match known {
        Some(at) => screen::face(ground, &ground.catalog.people[at], 24.0),
        None => screen::round(ground.pictures.get(&row_said.avatar), &row_said.name.chars().next().map(|first| first.to_uppercase().to_string()).unwrap_or_default(), 24.0),
    };
    let place = text(format!("#{}", row_said.place))
        .font(theme::MONO_BOLD)
        .size(13.0)
        .color(ui::faded(screen::medal(row_said.place as usize).unwrap_or(FAINT)));
    let mut name = row![text(row_said.name.clone()).font(theme::SANS_SEMI).size(14.0).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(8).align_y(iced::Center);
    if row_said.you {
        name = name.push(ui::mono_small(w.t("score-you"), ACCENT));
    }
    let combo = row_said.play.combo.map_or(String::new(), |combo| format!("{}x", w.lang().group(u64::from(combo))));
    let line = row![
        container(place).width(34.0),
        face,
        container(name).width(Length::Fill).clip(true),
        screen::mods(&row_said.play.mods),
        container(ui::mono_small(w.percent(f64::from(row_said.play.accuracy)), MUTED)).width(64.0).align_x(iced::alignment::Horizontal::Right),
        container(ui::mono_small(combo, MUTED)).width(58.0).align_x(iced::alignment::Horizontal::Right),
        container(text(worth(w, &row_said.play, by_score)).font(theme::MONO_BOLD).size(13.0).wrapping(text::Wrapping::None).color(ui::faded(INK))).width(92.0).align_x(iced::alignment::Horizontal::Right),
        container(text(row_said.play.grade.clone()).font(theme::MONO_BOLD).size(13.0).color(ui::faded(screen::grade_colour(&row_said.play.grade)))).width(26.0).align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(10)
    .align_y(iced::Center);
    let pressed = button(line).padding([7, 10]).width(Length::Fill).style(ui::button_faded(ui::calm(screen::row_style(row_said.you, chosen.then_some(theme::GRADE_S)))));
    match known {
        Some(at) => pressed.on_press(Message::Person(Some(at))).into(),
        None => pressed.into(),
    }
}

fn standing<'a>(ground: &Ground<'a>, board: &wire::MapBoard, scored: &Scored) -> Element<'a, Message> {
    let w = ground.words;
    let pair = |place: u32, of: u32| [("place", place.to_string()), ("of", of.to_string())];
    let (icon, colour, words) = match placed(board, scored.who, &scored.play, scored.passed) {
        Placed::Took { place, of } => (Icon::Trophy, screen::medal(place as usize).unwrap_or(GREEN), w.with("score-took", &pair(place, of))),
        Placed::Would { place, of, best } => {
            let mut pairs = pair(place, of).to_vec();
            pairs.push(("best", best.to_string()));
            (Icon::Up, MUTED, w.with("score-would", &pairs))
        }
        Placed::Failed { best: Some(best), .. } => (Icon::Circle, FAINT, w.with("score-failed-best", &[("best", best.to_string())])),
        Placed::Failed { best: None, .. } => (Icon::Circle, FAINT, w.t("score-failed")),
    };
    row![glyph(icon, 16.0, colour), text(words).font(theme::SANS_SEMI).size(theme::BODY).color(ui::faded(colour))].spacing(10).align_y(iced::Center).into()
}

pub(crate) fn score<'a>(ground: &Ground<'a>, scored: &'a Scored, picture: &dyn Fn(&str) -> Element<'a, Message>) -> Sheet<'a> {
    let w = ground.words;
    let play = &scored.play;
    let known = ground.catalog.people.iter().position(|person| person.id == scored.who);
    let mut body: Vec<Element<'a, Message>> = Vec::new();
    if let Some(url) = scored.map.cover() {
        body.push(picture(&url));
    }

    let grade = if scored.passed { play.grade.clone() } else { "F".to_owned() };
    let face: Element<'a, Message> = match known {
        Some(at) => screen::face(ground, &ground.catalog.people[at], 30.0),
        None => screen::round(None, &scored.name.chars().next().map(|first| first.to_uppercase().to_string()).unwrap_or_default(), 30.0),
    };
    let name = row![face, text(scored.name.clone()).font(theme::SANS_SEMI).size(theme::LEAD).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(10).align_y(iced::Center);
    let who = button(name).padding([2, 4]).style(ui::button_faded(theme::bare));
    let who = match known {
        Some(at) => who.on_press(Message::Person(Some(at))),
        None => who,
    };
    let pp = match (play.pp > 0.0, play.pp_if) {
        (true, _) => format!("{} pp", screen::decimal(w, play.pp, 0)),
        (false, Some(guess)) => format!("≈{} pp", screen::decimal(w, guess, 0)),
        (false, None) => "—".to_owned(),
    };
    let top = row![
        text(grade.clone()).font(theme::MONO_BOLD).size(40.0).color(ui::faded(screen::grade_colour(&grade))),
        column![who, screen::mods(&play.mods)].spacing(6),
        ui::grow(),
        text(pp).font(theme::MONO_BOLD).size(26.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
    ]
    .spacing(16)
    .align_y(iced::Center);
    let combo = match (play.combo, play.max_combo) {
        (Some(combo), Some(most)) => format!("{} / {}x", w.lang().group(u64::from(combo)), w.lang().group(u64::from(most))),
        (Some(combo), None) => format!("{}x", w.lang().group(u64::from(combo))),
        _ => "—".to_owned(),
    };
    let points = if play.score > 0 { w.lang().group(play.score) } else { "—".to_owned() };
    let figures = row![
        screen::figure(w.percent(f64::from(play.accuracy)), w.t("score-accuracy"), INK),
        screen::figure(combo, w.t("score-combo"), if play.full_combo { GREEN } else { INK }),
        screen::figure(points, w.t("score-points"), INK),
        screen::figure(format!("{} {}", w.day(play.at, ground.now_unix), w.clock(play.at)), w.t("score-played"), MUTED),
    ]
    .spacing(10);
    let mut result = column![top, figures].spacing(14);
    if play.counts.iter().any(Option::is_some) {
        result = result.push(crate::chronicle::counts_row(ground, play.counts, None, None));
    }
    body.push(well(result));

    let board = scored.map.beatmap.and_then(|beatmap| ground.boards.get(&beatmap).map(|board| (beatmap, board)));
    match (scored.map.beatmap, board) {
        (_, Some((beatmap, board))) => {
            body.push(standing(ground, board, scored));
            let by = w.t(if board.by_score() { "score-by-score" } else { "score-by-pp" });
            let mut head = row![ui::mono_small(w.t("score-board").to_uppercase(), FAINT), ui::mono_small(by, FAINT)].spacing(10).align_y(iced::Center);
            if ground.boards_waiting.contains(&beatmap) {
                head = head.push(ui::mono_small(w.t("score-board-refreshing"), FAINT));
            }
            head = head.push(ui::grow()).push(ui::mono_small(format!("{}  ·  {}", w.count("games", u64::from(board.plays)), w.count("players", u64::from(board.players))), FAINT));
            let mut list = column![head].spacing(4);
            if board.rows.is_empty() {
                list = list.push(container(said(w.t("score-board-empty"), FAINT)).padding([10, 0]));
            }
            for row_said in &board.rows {
                list = list.push(board_row(ground, row_said, board.by_score(), row_said.who == scored.who));
            }
            body.push(list.into());
            if board.records.len() > 1 {
                let mut records = column![ui::mono_small(w.t("score-records").to_uppercase(), FAINT)].spacing(6);
                for record in &board.records {
                    let value = if board.by_score() { w.lang().group(record.score) } else { format!("{} pp", screen::decimal(w, record.pp, 0)) };
                    let when = record.at.map_or(String::new(), |at| w.day(at, ground.now_unix));
                    records = records.push(
                        row![
                            container(ui::mono_small(when, FAINT)).width(120.0),
                            text(record.name.clone()).font(theme::SANS_SEMI).size(13.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                            ui::grow(),
                            ui::mono_small(value, MUTED),
                        ]
                        .spacing(10)
                        .align_y(iced::Center),
                    );
                }
                body.push(well(records));
            }
        }
        (Some(beatmap), None) if ground.boards_failed.contains(&beatmap) => body.push(said(w.t("score-board-failed"), ACCENT)),
        (Some(_), None) if !ground.catalog.staged => body.push(said(w.t("score-board-loading"), FAINT)),
        _ => {}
    }

    Sheet {
        source: w.t("panel-score"),
        tint: FAINT,
        title: scored.map.line.clone(),
        meta: match scored.map.stars {
            Some(stars) => format!("{}★", screen::decimal(w, stars, 2)),
            None => String::new(),
        },
        body,
        outside: scored.map.beatmap.map(|beatmap| (w.t("score-open-map"), format!("https://osu.ppy.sh/b/{beatmap}"))),
    }
}

fn holder<'a>(ground: &Ground<'a>, at: usize, person: &'a Person, earned: Option<i64>, chosen: bool, first: bool) -> Element<'a, Message> {
    let w = ground.words;
    let mut name = row![text(person.name.clone()).font(theme::SANS_SEMI).size(14.0).wrapping(text::Wrapping::None).color(ui::faded(INK)), screen::flag(ground, &person.country, 11.0)].spacing(8).align_y(iced::Center);
    if person.you {
        name = name.push(ui::mono_small(w.t("score-you"), ACCENT));
    }
    if first {
        name = name.push(ui::mono_small(w.t("title-first"), theme::GRADE_S));
    }
    let when = earned.map_or(String::new(), |at| w.with("title-earned", &[("day", w.day(at, ground.now_unix))]));
    let line = row![screen::face(ground, person, 26.0), container(name).width(Length::Fill).clip(true), ui::mono_small(when, FAINT)].spacing(12).align_y(iced::Center);
    button(line)
        .padding([7, 10])
        .width(Length::Fill)
        .style(ui::button_faded(ui::calm(screen::row_style(person.you, chosen.then_some(theme::GRADE_S)))))
        .on_press(Message::Person(Some(at)))
        .into()
}

pub(crate) fn title<'a>(ground: &Ground<'a>, code: &str, who: Option<i64>) -> Sheet<'a> {
    let w = ground.words;
    let catalog = ground.catalog;
    let Some(title) = catalog.title_of(code) else {
        return Sheet { source: w.t("kind-title"), tint: FAINT, title: code.to_owned(), meta: String::new(), body: Vec::new(), outside: None };
    };
    let holders = catalog.holders(code);
    let known = !holders.is_empty() || title.rarity != Rarity::Secret;
    let tint = if holders.is_empty() { FAINT } else { title.rarity.colour() };
    let earned = |person: &Person| -> Option<i64> {
        person.title_dates.get(code).copied().or_else(|| catalog.me.as_ref().filter(|me| me.person.id == person.id).and_then(|me| me.title_dates.get(code).copied()))
    };
    let mut ordered: Vec<usize> = holders.clone();
    ordered.sort_by_key(|at| (earned(&catalog.people[*at]).unwrap_or(i64::MAX), *at));
    let dated = ordered.iter().filter(|at| earned(&catalog.people[**at]).is_some()).count();

    let mut body: Vec<Element<'a, Message>> = Vec::new();
    let server = catalog.held.get(code).copied().unwrap_or(holders.len() as u32).max(holders.len() as u32);
    let everyone = catalog.players.max(catalog.people.len() as u32);
    let share = if everyone > 0 { w.percent(f64::from(server) * 100.0 / f64::from(everyone)) } else { String::new() };
    let figures = row![
        screen::figure(w.of(holders.len() as u64, catalog.people.len() as u64), w.t("title-held-chat"), INK),
        screen::figure(w.of(u64::from(server), u64::from(everyone)), w.t("title-held-server"), INK),
        screen::figure(share, w.t("title-share"), tint),
    ]
    .spacing(10);
    body.push(well(figures));

    let mine = catalog.you().filter(|you| !you.titles.iter().any(|held| held == code));
    let progress = catalog.me.as_ref().and_then(|me| me.title_progress.get(code).copied());
    if let (Some(_), Some(value), true) = (mine, progress, title.target > 1) {
        let share = (value as f32 / title.target as f32).clamp(0.0, 1.0);
        let k = ui::fade();
        let bar = container(
            container(Space::new().height(6.0))
                .width(Length::FillPortion((share * 1000.0).round().max(1.0) as u16))
                .style(move |_| container::Style { background: Some(Background::Color(Color { a: tint.a * k, ..tint })), border: Border { radius: 3.0.into(), ..Border::default() }, ..container::Style::default() }),
        );
        let rest = Space::new().width(Length::FillPortion(((1.0 - share) * 1000.0).round().max(1.0) as u16));
        let track = container(row![bar, rest])
            .width(Length::Fill)
            .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.07 * k))), border: Border { radius: 3.0.into(), ..Border::default() }, ..container::Style::default() });
        body.push(well(
            column![
                row![ui::mono_small(w.t("title-progress").to_uppercase(), FAINT), ui::grow(), ui::mono_small(w.of(u64::from(value.min(title.target)), u64::from(title.target)), MUTED)].align_y(iced::Center),
                track,
            ]
            .spacing(10),
        ));
    }

    let mut list = column![ui::mono_small(w.t("title-holders").to_uppercase(), FAINT)].spacing(4);
    if ordered.is_empty() {
        list = list.push(container(said(w.t("title-nobody"), FAINT)).padding([10, 0]));
    }
    for (place, at) in ordered.iter().enumerate() {
        let person = &catalog.people[*at];
        list = list.push(holder(ground, *at, person, earned(person), who == Some(person.id), place == 0 && dated > 1));
    }
    body.push(list.into());
    if let Some(you) = catalog.people.iter().find(|person| person.you) {
        body.push(crate::dossier::wear_button(ground, you, title));
    }

    Sheet {
        source: w.t(title.rarity.key()),
        tint,
        title: if known { title.name(w.lang()).to_owned() } else { "???".to_owned() },
        meta: if known { title.about(w.lang()).to_owned() } else { w.t("secret-title") },
        body,
        outside: None,
    }
}
