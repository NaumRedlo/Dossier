use std::f32::consts::{FRAC_PI_2, PI};

use iced::widget::canvas;
use iced::widget::{button, column, container, mouse_area, row, stack, Space};
use iced::{color, Alignment, Background, Border, Color, Element, Length, Padding};

use crate::duels::{Duel, Entry, Participant};
use crate::duels_screen::{bold, clock, fa, outlined, player_face, phase_of, seconds_left, slab_faded, strong, words, Message, Phase, State, CLOCK, LOST, OUTLINE, ROW, SIDE_A, SIDE_B, WON};
use crate::glyphs::{glyph, Icon};
use crate::lang::Words;
use crate::theme::{self, FAINT, INK, MUTED};
use crate::ui;

const ORDER: [(&str, &str); 8] = [("ban", "A"), ("ban", "B"), ("ban", "A"), ("ban", "B"), ("pick", "B"), ("pick", "A"), ("pick", "B"), ("pick", "A")];
const GOOD: Color = theme::GRADE_A;
const WAIT: Color = theme::GRADE_S;
const GOLD: Color = color!(0xf0c040);
const CARD_HIGH: f32 = 128.0;
const HERO_HIGH: f32 = 230.0;

fn plate(fill: Color, radius: f32) -> impl Fn(&iced::Theme) -> container::Style + 'static {
    let fill = fa(fill);
    move |_| container::Style { background: Some(Background::Color(fill)), border: Border { radius: radius.into(), ..Border::default() }, ..container::Style::default() }
}

fn side_colour(side: &str) -> Color {
    if side == "B" { SIDE_B } else { SIDE_A }
}

fn seat<'d>(duel: &'d Duel, side: &str) -> Option<&'d Participant> {
    duel.participants.iter().find(|person| person.side.as_deref() == Some(side))
}

fn name_of(duel: &Duel, player: Option<i64>) -> String {
    player.and_then(|player| duel.participants.iter().find(|person| person.player == player)).and_then(|person| person.name.clone()).unwrap_or_default()
}

fn side_of_player(duel: &Duel, player: Option<i64>) -> Option<String> {
    player.and_then(|player| duel.participants.iter().find(|person| person.player == player)).and_then(|person| person.side.clone())
}

fn pip<'a>(colour: Color) -> Element<'a, Message> {
    container(Space::new().width(8).height(8)).style(plate(colour, 4.0)).into()
}

fn presence<'a>(w: &Words, duel: &Duel, person: Option<&Participant>) -> Element<'a, Message> {
    let connection = person.map(|person| person.connection.as_str()).unwrap_or_default();
    let app = match connection {
        "present" | "ready" | "playing" => GOOD,
        "lost" => LOST,
        _ => WAIT,
    };
    let room = if phase_of(&duel.state) == Phase::Room { WAIT } else { GOOD };
    row![pip(app), words(w.t("duels-chip-app"), 13.0, MUTED), Space::new().width(10), pip(room), words(w.t("duels-chip-api"), 13.0, MUTED)].spacing(6).align_y(Alignment::Center).into()
}

fn head<'a>(w: &Words, duel: &Duel, caption: String, chips: bool) -> Element<'a, Message> {
    let a = seat(duel, "A");
    let b = seat(duel, "B");
    let name = |person: Option<&Participant>| person.and_then(|person| person.name.clone()).unwrap_or_default();
    let (left_name, right_name) = (name(a), name(b));
    let mut left = column![strong(left_name.clone(), 18.0, SIDE_A)].spacing(4);
    let mut right = column![strong(right_name.clone(), 18.0, SIDE_B)].spacing(4).align_x(Alignment::End);
    if chips {
        left = left.push(presence(w, duel, a));
        right = right.push(presence(w, duel, b));
    }
    let centre = column![bold(format!("{} : {}", duel.wins.a, duel.wins.b), 34.0, INK), words(caption, 13.0, FAINT)].spacing(2).align_x(Alignment::Center);
    let line = row![
        container(row![player_face(&left_name, a.map(|p| p.osu_user_id), 42.0), left].spacing(14).align_y(Alignment::Center)).width(Length::Fill),
        centre,
        container(row![right, player_face(&right_name, b.map(|p| p.osu_user_id), 42.0)].spacing(14).align_y(Alignment::Center)).width(Length::Fill).align_x(iced::alignment::Horizontal::Right),
    ]
    .align_y(Alignment::Center);
    let right_aligned = container(line).padding(Padding { top: 12.0, right: 20.0, bottom: 12.0, left: 20.0 }).width(Length::Fill).style(slab_faded());
    right_aligned.into()
}

pub(crate) fn slot_colour(slot: &str) -> Color {
    let rgb = match slot.get(..2).unwrap_or_default() {
        "NM" => [0x4f, 0xd0, 0xe2],
        "HD" => [0xe0, 0x86, 0x32],
        "HR" => [0xe6, 0x2c, 0x30],
        "DT" => [0x86, 0x4a, 0xe6],
        "FM" => [0xe0, 0x3c, 0x80],
        "EZ" => [0x76, 0xbd, 0x62],
        "FL" => [0xa0, 0x8c, 0xc5],
        _ => [0xd8, 0xd0, 0xcc],
    };
    Color::from_rgb8(rgb[0], rgb[1], rgb[2])
}

pub(crate) fn slot_tag<'a, M: 'a>(slot: &str) -> Element<'a, M> {
    let base = slot_colour(slot);
    container(strong(slot.to_owned(), 12.0, ui::mix(base, Color::WHITE, 0.45)))
        .center_x(42.0)
        .padding([3, 0])
        .style(plate(ui::mix(base, theme::GROUND, 0.74), 6.0))
        .into()
}

fn stars<'a>(entry: &Entry, size: f32) -> Element<'a, Message> {
    let value = entry.stars.unwrap_or_default();
    crate::dossier::star_pill(format!("{value:.2}").replace('.', ","), value, size)
}

pub(crate) fn veil_fill() -> Background {
    let k = ui::fade();
    let tint = |a: f32| Color::from_rgba(0.04, 0.016, 0.027, a * k);
    Background::Gradient(iced::gradient::Linear::new(iced::Radians(PI))
        .add_stop(0.0, tint(0.62))
        .add_stop(0.32, tint(0.32))
        .add_stop(0.6, tint(0.62))
        .add_stop(1.0, tint(0.94))
        .into())
}

