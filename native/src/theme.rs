use iced::widget::{button, container, text_input, toggler};
use iced::{color, font, Background, Border, Color, Font, Shadow, Theme};

pub const GROUND: Color = color!(0x0d0508);
pub const GROUND_TOP: Color = color!(0x3a1015);
pub const INK: Color = color!(0xece7e2);
pub const MUTED: Color = color!(0xa9a29b);
pub const FAINT: Color = color!(0x6b655f);
pub const ACCENT: Color = color!(0xe24848);
pub const ON_ACCENT: Color = color!(0xffffff);
pub const DANGER: Color = ACCENT;

pub const RAISED: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.045);
pub const SUNK: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.26);
pub const LINE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08);
pub const LINE_HIGH: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.16);
pub const ACCENT_SOFT: Color = Color::from_rgba(0.886, 0.282, 0.282, 0.16);
pub const TAG: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.06);
pub const KNOB_OFF: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.14);

pub const CAPTION: f32 = 12.0;
pub const BODY: f32 = 14.0;
pub const LEAD: f32 = 16.0;
pub const TITLE: f32 = 22.0;
pub const CODE: f32 = 28.0;

pub const CARD_RADIUS: f32 = 12.0;
pub const CONTROL_RADIUS: f32 = 8.0;
pub const CONTROL_HEIGHT: f32 = 32.0;
pub const COLUMN: f32 = 560.0;

pub const SANS: Font = Font::with_name("Commissioner");
pub const SANS_SEMI: Font = Font {
    weight: font::Weight::Semibold,
    ..Font::with_name("Commissioner")
};
pub const MONO: Font = Font::with_name("JetBrains Mono");
pub const MONO_BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::with_name("JetBrains Mono")
};

pub const FONTS: [&[u8]; 5] = [
    include_bytes!("../../app/ui/fonts/Commissioner-Regular.ttf"),
    include_bytes!("../../app/ui/fonts/Commissioner-SemiBold.ttf"),
    include_bytes!("../../app/ui/fonts/JetBrainsMono-Regular.ttf"),
    include_bytes!("../../app/ui/fonts/JetBrainsMono-Bold.ttf"),
    include_bytes!("../../app/ui/fonts/MPLUSRounded1c-Regular.ttf"),
];

pub fn theme() -> Theme {
    Theme::custom(
        "Dossier",
        iced::theme::Palette {
            background: GROUND,
            text: INK,
            primary: ACCENT,
            success: ACCENT,
            warning: MUTED,
            danger: DANGER,
        },
    )
}

fn border(color: Color, radius: f32) -> Border {
    Border {
        color,
        width: 1.0,
        radius: radius.into(),
    }
}

pub fn card(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(RAISED)),
        border: border(LINE, CARD_RADIUS),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn well(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(SUNK)),
        border: border(LINE, CONTROL_RADIUS),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn tile(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(SUNK)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: CONTROL_RADIUS.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn option(on: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        text_color: None,
        background: Some(Background::Color(if on { ACCENT_SOFT } else { SUNK })),
        border: border(if on { ACCENT } else { LINE }, CARD_RADIUS),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn tag(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(MUTED),
        background: Some(Background::Color(TAG)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn rule(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(LINE)),
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn qr_paper(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(Color::WHITE)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: CONTROL_RADIUS.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

fn plain(background: Option<Color>, text_color: Color, edge: Color) -> button::Style {
    button::Style {
        background: background.map(Background::Color),
        text_color,
        border: border(edge, CONTROL_RADIUS),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn primary(_: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => plain(Some(color!(0xea5a5a)), ON_ACCENT, Color::TRANSPARENT),
        button::Status::Pressed => plain(Some(color!(0xd13f3f)), ON_ACCENT, Color::TRANSPARENT),
        button::Status::Disabled => plain(Some(ACCENT_SOFT), FAINT, Color::TRANSPARENT),
        button::Status::Active => plain(Some(ACCENT), ON_ACCENT, Color::TRANSPARENT),
    }
}

pub fn quiet(_: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => plain(Some(RAISED), INK, Color::TRANSPARENT),
        button::Status::Pressed => plain(Some(SUNK), INK, Color::TRANSPARENT),
        button::Status::Disabled => plain(None, FAINT, Color::TRANSPARENT),
        button::Status::Active => plain(None, MUTED, Color::TRANSPARENT),
    }
}

pub fn choice(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let edge = match (on, status) {
            (true, _) => ACCENT,
            (false, button::Status::Hovered) => LINE_HIGH,
            _ => LINE,
        };
        button::Style {
            background: Some(Background::Color(if on { ACCENT_SOFT } else { SUNK })),
            text_color: INK,
            border: border(edge, CARD_RADIUS),
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

pub fn link(_: &Theme, status: button::Status) -> button::Style {
    let text_color = match status {
        button::Status::Hovered | button::Status::Pressed => INK,
        _ => MUTED,
    };
    plain(None, text_color, Color::TRANSPARENT)
}

pub fn field(_: &Theme, status: text_input::Status) -> text_input::Style {
    let edge = match status {
        text_input::Status::Focused { .. } => LINE_HIGH,
        text_input::Status::Hovered => LINE_HIGH,
        _ => LINE,
    };
    text_input::Style {
        background: Background::Color(SUNK),
        border: border(edge, CONTROL_RADIUS),
        icon: MUTED,
        placeholder: FAINT,
        value: INK,
        selection: ACCENT_SOFT,
    }
}

pub fn switch(_: &Theme, status: toggler::Status) -> toggler::Style {
    let on = match status {
        toggler::Status::Active { is_toggled } => is_toggled,
        toggler::Status::Hovered { is_toggled } => is_toggled,
        toggler::Status::Disabled { is_toggled } => is_toggled,
    };
    toggler::Style {
        background: Background::Color(if on { ACCENT } else { KNOB_OFF }),
        background_border_width: 0.0,
        background_border_color: Color::TRANSPARENT,
        foreground: Background::Color(if on { GROUND } else { INK }),
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
        text_color: None,
        border_radius: Some(9.0.into()),
        padding_ratio: 0.12,
    }
}
