use std::io::Read;
use std::sync::Arc;

use dossier_replay::{bits, Mods};
use dossier_sim::{TimedKind, TimedObject, Timeline};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::{column, container, row, Space};
use iced::{mouse, Alignment, Background, Border, Color, Element, Length, Padding, Point, Rectangle, Renderer, Size, Theme};

use crate::duels::Running;
use crate::duels_screen::{bold, fa, person, slab_faded, strong, words, Message, State, SIDE_A, SIDE_B};
use crate::glyphs::{glyph, Icon};
use crate::lang::Words;
use crate::theme::{self, FAINT, INK, MUTED};
use crate::ui;

const FIELD: Size = Size { width: 512.0, height: 384.0 };
const TRAIL: usize = 60;
const ASIDE: f32 = 300.0;
const FILE_LIMIT: u64 = 8 * 1024 * 1024;
const HIT_FADE_MS: f64 = 150.0;
const LEAD_FULL: f64 = 100_000.0;
const SPINNER_REACH: f64 = 180.0;
const LOOK_BACK_MS: f64 = 30_000.0;

const CURSOR: Color = theme::GRADE_S;
const BODY: Color = Color::from_rgb(0.07, 0.05, 0.06);
const GROUND: Color = Color::from_rgb(0.035, 0.022, 0.03);

pub struct Stage {
    pub beatmap: i64,
    plain: Timeline,
    hard: Timeline,
    combos: Vec<u32>,
    pub length_ms: f64,
}

impl std::fmt::Debug for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stage({}, {} objects)", self.beatmap, self.plain.objects.len())
    }
}

impl Stage {
    pub fn read(beatmap: i64, text: &str) -> Option<Stage> {
        let parsed = dossier_beatmap::Beatmap::parse(text).ok()?;
        if parsed.mode != 0 || parsed.objects.is_empty() {
            return None;
        }
        let plain = Timeline::build(&parsed, Mods::new(0));
        let hard = Timeline::build(&parsed, Mods::new(bits::HARD_ROCK));
        let mut combos = Vec::with_capacity(plain.objects.len());
        let mut number = 0;
        let mut after_spinner = true;
        for object in &plain.objects {
            number = if object.new_combo || after_spinner { 1 } else { number + 1 };
            after_spinner = object.is_spinner();
            combos.push(number);
        }
        let length_ms = plain.objects.iter().map(|object| object.end_ms).fold(0.0, f64::max);
        Some(Stage { beatmap, plain, hard, combos, length_ms })
    }

    fn timeline(&self, hard: bool) -> &Timeline {
        if hard { &self.hard } else { &self.plain }
    }
}

fn fits(text: &str, beatmap: i64) -> bool {
    text.trim_start_matches('\u{feff}').starts_with("osu file format")
        && text.lines().find_map(|line| line.strip_prefix("BeatmapID:")).is_none_or(|said| said.trim().parse::<i64>().ok() == Some(beatmap))
}

fn osu_text(beatmap: i64) -> Option<String> {
    let root = crate::sources::own_root().join("cache");
    let path = root.join("duel-maps").join(format!("{beatmap}.osu"));
    for known in [path.clone(), root.join("balance-maps").join(format!("{beatmap}-latest.osu"))] {
        if let Ok(text) = std::fs::read_to_string(&known) {
            if fits(&text, beatmap) {
                return Some(text);
            }
        }
    }
    let client = crate::bot::client().ok()?;
    for address in [format!("https://osu.direct/api/osu/{beatmap}")] {
        let Some(response) = crate::desk::mirror_get(&client, &address) else { continue };
        if !response.status().is_success() {
            continue;
        }
        let mut text = String::new();
        if response.take(FILE_LIMIT).read_to_string(&mut text).is_err() || !fits(&text, beatmap) {
            continue;
        }
        if let Some(folder) = path.parent() {
            let part = path.with_extension("part");
            if std::fs::create_dir_all(folder).is_ok() && std::fs::write(&part, &text).is_ok() {
                let _ = std::fs::rename(&part, &path);
            }
        }
        return Some(text);
    }
    None
}

pub fn stage(beatmap: i64) -> Option<Stage> {
    Stage::read(beatmap, &osu_text(beatmap)?)
}

struct Look {
    hard: bool,
    hidden: bool,
}

struct Field {
    trail: Vec<(f32, f32)>,
    colour: Color,
    progress: Option<f32>,
    alpha: f32,
    stage: Option<Arc<Stage>>,
    time: Option<f64>,
    look: Look,
    note: Option<(String, Option<String>)>,
    corner: Option<String>,
}