fn veil<'a>() -> Element<'a, Message> {
    let fill = veil_fill();
    container(Space::new().width(Length::Fill).height(Length::Fill))
        .style(move |_| container::Style { background: Some(fill), border: Border { radius: 14.0.into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn picture<'a>(state: &State, entry: &Entry, high: f32, wide: f32) -> Element<'a, Message> {
    faded_picture(state, entry, high, wide, 1.0)
}

fn faded_picture<'a>(state: &State, entry: &Entry, high: f32, wide: f32, opacity: f32) -> Element<'a, Message> {
    match entry.beatmapset_id.and_then(|set| state.covers.get(&set)) {
        Some(handle) => ui::framed(handle, wide, high, 14.0).opacity(opacity * ui::fade()).into(),
        None => container(Space::new().width(Length::Fill).height(high)).style(plate(ROW, 14.0)).into(),
    }
}

fn shade<'a>() -> Element<'a, Message> {
    container(Space::new().width(Length::Fill).height(Length::Fill)).style(plate(Color::from_rgba(0.03, 0.012, 0.018, 0.66), 14.0)).into()
}

fn ease(k: f32) -> f32 {
    let k = k.clamp(0.0, 1.0);
    1.0 - (1.0 - k).powi(3)
}

fn liveliness(state: &State, beatmap: i64) -> (f32, f32) {
    let hovered = state.hovered.filter(|(held, _)| *held == beatmap);
    let Some((_, since)) = hovered else { return (0.0, 0.0) };
    let mut lift = ease(since.elapsed().as_secs_f32() / 0.18);
    if state.rested == Some(beatmap) {
        lift *= 1.0 - ease(state.rested_at.map_or(1.0, |at| at.elapsed().as_secs_f32() / 0.3));
    }
    let beat = match (state.hearing == Some(beatmap), state.heard_at, state.tempos.get(&beatmap)) {
        (true, Some(at), Some(bpm)) => (-(at.elapsed().as_secs_f32() * bpm / 60.0).fract() * 5.0).exp(),
        _ => 0.0,
    };
    (lift, beat)
}

#[derive(Clone, Copy)]
struct Motion {
    fill: f32,
    fill_colour: Color,
    shade: f32,
    glare: f32,
    grip: bool,
}

impl Default for Motion {
    fn default() -> Self {
        Motion { fill: 0.0, fill_colour: Color::TRANSPARENT, shade: 1.0, glare: 0.0, grip: false }
    }
}

fn portions<'a>(share: f32, first: Element<'a, Message>, vertical: bool) -> Element<'a, Message> {
    let filled = (share.clamp(0.0, 1.0) * 1000.0).round() as u16;
    if filled == 0 {
        return Space::new().width(Length::Fill).height(Length::Fill).into();
    }
    let first = container(first);
    if filled >= 1000 {
        return first.width(Length::Fill).height(Length::Fill).into();
    }
    if vertical {
        column![first.height(Length::FillPortion(filled)).width(Length::Fill), Space::new().height(Length::FillPortion(1000 - filled))].height(Length::Fill).into()
    } else {
        row![first.width(Length::FillPortion(filled)).height(Length::Fill), Space::new().width(Length::FillPortion(1000 - filled))].width(Length::Fill).into()
    }
}

fn moving_card<'a>(state: &State, entry: &Entry, frame: Option<(Color, bool)>, dim: bool, lively: bool, press: Option<Message>, motion: Motion) -> Element<'a, Message> {
    let faded = dim && entry.slot == "TB";
    let (lift, beat) = if lively { liveliness(state, entry.beatmap_id) } else { (0.0, 0.0) };
    let title = entry.title.clone().unwrap_or_else(|| entry.beatmap_id.to_string());
    let artist = entry.artist.clone().unwrap_or_default();
    let version = entry.version.clone().unwrap_or_default();
    let mapper = entry.mapper.clone().unwrap_or_default();
    let fit = |said: &str, size: f32, room: f32| ui::fitted(said, theme::SANS, size, room);
    let inside = column![
        row![slot_tag(&entry.slot), Space::new().width(Length::Fill), stars(entry, 12.0)].align_y(Alignment::Center),
        Space::new().height(Length::Fill),
        words(fit(&version, 12.0, 230.0), 12.0, MUTED),
        strong(ui::fitted(&title, theme::SANS_SEMI, 15.0, 230.0), 15.0, INK),
        words(fit(&artist, 12.0, 230.0), 12.0, color!(0xd8c4c1)),
        row![crate::duels_screen::mapper_face(&mapper, entry.beatmap_id, 20.0), words(fit(&mapper, 12.0, 200.0), 12.0, color!(0xd8c4c1))].spacing(6).align_y(Alignment::Center),
    ]
    .spacing(2)
    .padding(Padding { top: 10.0, right: 12.0, bottom: 10.0, left: 12.0 })
    .height(CARD_HIGH);
    let flash = container(Space::new().width(Length::Fill).height(Length::Fill)).style(plate(Color::from_rgba(1.0, 0.96, 0.92, 0.13 * beat + 0.35 * motion.glare), 14.0));
    let cover = if faded { faded_picture(state, entry, CARD_HIGH, 300.0, 0.28) } else { picture(state, entry, CARD_HIGH, 300.0) };
    let mut layers: Vec<Element<'a, Message>> = vec![cover, flash.into(), veil(), inside.into()];
    if dim {
        layers.push(portions(motion.shade, shade(), true));
    }
    if motion.fill > 0.0 {
        let bar = container(Space::new().width(Length::Fill).height(5)).style(plate(motion.fill_colour, 2.5));
        layers.push(column![Space::new().height(Length::Fill), container(portions(motion.fill, bar.into(), false)).height(5).padding(Padding { left: 12.0, right: 12.0, ..Padding::ZERO })].padding(Padding { bottom: 7.0, ..Padding::ZERO }).into());
    }
    let layered = iced::widget::Stack::with_children(layers).width(Length::Fill).height(CARD_HIGH);
    let (edge_colour, glows) = frame.unwrap_or((Color::TRANSPARENT, false));
    let edge_colour = fa(edge_colour);
    let width = if frame.is_some() { if glows { 3.0 } else { 1.5 } } else { 0.0 };
    let body: Element<'a, Message> = container(layered).width(Length::Fill).clip(true).style(move |_| container::Style {
        border: Border { color: edge_colour, width, radius: 14.0.into() },
        shadow: iced::Shadow { color: if glows { Color { a: 0.4, ..edge_colour } } else { Color::TRANSPARENT }, offset: iced::Vector::ZERO, blur_radius: if glows { 18.0 } else { 0.0 } },
        ..container::Style::default()
    }).into();
    let pressable: Element<'a, Message> = match press {
        Some(message) => button(body).padding(0).style(|_, _| button::Style::default()).on_press(message).into(),
        None => body,
    };
    if !lively {
        return pressable;
    }
    let mut watched = mouse_area(pressable).on_enter(Message::Hover(Some(entry.beatmap_id))).on_exit(Message::Hover(None));
    if motion.grip {
        watched = watched.on_press(Message::Grip(entry.slot.clone())).on_release(Message::Release).interaction(iced::mouse::Interaction::Pointer);
    }
    iced::widget::float(watched).scale(1.0 + 0.03 * lift + 0.02 * beat).into()
}

fn grid<'a>(cells: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut rows = column![].spacing(10);
    let mut cells = cells.into_iter().peekable();
    while cells.peek().is_some() {
        let mut line = row![].spacing(10);
        for _ in 0..3 {
            match cells.next() {
                Some(cell) => line = line.push(container(cell).width(Length::Fill)),
                None => line = line.push(Space::new().width(Length::Fill)),
            }
        }
        rows = rows.push(line);
    }
    rows.into()
}

fn edge<'a>(colour: Color, left: bool) -> Element<'a, Message> {
    let colour = fa(colour);
    container(Space::new().width(Length::Fill).height(Length::Fill))
        .style(move |_| container::Style {
            background: Some(Background::Gradient(iced::gradient::Linear::new(iced::Radians(if left { FRAC_PI_2 } else { FRAC_PI_2 * 3.0 }))
                .add_stop(0.0, Color { a: 0.26 * colour.a, ..colour })
                .add_stop(0.16, Color { a: 0.09 * colour.a, ..colour })
                .add_stop(0.34, Color { a: 0.0, ..colour })
                .into())),
            ..container::Style::default()
        })
        .into()
}

fn status<'a>(heading: String, right: Vec<Element<'a, Message>>, lit: Option<Color>, progress: Option<f32>) -> Element<'a, Message> {
    let mut line = row![strong(heading, 18.0, INK), Space::new().width(Length::Fill)].spacing(14).align_y(Alignment::Center);
    for part in right {
        line = line.push(part);
    }
    let mut body = column![line].spacing(10);
    if let Some(fraction) = progress {
        body = body.push(track(fraction, lit.unwrap_or(theme::ACCENT)));
    }
    container(body)
        .padding(Padding { top: 14.0, right: 18.0, bottom: 14.0, left: 18.0 })
        .width(640)
        .style({
            let base = slab_faded();
            let lit = lit.map(|colour| fa(Color { a: 0.55, ..colour }));
            move |theme| {
                let mut style = base(theme);
                if let Some(colour) = lit {
                    style.border = Border { color: colour, width: 1.5, radius: 16.0.into() };
                }
                style
            }
        })
        .into()
}

fn centred<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    container(inside).center_x(Length::Fill).into()
}

fn left_clock<'a>(duel: &Duel, now: i64) -> Option<Element<'a, Message>> {
    seconds_left(duel, now).map(|left| bold(clock(left), 22.0, CLOCK))
}

pub(crate) fn sign_tile<'a, M: 'a>(icon: Icon, colour: Color) -> Element<'a, M> {
    let (fill, edge) = (fa(ROW), fa(Color::from_rgba(1.0, 1.0, 1.0, 0.06)));
    container(glyph(icon, 20.0, colour)).center_x(40.0).center_y(40.0).style(move |_| container::Style {
        background: Some(Background::Color(fill)),
        border: Border { color: edge, width: 1.0, radius: 10.0.into() },
        ..container::Style::default()
    }).into()
}

