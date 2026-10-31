use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::{column, container, row, stack, Space};
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

const CURSOR: Color = theme::GRADE_S;

struct Field {
    trail: Vec<(f32, f32)>,
    colour: Color,
    progress: Option<f32>,
    alpha: f32,
}

impl<M> canvas::Program<M> for Field {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let seen = |colour: Color| Color { a: colour.a * self.alpha, ..colour };
        let scale = (bounds.width / FIELD.width).min(bounds.height / FIELD.height);
        let left = (bounds.width - FIELD.width * scale) / 2.0;
        let top = (bounds.height - FIELD.height * scale) / 2.0;
        let at = |(x, y): (f32, f32)| Point::new(left + x.clamp(0.0, FIELD.width) * scale, top + y.clamp(0.0, FIELD.height) * scale);
        frame.fill(&Path::rounded_rectangle(Point::ORIGIN, bounds.size(), 16.0.into()), seen(Color::from_rgba(0.024, 0.012, 0.02, 0.72)));
        frame.stroke(&Path::rounded_rectangle(Point::new(0.5, 0.5), Size::new(bounds.width - 1.0, bounds.height - 1.0), 16.0.into()), Stroke::default().with_width(1.0).with_color(seen(Color::from_rgba(1.0, 1.0, 1.0, 0.08))));
        let count = self.trail.len();
        for (index, pair) in self.trail.windows(2).enumerate() {
            let fade = (index + 1) as f32 / count.max(1) as f32;
            let path = Path::line(at(pair[0]), at(pair[1]));
            frame.stroke(&path, Stroke::default().with_width(3.0 * fade + 1.0).with_color(seen(Color { a: 0.5 * fade, ..CURSOR })).with_line_cap(canvas::LineCap::Round));
        }
        if let Some(last) = self.trail.last() {
            frame.stroke(&Path::circle(at(*last), 16.0), Stroke::default().with_width(4.0).with_color(seen(self.colour)));
            frame.stroke(&Path::circle(at(*last), 25.0), Stroke::default().with_width(1.5).with_color(seen(Color { a: 0.5, ..INK })));
            frame.fill(&Path::circle(at(*last), 6.0), seen(CURSOR));
        }
        if let Some(progress) = self.progress {
            let (high, inset) = (5.0, 14.0);
            let wide = bounds.width - inset * 2.0;
            let top = bounds.height - high - 8.0;
            frame.fill(&Path::rounded_rectangle(Point::new(inset, top), Size::new(wide, high), (high / 2.0).into()), seen(Color::from_rgba(1.0, 1.0, 1.0, 0.08)));
            frame.fill(&Path::rounded_rectangle(Point::new(inset, top), Size::new((wide * progress.clamp(0.0, 1.0)).max(high), high), (high / 2.0).into()), seen(self.colour));
        }
        vec![frame.into_geometry()]
    }
}

fn numbers<'a>(w: &Words, said: Option<&serde_json::Value>) -> Element<'a, Message> {
    let Some(said) = said else { return Space::new().into() };
    let score = said.get("score").and_then(serde_json::Value::as_i64).unwrap_or_default();
    let combo = said.get("combo").and_then(serde_json::Value::as_i64).unwrap_or_default();
    let accuracy = said.get("accuracy").and_then(serde_json::Value::as_f64).unwrap_or_default();
    let plate = fa(Color::from_rgba(0.0, 0.0, 0.0, 0.55));
    container(column![
        bold(w.lang().group(score.max(0) as u64), 20.0, INK),
        words(format!("{:.2}%", accuracy * 100.0).replace('.', ","), 13.0, MUTED),
        strong(format!("{combo}x"), 15.0, theme::GRADE_S),
    ].spacing(1).align_x(Alignment::End))
    .padding(Padding { top: 6.0, right: 10.0, bottom: 6.0, left: 10.0 })
    .style(move |_| container::Style { background: Some(Background::Color(plate)), border: Border { radius: 8.0.into(), ..Border::default() }, ..container::Style::default() })
    .into()
}

fn side<'a>(w: &Words, state: &State, running: &Running, index: usize, now: std::time::Instant, wide: f32) -> Element<'a, Message> {
    let Some(seat) = running.players.get(index) else { return Space::new().into() };
    let colour = if seat.side.as_deref() == Some("B") { SIDE_B } else { SIDE_A };
    let round = state.rounds.get(&seat.player).copied().unwrap_or_default();
    let trail: Vec<(f32, f32)> = state.broadcast.visible(seat.player, round, now).iter().rev().take(TRAIL).rev().map(|frame| (frame.x, frame.y)).collect();
    let said = numbers(w, state.heard_states.get(&seat.player));
    let heard = state.heard_states.get(&seat.player);
    let progress = heard.and_then(|said| said.get("time")).and_then(serde_json::Value::as_f64).zip(running.song_seconds).map(|(time, seconds)| (time / (f64::from(seconds) * 1000.0).max(1.0)) as f32);
    let field = Canvas::new(Field { trail, colour, progress, alpha: ui::fade() }).width(Length::Fill).height(wide * FIELD.height / FIELD.width);
    let layered = stack![field, container(said).padding(10).width(Length::Fill).align_x(iced::alignment::Horizontal::Right)].width(Length::Fill);
    column![layered, strong(seat.name.clone().unwrap_or_default(), 16.0, colour)].spacing(8).width(Length::Fill).into()
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
    container(list).padding(18).width(ASIDE).style(slab_faded()).into()
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
    let title = running.title.clone().unwrap_or_else(|| w.t("duels-drafting"));
    let line = row![
        running.slot.as_deref().map(crate::duel_view::slot_tag).unwrap_or_else(|| Space::new().into()),
        strong(title, 18.0, INK),
        running.stars.map(|value| crate::dossier::star_pill(format!("{value:.2}").replace('.', ","), value, 13.0)).unwrap_or_else(|| Space::new().into()),
        Space::new().width(Length::Fill),
        glyph(Icon::Eye, 16.0, FAINT),
        words(running.watching.to_string(), 14.0, FAINT),
    ].spacing(10).align_y(Alignment::Center);
    let fields = row![side(w, state, running, 0, now, field), side(w, state, running, 1, now, field)].spacing(14);
    let page = column![head, line, fields, words(w.t("duels-delay"), 14.0, FAINT)].spacing(12).width(Length::Fill).height(Length::Fill);
    let page = row![page, others(w, state, running)].spacing(16).height(Length::Fill);
    container(page).padding(Padding { top: 8.0, right: 40.0, bottom: 24.0, left: 40.0 }).width(Length::Fill).height(Length::Fill).into()
}
