mod easing;
mod parse;

use crate::timing::SampleSet;

pub use easing::ease;
pub use parse::{parse, parse_reporting, ParseError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    Background,

    Fail,

    Pass,
    Foreground,
    Overlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    TopLeft,
    TopCentre,
    TopRight,
    CentreLeft,
    Centre,
    CentreRight,
    BottomLeft,
    BottomCentre,
    BottomRight,
}

impl Origin {
    #[must_use]
    pub fn fractions(self) -> (f32, f32) {
        let across = match self {
            Self::TopLeft | Self::CentreLeft | Self::BottomLeft => 0.0,
            Self::TopCentre | Self::Centre | Self::BottomCentre => 0.5,
            Self::TopRight | Self::CentreRight | Self::BottomRight => 1.0,
        };
        let down = match self {
            Self::TopLeft | Self::TopCentre | Self::TopRight => 0.0,
            Self::CentreLeft | Self::Centre | Self::CentreRight => 0.5,
            Self::BottomLeft | Self::BottomCentre | Self::BottomRight => 1.0,
        };
        (across, down)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Fade(f32, f32),
    Move(f32, f32, f32, f32),
    MoveX(f32, f32),
    MoveY(f32, f32),
    Scale(f32, f32),

    ScaleVector(f32, f32, f32, f32),

    Rotate(f32, f32),
    Colour([u8; 3], [u8; 3]),

    Parameter(Switch),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    FlipHorizontally,
    FlipVertically,
    Additive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub easing: u8,
    pub start_ms: f64,
    pub end_ms: f64,
    pub change: Change,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Addition {
    Whistle,
    Finish,
    Clap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HitSoundMatch {
    pub set: Option<SampleSet>,
    pub addition_set: Option<SampleSet>,
    pub addition: Option<Addition>,
    pub custom: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sounded {
    pub time_ms: f64,
    pub set: SampleSet,
    pub addition_set: SampleSet,
    pub addition: Option<Addition>,
    pub custom: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fires {
    HitSound(HitSoundMatch),
    Passing,
    Failing,

    Unreadable,
}

impl HitSoundMatch {
    pub fn matches(self, hit: &Sounded) -> bool {
        self.set.is_none_or(|set| set == hit.set)
            && self.addition_set.is_none_or(|set| set == hit.addition_set)
            && self.addition == hit.addition.filter(|_| self.addition.is_some())
            && self.custom.is_none_or(|n| n == hit.custom)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trigger {
    pub fires: Fires,

    pub start_ms: f64,
    pub end_ms: f64,

    pub group: i32,

    pub body: Vec<Command>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    pub layer: Layer,
    pub origin: Origin,

    pub path: String,
    pub x: f32,
    pub y: f32,

    pub animation: Option<Animation>,
    pub commands: Vec<Command>,

    pub triggers: Vec<Trigger>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Animation {
    pub frames: u32,
    pub frame_ms: f64,

    pub once: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Video {
    pub path: String,
    pub start_ms: f64,
    pub offset: (f32, f32),
}

impl Storyboard {
    #[must_use]
    pub fn fired(&self, sounds: &[Sounded]) -> Self {
        let mut out = self.clone();
        for sprite in &mut out.sprites {
            for trigger in std::mem::take(&mut sprite.triggers) {
                let at: Vec<f64> = match trigger.fires {
                    Fires::HitSound(what) => sounds
                        .iter()
                        .filter(|hit| {
                            hit.time_ms >= trigger.start_ms
                                && hit.time_ms < trigger.end_ms
                                && what.matches(hit)
                        })
                        .map(|hit| hit.time_ms)
                        .collect(),

                    Fires::Passing => vec![trigger.start_ms],
                    Fires::Failing | Fires::Unreadable => Vec::new(),
                };
                for when in at {
                    sprite.commands.extend(trigger.body.iter().map(|c| Command {
                        easing: c.easing,
                        start_ms: c.start_ms + when,
                        end_ms: c.end_ms + when,
                        change: c.change.clone(),
                    }));
                }
            }
        }
        out
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Storyboard {
    pub sprites: Vec<Sprite>,
    pub video: Option<Video>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Drawn<'a> {
    pub layer: Layer,
    pub origin: Origin,
    pub path: &'a str,

    pub frame: u32,
    pub animated: bool,
    pub x: f32,
    pub y: f32,
    pub scale: (f32, f32),

    pub rotation: f32,
    pub colour: [u8; 3],
    pub alpha: f32,
    pub flip: (bool, bool),
    pub additive: bool,
}

impl Sprite {
    #[must_use]
    pub fn alive(&self) -> Option<(f64, f64)> {
        let mut from = f64::MAX;
        let mut to = f64::MIN;
        for command in &self.commands {
            from = from.min(command.start_ms);
            to = to.max(command.end_ms.max(command.start_ms));
        }
        (from <= to).then_some((from, to))
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn at(&self, time_ms: f64) -> Option<Drawn<'_>> {
        let (from, to) = self.alive()?;
        if time_ms < from || time_ms > to {
            return None;
        }

        let mut drawn = Drawn {
            layer: self.layer,
            origin: self.origin,
            path: &self.path,
            frame: 0,
            animated: self.animation.is_some(),
            x: self.x,
            y: self.y,
            scale: (1.0, 1.0),
            rotation: 0.0,
            colour: [255, 255, 255],
            alpha: 1.0,
            flip: (false, false),
            additive: false,
        };

        let mut seen = Seen::default();
        for command in &self.commands {
            let span = command.end_ms.max(command.start_ms);
            match &command.change {
                Change::Parameter(switch) => {
                    let forever = command.end_ms <= command.start_ms;
                    if time_ms >= command.start_ms && (forever || time_ms <= command.end_ms) {
                        match switch {
                            Switch::FlipHorizontally => drawn.flip.0 = true,
                            Switch::FlipVertically => drawn.flip.1 = true,
                            Switch::Additive => drawn.additive = true,
                        }
                    }
                }
                change => {
                    let started = time_ms >= command.start_ms;

                    let held = !started && !seen.has(change);
                    if !started && !held {
                        continue;
                    }
                    let along = if held {
                        0.0
                    } else {
                        progress(command, time_ms.min(span))
                    };
                    apply(&mut drawn, change, command.easing, along);
                    if started {
                        seen.mark(change);
                    }
                }
            }
        }

        if let Some(animation) = self.animation {
            drawn.frame = frame_of(animation, time_ms - from);
        }
        Some(drawn)
    }
}

#[derive(Default)]
struct Seen {
    fade: bool,
    move_: bool,
    move_x: bool,
    move_y: bool,
    scale: bool,
    vector: bool,
    rotate: bool,
    colour: bool,
}

impl Seen {
    fn slot(&mut self, change: &Change) -> Option<&mut bool> {
        Some(match change {
            Change::Fade(..) => &mut self.fade,
            Change::Move(..) => &mut self.move_,
            Change::MoveX(..) => &mut self.move_x,
            Change::MoveY(..) => &mut self.move_y,
            Change::Scale(..) => &mut self.scale,
            Change::ScaleVector(..) => &mut self.vector,
            Change::Rotate(..) => &mut self.rotate,
            Change::Colour(..) => &mut self.colour,
            Change::Parameter(_) => return None,
        })
    }

    fn has(&mut self, change: &Change) -> bool {
        self.slot(change).is_some_and(|seen| *seen)
    }

    fn mark(&mut self, change: &Change) {
        if let Some(seen) = self.slot(change) {
            *seen = true;
        }
    }
}

fn progress(command: &Command, time_ms: f64) -> f64 {
    let length = command.end_ms - command.start_ms;
    if length <= 0.0 {
        return 1.0;
    }
    ((time_ms - command.start_ms) / length).clamp(0.0, 1.0)
}

fn apply(drawn: &mut Drawn<'_>, change: &Change, easing: u8, along: f64) {
    let tween = |from: f32, to: f32| -> f32 {
        ease(
            easing,
            along,
            f64::from(from),
            f64::from(to) - f64::from(from),
            1.0,
        ) as f32
    };
    match *change {
        Change::Fade(a, b) => drawn.alpha = tween(a, b),
        Change::Move(ax, ay, bx, by) => {
            drawn.x = tween(ax, bx);
            drawn.y = tween(ay, by);
        }
        Change::MoveX(a, b) => drawn.x = tween(a, b),
        Change::MoveY(a, b) => drawn.y = tween(a, b),
        Change::Scale(a, b) => {
            let s = tween(a, b);
            drawn.scale = (s, s);
        }
        Change::ScaleVector(ax, ay, bx, by) => {
            drawn.scale = (tween(ax, bx), tween(ay, by));
        }
        Change::Rotate(a, b) => drawn.rotation = tween(a, b),
        Change::Colour(from, to) => {
            let channel = |i: usize| {
                ease(
                    easing,
                    along,
                    f64::from(from[i]),
                    f64::from(to[i]) - f64::from(from[i]),
                    1.0,
                )
                .clamp(0.0, 255.0) as u8
            };
            drawn.colour = [channel(0), channel(1), channel(2)];
        }
        Change::Parameter(_) => {}
    }
}

fn frame_of(animation: Animation, since_ms: f64) -> u32 {
    if animation.frame_ms <= 0.0 || animation.frames <= 1 {
        return 0;
    }
    let step = (since_ms / animation.frame_ms).floor().max(0.0);
    if animation.once {
        (step as u32).min(animation.frames - 1)
    } else {
        (step as u32) % animation.frames
    }
}

impl Storyboard {
    #[must_use]
    pub fn at(&self, time_ms: f64) -> Vec<Drawn<'_>> {
        let mut out: Vec<Drawn<'_>> = self
            .sprites
            .iter()
            .filter_map(|sprite| sprite.at(time_ms))
            .filter(|drawn| drawn.alpha > 0.0)
            .collect();

        out.sort_by_key(|drawn| drawn.layer);
        out
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty() && self.video.is_none()
    }

    pub fn absorb(&mut self, other: Storyboard) {
        self.sprites.extend(other.sprites);
        if self.video.is_none() {
            self.video = other.video;
        }
    }
}