pub(crate) fn track<'a, M: 'a>(share: f32, colour: Color) -> Element<'a, M> {
    let filled = (share.clamp(0.0, 1.0) * 1000.0).round() as u16;
    let bar = row![
        container(Space::new()).width(Length::FillPortion(filled.max(1))).height(6).style(plate(colour, 3.0)),
        Space::new().width(Length::FillPortion((1000 - filled).max(1))),
    ];
    container(bar).width(Length::Fill).style(plate(Color::from_rgba(1.0, 1.0, 1.0, 0.08), 3.0)).into()
}

fn deadline_total(duel: &Duel) -> Option<i64> {
    duel.deadlines.iter().find_map(|deadline| match deadline.kind.as_str() {
        "room_join" => Some(600),
        _ => None,
    })
}

fn room_view<'a>(w: &Words, state: &State, duel: &Duel, now: i64) -> Element<'a, Message> {
    let waiting = matches!(duel.state.as_str(), "WAITING_PLAYERS" | "ROOM_SETUP");
    let heading = if waiting { w.t("duels-room-waiting-players") } else { w.t("duels-room-opening") };
    let mut lines = column![row![sign_tile(Icon::Clock, WAIT), strong(heading, 20.0, INK)].spacing(14).align_y(Alignment::Center)].spacing(14);
    let left = seconds_left(duel, now);
    match (left, deadline_total(duel)) {
        (Some(left), Some(total)) => {
            let spent = (total - left).clamp(0, total);
            lines = lines.push(column![
                row![words(w.t("duels-room-waiting"), 13.0, MUTED), Space::new().width(Length::Fill), strong(w.with("duels-room-of", &[("spent", clock(spent)), ("total", clock(total))]), 13.0, INK)].align_y(Alignment::Center),
                track(spent as f32 / total.max(1) as f32, CLOCK),
            ].spacing(8));
        }
        (Some(left), None) => {
            lines = lines.push(row![words(w.t("duels-room-left"), 13.0, MUTED), Space::new().width(Length::Fill), bold(clock(left), 16.0, CLOCK)].align_y(Alignment::Center));
        }
        _ => {}
    }
    if let Some(room) = &duel.room {
        let password = room.password.clone().unwrap_or_default();
        lines = lines.push(row![
            column![words(w.t("duels-room-name"), 13.0, FAINT), strong(room.name.clone(), 15.0, INK)].spacing(3).width(Length::Fill),
            column![words(w.t("duels-room-password"), 13.0, FAINT), strong(password.clone(), 15.0, INK)].spacing(3),
            outlined(strong(w.t("duels-copy"), theme::BODY, INK), Some(Message::Copy(password))),
        ].spacing(18).align_y(Alignment::End));
    }
    let mut buttons = row![].spacing(10);
    if duel.actions.iter().any(|action| action == "reinvite") {
        buttons = buttons.push(ui::primary(w.t("duels-reinvite"), (!state.acting).then(|| Message::Act("reinvite", serde_json::json!({})))));
    }
    if duel.actions.iter().any(|action| action == "surrender") {
        buttons = buttons.push(outlined(strong(w.t("duels-cancel-duel"), theme::BODY, INK), (!state.acting).then(|| Message::Act("surrender", serde_json::json!({})))));
    }
    lines = lines.push(buttons);
    centred(container(lines).padding(22).width(660).style(slab_faded()).into())
}

fn entries(duel: &Duel) -> Vec<Entry> {
    duel.pool.as_ref().map(|pool| pool.entries.clone()).unwrap_or_default()
}

fn reveal_timing(state: &State) -> Option<f32> {
    let moments = &state.moments;
    (moments.phase == Some(Phase::Reveal)).then(|| crate::duels_screen::since(moments.phase_at)).filter(|t| *t < crate::duels_screen::REVEAL)
}

fn reveal_delay(index: usize, slot: &str) -> f32 {
    0.3 + index as f32 * 0.07 + if slot == "TB" { 0.3 } else { 0.0 }
}

fn reveal_view<'a>(w: &Words, state: &State, duel: &Duel, now: i64) -> Element<'a, Message> {
    let timing = reveal_timing(state);
    let after = timing.map_or(1.0, |t| ease((t - (reveal_delay(8, "TB") + 0.45)) / 0.4));
    let mut header = row![strong(w.t("duels-pool"), 20.0, INK), Space::new().width(Length::Fill)].spacing(14).align_y(Alignment::Center);
    if let Some(clock) = ui::fading(after, || left_clock(duel, now)) {
        header = header.push(clock);
    }
    let sets: Vec<i64> = entries(duel).iter().filter_map(|entry| entry.beatmapset_id).collect();
    let fetching = state.fetching;
    let made = ui::fading(after, || {
        let download = row![glyph(Icon::Download, 16.0, Color::WHITE), strong(w.t(if fetching { "duels-downloading" } else { "duels-download-all" }), theme::BODY, Color::WHITE)].spacing(8).align_y(Alignment::Center);
        let made = button(container(download).center_y(theme::CONTROL_HEIGHT - 2.0)).padding([0, 14]).style(ui::button_faded(theme::primary));
        if fetching || sets.is_empty() { Element::from(made) } else { Element::from(made.on_press(Message::Download(sets))) }
    });
    header = header.push(made);
    let cells = entries(duel).iter().enumerate().map(|(index, entry)| {
        let k = timing.map_or(1.0, |t| ease((t - reveal_delay(index, &entry.slot)) / 0.75));
        ui::fading(k, || moving_card(state, entry, None, false, true, None, Motion { glare: 1.0 - k, ..Motion::default() }))
    }).collect();
    let mut page = column![header, grid(cells)].spacing(12).height(Length::Fill);
    if let Some(room) = &duel.room {
        let password = room.password.clone().unwrap_or_default();
        page = page.push(Space::new().height(Length::Fill)).push(row![
            words(w.t("duels-room-name"), 14.0, FAINT),
            strong(room.name.clone(), 14.0, INK),
            Space::new().width(18),
            words(w.t("duels-room-password"), 14.0, FAINT),
            strong(password.clone(), 14.0, INK),
            Space::new().width(18),
            mouse_area(strong(w.t("duels-copy"), 14.0, MUTED)).on_press(Message::Copy(password)).interaction(iced::mouse::Interaction::Pointer),
        ].spacing(10).align_y(Alignment::Center));
    }
    page.into()
}