fn share(value: f64) -> f32 {
    value.clamp(0.0, 1.0) as f32
}

fn dim(colour: Color, alpha: f32) -> Color {
    Color { a: colour.a * alpha, ..colour }
}

fn mix(from: Color, to: Color, by: f32) -> Color {
    let by = by.clamp(0.0, 1.0);
    Color::from_rgb(from.r + (to.r - from.r) * by, from.g + (to.g - from.g) * by, from.b + (to.b - from.b) * by)
}

fn shade(colour: Color, by: f32) -> Color {
    Color { r: colour.r * by, g: colour.g * by, b: colour.b * by, a: colour.a }
}

impl Field {
    fn object_alpha(&self, object: &TimedObject, preempt: f64, fade_in: f64, time: f64) -> f32 {
        let shown = share((time - (object.start_ms - preempt)) / fade_in.max(1.0));
        let gone = if time > object.end_ms { 1.0 - share((time - object.end_ms) / HIT_FADE_MS) } else { 1.0 };
        let hidden = if self.look.hidden {
            let from = object.start_ms - preempt * 0.6;
            let until = if object.is_slider() { object.end_ms.max(from + 1.0) } else { from + preempt * 0.3 };
            1.0 - share((time - from) / (until - from))
        } else {
            1.0
        };
        shown * gone * hidden
    }

    fn draw_objects(&self, frame: &mut Frame, at: &dyn Fn(f64, f64) -> Point, scale: f32, seen: &dyn Fn(Color) -> Color) {
        let (Some(stage), Some(time)) = (self.stage.as_ref(), self.time) else { return };
        let timeline = stage.timeline(self.look.hard);
        let preempt = timeline.difficulty.preempt_ms();
        let fade_in = timeline.difficulty.fade_in_ms_in_lazer();
        let radius = timeline.difficulty.circle_radius() as f32 * scale;
        let last = timeline.objects.partition_point(|object| object.start_ms - preempt <= time);
        let border = Color::from_rgba(1.0, 1.0, 1.0, 0.92);
        for object in timeline.objects[..last].iter().rev() {
            if object.start_ms < time - LOOK_BACK_MS {
                break;
            }
            if object.end_ms + HIT_FADE_MS < time {
                continue;
            }
            let alpha = self.object_alpha(object, preempt, fade_in, time);
            if alpha <= 0.0 {
                continue;
            }
            let centre = at(object.pos.x, object.pos.y);
            match &object.kind {
                TimedKind::Spinner => {
                    let left = share((object.end_ms - time) / object.duration_ms().max(1.0));
                    let middle = at(256.0, 192.0);
                    frame.stroke(&Path::circle(middle, (SPINNER_REACH as f32 * scale * left).max(radius * 0.3)), Stroke::default().with_width(3.0).with_color(seen(dim(self.colour, alpha))));
                    frame.fill(&Path::circle(middle, radius * 0.25), seen(dim(border, alpha)));
                }
                TimedKind::Slider { path, .. } => {
                    let points = path.points();
                    if let Some((head, rest)) = points.split_first() {
                        let body = Path::new(|builder| {
                            builder.move_to(at(head.x, head.y));
                            for point in rest {
                                builder.line_to(at(point.x, point.y));
                            }
                        });
                        let wide = |width: f32| Stroke::default().with_width(width).with_line_cap(canvas::LineCap::Round).with_line_join(canvas::LineJoin::Round);
                        frame.stroke(&body, wide(radius * 2.0).with_color(seen(dim(border, alpha * 0.85))));
                        frame.stroke(&body, wide(radius * 1.74).with_color(seen(mix(GROUND, mix(BODY, self.colour, 0.22), alpha))));
                    }
                    if time < object.start_ms {
                        self.draw_circle(frame, centre, radius, stage.combos.get(object.index).copied(), alpha, seen);
                    }
                    if let Some(ball) = object.ball_at(time) {
                        let ball = at(ball.x, ball.y);
                        frame.stroke(&Path::circle(ball, radius * 2.2), Stroke::default().with_width(2.0).with_color(seen(dim(border, alpha * 0.55))));
                        frame.fill(&Path::circle(ball, radius * 0.86), seen(dim(self.colour, alpha)));
                    }
                }
                TimedKind::Circle => {
                    let grown = if time > object.start_ms { 1.0 + 0.35 * share((time - object.start_ms) / HIT_FADE_MS) } else { 1.0 };
                    let faded = if time > object.start_ms { 1.0 - share((time - object.start_ms) / HIT_FADE_MS) } else { 1.0 };
                    self.draw_circle(frame, centre, radius * grown, stage.combos.get(object.index).copied(), alpha * faded, seen);
                }
            }
            if !self.look.hidden && time < object.start_ms && !object.is_spinner() {
                let closing = share((time - (object.start_ms - preempt)) / preempt.max(1.0));
                let ring = radius * (4.0 - 3.0 * closing);
                frame.stroke(&Path::circle(centre, ring), Stroke::default().with_width(2.5).with_color(seen(dim(self.colour, alpha * 0.9))));
            }
        }
    }

