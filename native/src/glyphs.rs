use iced::widget::svg;
use iced::{Color, Element};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Chevron,
    Target,
    Keys,
    Chain,
    Play,
    Star,
    Clock,
    Heart,
    Trophy,
    Chart,
    News,
    Send,
    Gear,
    Calendar,
    Flame,
    Swords,
    External,
    Compare,
    Up,
    Down,
    Search,
    Film,
    Flag,
    Metronome,
    Person,
    Chat,
    Circle,
    Refresh,
    Check,
    Warn,
    Right,
    Plus,
    Close,
    Pool,
    Copy,
    File,
    Shapes,
    Medal,
    Bell,
    Undo,
    Redo,
    Link,
    Sliders,
    People,
    Globe,
    Ban,
}

fn drawn(icon: Icon) -> &'static [u8] {
    match icon {
        Icon::Chevron => include_bytes!("../assets/icons/chevron-down.svg"),
        Icon::Target => include_bytes!("../assets/icons/target.svg"),
        Icon::Keys => include_bytes!("../assets/icons/keyboard.svg"),
        Icon::Chain => include_bytes!("../assets/icons/link-2.svg"),
        Icon::Play => include_bytes!("../assets/icons/play.svg"),
        Icon::Star => include_bytes!("../assets/icons/star-filled.svg"),
        Icon::Clock => include_bytes!("../assets/icons/clock.svg"),
        Icon::Heart => include_bytes!("../assets/icons/heart-filled.svg"),
        Icon::Trophy => include_bytes!("../assets/icons/trophy.svg"),
        Icon::Chart => include_bytes!("../assets/icons/chart-line.svg"),
        Icon::News => include_bytes!("../assets/icons/newspaper.svg"),
        Icon::Send => include_bytes!("../assets/icons/send.svg"),
        Icon::Gear => include_bytes!("../assets/icons/settings.svg"),
        Icon::Calendar => include_bytes!("../assets/icons/calendar.svg"),
        Icon::Flame => include_bytes!("../assets/icons/flame.svg"),
        Icon::Swords => include_bytes!("../assets/icons/swords.svg"),
        Icon::External => include_bytes!("../assets/icons/external-link.svg"),
        Icon::Compare => include_bytes!("../assets/icons/arrow-up-down.svg"),
        Icon::Up => include_bytes!("../assets/icons/arrow-up.svg"),
        Icon::Down => include_bytes!("../assets/icons/arrow-down.svg"),
        Icon::Search => include_bytes!("../assets/icons/search.svg"),
        Icon::Film => include_bytes!("../assets/icons/film.svg"),
        Icon::Flag => include_bytes!("../assets/icons/flag.svg"),
        Icon::Metronome => include_bytes!("../assets/icons/metronome.svg"),
        Icon::Person => include_bytes!("../assets/icons/user.svg"),
        Icon::Chat => include_bytes!("../assets/icons/message-square.svg"),
        Icon::Circle => include_bytes!("../assets/icons/circle-dot.svg"),
        Icon::Refresh => include_bytes!("../assets/icons/refresh-cw.svg"),
        Icon::Check => include_bytes!("../assets/icons/check.svg"),
        Icon::Warn => include_bytes!("../assets/icons/triangle-alert.svg"),
        Icon::Right => include_bytes!("../assets/icons/chevron-right.svg"),
        Icon::Plus => include_bytes!("../assets/icons/plus.svg"),
        Icon::Close => include_bytes!("../assets/icons/x.svg"),
        Icon::Pool => include_bytes!("../assets/icons/layers.svg"),
        Icon::Copy => include_bytes!("../assets/icons/copy.svg"),
        Icon::File => include_bytes!("../assets/icons/file.svg"),
        Icon::Shapes => include_bytes!("../assets/icons/shapes.svg"),
        Icon::Medal => include_bytes!("../assets/icons/medal.svg"),
        Icon::Bell => include_bytes!("../assets/icons/bell.svg"),
        Icon::Undo => include_bytes!("../assets/icons/undo-2.svg"),
        Icon::Redo => include_bytes!("../assets/icons/redo-2.svg"),
        Icon::Link => include_bytes!("../assets/icons/link.svg"),
        Icon::Sliders => include_bytes!("../assets/icons/sliders-horizontal.svg"),
        Icon::People => include_bytes!("../assets/icons/users.svg"),
        Icon::Globe => include_bytes!("../assets/icons/globe.svg"),
        Icon::Ban => include_bytes!("../assets/icons/ban.svg"),
    }
}

fn handle(icon: Icon) -> svg::Handle {
    static HELD: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<Icon, svg::Handle>>> = std::sync::OnceLock::new();
    let held = HELD.get_or_init(Default::default);
    let mut held = held.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    held.entry(icon).or_insert_with(|| svg::Handle::from_memory(drawn(icon))).clone()
}

pub fn glyph<'a, Message: 'a>(icon: Icon, side: f32, colour: Color) -> Element<'a, Message> {
    let alpha = crate::ui::fade();
    svg(handle(icon))
        .width(side)
        .height(side)
        .opacity(colour.a * alpha)
        .style(move |_, _| svg::Style { color: Some(Color { a: 1.0, ..colour }) })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_a_stroked_svg_from_the_set() {
        for icon in [Icon::Chevron, Icon::Target, Icon::Keys, Icon::Chain, Icon::Play, Icon::Star, Icon::Clock, Icon::Heart, Icon::Trophy, Icon::Chart, Icon::News, Icon::Send, Icon::Gear, Icon::Calendar, Icon::Flame, Icon::Swords, Icon::External, Icon::Compare, Icon::Up, Icon::Down, Icon::Search, Icon::Film, Icon::Flag, Icon::Metronome, Icon::Person, Icon::Chat, Icon::Circle, Icon::Refresh, Icon::Check, Icon::Warn, Icon::Right, Icon::Plus, Icon::Close, Icon::Pool, Icon::Copy, Icon::File, Icon::Shapes, Icon::Medal, Icon::Bell, Icon::Undo, Icon::Redo, Icon::Link, Icon::Sliders, Icon::People, Icon::Globe, Icon::Ban] {
            let said = std::str::from_utf8(drawn(icon)).unwrap();
            assert!(said.contains("<svg") && said.contains("stroke=\"currentColor\""), "{icon:?}");
        }
    }
}