fn draft_view<'a>(w: &Words, state: &State, duel: &Duel, now: i64) -> (Element<'a, Message>, Option<(Color, bool)>) {
    let at = duel.draft.len().min(ORDER.len());
    let acting = ORDER.get(at).copied();
    let mine = state.side_of(duel);
    let actor_colour = acting.map(|(_, side)| side_colour(side)).unwrap_or(GOOD);
    let my_turn = acting.is_some_and(|(kind, _)| duel.actions.iter().any(|action| action == kind));
    let prompt = match acting {
        Some((kind, side)) if Some(side) == mine.as_deref() => w.t(if kind == "ban" { "duels-your-ban" } else { "duels-your-pick" }),
        Some((kind, side)) => w.with(if kind == "ban" { "duels-their-ban" } else { "duels-their-pick" }, &[("name", seat(duel, side).and_then(|person| person.name.clone()).unwrap_or_default())]),
        None => w.t("duels-pool-ready"),
    };
    let mut header = row![strong(prompt, 20.0, actor_colour), Space::new().width(Length::Fill)].spacing(14).align_y(Alignment::Center);
    let sets = entries(duel).iter().filter_map(|entry| entry.beatmapset_id).collect::<Vec<_>>();
    header = header.push(outlined(strong(w.t(if state.fetching { "duels-downloading" } else { "duels-download-all" }), theme::BODY, INK),
        (!state.fetching && !sets.is_empty()).then_some(Message::Download(sets))));
    if let Some(clock) = left_clock(duel, now) {
        header = header.push(clock);
    }
    let strip = container(header).padding(Padding { top: 12.0, right: 18.0, bottom: 12.0, left: 18.0 }).width(Length::Fill).style(slab_faded());
    let chips = row(ORDER.iter().enumerate().map(|(index, (kind, side))| {
        let colour = side_colour(side);
        let done = duel.draft.get(index);
        let label = done.map(|turn| turn.slot.clone()).unwrap_or_else(|| w.t(if *kind == "ban" { "duels-ban" } else { "duels-pick" }));
        let (fill, ink) = if done.is_some() {
            (ui::mix(colour, theme::GROUND, 0.8), INK)
        } else if index == at {
            (colour, Color::WHITE)
        } else {
            (ui::mix(colour, theme::GROUND, 0.92), FAINT)
        };
        container(strong(label, 13.0, ink)).center_x(Length::Fill).padding([7, 0]).style(plate(fill, 9.0)).into()
    }).collect::<Vec<Element<'a, Message>>>()).spacing(6);
    let cells = entries(duel).iter().map(|entry| {
        let taken = duel.draft.iter().find(|turn| turn.slot == entry.slot);
        let mark = taken.and_then(|turn| side_of_player(duel, turn.by)).map(|side| side_colour(&side));
        let free = taken.is_none() && entry.slot != "TB" && !entry.used && entry.banned_by.is_none();
        let banned = taken.is_some_and(|turn| turn.kind == "ban") || entry.banned_by.is_some();
        let frame = match (entry.slot.as_str(), mark) {
            ("TB", _) => None,
            (_, Some(colour)) => Some((colour, !banned)),
            _ if free && acting.is_some() => Some((Color { a: 0.45, ..actor_colour }, false)),
            _ => None,
        };
        let moments = &state.moments;
        let latest = duel.draft.last().filter(|turn| turn.slot == entry.slot && duel.draft.len() == moments.turns);
        let fresh = latest.map(|turn| (crate::duels_screen::since(moments.turn_at), Some(turn.by) != Some(state.player)));
        let mut motion = Motion { grip: free && my_turn, fill_colour: actor_colour, ..Motion::default() };
        if let Some((slot, at)) = &moments.hold {
            if *slot == entry.slot {
                motion.fill = (at.elapsed().as_secs_f32() / crate::duels_screen::HOLD).min(1.0);
            }
        }
        if let Some((slot, share, at)) = &moments.let_go {
            if *slot == entry.slot && motion.fill == 0.0 {
                motion.fill = share * (1.0 - ease(at.elapsed().as_secs_f32() / crate::duels_screen::LET_GO));
            }
        }
        if moments.sent.as_deref() == Some(entry.slot.as_str()) {
            motion.fill = 1.0;
        }
        if let Some((t, theirs)) = fresh {
            let start = if theirs { 0.5 } else { 0.0 };
            if theirs && t < start {
                motion.fill = ease(t / start);
                motion.fill_colour = mark.unwrap_or(actor_colour);
            }
            let k = ease((t - start) / 0.45);
            motion.shade = k;
        }
        let frame = if motion.fill > 0.0 { Some((motion.fill_colour, true)) } else { frame };
        moving_card(state, entry, frame, banned || entry.slot == "TB", free, None, motion)
    }).collect();
    let body = column![strip, chips, grid(cells)].spacing(12).into();
    (body, acting.map(|(_, side)| (side_colour(side), side == "A")))
}

fn current_entry(duel: &Duel) -> Option<Entry> {
    let slot = duel.rounds.last()?.slot.clone();
    entries(duel).into_iter().find(|entry| entry.slot == slot)
}

fn hero<'a>(w: &Words, state: &State, duel: &Duel) -> Element<'a, Message> {
    let Some(entry) = current_entry(duel) else { return Space::new().height(HERO_HIGH).into() };
    let picker = duel.rounds.last().and_then(|round| round.picked_by);
    let note = match picker {
        Some(player) => w.with("duels-picked-by", &[("name", name_of(duel, Some(player)))]),
        None => w.t("duels-decider"),
    };
    let inside = column![
        row![slot_tag(&entry.slot), stars(&entry, 14.0), Space::new().width(Length::Fill), words(note, 14.0, color!(0xd8c4c1))].spacing(10).align_y(Alignment::Center),
        Space::new().height(Length::Fill),
        words(entry.version.clone().unwrap_or_default(), 15.0, MUTED),
        strong(ui::fitted(&entry.title.clone().unwrap_or_default(), theme::SANS_SEMI, 30.0, 640.0), 30.0, INK),
        words(entry.artist.clone().unwrap_or_default(), 16.0, color!(0xd8c4c1)),
        row![crate::duels_screen::mapper_face(&entry.mapper.clone().unwrap_or_default(), entry.beatmap_id, 22.0), words(entry.mapper.clone().unwrap_or_default(), 14.0, color!(0xd8c4c1))].spacing(8).align_y(Alignment::Center),
    ]
    .spacing(4)
    .padding(Padding { top: 18.0, right: 22.0, bottom: 18.0, left: 22.0 })
    .height(HERO_HIGH);
    let layered = stack![picture(state, &entry, HERO_HIGH, 900.0), veil(), inside].width(Length::Fill).height(HERO_HIGH);
    let framed = container(layered).width(Length::Fill).clip(true).style(|_| container::Style { border: Border { radius: 16.0.into(), ..Border::default() }, ..container::Style::default() });
    mouse_area(framed).on_enter(Message::Hover(Some(entry.beatmap_id))).on_exit(Message::Hover(None)).into()
}

fn players<'a>(w: &Words, duel: &Duel) -> Element<'a, Message> {
    let line = |side: &str| -> Element<'a, Message> {
        let person_ = seat(duel, side);
        let name = person_.and_then(|person| person.name.clone()).unwrap_or_default();
        let (said, colour) = match person_.map(|person| person.connection.as_str()).unwrap_or_default() {
            "ready" => (w.t("duels-player-ready"), GOOD),
            "playing" if duel.state == "VERIFYING" => (w.t("duels-player-done"), GOOD),
            "playing" => (w.t("duels-player-playing"), MUTED),
            "lost" => (w.t("duels-player-lost"), LOST),
            _ => (w.t("duels-player-preparing"), WAIT),
        };
        container(row![player_face(&name, person_.map(|p| p.osu_user_id), 34.0), strong(name.clone(), 16.0, side_colour(side)), Space::new().width(Length::Fill), strong(said, 14.0, colour)].spacing(12).align_y(Alignment::Center))
            .padding(Padding { top: 10.0, right: 16.0, bottom: 10.0, left: 16.0 })
            .width(Length::Fill)
            .style(slab_faded())
            .into()
    };
    row![line("A"), line("B")].spacing(12).into()
}

fn round_status<'a>(w: &Words, state: &State, duel: &Duel, now: i64) -> Element<'a, Message> {
    let left = left_clock(duel, now);
    let ready = duel.actions.iter().any(|action| action == "ready");
    match duel.state.as_str() {
        "ROUND_SETUP" | "WAITING_READY" | "STARTING" => {
            let mut right: Vec<Element<'a, Message>> = left.into_iter().collect();
            if ready {
                right.push(ui::primary(w.t("duels-ready"), (!state.acting).then(|| Message::Act("ready", serde_json::json!({})))));
            }
            status(w.t(if duel.state == "STARTING" { "duels-starting" } else { "duels-preparing" }), right, None, None)
        }
        "PLAYING" => {
            let total = current_entry(duel).and_then(|entry| entry.song_seconds).map(|seconds| seconds.round() as i64);
            let spent = state.played_ms.map(|ms| (ms / 1000).max(0));
            match (spent, total) {
                (Some(spent), Some(total)) if total > 0 => status(w.t("duels-map-running"), vec![strong(w.with("duels-room-of", &[("spent", clock(spent.min(total))), ("total", clock(total))]), 14.0, INK)], None, Some(spent as f32 / total as f32)),
                _ => status(w.t("duels-map-running"), Vec::new(), None, None),
            }
        }
        "VERIFYING" => status(w.t("duels-waiting-match"), left.into_iter().collect(), None, Some(1.0)),
        "ABORTING" | "PAUSED" => status(w.t("duels-stopped"), left.into_iter().collect(), Some(LOST), None),
        _ => status(w.t("duels-recovering"), left.into_iter().collect(), Some(WAIT), None),
    }
}

fn round_view<'a>(w: &Words, state: &State, duel: &Duel, now: i64) -> Element<'a, Message> {
    let status_line = round_status(w, state, duel, now);
    let tie = duel.rounds.last().map(|round| round.ties).unwrap_or_default();
    let mut body = column![hero(w, state, duel), players(w, duel)].spacing(12);
    if tie > 0 {
        body = body.push(centred(status(w.with("duels-replay", &[("count", tie.to_string())]), Vec::new(), Some(WAIT), None)));
    }
    body.push(centred(status_line)).into()
}