    fn draw_circle(&self, frame: &mut Frame, centre: Point, radius: f32, number: Option<u32>, alpha: f32, seen: &dyn Fn(Color) -> Color) {
        if alpha <= 0.0 {
            return;
        }
        frame.fill(&Path::circle(centre, radius), seen(dim(shade(self.colour, 0.78), alpha)));
        frame.stroke(&Path::circle(centre, radius - 1.5), Stroke::default().with_width(3.0).with_color(seen(Color::from_rgba(1.0, 1.0, 1.0, 0.92 * alpha))));
        if let Some(number) = number.filter(|_| radius >= 9.0) {
            frame.fill_text(canvas::Text {
                content: number.to_string(),
                position: centre,
                color: seen(Color::from_rgba(1.0, 1.0, 1.0, alpha)),
                size: (radius * 0.95).into(),
                font: theme::SANS_SEMI,
                align_x: iced::widget::text::Alignment::Center,
                align_y: iced::alignment::Vertical::Center,
                ..canvas::Text::default()
            });
        }
    }
}

impl<M> canvas::Program<M> for Field {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let seen = |colour: Color| Color { a: colour.a * self.alpha, ..colour };
        let scale = (bounds.width / FIELD.width).min(bounds.height / FIELD.height) * 0.86;
        let left = (bounds.width - FIELD.width * scale) / 2.0;
        let top = (bounds.height - FIELD.height * scale) / 2.0;
        let flip = self.look.hard;
        let at = move |x: f64, y: f64| {
            let y = if flip { FIELD.height as f64 - y } else { y };
            Point::new(left + x as f32 * scale, top + y as f32 * scale)
        };
        let cursor_at = |(x, y): (f32, f32)| Point::new(left + x.clamp(-64.0, FIELD.width + 64.0) * scale, top + y.clamp(-48.0, FIELD.height + 48.0) * scale);
        frame.fill(&Path::rounded_rectangle(Point::ORIGIN, bounds.size(), 16.0.into()), seen(Color::from_rgba(0.024, 0.012, 0.02, 0.72)));
        self.draw_objects(&mut frame, &at, scale, &seen);
        let count = self.trail.len();
        for (index, pair) in self.trail.windows(2).enumerate() {
            let fade = (index + 1) as f32 / count.max(1) as f32;
            let path = Path::line(cursor_at(pair[0]), cursor_at(pair[1]));
            frame.stroke(&path, Stroke::default().with_width(3.0 * fade + 1.0).with_color(seen(Color { a: 0.5 * fade, ..CURSOR })).with_line_cap(canvas::LineCap::Round));
        }
        if let Some(last) = self.trail.last() {
            frame.stroke(&Path::circle(cursor_at(*last), 11.0), Stroke::default().with_width(3.0).with_color(seen(INK)));
            frame.fill(&Path::circle(cursor_at(*last), 5.0), seen(CURSOR));
        }
        if let Some((said, more)) = &self.note {
            let middle = Point::new(bounds.width / 2.0, bounds.height / 2.0 - if more.is_some() { 10.0 } else { 0.0 });
            frame.fill_text(canvas::Text {
                content: said.clone(),
                position: middle,
                color: seen(MUTED),
                size: 16.0.into(),
                font: theme::SANS_SEMI,
                align_x: iced::widget::text::Alignment::Center,
                align_y: iced::alignment::Vertical::Center,
                ..canvas::Text::default()
            });
            if let Some(more) = more.as_ref().filter(|_| bounds.width >= 420.0) {
                frame.fill_text(canvas::Text {
                    content: more.clone(),
                    position: Point::new(middle.x, middle.y + 24.0),
                    color: seen(FAINT),
                    size: 13.0.into(),
                    font: theme::SANS,
                    align_x: iced::widget::text::Alignment::Center,
                    align_y: iced::alignment::Vertical::Center,
                    ..canvas::Text::default()
                });
            }
        }
        if let Some(corner) = &self.corner {
            frame.fill_text(canvas::Text {
                content: corner.clone(),
                position: Point::new(14.0, bounds.height - 26.0),
                color: seen(FAINT),
                size: 12.0.into(),
                font: theme::SANS,
                align_y: iced::alignment::Vertical::Center,
                ..canvas::Text::default()
            });
        }
        if let Some(progress) = self.progress {
            let bar = ui::EdgeBar { fraction: progress, colour: self.colour, radius: 16.0, edge: 0.0, thick: crate::duel_view::EDGE_THICK, tint: 0.0, alpha: self.alpha };
            bar.paint(&mut frame, bounds.size());
        }
        vec![frame.into_geometry()]
    }
}

