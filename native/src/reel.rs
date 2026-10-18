use std::collections::VecDeque;
use std::time::{Duration, Instant};

use iced::animation::Easing;
use iced::widget::{column, container, stack, text, Space};
use iced::{Animation, Background, Color, Element, Length, Padding};

use super::{Live, Main, Message};
use crate::{live, theme};

pub const FADE: Duration = Duration::from_millis(900);
pub const RECENT: usize = 12;
pub const GIVE_UP: u32 = 8;
pub const START_LEAST: f64 = 0.05;
pub const START_MOST: f64 = 0.55;
pub const LOGO: f32 = 2.2;
pub const SHADE: f32 = 0.5;

#[derive(Debug, Clone, Default)]
pub struct Reel {
    pub now: Option<(usize, Live)>,
    pub before: Option<crate::film::Frame>,
    pub recent: VecDeque<usize>,
    pub failed: u32,
    pub serial: u64,
    seed: u64,
}

impl Reel {
    fn roll(&mut self) -> u64 {
        if self.seed == 0 {
            self.seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0x9e37_79b9_7f4a_7c15, |since| since.as_nanos() as u64) | 1;
        }
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.seed = x;
        x
    }

    pub fn running(&self) -> bool {
        self.now.is_some()
    }

    pub fn fading(&self, now: Instant) -> bool {
        self.now.as_ref().is_some_and(|(_, live)| live.fade.is_animating(now))
    }

    fn halt(&mut self) {
        if let Some((_, live)) = self.now.take() {
            live.control.stop();
            self.before = live.frame.or(self.before.take());
        }
    }
}

pub fn pick(candidates: &[usize], recent: &VecDeque<usize>, roll: u64) -> Option<usize> {
    let fresh: Vec<usize> = candidates.iter().copied().filter(|at| !recent.contains(at)).collect();
    let pool = if fresh.is_empty() { candidates.to_vec() } else { fresh };
    (!pool.is_empty()).then(|| pool[(roll % pool.len() as u64) as usize])
}

pub fn start_at(roll: u64) -> f64 {
    START_LEAST + (START_MOST - START_LEAST) * ((roll >> 16) % 1000) as f64 / 1000.0
}

impl Main {
    pub(super) fn reel_next(&mut self) -> Task {
        self.reel.halt();
        if self.reel.failed >= GIVE_UP || !self.settings.live_scene || self.gallery {
            return iced::Task::none();
        }
        let candidates: Vec<usize> = self.entries().iter().enumerate().filter(|(_, entry)| entry.map.is_some()).map(|(at, _)| at).collect();
        let roll = self.reel.roll();
        let Some(at) = pick(&candidates, &self.reel.recent, roll) else {
            return iced::Task::none();
        };
        self.reel.recent.push_back(at);
        while self.reel.recent.len() > RECENT.min(candidates.len().saturating_sub(1)).max(1) {
            self.reel.recent.pop_front();
        }
        let start = start_at(self.reel.roll());
        let entry = &self.entries()[at];
        let Some(map) = entry.map.as_ref() else {
            return iced::Task::none();
        };
        let ask = live::Ask { replay: entry.path.clone(), map: map.file.clone(), map_hash: entry.map_hash.clone(), skin: self.settings.skin.clone(), start, once: true };
        let control = std::sync::Arc::new(live::Control::default());
        self.reel.serial += 1;
        let serial = self.reel.serial;
        self.reel.now = Some((at, Live {
            control: control.clone(),
            for_path: ask.replay.clone(),
            frame: None,
            at_ms: 0.0,
            fade: Animation::new(false).duration(FADE).easing(Easing::EaseInOutCubic),
            rest: Animation::new(false),
        }));
        live::play(ask, control).map(move |frame| Message::Reel(serial, frame))
    }

    pub(super) fn reel_stop(&mut self) {
        self.reel.halt();
        self.reel.failed = 0;
    }

