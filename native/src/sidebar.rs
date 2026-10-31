use iced::widget::{button, column, container, mouse_area, row, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Vector};

use crate::glyphs::{glyph, Icon};
use crate::theme::{self, INK, MUTED};
use crate::ui;

pub const WIDE: f32 = 330.0;
pub const NARROW: f32 = 0.0;
const HOT: f32 = 16.0;
const LEAF: f32 = 48.0;
const GAP: f32 = 6.0;
const SHORTEST: f32 = 216.0;
const BULGE: f32 = 62.0;
const CHOSEN_LONGER: f32 = 8.0;
const LEAN_LONGER: f32 = 18.0;
const STAGGER: f32 = 0.04;
const PLATE: f32 = 30.0;
const REST: Color = Color::from_rgb(0.357, 0.255, 0.271);
const FACE: (Color, Color) = (Color::from_rgb(0.047, 0.02, 0.027), Color::from_rgb(0.102, 0.05, 0.063));
const LIT: (Color, Color) = (Color::from_rgb(0.067, 0.031, 0.039), Color::from_rgb(0.157, 0.082, 0.102));
const CHOSEN: (Color, Color) = (Color::from_rgb(0.102, 0.035, 0.043), Color::from_rgb(0.31, 0.09, 0.106));
const PLATE_FACE: Color = Color::from_rgb(0.169, 0.094, 0.11);
const GLOW: Color = Color::from_rgba(0.914, 0.196, 0.251, 0.28);

pub fn leaf_length(at: usize, count: usize, chosen: bool, lean: f32) -> f32 {
    let middle = count.saturating_sub(1) as f32 / 2.0;
    let away = if middle > 0.0 { (at as f32 - middle) / middle } else { 0.0 };
    SHORTEST + BULGE * (1.0 - away * away) + if chosen { CHOSEN_LONGER } else { 0.0 } + LEAN_LONGER * lean
}

pub fn unfolded(open: f32, at: usize, chosen: Option<usize>, count: usize) -> f32 {
    let from = chosen.unwrap_or(count / 2);
    let farthest = from.max(count.saturating_sub(1).saturating_sub(from)) as f32;
    let wait = at.abs_diff(from) as f32 * STAGGER;
    let x = ((open - wait) / (1.0 - farthest * STAGGER).max(0.1)).clamp(0.0, 1.0);
    let rest = x - 1.0;
    1.0 + 2.2 * rest * rest * rest + 1.2 * rest * rest
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
    on: bool,
    press: Message,
    length: f32,
    shown: f32,
    open: f32,
    lit: bool,
    tick: f32,
}

fn leaf<'a, Message: Clone + 'a>(made: Leaf<Message>, lean: impl Fn(Option<&'static str>) -> Message) -> Element<'a, Message> {
    let Leaf { key, icon, label, on, press, length, shown, open, lit, tick } = made;
    let width = (length * shown).max(0.0);
    let colour = if on || lit { INK } else { MUTED };
    let words = ((shown - 0.35) / 0.65).clamp(0.0, 1.0);
    let ink = ui::faded(colour);
    let plate_fill = ui::faded(if on { theme::ACCENT } else { PLATE_FACE });
    let plate = container(glyph(icon, 17.0, if on { Color::WHITE } else { colour })).center(PLATE).style(move |_| container::Style {
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
    let inside = container(line).width(Length::Fill).height(LEAF).padding(Padding::ZERO.right(10.0)).align_x(iced::alignment::Horizontal::Right).align_y(iced::Center).clip(true);
    let (from, to) = if on { CHOSEN } else if lit { LIT } else { FACE };
    let k = ui::fade();
    let face = button(inside).padding(0).width(Length::Fill).on_press(press).style(move |_: &iced::Theme, _| button::Style {
        background: Some(Background::Gradient(iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2)).add_stop(0.0, Color { a: k, ..from }).add_stop(1.0, Color { a: k, ..to }).into())),
        text_color: colour,
        border: Border { radius: iced::border::radius(0.0).top_right(LEAF / 2.0).bottom_right(LEAF / 2.0), ..Border::default() },
        shadow: if on {
            Shadow { color: Color { a: GLOW.a * k, ..GLOW }, offset: Vector::new(4.0, 0.0), blur_radius: 22.0 }
        } else {
            Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.42 * k), offset: Vector::new(5.0, 6.0), blur_radius: 20.0 }
        },
        snap: true,
    });
    let hovered: Element<'a, Message> = if width < 1.0 { Space::new().into() } else { mouse_area(face).on_enter(lean(Some(key))).on_exit(lean(None)).into() };
    let body = container(hovered).id(iced::widget::Id::from(format!("side-{key}"))).width(width).height(LEAF);
    let rest = ui::faded(Color { a: 1.0 - open, ..if on { theme::ACCENT } else { REST } });
    let mark = container(Space::new().width(tick).height(if on { 22.0 } else { 6.0 })).style(move |_| container::Style {
        background: Some(Background::Color(rest)),
        border: Border { radius: iced::border::radius(0.0).top_right(3.0).bottom_right(3.0), ..Border::default() },
        ..container::Style::default()
    });
    iced::widget::stack![container(mark).height(LEAF).align_y(iced::Center), body].width(Length::Fill).height(LEAF).into()
}