fn pill<'a>(said: String, on: bool, press: Message) -> Element<'a, Message> {
    let (picked, row_fill, picked_edge, edge) = (fa(theme::GLASS_PICKED), fa(ROW), fa(color!(0xb33a3a)), fa(OUTLINE));
    button(container(strong(said, 13.0, if on { INK } else { MUTED })).center_y(30.0))
        .padding([0, 12])
        .style(move |_, status| {
            let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if on { picked } else if lit { row_fill } else { Color::TRANSPARENT })),
                text_color: INK,
                border: Border { color: if on { picked_edge } else { edge }, width: if on { 1.5 } else { 1.0 }, radius: theme::CONTROL_RADIUS.into() },
                ..button::Style::default()
            }
        })
        .on_press(press)
        .into()
}

fn wide_button<'a>(said: String, main: bool, on: Option<Message>) -> Element<'a, Message> {
    let primary = ui::button_faded(theme::primary);
    let (row_fill, edge) = (fa(ROW), fa(OUTLINE));
    let made = button(container(strong(said, theme::BODY, if main { Color::WHITE } else { INK })).center_x(Length::Fill).center_y(theme::CONTROL_HEIGHT - 2.0))
        .width(Length::Fill)
        .padding(0)
        .style(move |theme, status| {
            if main {
                return primary(theme, status);
            }
            let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if lit { row_fill } else { Color::TRANSPARENT })),
                text_color: INK,
                border: Border { color: edge, width: 1.0, radius: theme::CONTROL_RADIUS.into() },
                ..button::Style::default()
            }
        });
    match on {
        Some(message) => made.on_press(message).into(),
        None => made.into(),
    }
}

fn result_view<'a>(w: &Words, state: &State, duel: &Duel, now: i64) -> Element<'a, Message> {
    if duel.state != "COMPLETED" {
        let reason = match duel.outcome.as_deref() {
            Some("pool_unavailable") => Some("duels-pool-unavailable"),
            Some("room_unresponsive") => Some("duels-room-unresponsive"),
            Some("room_lost") => Some("duels-room-lost"),
            Some("evidence_missing") => Some("duels-evidence-missing"),
            _ => None,
        };
        let mut message = column![bold(w.t("duels-no-result"), 32.0, INK)].spacing(18);
        if let Some(reason) = reason {
            message = message.push(words(w.t(reason), 17.0, MUTED));
        }
        message = message.push(ui::primary(w.t("duels-return"), Some(Message::Close)));
        return container(container(message).padding(28).width(Length::Fill).max_width(640).style(slab_faded()))
            .center_x(Length::Fill).center_y(Length::Fill).into();
    }
    let moments = &state.moments;
    let t = if moments.phase == Some(Phase::Result) && moments.from.is_some_and(|from| from != Phase::Result) { crate::duels_screen::since(moments.phase_at) / crate::duels_screen::RESULT * 1.03 } else { 2.0 };
    let span = |from: f32, to: f32| ease((t - from) / (to - from));
    let me = state.player;
    let mine = state.side_of(duel).unwrap_or_else(|| "A".into());
    let theirs = if mine == "A" { "B" } else { "A" };
    let (verdict, colour) = match (duel.state.as_str(), duel.winner) {
        ("COMPLETED", Some(winner)) if Some(winner) == me => (w.t("duels-won-title"), GOLD),
        ("COMPLETED", Some(_)) => (w.t("duels-lost-title"), LOST),
        _ => (w.t("duels-no-result"), INK),
    };
    let (mine_wins, their_wins) = if mine == "A" { (duel.wins.a, duel.wins.b) } else { (duel.wins.b, duel.wins.a) };
    let caption = if duel.mode == "direct" { w.t("duels-friendly") } else { w.t("duels-rated") };
    let me_name = seat(duel, &mine).and_then(|person| person.name.clone()).unwrap_or_default();
    let them_name = seat(duel, theirs).and_then(|person| person.name.clone()).unwrap_or_default();
    let mut rows = column![ui::fading(span(0.5, 0.6), || row![Space::new().width(Length::Fill), container(strong(me_name.clone(), 13.0, side_colour(&mine))).width(110).align_x(iced::alignment::Horizontal::Right), container(strong(them_name.clone(), 13.0, side_colour(theirs))).width(110).align_x(iced::alignment::Horizontal::Right)].spacing(14).padding(Padding { right: 14.0, ..Padding::ZERO }))].spacing(6);
    let pool = entries(duel);
    for (index, round) in duel.rounds.iter().enumerate() {
        let row_k = span(0.56 + index as f32 * 0.04, 0.68 + index as f32 * 0.04);
        let made = ui::fading(row_k, || -> Element<'a, Message> {
        let entry = pool.iter().find(|entry| entry.slot == round.slot);
        let mine_player = seat(duel, &mine).map(|person| person.player);
        let their_player = seat(duel, theirs).map(|person| person.player);
        let score = |player: Option<i64>| player.and_then(|player| round.scores.get(&player.to_string())).map(|played| w.lang().group(played.score.max(0) as u64)).unwrap_or_default();
        let won = round.winner.is_some() && round.winner == mine_player;
        let lost = round.winner.is_some() && round.winner == their_player;
        let picker = round.picked_by.map(|player| (name_of(duel, Some(player)), side_of_player(duel, Some(player)).unwrap_or_default()));
        let title = entry.and_then(|entry| entry.title.clone()).unwrap_or_default();
        let pick_line: Element<'a, Message> = match picker {
            Some((name, side)) => row![words(w.t("duels-picked"), 12.0, FAINT), strong(name, 12.0, side_colour(&side))].spacing(4).into(),
            None => words(w.t("duels-decider"), 12.0, FAINT),
        };
        let thumb: Element<'a, Message> = match entry.and_then(|entry| entry.beatmapset_id).and_then(|set| state.covers.get(&set)) {
            Some(handle) => ui::framed(handle, 92.0, 46.0, 8.0).width(92).opacity(ui::fade()).into(),
            None => container(Space::new()).width(92).height(46).style(plate(ROW, 8.0)).into(),
        };
        let line = row![
            thumb,
            slot_tag(&round.slot),
            column![iced::widget::text(ui::fitted(&title, theme::SANS_SEMI, 15.0, 240.0)).size(15.0).font(theme::SANS_SEMI).color(fa(INK)).wrapping(iced::widget::text::Wrapping::None), pick_line].spacing(1).width(Length::Fill),
            container(strong(score(mine_player), 16.0, if won { WON } else { MUTED })).width(110).align_x(iced::alignment::Horizontal::Right),
            container(strong(score(their_player), 16.0, if lost { WON } else { MUTED })).width(110).align_x(iced::alignment::Horizontal::Right),
        ].spacing(14).align_y(Alignment::Center);
        container(line).padding(Padding { top: 10.0, right: 14.0, bottom: 10.0, left: 14.0 }).style(plate(ROW, 12.0)).into()
        });
        rows = rows.push(made);
    }
    let verdict_wide = ui::text_width(&verdict, theme::SANS_SEMI, 64.0) + 4.0;
    let wiped = container(iced::widget::text(verdict.clone()).size(64.0).font(theme::SANS_SEMI).color(fa(colour)).wrapping(iced::widget::text::Wrapping::None)).width(verdict_wide * span(0.38, 0.56)).clip(true);
    let header_head = row![container(wiped).width(verdict_wide), ui::fading(span(0.36, 0.46), || bold(format!("{mine_wins} : {their_wins}"), 40.0, INK))].spacing(20).align_y(Alignment::End);
    let left = column![
        ui::fading(span(0.38, 0.5), || words(caption, 15.0, FAINT)),
        header_head,
        ui::fading(span(0.5, 0.62), || row![player_face(&me_name, seat(duel, &mine).map(|p| p.osu_user_id), 32.0), strong(me_name.clone(), 15.0, side_colour(&mine)), words(w.t("duels-and"), 14.0, FAINT), player_face(&them_name, seat(duel, theirs).map(|p| p.osu_user_id), 32.0), strong(them_name.clone(), 15.0, side_colour(theirs))].spacing(10).align_y(Alignment::Center)),
        ui::fading(span(0.5, 0.6), || container(rows).padding(14).width(Length::Fill).style(slab_faded())),
    ]
    .spacing(16)
    .width(Length::Fill);
    let aside_k = span(0.72, 0.84);
    let aside = ui::fading(aside_k, || {
        let mut aside = column![].spacing(12).width(360);
        let group = |value: i64| w.lang().group(value.max(0) as u64);
        if let Some(projection) = &duel.rating {
            let change = projection.after - projection.before;
            let (sign, tint) = match change {
                1.. => (format!("+{change}"), WON),
                0 => ("0".to_owned(), FAINT),
                _ => (format!("−{}", -change), LOST),
            };
            let counted = projection.before + ((projection.after - projection.before) as f32 * span(0.8, 0.95)).round() as i64;
            let line = row![words(group(projection.before), 17.0, MUTED), words("→".to_owned(), 17.0, FAINT), bold(group(counted), 32.0, INK), ui::fading(span(0.95, 1.0), || strong(sign, 17.0, tint))].spacing(10).align_y(Alignment::Center);
            aside = aside.push(container(column![words(w.t("duels-rating"), 14.0, FAINT), line].spacing(6)).padding(18).width(Length::Fill).style(slab_faded()));
        } else if let Some(me) = &state.me {
            let rating = me.rating.rating.map(group).unwrap_or_else(|| "?".into());
            aside = aside.push(container(column![words(w.t("duels-rating"), 14.0, FAINT), bold(rating, 32.0, INK)].spacing(6)).padding(18).width(Length::Fill).style(slab_faded()));
        }
        if duel.actions.iter().any(|action| action == "bonus") {
            let mut top = row![strong(w.t("duels-bonus"), 18.0, INK), Space::new().width(Length::Fill)].align_y(Alignment::Center);
            if let Some(clock) = left_clock(duel, now) {
                top = top.push(clock);
            }
            let mut panel = column![
                top,
                row![
                    wide_button(w.t("duels-no"), false, (!state.acting).then(|| Message::Act("bonus-tb", serde_json::json!({ "accept": false })))),
                    wide_button(w.t("duels-yes"), true, (!state.acting).then(|| Message::Act("bonus-tb", serde_json::json!({ "accept": true })))),
                ].spacing(8),
            ].spacing(12);
            let linger = duel.deadlines.iter().filter(|deadline| deadline.kind == "room_linger").filter_map(|deadline| deadline.at.as_deref()).filter_map(|at| chrono::DateTime::parse_from_rfc3339(at).ok()).map(|at| (at.timestamp() - now).max(0)).min();
            if let Some(open) = linger {
                panel = panel.push(words(w.with("duels-room-open", &[("left", clock(open))]), 13.0, FAINT));
            }
            aside = aside.push(container(panel).padding(18).width(Length::Fill).style({
                let base = slab_faded();
                let gold = fa(Color { a: 0.45, ..GOLD });
                move |theme| {
                    let mut style = base(theme);
                    style.border = Border { color: gold, width: 1.5, radius: 16.0.into() };
                    style
                }
            }));
        }
        if let Some(round) = duel.rounds.first().filter(|_| duel.state == "COMPLETED") {
            let slot = round.slot.clone();
            let held = state.rated.get(&slot).cloned().unwrap_or_default();
            let fit = |value: &'static str, label: String| pill(label, held.0.as_deref() == Some(value), Message::Rate(slot.clone(), Some(value.into()), None));
            let hard = |value: &'static str, label: String| pill(label, held.1.as_deref() == Some(value), Message::Rate(slot.clone(), None, Some(value.into())));
            aside = aside.push(container(column![
                strong(w.t("duels-rate-pool"), 18.0, INK),
                words(w.with("duels-rate-fit", &[("slot", slot.clone())]), 14.0, INK),
                row![fit("fits", w.t("duels-yes")), fit("off", w.t("duels-no"))].spacing(8),
                words(w.t("duels-rate-hard"), 14.0, INK),
                row![hard("easy", w.t("duels-easy")), hard("fine", w.t("duels-fine")), hard("hard", w.t("duels-hard"))].spacing(8),
            ].spacing(10)).padding(18).width(Length::Fill).style(slab_faded()));
        }
        let them = seat(duel, theirs);
        if duel.mode == "direct" && duel.state == "COMPLETED" {
            if let Some(them) = them {
                aside = aside.push(row![
                    wide_button(w.t("duels-rematch"), true, (!state.acting).then_some(Message::Challenge(them.player))),
                    wide_button(w.t("duels-profile"), false, Some(Message::Profile(them.osu_user_id))),
                ].spacing(8));
            }
        }
        aside
    });
    let rows_built = column![Space::new().height(Length::Fill), row![left, aside].spacing(28).align_y(Alignment::End)].height(Length::Fill);
    let big_k = 1.0 - span(0.32, 0.42);
    if big_k <= 0.0 || duel.winner.is_none() {
        return rows_built.into();
    }
    let last_mine = duel.rounds.last().and_then(|round| round.winner).is_some_and(|winner| Some(winner) == seat(duel, &mine).map(|person| person.player));
    let flip = span(0.14, 0.2);
    let (shown_mine, shown_theirs) = if flip < 0.5 { if last_mine { (mine_wins.saturating_sub(1), their_wins) } else { (mine_wins, their_wins.saturating_sub(1)) } } else { (mine_wins, their_wins) };
    let pulse = 1.0 + 0.08 * (PI * flip).sin();
    let big = ui::fading(big_k, || row![bold(shown_mine.to_string(), 150.0, INK), bold(":".to_owned(), 150.0, INK), bold(shown_theirs.to_string(), 150.0, INK)].spacing(24).align_y(Alignment::Center));
    let big = container(iced::widget::float(big).scale(pulse.max(1.0001))).center_x(Length::Fill).center_y(Length::Fill);
    stack![rows_built, big].width(Length::Fill).height(Length::Fill).into()
}

