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

pub const RAISED: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.0077);
pub const SUNK: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.4844);
pub const LINE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.0154);
pub const LINE_HIGH: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.0392);
pub const ACCENT_SOFT: Color = Color::from_rgba(0.886, 0.282, 0.282, 0.0392);
pub const TAG: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.0108);
pub const KNOB_OFF: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.0324);

pub const CAPTION: f32 = 12.0;
pub const BODY: f32 = 14.0;
pub const LEAD: f32 = 16.0;
pub const TITLE: f32 = 22.0;
pub const CODE: f32 = 28.0;

pub const CARD_RADIUS: f32 = 12.0;
pub const CONTROL_RADIUS: f32 = 8.0;
pub const PROGRESS_WIDTH: f32 = 150.0;
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

pub fn rule_high(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(LINE_HIGH)),
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

pub const GRADE_SS: Color = color!(0xf7e08a);
pub const GRADE_S: Color = color!(0xf0c040);
pub const GRADE_A: Color = color!(0x8cd04a);
pub const GRADE_B: Color = color!(0x58aefc);
pub const GRADE_C: Color = color!(0xb06ce8);
pub const GRADE_D: Color = ACCENT;
pub const HIT_300: Color = GRADE_B;
pub const HIT_100: Color = GRADE_A;
pub const HIT_50: Color = GRADE_S;

pub const MOD_HARD: Color = color!(0xd64e48);
pub const MOD_EASY: Color = color!(0x7ac65c);
pub const MOD_AUTO: Color = color!(0x5694d6);
pub const MOD_OTHER: Color = color!(0x9668ce);

pub const SCRIM: Color = Color::from_rgba(0.027, 0.012, 0.016, 0.72);
pub const CHIP: Color = Color::from_rgba(0.027, 0.012, 0.016, 0.72);

pub const FRAME_W: f32 = 108.0;
pub const FRAME_H: f32 = 61.0;
pub const FRAME_RADIUS: f32 = 6.0;

pub fn scrim(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(SCRIM)),
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn chip(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(CHIP)),
        border: border(Color::TRANSPARENT, 3.0),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn badge(colour: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        text_color: Some(ON_ACCENT),
        background: Some(Background::Color(colour)),
        border: border(Color::TRANSPARENT, 4.0),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn hatched(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.012))),
        border: border(LINE, FRAME_RADIUS),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn frame(chosen: bool, lit: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let lit = lit || matches!(status, button::Status::Hovered);
        let edge = if chosen {
            Border { color: ACCENT, width: 2.0, radius: FRAME_RADIUS.into() }
        } else if lit {
            Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.08), width: 1.0, radius: FRAME_RADIUS.into() }
        } else {
            Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.02), width: 1.0, radius: FRAME_RADIUS.into() }
        };
        button::Style {
            background: Some(Background::Color(Color::from_rgb(0.04, 0.02, 0.03))),
            text_color: INK,
            border: edge,
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

pub fn word(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let text_color = match (on, status) {
            (true, _) => INK,
            (false, button::Status::Hovered | button::Status::Pressed) => INK,
            _ => MUTED,
        };
        plain(None, text_color, Color::TRANSPARENT)
    }
}

pub fn field_faded(alpha: f32) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |theme, status| {
        let mut style = field(theme, status);
        let dim = |c: Color| Color { a: c.a * alpha, ..c };
        style.background = match style.background {
            Background::Color(c) => Background::Color(dim(c)),
            other => other,
        };
        style.border.color = dim(style.border.color);
        style.icon = dim(style.icon);
        style.placeholder = dim(style.placeholder);
        style.value = dim(style.value);
        style
    }
}