pub fn view<'a, Message: Clone + 'a>(
    entries: Vec<Entry<Message>>,
    open: f32,
    leans: &std::collections::HashMap<String, f32>,
    hover: impl Fn(bool) -> Message,
    lean: impl Fn(Option<&'static str>) -> Message,
    moved: impl Fn(&'static str, Option<&'static str>) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let open = open.clamp(0.0, 1.0);
    let count = entries.iter().filter(|entry| key_of(entry).is_some()).count();
    let chosen = entries.iter().filter(|entry| key_of(entry).is_some()).position(|entry| matches!(entry, Entry::Item { on: true, .. }));
    let mut list = column![].spacing(GAP).width(Length::Fill);
    let mut at = 0;
    for (_, items) in split(entries) {
        let mut pieces: Vec<(&'static str, Element<'a, Message>)> = Vec::new();
        for entry in items {
            if let Entry::Item { key, icon, label, on, press } = entry {
                let leaning = leans.get(&format!("side-lean-{key}")).copied().unwrap_or(0.0);
                let made = Leaf { key, icon, label, on, press, length: leaf_length(at, count, on, leaning), shown: unfolded(open, at, chosen, count), open, lit: leaning > 0.5, tick: if on { 4.0 } else { 2.5 + 6.0 * (leaf_length(at, count, false, 0.0) - SHORTEST) / BULGE } };
                pieces.push((key, leaf(made, &lean)));
                at += 1;
            }
        }
        if !pieces.is_empty() {
            list = list.push(crate::board::board(pieces, GAP, moved.clone()).anywhere().radius(LEAF / 2.0).solid(FACE.1));
        }
    }
    let zone = container(list).width(if open > 0.001 { WIDE } else { HOT }).height(Length::Fill).center_y(Length::Fill);
    mouse_area(zone).on_enter(hover(true)).on_exit(hover(false)).into()
}

pub fn shade<'a, Message: 'a>(open: f32) -> Element<'a, Message> {
    let k = open.clamp(0.0, 1.0) * ui::fade();
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
        assert!(leaf_length(2, 5, false, 0.0) > leaf_length(1, 5, false, 0.0));
        assert_eq!(leaf_length(0, 5, false, 0.0), leaf_length(4, 5, false, 0.0));
        assert!(leaf_length(1, 5, true, 0.0) > leaf_length(1, 5, false, 0.0));
        assert!(leaf_length(1, 5, false, 1.0) > leaf_length(1, 5, true, 0.0));
        assert!(leaf_length(8, 9, true, 1.0) <= WIDE, "a leaf left the zone that keeps the fan open");
    }

    #[test]
    fn the_fan_opens_from_the_chosen_bookmark_and_comes_to_rest() {
        assert!(unfolded(0.25, 2, Some(2), 9) > unfolded(0.25, 6, Some(2), 9));
        for at in 0..9 {
            assert_eq!(unfolded(0.0, at, Some(2), 9), 0.0);
            assert!((unfolded(1.0, at, Some(2), 9) - 1.0).abs() < 0.001);
        }
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
