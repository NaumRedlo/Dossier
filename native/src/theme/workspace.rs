use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme};

use super::{ACCENT, CARD_RADIUS, CONTROL_RADIUS, FAINT, GROUND, INK, MUTED};

pub const GAP: f32 = 16.0;
pub const INSET: f32 = 20.0;
pub const CONTROL: f32 = 36.0;
pub const SPLIT_MIN: f32 = 960.0;
pub const SURFACE: Color = Color::from_rgba(0.051, 0.02, 0.031, 0.88);
pub const ITEM: Color = Color::from_rgba(1.0, 0.92, 0.92, 0.045);
pub const HOVER: Color = Color::from_rgba(1.0, 0.92, 0.92, 0.09);

pub fn surface(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        text_color: Some(INK),
        border: Border { radius: CARD_RADIUS.into(), ..Border::default() },
        ..container::Style::default()
    }
}

pub fn choice(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let disabled = status == button::Status::Disabled;
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let fill = if disabled { ITEM } else if selected {
            crate::ui::mix(ACCENT, GROUND, if hovered { 0.65 } else { 0.78 })
        } else if hovered { HOVER } else { ITEM };
        button::Style {
            background: Some(Background::Color(fill)),
            text_color: if disabled { FAINT } else if selected || hovered { INK } else { MUTED },
            border: Border {
                color: if selected && !disabled { Color { a: 0.8, ..ACCENT } } else { Color::TRANSPARENT },
                width: if selected && !disabled { 1.0 } else { 0.0 },
                radius: CONTROL_RADIUS.into(),
            },
            ..button::Style::default()
        }
    }
}