fn numbers<'a>(w: &Words, said: Option<&serde_json::Value>) -> Element<'a, Message> {
    let Some(said) = said else { return Space::new().into() };
    let score = said.get("score").and_then(serde_json::Value::as_i64).unwrap_or_default();
    let combo = said.get("combo").and_then(serde_json::Value::as_i64).unwrap_or_default();
    let accuracy = said.get("accuracy").and_then(serde_json::Value::as_f64).unwrap_or_default();
    row![
        strong(format!("{combo}x"), 15.0, theme::GRADE_S),
        words(format!("{:.2}%", accuracy * 100.0).replace('.', ","), 14.0, MUTED),
        bold(w.lang().group(score.max(0) as u64), 20.0, INK),
    ].spacing(14).align_y(Alignment::Center).into()
}

fn look_of(running: &Running, said: Option<&serde_json::Value>) -> Look {
    let mods = said.and_then(|said| said.get("mods")).and_then(serde_json::Value::as_u64).map(|mods| mods as u32);
    let slot = running.slot.as_deref().unwrap_or_default();
    match mods {
        Some(mods) => Look { hard: mods & bits::HARD_ROCK != 0, hidden: mods & bits::HIDDEN != 0 },
        None => Look { hard: slot.starts_with("HR"), hidden: slot.starts_with("HD") },
    }
}

fn caption(w: &Words, state: &str) -> String {
    w.t(match state {
        "DRAFTING" | "SELECTING" => "duels-caption-draft",
        "ROUND_SETUP" | "WAITING_READY" | "STARTING" => "duels-preparing",
        "VERIFYING" | "FINALIZING" => "duels-waiting-match",
        "ABORTING" => "duels-stopped",
        "PAUSED" | "RECOVERING" => "duels-recovering",
        _ => "duels-caption-room",
    })
}

pub fn score_of(state: &State, player: i64) -> Option<i64> {
    state.heard_states.get(&player).and_then(|said| said.get("score")).and_then(serde_json::Value::as_i64)
}

fn side<'a>(w: &Words, state: &State, running: &Running, index: usize, now: std::time::Instant, wide: f32) -> Element<'a, Message> {
    let Some(seat) = running.players.get(index) else { return Space::new().into() };
    let colour = if seat.side.as_deref() == Some("B") { SIDE_B } else { SIDE_A };
    let round = state.rounds.get(&seat.player).copied().unwrap_or_default();
    let shown = state.broadcast.visible(seat.player, round, now);
    let trail: Vec<(f32, f32)> = shown.iter().rev().take(TRAIL).rev().map(|frame| (frame.x, frame.y)).collect();
    let heard = state.heard_states.get(&seat.player);
    let arrived = state.watch_stage.as_ref().filter(|(beatmap, _)| Some(*beatmap) == running.beatmap_id);
    let stage = arrived.and_then(|(_, stage)| stage.clone());
    let playing = running.state == "PLAYING";
    let time = shown.last().filter(|_| playing).map(|frame| frame.time_ms as f64);
    let length = running.song_seconds.map(|seconds| f64::from(seconds) * 1000.0).or_else(|| stage.as_ref().map(|stage| stage.length_ms));
    let progress = heard.and_then(|said| said.get("time")).and_then(serde_json::Value::as_f64).filter(|_| playing).zip(length).map(|(time, length)| (time / length.max(1.0)) as f32);
    let note = if !playing {
        Some((caption(w, &running.state), None))
    } else if shown.is_empty() {
        Some((w.t("duels-watch-waiting"), Some(w.t("duels-watch-witness"))))
    } else {
        None
    };
    let corner = match arrived {
        _ if note.is_some() || running.beatmap_id.is_none() => None,
        Some((_, None)) => Some(w.t("duels-watch-no-map")),
        Some((_, Some(_))) => None,
        None => Some(w.t("duels-watch-map-loading")),
    };
    let look = look_of(running, heard);
    let field = Canvas::new(Field { trail, colour, progress, alpha: ui::fade(), stage, time, look, note, corner }).width(Length::Fill).height(wide * FIELD.height / FIELD.width);
    let said = if playing { numbers(w, heard) } else { Space::new().into() };
    let under = row![strong(seat.name.clone().unwrap_or_default(), 16.0, colour), Space::new().width(Length::Fill), said].spacing(12).align_y(Alignment::Center);
    column![field, under].spacing(8).width(Length::Fill).into()
}

