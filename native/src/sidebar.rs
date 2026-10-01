use iced::widget::{button, column, container, row, text, tooltip, Space};
use iced::{Color, Element, Length, Padding};

use crate::glyphs::{glyph, Icon};
use crate::theme::{self, FAINT, INK, MUTED};
use crate::ui;

pub const WIDE: f32 = 212.0;
pub const NARROW: f32 = 76.0;
pub const WIDE_FROM: f32 = 1180.0;

pub enum Entry<Message> {
    Caption(String),
    Item { key: &'static str, icon: Icon, label: String, on: bool, press: Message },
}

pub fn width_for(window: f32) -> f32 {
    if window >= WIDE_FROM {
        WIDE
    } else {
        NARROW
    }
}

pub fn view<'a, Message: Clone + 'a>(entries: Vec<Entry<Message>>, window: f32) -> Element<'a, Message> {
    let wide = window >= WIDE_FROM;
    let mut list = column![].spacing(3);
    for entry in entries {
        match entry {
            Entry::Caption(said) if wide => {
                list = list.push(container(text(said).font(theme::SANS).size(12.0).color(ui::faded(FAINT))).padding(Padding { top: 16.0, right: 0.0, bottom: 5.0, left: 12.0 }));
            }
            Entry::Caption(_) => {
                list = list.push(container(container(Space::new().height(1.0)).width(Length::Fill).style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.07 * ui::fade()))),
                    ..container::Style::default()
                }))
                .padding(Padding { top: 10.0, right: 10.0, bottom: 10.0, left: 10.0 }));
            }
            Entry::Item { key, icon, label, on, press } => {
                let colour = if on { INK } else { MUTED };
                let inside: Element<'a, Message> = if wide {
                    row![glyph(icon, 17.0, colour), text(label.clone()).font(theme::SANS_SEMI).size(14.0).wrapping(text::Wrapping::None).color(ui::faded(colour))]
                        .spacing(12)
                        .align_y(iced::Center)
                        .into()
                } else {
                    container(glyph(icon, 19.0, colour)).center_x(Length::Fill).into()
                };
                let item: Element<'a, Message> = container(
                    button(container(inside).padding(if wide { [9, 12] } else { [10, 0] }).width(Length::Fill))
                        .padding(0)
                        .width(Length::Fill)
                        .style(ui::button_faded(ui::calm(theme::side(on))))
                        .on_press(press),
                )
                .id(iced::widget::Id::from(format!("side-{key}")))
                .into();
                list = list.push(if wide {
                    item
                } else {
                    tooltip(item, container(text(label).font(theme::SANS_SEMI).size(13.0).color(INK)).padding([6, 10]).style(ui::box_faded(theme::slab)), tooltip::Position::Right).gap(8).into()
                });
            }
        }
    }
    container(list)
        .width(width_for(window))
        .padding(Padding { top: 30.0, right: 12.0, bottom: 0.0, left: if wide { 28.0 } else { 16.0 } })
        .into()
}
