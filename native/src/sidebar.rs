use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path};
use iced::widget::{button, column, container, mouse_area, row, text, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Point, Rectangle, Renderer, Shadow, Theme, Vector};

use crate::glyphs::{glyph, Icon};
use crate::theme::{self, INK, MUTED};
use crate::ui;

pub const WIDE: f32 = 330.0;
pub const NARROW: f32 = 0.0;
const HOT: f32 = 16.0;
const LEAF: f32 = 48.0;
const GAP: f32 = 12.0;
const FILLET: f32 = 24.0;
const TICK_HIGH: f32 = 6.0;
const TICK_CHOSEN_HIGH: f32 = 22.0;
const TICK_CHOSEN_WIDE: f32 = 4.0;
const SHORTEST: f32 = 216.0;
const BULGE: f32 = 62.0;
const CHOSEN_LONGER: f32 = 8.0;
const LEAN_LONGER: f32 = 18.0;
const STAGGER: f32 = 0.04;
const PLATE: f32 = 30.0;
const REST: Color = Color::from_rgb(0.357, 0.255, 0.271);
const FACE: (Color, Color) = (Color::from_rgb(0.086, 0.043, 0.055), Color::from_rgb(0.102, 0.05, 0.063));
const LIT: (Color, Color) = (Color::from_rgb(0.122, 0.063, 0.078), Color::from_rgb(0.157, 0.082, 0.102));
const CHOSEN: (Color, Color) = (Color::from_rgb(0.216, 0.059, 0.075), Color::from_rgb(0.31, 0.09, 0.106));
const PLATE_FACE: Color = Color::from_rgb(0.169, 0.094, 0.11);
const GLOW: Color = Color::from_rgba(0.914, 0.196, 0.251, 0.28);
const DROP: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.42);

pub fn leaf_length(at: usize, count: usize, picked: f32, lean: f32) -> f32 {
    let middle = count.saturating_sub(1) as f32 / 2.0;
    let away = if middle > 0.0 { (at as f32 - middle) / middle } else { 0.0 };
    SHORTEST + BULGE * (1.0 - away * away) + CHOSEN_LONGER * picked + LEAN_LONGER * lean
}

fn soft(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

pub fn unfolded(open: f32, at: usize, chosen: Option<usize>, count: usize) -> f32 {
    let from = chosen.unwrap_or(count / 2);
    let farthest = from.max(count.saturating_sub(1).saturating_sub(from)) as f32;
    let wait = at.abs_diff(from) as f32 * STAGGER;
    soft((open - wait) / (1.0 - farthest * STAGGER).max(0.1))
}

pub enum Entry<Message> {
    Caption(String),
    Item { key: &'static str, icon: Icon, label: String, on: bool, press: Message },
}

fn split<Message>(entries: Vec<Entry<Message>>) -> Vec<(Option<String>, Vec<Entry<Message>>)> {
    let mut groups: Vec<(Option<String>, Vec<Entry<Message>>)> = vec![(None, Vec::new())];
    for entry in entries {
        match entry {
            Entry::Caption(said) => groups.push((Some(said), Vec::new())),
            item => groups.last_mut().expect("a group").1.push(item),
        }
    }
    groups.retain(|(caption, items)| caption.is_some() || !items.is_empty());
    groups
}

fn key_of<Message>(entry: &Entry<Message>) -> Option<&'static str> {
    match entry {
        Entry::Item { key, .. } => Some(key),
        Entry::Caption(_) => None,
    }
}

pub fn arranged<Message>(entries: Vec<Entry<Message>>, kept: &[String]) -> Vec<Entry<Message>> {
    let place = |entry: &Entry<Message>| key_of(entry).and_then(|key| kept.iter().position(|own| own == key)).unwrap_or(usize::MAX);
    split(entries)
        .into_iter()
        .flat_map(|(caption, mut items)| {
            items.sort_by_key(|item| place(item));
            caption.map(Entry::Caption).into_iter().chain(items)
        })
        .collect()
}

pub fn groups<Message>(entries: &[Entry<Message>]) -> Vec<Vec<&'static str>> {
    let mut groups: Vec<Vec<&'static str>> = vec![Vec::new()];
    for entry in entries {
        match key_of(entry) {
            Some(key) => groups.last_mut().expect("a group").push(key),
            None => groups.push(Vec::new()),
        }
    }
    groups.retain(|group| !group.is_empty());
    groups
}