fn lead<'a>(w: &Words, state: &State, running: &Running) -> Element<'a, Message> {
    let (Some(a), Some(b)) = (running.players.first(), running.players.get(1)) else { return Space::new().into() };
    let (Some(first), Some(second)) = (score_of(state, a.player), score_of(state, b.player)) else { return Space::new().height(22).into() };
    if running.state != "PLAYING" {
        return Space::new().height(22).into();
    }
    let gap = first - second;
    let reach = share(gap.unsigned_abs() as f64 / LEAD_FULL);
    let colour = if gap >= 0 { SIDE_A } else { SIDE_B };
    let track = fa(Color::from_rgba(1.0, 1.0, 1.0, 0.07));
    let fill = fa(colour);
    let bar = |grow: f32, painted: bool| -> Element<'a, Message> {
        let part = (grow * 1000.0).round().max(0.0) as u16;
        if part == 0 {
            return Space::new().width(Length::FillPortion(1)).into();
        }
        container(Space::new()).width(Length::FillPortion(part)).height(6).style(move |_| container::Style {
            background: Some(Background::Color(if painted { fill } else { track })),
            border: Border { radius: 3.0.into(), ..Border::default() },
            ..container::Style::default()
        }).into()
    };
    let left: Element<'a, Message> = if gap > 0 {
        row![bar(1.0 - reach, false), bar(reach, true)].width(Length::Fill).align_y(Alignment::Center).into()
    } else {
        row![bar(1.0, false)].width(Length::Fill).align_y(Alignment::Center).into()
    };
    let right: Element<'a, Message> = if gap < 0 {
        row![bar(reach, true), bar(1.0 - reach, false)].width(Length::Fill).align_y(Alignment::Center).into()
    } else {
        row![bar(1.0, false)].width(Length::Fill).align_y(Alignment::Center).into()
    };
    let said = if gap == 0 { w.t("duels-watch-even") } else { format!("+{}", w.lang().group(gap.unsigned_abs())) };
    let label = container(strong(said, 14.0, if gap == 0 { MUTED } else { colour })).width(120).align_x(iced::alignment::Horizontal::Center);
    row![left, label, right].spacing(10).align_y(Alignment::Center).width(Length::Fill).height(22).into()
}

fn others<'a>(w: &Words, state: &State, running: &Running) -> Element<'a, Message> {
    let mut list = column![container(strong(w.t("duels-running"), 18.0, INK)).padding(Padding { top: 0.0, right: 0.0, bottom: 4.0, left: 0.0 })].spacing(8);
    for duel in &state.live.running {
        let on = duel.id == running.id;
        let line = row![
            crate::duels_screen::pair(duel, 30.0),
            container(crate::duels_screen::names(w, duel, 14.0)).width(Length::Fill),
            bold(format!("{} : {}", duel.wins.a, duel.wins.b), 16.0, INK),
        ].spacing(12).align_y(Alignment::Center);
        let fill = fa(if on { crate::desk_screen::LIT } else { crate::duels_screen::ROW });
        let edge = fa(Color { a: 0.5, ..theme::ACCENT });
        let tile = container(line).padding([10, 12]).width(Length::Fill).style(move |_| container::Style {
            background: Some(Background::Color(fill)),
            border: Border { radius: 12.0.into(), width: if on { 1.5 } else { 0.0 }, color: edge },
            ..container::Style::default()
        });
        let press = if on { Message::Unwatch } else { Message::Watch(duel.id.clone()) };
        list = list.push(iced::widget::mouse_area(tile).on_press(press).interaction(mouse::Interaction::Pointer));
    }
    container(iced::widget::scrollable(list).direction(ui::hidden_bar())).padding(18).width(ASIDE).height(Length::Fill).style(slab_faded()).into()
}