struct Halves {
    slide: f32,
    seam: f32,
    alpha: f32,
}

impl canvas::Program<Message> for Halves {
    type State = ();

    fn draw(&self, _: &(), renderer: &iced::Renderer, _: &iced::Theme, bounds: iced::Rectangle, _: iced::mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let (wide, high) = (bounds.width, bounds.height);
        let (top, bottom) = (0.57 * wide, 0.43 * wide);
        let shift = (1.0 - self.slide) * 1.05 * wide;
        let radius = 18.0_f32.min(high / 2.0);
        let at = |x: f32, y: f32, offset: f32| iced::Point::new(x + offset, y);
        let left_half = |offset: f32| {
            let mut path = canvas::path::Builder::new();
            path.move_to(at(0.0, radius, offset));
            path.arc_to(at(0.0, 0.0, offset), at(radius, 0.0, offset), radius);
            path.line_to(at(top, 0.0, offset));
            path.line_to(at(bottom, high, offset));
            path.line_to(at(radius, high, offset));
            path.arc_to(at(0.0, high, offset), at(0.0, high - radius, offset), radius);
            path.close();
            path.build()
        };
        let right_half = |offset: f32| {
            let mut path = canvas::path::Builder::new();
            path.move_to(at(top, 0.0, offset));
            path.line_to(at(wide - radius, 0.0, offset));
            path.arc_to(at(wide, 0.0, offset), at(wide, radius, offset), radius);
            path.line_to(at(wide, high - radius, offset));
            path.arc_to(at(wide, high, offset), at(wide - radius, high, offset), radius);
            path.line_to(at(bottom, high, offset));
            path.close();
            path.build()
        };
        let tint = |colour: Color, from: iced::Point, to: iced::Point| {
            canvas::Fill::from(canvas::gradient::Linear::new(from, to)
                .add_stop(0.0, Color { a: 0.2 * self.alpha, ..colour })
                .add_stop(0.6, Color { a: 0.045 * self.alpha, ..colour })
                .add_stop(1.0, Color { a: 0.045 * self.alpha, ..colour }))
        };
        frame.fill(&left_half(-shift), tint(SIDE_A, iced::Point::ORIGIN, iced::Point::new(wide * 0.6, high * 0.3)));
        frame.fill(&right_half(shift), tint(SIDE_B, iced::Point::new(wide, 0.0), iced::Point::new(wide * 0.4, high * 0.3)));
        if self.seam > 0.0 {
            let seam = canvas::Path::line(iced::Point::new(top, 0.0), iced::Point::new(bottom, high));
            let stroke = canvas::Stroke::default().with_width(3.0).with_line_cap(canvas::LineCap::Round).with_color(Color { a: 0.85 * self.seam * self.alpha, ..INK });
            frame.stroke(&seam, stroke);
        }
        vec![frame.into_geometry()]
    }
}