    pub(super) fn reel_frame(&mut self, serial: u64, frame: live::Frame) -> Task {
        if serial != self.reel.serial || !self.reel.running() {
            return iced::Task::none();
        }
        match frame {
            live::Frame::Picture { frame, at_ms, .. } => {
                if let Some((_, live)) = self.reel.now.as_mut() {
                    if live.frame.is_none() {
                        live.fade.go_mut(true, Instant::now());
                    }
                    live.frame = Some(frame);
                    live.at_ms = at_ms;
                }
                self.reel.failed = 0;
                iced::Task::none()
            }
            live::Frame::Ended => self.reel_next(),
            live::Frame::Failed(_) => {
                self.reel.failed += 1;
                self.reel_next()
            }
        }
    }

    pub(super) fn reel_view(&self, rest: f32) -> Element<'_, Message> {
        let blank = || Space::new().width(Length::Fill).height(Length::Fill).into();
        if rest <= 0.001 {
            return blank();
        }
        let veil = container(Space::new().width(Length::Fill).height(Length::Fill)).style(move |_| container::Style {
            background: Some(Background::Color(Color { a: rest, ..theme::GROUND })),
            ..container::Style::default()
        });
        let current = self.reel.now.as_ref().and_then(|(at, live)| live.frame.as_ref().map(|frame| (*at, frame, live.fade.interpolate(0.0, 1.0, self.now))));
        let seen = current.as_ref().map_or(0.0, |(_, _, seen)| *seen);
        let mut layers = stack![veil].width(Length::Fill).height(Length::Fill);
        if let Some(before) = self.reel.before.as_ref().filter(|_| seen < 0.999) {
            layers = layers.push(crate::film::show(before, crate::film::Fit::Cover, rest * (1.0 - seen)));
        }
        if let Some((_, frame, seen)) = &current {
            layers = layers.push(crate::film::show(frame, crate::film::Fit::Cover, rest * seen));
        }
        let shade = container(Space::new().width(Length::Fill).height(Length::Fill)).style(move |_| container::Style {
            background: Some(Background::Gradient(iced::Gradient::Linear(
                iced::gradient::Linear::new(iced::Radians(std::f32::consts::PI))
                    .add_stop(0.0, Color { a: rest * SHADE * 0.8, ..theme::GROUND })
                    .add_stop(0.5, Color { a: rest * SHADE, ..theme::GROUND })
                    .add_stop(1.0, Color { a: rest * 0.85, ..theme::GROUND }),
            ))),
            ..container::Style::default()
        });
        layers = layers.push(shade);
        if let Some((at, _, seen)) = current {
            if let Some(entry) = self.entries().get(at) {
                let song = entry.song().unwrap_or_else(|| self.words.t("unknown-map"));
                let caption = column![
                    text(entry.player.clone()).font(theme::SANS_SEMI).size(theme::BODY).color(Color { a: rest * seen, ..theme::INK }),
                    text(song).font(theme::SANS).size(theme::CAPTION).color(Color { a: rest * seen * 0.7, ..theme::INK }),
                ]
                .spacing(2);
                layers = layers.push(container(caption).padding(Padding { top: 0.0, right: 40.0, bottom: 36.0, left: 40.0 }).width(Length::Fill).height(Length::Fill).align_y(iced::alignment::Vertical::Bottom));
            }
        }
        layers.into()
    }
}

type Task = iced::Task<Message>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reel_skips_what_it_showed_lately_and_starts_inside_the_first_half() {
        let mut recent = VecDeque::new();
        recent.extend([1, 2]);
        for roll in 0..50 {
            let picked = pick(&[1, 2, 3, 4], &recent, roll).unwrap();
            assert!(picked == 3 || picked == 4);
            let start = start_at(roll * 7919);
            assert!((START_LEAST..=START_MOST).contains(&start));
        }
        assert_eq!(pick(&[1], &recent, 5), Some(1), "one replay is shown again rather than nothing");
        assert_eq!(pick(&[], &recent, 5), None);
    }
}