pub fn moved(groups: &[Vec<&'static str>], kept: &[String], what: &'static str, before: Option<&'static str>) -> Vec<String> {
    let mut groups = groups.to_vec();
    for group in groups.iter_mut().filter(|group| group.contains(&what)) {
        group.retain(|key| *key != what);
        let at = before.and_then(|before| group.iter().position(|key| *key == before)).unwrap_or(group.len());
        group.insert(at, what);
    }
    let shown: Vec<String> = groups.concat().into_iter().map(str::to_owned).collect();
    let others = kept.iter().filter(|key| !shown.contains(key)).cloned();
    shown.iter().cloned().chain(others).collect()
}

struct Leaf<Message> {
    key: &'static str,
    icon: Icon,
    label: String,
    press: Message,
    length: f32,
    shown: f32,
    picked: f32,
    lit: f32,
    tick: f32,
}

struct Fillets {
    colour: Color,
    high: f32,
    grown: f32,
}

impl<M> canvas::Program<M> for Fillets {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let middle = bounds.height / 2.0;
        let (reach, wide) = (GAP / 2.0 * self.grown, FILLET * self.grown);
        for side in [-1.0, 1.0] {
            let meets = middle + side * self.high / 2.0;
            let shape = Path::new(|path| {
                path.move_to(Point::new(0.0, meets + side * reach));
                path.quadratic_curve_to(Point::new(0.0, meets), Point::new(wide, meets));
                path.line_to(Point::new(wide, meets - side));
                path.line_to(Point::new(0.0, meets - side));
                path.close();
            });
            frame.fill(&shape, self.colour);
        }
        vec![frame.into_geometry()]
    }
}

fn leaf<'a, Message: Clone + 'a>(made: Leaf<Message>, lean: impl Fn(Option<&'static str>) -> Message) -> Element<'a, Message> {
    let Leaf { key, icon, label, press, length, shown, picked, lit, tick } = made;
    let k = ui::fade();
    let grown = soft(shown / 0.45);
    let tick = tick + (TICK_CHOSEN_WIDE - tick) * picked;
    let width = tick + (length - tick).max(0.0) * shown;
    let rest_high = TICK_HIGH + (TICK_CHOSEN_HIGH - TICK_HIGH) * picked;
    let high = rest_high + (LEAF - rest_high) * grown;
    let round = (high / 2.0).min(width);
    let colour = ui::mix(MUTED, INK, lit.max(picked));
    let words = ((shown - 0.4) / 0.6).clamp(0.0, 1.0);
    let inside: Element<'a, Message> = if words > 0.0 {
        let ink = ui::faded(colour);
        let plate_fill = Color { a: k * words, ..ui::mix(PLATE_FACE, theme::ACCENT, picked) };
        let plate = container(glyph(icon, 17.0, Color { a: words, ..ui::mix(colour, Color::WHITE, picked) })).center(PLATE).style(move |_| container::Style {
            background: Some(Background::Color(plate_fill)),
            border: Border { radius: (PLATE / 2.0).into(), ..Border::default() },
            ..container::Style::default()
        });
        let line = row![
            text(label).font(theme::SANS_SEMI).size(14.0).wrapping(text::Wrapping::None).color(Color { a: ink.a * words, ..ink }),
            plate,
        ]
        .spacing(12)
        .align_y(iced::Center);
        container(line).width(Length::Fill).height(high).padding(Padding::ZERO.right(10.0)).align_x(iced::alignment::Horizontal::Right).align_y(iced::Center).clip(true).into()
    } else {
        Space::new().width(Length::Fill).height(high).into()
    };
    let rest = ui::mix(REST, theme::ACCENT, picked);
    let stop = |pick: fn(&(Color, Color)) -> Color| {
        let face = ui::mix(ui::mix(pick(&FACE), pick(&LIT), lit), pick(&CHOSEN), picked);
        Color { a: k, ..ui::mix(rest, face, grown) }
    };
    let (from, to) = (stop(|pair| pair.0), stop(|pair| pair.1));
    let cast = Color { a: (DROP.a + (GLOW.a - DROP.a) * picked) * k * grown, ..ui::mix(DROP, GLOW, picked) };
    let shadow = Shadow { color: cast, offset: Vector::new(5.0 - picked, 6.0 * (1.0 - picked)), blur_radius: 20.0 + 2.0 * picked };
    let face = button(inside).padding(0).width(Length::Fill).on_press_maybe((shown > 0.3).then_some(press)).style(move |_: &Theme, _| button::Style {
        background: Some(Background::Gradient(iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2)).add_stop(0.0, from).add_stop(1.0, to).into())),
        text_color: colour,
        border: Border { radius: iced::border::radius(0.0).top_right(round).bottom_right(round), ..Border::default() },
        shadow,
        snap: true,
    });
    let hovered = mouse_area(face).on_enter(lean(Some(key))).on_exit(lean(None));
    let body = container(hovered).id(iced::widget::Id::from(format!("side-{key}"))).width(width).height(high);
    let placed = container(body).width(Length::Fill).height(LEAF + GAP).align_y(iced::Center);
    if grown < 0.01 {
        return placed.into();
    }
    let fillets = Canvas::new(Fillets { colour: from, high, grown }).width(FILLET).height(LEAF + GAP);
    iced::widget::stack![fillets, placed].width(Length::Fill).height(LEAF + GAP).into()
}

