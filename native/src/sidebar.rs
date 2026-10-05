use iced::widget::{button, column, container, mouse_area, row, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding};

use crate::glyphs::{glyph, Icon};
use crate::theme::{self, INK, MUTED};
use crate::ui;

pub const WIDE: f32 = 244.0;
pub const NARROW: f32 = 84.0;
const EDGE: f32 = 16.0;
const SLOT: f32 = NARROW - EDGE - 12.0;
const ITEM: f32 = 46.0;
const CAPTION: f32 = 20.0;
const IDLE_ICON: f32 = 24.0;
const OPEN_ICON: f32 = 22.0;
const PANEL: Color = Color::from_rgb(0.055, 0.025, 0.033);
const SHADE: Color = Color::from_rgb(0.02, 0.008, 0.012);

pub fn width_at(open: f32) -> f32 {
    NARROW + (WIDE - NARROW) * open.clamp(0.0, 1.0)
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

fn item<'a, Message: Clone + 'a>(key: &'static str, icon: Icon, label: String, on: bool, press: Message, open: f32) -> Element<'a, Message> {
    let colour = if on { INK } else { MUTED };
    let size = IDLE_ICON + (OPEN_ICON - IDLE_ICON) * open;
    let words = ((open - 0.3) / 0.7).clamp(0.0, 1.0);
    let mut line = row![glyph(icon, size, colour)].spacing(12).align_y(iced::Center);
    if words > 0.0 {
        let faded = ui::faded(colour);
        line = line.push(text(label).font(theme::SANS_SEMI).size(15.0).wrapping(text::Wrapping::None).color(Color { a: faded.a * words, ..faded }));
    }
    let inside = container(line).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: (SLOT - size) / 2.0 }).height(ITEM).width(Length::Fill).align_y(iced::Center).clip(true);
    container(button(inside).padding(0).width(Length::Fill).style(ui::button_faded(move |theme, status| {
        if open < 0.5 { return theme::side(on)(theme, status); }
        button::Style {
            background: matches!(status, button::Status::Hovered | button::Status::Pressed).then_some(Background::Color(Color::from_rgb(0.18, 0.18, 0.18))),
            text_color: colour,
            border: Border { radius: 10.0.into(), ..Border::default() },
            ..button::Style::default()
        }
    })).on_press(press))
        .id(iced::widget::Id::from(format!("side-{key}")))
        .width(Length::Fill)
        .into()
}

pub fn view<'a, Message: Clone + 'a>(
    entries: Vec<Entry<Message>>,
    open: f32,
    hover: impl Fn(bool) -> Message,
    moved: impl Fn(&'static str, Option<&'static str>) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let open = open.clamp(0.0, 1.0);
    let mut list = column![].spacing(3);
    for (said, items) in split(entries) {
        if let Some(said) = said {
            let caption: Element<'a, Message> = if open > 0.3 {
                text(said).font(theme::SANS_SEMI).size(11.0).color(Color { a: ui::fade() * ((open - 0.3) / 0.7), ..MUTED }).into()
            } else { Space::new().into() };
            list = list.push(container(caption).height(CAPTION).padding([0, 10]).align_y(iced::Center));
        }
        let pieces: Vec<(&'static str, Element<'a, Message>)> = items
            .into_iter()
            .filter_map(|entry| match entry {
                Entry::Item { key, icon, label, on, press } => Some((key, item(key, icon, label, on, press, open))),
                Entry::Caption(_) => None,
            })
            .collect();
        if !pieces.is_empty() {
            list = list.push(crate::board::board(pieces, 3.0, moved.clone()).anywhere().radius(12.0).solid(PANEL));
        }
    }
    let k = open * ui::fade();
    let panel = container(list)
        .width(width_at(open))
        .padding(Padding { top: 30.0, right: 12.0, bottom: 14.0, left: EDGE })
        .style(move |_| container::Style {
            background: (k > 0.0).then_some(Background::Color(Color { a: 0.5 * k, ..SHADE })),
            border: Border { radius: iced::border::Radius { top_left: 0.0, top_right: 18.0, bottom_right: 18.0, bottom_left: 0.0 }, ..Border::default() },
            ..container::Style::default()
        });
    mouse_area(panel).on_enter(hover(true)).on_exit(hover(false)).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<Entry<()>> {
        let item = |key: &'static str| Entry::Item { key, icon: Icon::Gear, label: key.to_owned(), on: false, press: () };
        vec![item("profile"), item("feed"), Entry::Caption("People".into()), item("chat"), item("game"), item("compare"), Entry::Caption("Standing".into()), item("boards"), item("titles")]
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