fn meeting_side<'a>(w: &Words, duel: &Duel, side: &str, k: f32) -> Element<'a, Message> {
    let person_ = seat(duel, side);
    let name = person_.and_then(|person| person.name.clone()).unwrap_or_default();
    let record = person_.and_then(|person| duel.meeting.as_ref()?.players.get(&person.player.to_string()).cloned()).unwrap_or_default();
    let left = side == "A";
    let align = if left { Alignment::Start } else { Alignment::End };
    let block = ui::fading(k, || {
        let rating: Element<'a, Message> = match record.rating {
            Some(value) => row![bold(w.lang().group(value.max(0) as u64), 34.0, INK), words(format!("± {}", record.rd.unwrap_or_default()), 20.0, MUTED)].spacing(12).align_y(Alignment::End).into(),
            None => bold(w.t("duels-calibration"), 28.0, INK),
        };
        let tally = |value: u32, label: String, colour: Color| -> Element<'a, Message> { column![bold(value.to_string(), 26.0, colour), words(label, 14.0, FAINT)].spacing(2).align_x(align).into() };
        column![
            player_face(&name, person_.map(|p| p.osu_user_id), 128.0),
            bold(name.clone(), 46.0, side_colour(side)),
            rating,
            row![tally(record.won, w.t("duels-won"), GOOD), tally(record.lost, w.t("duels-lost"), LOST)].spacing(22),
        ]
        .spacing(18)
        .align_x(align)
    });
    let placed = container(block).width(Length::Fill).padding(Padding { top: 120.0, left: 70.0, right: 70.0, bottom: 0.0 });
    if left { placed.into() } else { placed.align_x(iced::alignment::Horizontal::Right).into() }
}

fn meeting_view<'a>(w: &Words, duel: &Duel, t: f32) -> Element<'a, Message> {
    let out = 1.0 - ease((t - 3.7) / 0.55);
    let caption = if duel.mode == "direct" { w.t("duels-friendly") } else { w.t("duels-rated") };
    let halves = canvas::Canvas::new(Halves { slide: ease(t / 0.7), seam: ease((t - 0.55) / 0.5), alpha: out }).width(Length::Fill).height(Length::Fill);
    let between = duel.meeting.as_ref().map(|meeting| meeting.between.clone()).unwrap_or_default();
    let count = |side: &str| seat(duel, side).and_then(|person| between.get(&person.player.to_string()).copied()).unwrap_or_default();
    let (a, b) = (count("A"), count("B"));
    let history: Element<'a, Message> = if a + b > 0 {
        ui::fading(ease((t - 1.05) / 0.6) * out, || column![
            words(w.n("duels-met", u64::from(a + b)), 15.0, FAINT),
            row![bold(a.to_string(), 52.0, SIDE_A), bold(":".to_owned(), 52.0, INK), bold(b.to_string(), 52.0, SIDE_B)].spacing(14).align_y(Alignment::Center),
        ].spacing(6).align_x(Alignment::Center).into())
    } else {
        Space::new().into()
    };
    let top = ui::fading(ease((t - 0.3) / 0.5) * out, || container(words(caption, 15.0, FAINT)).center_x(Length::Fill).padding(Padding { top: 34.0, ..Padding::ZERO }));
    let sides = ease((t - 0.45) / 0.6) * out;
    stack![
        container(halves).width(Length::Fill).height(Length::Fill).clip(true),
        top,
        meeting_side(w, duel, "A", sides),
        meeting_side(w, duel, "B", sides),
        container(history).center_x(Length::Fill).align_y(iced::alignment::Vertical::Bottom).height(Length::Fill).padding(Padding { bottom: 46.0, ..Padding::ZERO }),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

const GRID_FROM_HERO: f32 = 105.0;

fn grow_card<'a>(state: &State, entry: &Entry, wide: f32, high: f32, title: f32, note: Option<(String, f32)>, edge: Color) -> Element<'a, Message> {
    let size = |small: f32, large: f32| small + (large - small) * ((title - 15.0) / 15.0).clamp(0.0, 1.0);
    let mut top = row![slot_tag(&entry.slot), stars(entry, size(12.0, 14.0)), Space::new().width(Length::Fill)].spacing(10).align_y(Alignment::Center);
    if let Some((said, k)) = note {
        top = top.push(ui::fading(k, || words(said, 14.0, color!(0xd8c4c1))));
    }
    let inside = column![
        top,
        Space::new().height(Length::Fill),
        words(entry.version.clone().unwrap_or_default(), size(12.0, 15.0), MUTED),
        strong(ui::fitted(&entry.title.clone().unwrap_or_default(), theme::SANS_SEMI, title, (wide - 40.0).max(60.0)), title, INK),
        words(entry.artist.clone().unwrap_or_default(), size(12.0, 16.0), color!(0xd8c4c1)),
        row![crate::duels_screen::mapper_face(&entry.mapper.clone().unwrap_or_default(), entry.beatmap_id, size(20.0, 22.0)), words(entry.mapper.clone().unwrap_or_default(), size(12.0, 14.0), color!(0xd8c4c1))].spacing(8).align_y(Alignment::Center),
    ]
    .spacing(size(2.0, 4.0))
    .padding(Padding { top: size(10.0, 18.0), right: size(12.0, 22.0), bottom: size(10.0, 18.0), left: size(12.0, 22.0) })
    .height(high);
    let layered = stack![picture(state, entry, high, wide.max(1.0)), veil(), inside].width(wide).height(high);
    let edge = fa(edge);
    container(layered).width(wide).height(high).clip(true).style(move |_| container::Style {
        border: Border { color: edge, width: 3.0, radius: 14.0.into() },
        shadow: iced::Shadow { color: Color { a: 0.35 * edge.a, ..edge }, offset: iced::Vector::ZERO, blur_radius: 18.0 },
        ..container::Style::default()
    }).into()
}

fn round_transition<'a>(w: &Words, state: &State, duel: &Duel, now: i64, t: f32, width: f32) -> Element<'a, Message> {
    let Some(current) = current_entry(duel) else { return round_view(w, state, duel, now) };
    let column_wide = (width - 20.0) / 3.0;
    let place = |index: usize| {
        let (row_at, col) = (index / 3, index % 3);
        (col as f32 * (column_wide + 10.0), GRID_FROM_HERO + row_at as f32 * (CARD_HIGH + 10.0))
    };
    let mut layers: Vec<Element<'a, Message>> = Vec::new();
    let pool = entries(duel);
    let mut grown_from = (0.0, GRID_FROM_HERO);
    for (index, entry) in pool.iter().enumerate() {
        let (x, y) = place(index);
        if entry.slot == current.slot {
            grown_from = (x, y);
            continue;
        }
        let k = ease((t - (0.35 + 0.04 * index as f32)) / 0.45);
        if k >= 1.0 {
            continue;
        }
        let taken = duel.draft.iter().find(|turn| turn.slot == entry.slot);
        let banned = taken.is_some_and(|turn| turn.kind == "ban") || entry.banned_by.is_some();
        let mark = taken.and_then(|turn| side_of_player(duel, turn.by)).map(|side| side_colour(&side));
        let frame = mark.map(|colour| (colour, !banned));
        let card = ui::fading(1.0 - k, || moving_card(state, entry, frame, banned || entry.slot == "TB", false, None, Motion::default()));
        layers.push(iced::widget::pin(container(card).width(column_wide)).x(x).y(y + 14.0 * k).into());
    }
    let g = ease((t - 0.95) / 0.75);
    let lerp = |from: f32, to: f32| from + (to - from) * g;
    let picker = duel.rounds.last().and_then(|round| round.picked_by);
    let note = match picker {
        Some(player) => w.with("duels-picked-by", &[("name", name_of(duel, Some(player)))]),
        None => w.t("duels-decider"),
    };
    let edge = picker.and_then(|player| side_of_player(duel, Some(player))).map(|side| side_colour(&side)).unwrap_or(GOLD);
    let grower = grow_card(state, &current, lerp(column_wide, width), lerp(CARD_HIGH, HERO_HIGH), lerp(15.0, 30.0), Some((note, ease((t - 1.6) / 0.4))), Color { a: 1.0 - g, ..edge });
    layers.push(iced::widget::pin(grower).x(lerp(grown_from.0, 0.0)).y(lerp(grown_from.1, 0.0)).into());
    let players_k = ease((t - 1.65) / 0.5);
    let status_k = ease((t - 1.85) / 0.5);
    let below = column![
        Space::new().height(HERO_HIGH),
        ui::fading(players_k, || players(w, duel)),
        ui::fading(status_k, || centred(round_status(w, state, duel, now))),
    ]
    .spacing(12);
    let mut all: Vec<Element<'a, Message>> = vec![below.into()];
    all.extend(layers);
    iced::widget::Stack::with_children(all).width(Length::Fill).height(Length::Fill).into()
}

pub fn view<'a>(state: &State, duel: &Duel, w: &Words, now: i64, wide: f32) -> Element<'a, Message> {
    let width = wide.clamp(300.0, 1200.0);
    let phase = phase_of(&duel.state);
    let caption = match phase {
        Phase::Room => w.t("duels-caption-room"),
        Phase::Reveal => w.t("duels-caption-room"),
        Phase::Draft => w.t("duels-caption-draft"),
        Phase::Ready | Phase::Playing => w.with("duels-caption-round", &[("round", duel.rounds.last().map_or(1, |round| round.ordinal.max(1)).to_string())]),
        Phase::Result => String::new(),
    };
    let mut glow = None;
    let body: Element<'a, Message> = match phase {
        Phase::Room => {
            let met = crate::duels_screen::since(state.moments.met_at);
            let room_k = if met < crate::duels_screen::MEET { ease((met - 4.05) / 0.6) } else { 1.0 };
            let room = ui::fading(room_k, || column![head(w, duel, caption, true), Space::new().height(Length::Fill), room_view(w, state, duel, now), Space::new().height(Length::Fill)].spacing(14));
            let room: Element<'a, Message> = room.into();
            if met < crate::duels_screen::MEET {
                stack![meeting_view(w, duel, met), room].width(Length::Fill).height(Length::Fill).into()
            } else {
                room
            }
        }
        Phase::Reveal => column![head(w, duel, caption, true), reveal_view(w, state, duel, now)].spacing(14).into(),
        Phase::Draft => {
            let (inside, lit) = draft_view(w, state, duel, now);
            glow = lit;
            column![head(w, duel, caption, false), inside].spacing(12).into()
        }
        Phase::Ready if state.moments.from == Some(Phase::Draft) && crate::duels_screen::since(state.moments.phase_at) < crate::duels_screen::ROUND => {
            let t = crate::duels_screen::since(state.moments.phase_at);
            column![head(w, duel, caption, false), round_transition(w, state, duel, now, t, width)].spacing(12).into()
        }
        Phase::Ready | Phase::Playing => column![head(w, duel, caption, false), round_view(w, state, duel, now)].spacing(12).into(),
        Phase::Result => result_view(w, state, duel, now),
    };
    let mut page = column![body].width(Length::Fill).height(Length::Fill);
    if let Some(why) = &state.failed {
        page = page.push(row![glyph(Icon::Warn, 16.0, LOST), words(w.with("duels-failed", &[("reason", why.clone())]), 14.0, MUTED)].spacing(8).align_y(Alignment::Center));
    }
    let framed = container(container(page).width(Length::Fill).max_width(1200).height(Length::Fill))
        .padding(Padding { top: 8.0, right: 40.0, bottom: 24.0, left: 40.0 }).center_x(Length::Fill).height(Length::Fill);
    match glow {
        Some((colour, left)) => stack![edge(colour, left), framed].width(Length::Fill).height(Length::Fill).into(),
        None => framed.into(),
    }
}