pub fn view<'a, Message: Clone + 'a>(
    entries: Vec<Entry<Message>>,
    open: f32,
    marks: &std::collections::HashMap<String, f32>,
    hover: impl Fn(bool) -> Message,
    lean: impl Fn(Option<&'static str>) -> Message,
    moved: impl Fn(&'static str, Option<&'static str>) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let open = open.clamp(0.0, 1.0);
    let count = entries.iter().filter(|entry| key_of(entry).is_some()).count();
    let chosen = entries.iter().filter(|entry| key_of(entry).is_some()).position(|entry| matches!(entry, Entry::Item { on: true, .. }));
    let mut list = column![].width(Length::Fill);
    let mut at = 0;
    for (_, items) in split(entries) {
        let mut pieces: Vec<(&'static str, Element<'a, Message>)> = Vec::new();
        for entry in items {
            if let Entry::Item { key, icon, label, on, press } = entry {
                let lit = marks.get(&format!("side-lean-{key}")).copied().unwrap_or(0.0);
                let picked = marks.get(&format!("side-on-{key}")).copied().unwrap_or(if on { 1.0 } else { 0.0 });
                let length = leaf_length(at, count, picked, lit);
                let tick = 2.5 + 6.0 * (leaf_length(at, count, 0.0, 0.0) - SHORTEST) / BULGE;
                let made = Leaf { key, icon, label, press, length, shown: unfolded(open, at, chosen, count), picked, lit, tick };
                pieces.push((key, leaf(made, &lean)));
                at += 1;
            }
        }
        if !pieces.is_empty() {
            list = list.push(crate::board::board(pieces, 0.0, moved.clone()).anywhere().radius(LEAF / 2.0));
        }
    }
    let zone = container(list).width(if open > 0.001 { WIDE } else { HOT }).height(Length::Fill).center_y(Length::Fill);
    mouse_area(zone).on_enter(hover(true)).on_exit(hover(false)).into()
}

pub fn shade<'a, Message: 'a>(open: f32) -> Element<'a, Message> {
    let k = soft(open) * ui::fade();
    if k < 0.001 {
        return Space::new().width(Length::Shrink).height(Length::Shrink).into();
    }
    let ground = theme::GROUND;
    container(Space::new().width(440.0).height(Length::Fill))
        .style(move |_| container::Style {
            background: Some(Background::Gradient(iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
                .add_stop(0.0, Color { a: 0.82 * k, ..ground })
                .add_stop(0.55, Color { a: 0.5 * k, ..ground })
                .add_stop(1.0, Color { a: 0.0, ..ground })
                .into())),
            ..container::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<Entry<()>> {
        let item = |key: &'static str| Entry::Item { key, icon: Icon::Gear, label: key.to_owned(), on: false, press: () };
        vec![item("profile"), item("feed"), Entry::Caption("People".into()), item("chat"), item("game"), item("compare"), Entry::Caption("Standing".into()), item("boards"), item("titles")]
    }

    #[test]
    fn the_bookmarks_form_an_arc_longest_in_the_middle() {
        assert!(leaf_length(2, 5, 0.0, 0.0) > leaf_length(1, 5, 0.0, 0.0));
        assert_eq!(leaf_length(0, 5, 0.0, 0.0), leaf_length(4, 5, 0.0, 0.0));
        assert!(leaf_length(1, 5, 1.0, 0.0) > leaf_length(1, 5, 0.5, 0.0));
        assert!(leaf_length(1, 5, 0.5, 0.0) > leaf_length(1, 5, 0.0, 0.0));
        assert!(leaf_length(1, 5, 0.0, 1.0) > leaf_length(1, 5, 1.0, 0.0));
        assert!(leaf_length(8, 9, 1.0, 1.0) <= WIDE, "a leaf left the zone that keeps the fan open");
    }

    #[test]
    fn the_fan_opens_from_the_chosen_bookmark_and_comes_to_rest() {
        assert!(unfolded(0.25, 2, Some(2), 9) > unfolded(0.25, 6, Some(2), 9));
        for at in 0..9 {
            assert_eq!(unfolded(0.0, at, Some(2), 9), 0.0);
            assert!((unfolded(1.0, at, Some(2), 9) - 1.0).abs() < 0.001);
        }
        let steps: Vec<f32> = (0..=100).map(|step| unfolded(step as f32 / 100.0, 6, Some(2), 9)).collect();
        assert!(steps.windows(2).all(|pair| pair[1] >= pair[0] && pair[1] <= 1.0), "a leaf shook on its way in or out");
    }

    #[test]
    fn a_kept_order_moves_icons_only_inside_their_own_group() {
        let kept = ["compare", "titles", "chat", "feed"].map(str::to_owned);
        let shown = groups(&arranged(entries(), &kept));
        assert_eq!(shown, [vec!["feed", "profile"], vec!["compare", "chat", "game"], vec!["titles", "boards"]]);
        assert!(matches!(arranged(entries(), &kept)[2], Entry::Caption(_)), "a caption left its place");
    }

    #[test]
    fn a_dragged_icon_lands_before_its_neighbour_and_the_other_sidebar_keeps_its_order() {
        let kept = ["bot-side".to_owned()];
        let shown = groups(&arranged(entries(), &kept));
        let next = moved(&shown, &kept, "compare", Some("chat"));
        assert_eq!(next, ["profile", "feed", "compare", "chat", "game", "boards", "titles", "bot-side"]);
        let last = moved(&groups(&arranged(entries(), &next)), &next, "profile", None);
        assert_eq!(&last[..2], ["feed", "profile"], "an icon dropped at the end of its group went elsewhere");
    }
}