pub fn view<'a>(state: &State, running: &Running, w: &Words, wide: f32) -> Element<'a, Message> {
    let field = ((wide - 80.0 - ASIDE - 16.0 - 14.0) / 2.0).max(160.0);
    let now = std::time::Instant::now();
    let names: Vec<String> = running.players.iter().map(|seat| seat.name.clone().unwrap_or_default()).collect();
    let a = names.first().cloned().unwrap_or_default();
    let b = names.get(1).cloned().unwrap_or_default();
    let head = container(row![
        container(row![person(&a, 42.0), strong(a.clone(), 18.0, SIDE_A)].spacing(14).align_y(Alignment::Center)).width(Length::Fill),
        column![bold(format!("{} : {}", running.wins.a, running.wins.b), 34.0, INK), words(w.t("duels-watching"), 13.0, FAINT)].spacing(2).align_x(Alignment::Center),
        container(row![strong(b.clone(), 18.0, SIDE_B), person(&b, 42.0)].spacing(14).align_y(Alignment::Center)).width(Length::Fill).align_x(iced::alignment::Horizontal::Right),
    ].align_y(Alignment::Center)).padding(Padding { top: 12.0, right: 20.0, bottom: 12.0, left: 20.0 }).width(Length::Fill).style(slab_faded());
    let title = running.title.clone().unwrap_or_else(|| caption(w, &running.state));
    let line = row![
        running.slot.as_deref().map(crate::duel_view::slot_tag).unwrap_or_else(|| Space::new().into()),
        strong(title, 18.0, INK),
        running.stars.map(|value| crate::dossier::star_pill(format!("{value:.2}").replace('.', ","), value, 13.0)).unwrap_or_else(|| Space::new().into()),
        Space::new().width(Length::Fill),
        glyph(Icon::Eye, 16.0, FAINT),
        words(running.watching.to_string(), 14.0, FAINT),
    ].spacing(10).align_y(Alignment::Center);
    let fields = row![side(w, state, running, 0, now, field), side(w, state, running, 1, now, field)].spacing(14);
    let page = column![head, line, lead(w, state, running), fields, words(w.t("duels-delay"), 14.0, FAINT)].spacing(12).width(Length::Fill).height(Length::Fill);
    let page = row![page, others(w, state, running)].spacing(16).height(Length::Fill);
    container(page).padding(Padding { top: 8.0, right: 40.0, bottom: 24.0, left: 40.0 }).width(Length::Fill).height(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: &str = include_str!("../../crates/dossier-assay/corpus/maps/898576.osu");

    #[test]
    fn a_stage_numbers_combos_and_keeps_a_mirrored_copy_for_hard_rock() {
        let stage = Stage::read(898576, MAP).expect("the map reads");
        assert_eq!(stage.plain.objects.len(), stage.hard.objects.len());
        assert_eq!(stage.combos.first(), Some(&1));
        assert!(stage.combos.iter().all(|number| *number >= 1));
        assert!(stage.length_ms > 60_000.0);
        assert!(stage.hard.difficulty.circle_radius() < stage.plain.difficulty.circle_radius());
        assert!(stage.hard.difficulty.preempt_ms() < stage.plain.difficulty.preempt_ms());
        assert!(Stage::read(1, "not a map").is_none());
    }

    #[test]
    fn only_a_file_for_the_asked_map_is_kept() {
        assert!(fits(MAP, 898576));
        assert!(!fits(MAP, 5));
        assert!(!fits("<html>", 898576));
        assert!(fits("osu file format v14\n[Metadata]\nTitle:x", 5));
    }

    #[test]
    fn a_slot_or_the_players_own_mods_decide_how_the_field_looks() {
        let hard = Running { slot: Some("HR2".into()), ..Running::default() };
        assert!(look_of(&hard, None).hard && !look_of(&hard, None).hidden);
        let tie = Running { slot: Some("TB".into()), ..Running::default() };
        let said = serde_json::json!({ "mods": bits::HIDDEN | bits::HARD_ROCK | bits::NO_FAIL });
        let look = look_of(&tie, Some(&said));
        assert!(look.hard && look.hidden);
        assert!(!look_of(&tie, None).hard);
    }
}