pub fn progressing(_: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => Color::from_rgba(0.886, 0.282, 0.282, 0.07),
        _ => ACCENT_SOFT,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: INK,
        border: border(Color::from_rgba(0.886, 0.282, 0.282, 0.18), CONTROL_RADIUS),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn bar(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(ACCENT)),
        border: border(Color::TRANSPARENT, 1.0),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn row(chosen: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let background = match (chosen, status) {
            (true, _) => Some(Color::from_rgba(0.886, 0.282, 0.282, 0.08)),
            (false, button::Status::Hovered) => Some(RAISED),
            _ => None,
        };
        button::Style {
            background: background.map(Background::Color),
            text_color: INK,
            border: border(Color::TRANSPARENT, CONTROL_RADIUS),
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

pub fn bare(_: &Theme, _: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: INK,
        border: border(Color::TRANSPARENT, 14.0),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn tab(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let colour = match (on, status) {
            (true, _) => INK,
            (false, button::Status::Hovered) => MUTED,
            _ => FAINT,
        };
        button::Style {
            background: None,
            text_color: colour,
            border: border(Color::TRANSPARENT, 0.0),
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

pub fn tile_bad(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(Color::from_rgba(0.18, 0.05, 0.06, 0.97))),
        border: border(Color::from_rgba(0.886, 0.282, 0.282, 0.22), 10.0),
        shadow: Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.45), offset: iced::Vector::new(0.0, 8.0), blur_radius: 24.0 },
        snap: true,
    }
}

pub fn small(_: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => Color::from_rgba(1.0, 1.0, 1.0, 0.12),
        _ => Color::from_rgba(1.0, 1.0, 1.0, 0.07),
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: INK,
        border: border(Color::TRANSPARENT, 7.0),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn danger_words(_: &Theme, status: button::Status) -> button::Style {
    let colour = match status {
        button::Status::Hovered | button::Status::Pressed => Color::from_rgb(1.0, 0.45, 0.45),
        _ => ACCENT,
    };
    button::Style {
        background: None,
        text_color: colour,
        border: border(Color::TRANSPARENT, 0.0),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn ghost(_: &Theme, status: button::Status) -> button::Style {
    let colour = match status {
        button::Status::Hovered | button::Status::Pressed => INK,
        _ => FAINT,
    };
    button::Style {
        background: None,
        text_color: colour,
        border: border(Color::TRANSPARENT, 6.0),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn corner(bad: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let edge = if bad { Color::from_rgba(0.886, 0.282, 0.282, 0.22) } else { Color::from_rgba(1.0, 1.0, 1.0, 0.1) };
        let ground = if bad { Color::from_rgba(0.18, 0.05, 0.06, 1.0) } else { Color::from_rgba(0.047, 0.02, 0.027, 1.0) };
        let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(if lit { Color { r: ground.r + 0.06, g: ground.g + 0.04, b: ground.b + 0.04, a: 1.0 } } else { ground })),
            text_color: INK,
            border: border(edge, 10.0),
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

pub fn toast(pulse: f32, bad: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let mut style = bubble(theme);
        let tint = if bad { ACCENT } else { INK };
        let lift = 0.05 * pulse;
        if let Some(Background::Color(c)) = style.background {
            style.background = Some(Background::Color(Color { r: c.r + (tint.r - c.r) * lift, g: c.g + (tint.g - c.g) * lift, b: c.b + (tint.b - c.b) * lift, a: c.a }));
        }
        style.border.color = Color { a: 0.1 + 0.35 * pulse, ..tint };
        style
    }
}

pub fn segment_pill(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
        border: border(Color::TRANSPARENT, 8.0),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn segment(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let colour = match (on, status) {
            (true, _) => INK,
            (false, button::Status::Hovered) => MUTED,
            _ => FAINT,
        };
        button::Style {
            background: None,
            text_color: colour,
            border: border(Color::TRANSPARENT, 8.0),
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

pub fn badge_of(colour: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        text_color: None,
        background: Some(Background::Color(colour)),
        border: border(Color::from_rgba(0.047, 0.02, 0.027, 1.0), 8.0),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn stage(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(color!(0x0c0507))),
        border: border(Color::from_rgba(1.0, 1.0, 1.0, 0.08), CARD_RADIUS),
        shadow: Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.6), offset: iced::Vector::new(0.0, 30.0), blur_radius: 80.0 },
        snap: true,
    }
}

pub fn bubble(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(Color::from_rgba(0.047, 0.02, 0.027, 0.97))),
        border: border(Color::from_rgba(1.0, 1.0, 1.0, 0.1), 10.0),
        shadow: Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.45), offset: iced::Vector::new(0.0, 8.0), blur_radius: 24.0 },
        snap: true,
    }
}

pub fn bubble_faded(alpha: f32) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let mut style = bubble(theme);
        if let Some(Background::Color(c)) = style.background {
            style.background = Some(Background::Color(Color { a: c.a * alpha, ..c }));
        }
        style.border.color = Color { a: style.border.color.a * alpha, ..style.border.color };
        style.shadow.color = Color { a: style.shadow.color.a * alpha, ..style.shadow.color };
        style
    }
}