pub fn turn_label(w: &Words, state: &State, duel: &Duel) -> (String, Color, bool) {
    let mine = state.side_of(duel);
    match phase_of(&duel.state) {
        Phase::Draft => {
            let at = duel.draft.len().min(ORDER.len());
            match ORDER.get(at) {
                Some((kind, side)) if Some(*side) == mine.as_deref() => (w.t(if *kind == "ban" { "duels-your-ban" } else { "duels-your-pick" }), side_colour(side), true),
                Some((kind, side)) => (w.with(if *kind == "ban" { "duels-their-ban" } else { "duels-their-pick" }, &[("name", seat(duel, side).and_then(|person| person.name.clone()).unwrap_or_default())]), side_colour(side), false),
                None => (w.t("duels-pool-ready"), GOOD, false),
            }
        }
        Phase::Ready if duel.actions.iter().any(|action| action == "ready") => (w.t("duels-ready-asked"), side_colour(mine.as_deref().unwrap_or("A")), true),
        Phase::Ready => (w.t("duels-preparing"), MUTED, false),
        Phase::Playing => (w.t("duels-map-running"), MUTED, false),
        Phase::Reveal => (w.t("duels-caption-reveal"), MUTED, false),
        Phase::Room => (w.t("duels-caption-room"), MUTED, false),
        Phase::Result => (w.t("duels-done"), MUTED, false),
    }
}

pub fn away_strip<'a>(state: &State, w: &Words, now: i64) -> Option<Element<'a, Message>> {
    let now = state.server_now(now);
    let duel = state.shown().filter(|duel| phase_of(&duel.state) != Phase::Result)?;
    let (label, colour, yours) = turn_label(w, state, duel);
    let a = seat(duel, "A").and_then(|person| person.name.clone()).unwrap_or_default();
    let b = seat(duel, "B").and_then(|person| person.name.clone()).unwrap_or_default();
    let mut line = row![
        glyph(Icon::Swords, 18.0, colour),
        stack![row![player_face(&a, seat(duel, "A").map(|p| p.osu_user_id), 28.0)], row![Space::new().width(20), player_face(&b, seat(duel, "B").map(|p| p.osu_user_id), 28.0)]].width(48).height(28),
        bold(format!("{} : {}", duel.wins.a, duel.wins.b), 16.0, INK),
        strong(label, 15.0, colour),
    ]
    .spacing(14)
    .align_y(Alignment::Center);
    if let Some(clock) = left_clock(duel, now) {
        line = line.push(clock);
    }
    line = line.push(ui::primary(w.t(if yours { "duels-return" } else { "duels-open" }), Some(Message::Open)));
    let lit = yours.then_some(colour);
    let strip = container(line).padding(Padding { top: 10.0, right: 12.0, bottom: 10.0, left: 16.0 }).style(move |theme| {
        let mut style = theme::slab(theme);
        style.background = Some(Background::Color(Color::from_rgba(0.086, 0.04, 0.055, 0.94)));
        style.border = Border { color: Color { a: 0.7, ..lit.unwrap_or(OUTLINE) }, width: 1.5, radius: 16.0.into() };
        style
    });
    let placed = column![Space::new().height(Length::Fill), container(iced::widget::opaque(strip)).center_x(Length::Fill), Space::new().height(22)];
    let layer: Element<'a, Message> = match lit {
        Some(colour) => stack![
            container(Space::new().width(Length::Fill).height(Length::Fill)).style(move |_| container::Style {
                background: Some(Background::Gradient(iced::gradient::Linear::new(iced::Radians(0.0))
                    .add_stop(0.0, Color { a: 0.3, ..colour })
                    .add_stop(0.14, Color { a: 0.1, ..colour })
                    .add_stop(0.3, Color { a: 0.0, ..colour })
                    .into())),
                ..container::Style::default()
            }),
            placed,
        ].width(Length::Fill).height(Length::Fill).into(),
        None => placed.width(Length::Fill).height(Length::Fill).into(),
    };
    Some(layer)
}

pub fn leaving<'a>(w: &Words) -> Element<'a, Message> {
    let dialog = container(column![
        row![glyph(Icon::Warn, 24.0, WAIT), strong(w.t("duels-leaving"), 22.0, INK)].spacing(12).align_y(Alignment::Center),
        words(w.t("duels-leaving-note"), 16.0, MUTED),
        row![wide_button(w.t("duels-stay"), true, Some(Message::Stay)), wide_button(w.t("duels-leave"), false, Some(Message::Leave))].spacing(10),
    ].spacing(16))
    .padding(26)
    .width(480)
    .style(|theme| {
        let mut style = theme::slab(theme);
        style.background = Some(Background::Color(Color::from_rgba(0.086, 0.04, 0.055, 0.97)));
        style
    });
    container(iced::widget::opaque(dialog))
        .center(Length::Fill)
        .style(|_| container::Style { background: Some(Background::Color(Color::from_rgba(0.02, 0.008, 0.016, 0.62))), ..container::Style::default() })
        .into()
}
