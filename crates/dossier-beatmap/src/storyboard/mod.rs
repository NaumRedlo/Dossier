//! The storyboard: everything a mapper drew that is not a note.
//!
//! A storyboard is a list of pictures and, under each, a list of things that
//! happen to it — fade, move, scale, turn, tint — each with a start, an end
//! and a curve. There is no state to walk: a sprite at a time is a question
//! answered from the commands alone, which is what makes seeking free and what
//! lets a render start at minute two without playing the first two.
//!
//! It arrives in two places and both are read. `[Events]` inside the `.osu`
//! belongs to the one difficulty; a sibling `.osb` belongs to the whole set,
//! and where the two collide the difficulty's own wins.
//!
//! What is here is the reading and the arithmetic. Nothing in this module
//! opens a file or draws anything.

mod easing;
mod parse;

use crate::timing::SampleSet;

pub use easing::ease;
pub use parse::{parse, parse_reporting, ParseError};

/// Which pile a sprite is drawn in.
///
/// The order is the drawing order, and `Overlay` is the only one that goes
/// over the play rather than under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    Background,
    /// Only while the player is failing, which a replay never is here.
    Fail,
    /// Only while the player is passing, which is what a replay is.
    Pass,
    Foreground,
    Overlay,
}

/// Which point of the picture sits on the coordinates it was given.
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
    /// How far from the picture's top-left corner the origin sits, as a
    /// fraction of its width and height.
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

/// What happens to a sprite between two moments.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Fade(f32, f32),
    Move(f32, f32, f32, f32),
    MoveX(f32, f32),
    MoveY(f32, f32),
    Scale(f32, f32),
    /// Scale with a different factor on each axis.
    ScaleVector(f32, f32, f32, f32),
    /// In radians, and clockwise, which is what the file states.
    Rotate(f32, f32),
    Colour([u8; 3], [u8; 3]),
    /// The three switches: mirrored across, mirrored down, and drawn by adding
    /// light rather than covering what is under it. A parameter holds for
    /// exactly as long as its command runs — unlike every other change, which
    /// leaves its end value behind.
    Parameter(Switch),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    FlipHorizontally,
    FlipVertically,
    Additive,
}

/// One line of a storyboard: a change, when it runs, and how it gets there.
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub easing: u8,
    pub start_ms: f64,
    pub end_ms: f64,
    pub change: Change,
}

/// One of the three sounds a note can carry besides its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Addition {
    Whistle,
    Finish,
    Clap,
}

/// What makes a trigger fire.
///
/// `HitSound` is written as a run of optional parts —
/// `HitSound[SampleSet][AdditionsSampleSet][Addition][CustomSampleSet]` — and
/// every part that *is* written has to match. `HitSoundClap` fires on any clap;
/// `HitSoundSoftClap` only on a clap whose additions are soft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HitSoundMatch {
    pub set: Option<SampleSet>,
    pub addition_set: Option<SampleSet>,
    pub addition: Option<Addition>,
    pub custom: Option<u32>,
}

/// A sound the play actually made, for a trigger to match against.
///
/// Supplied by whoever knows — the thing that decided which samples to play,
/// which is the judgement and not the storyboard. A trigger fired on a guess is
/// a sprite that appears when nothing happened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sounded {
    pub time_ms: f64,
    pub set: SampleSet,
    pub addition_set: SampleSet,
    pub addition: Option<Addition>,
    pub custom: u32,
}

/// What a trigger listens for.
///
/// `Passing` and `Failing` follow the health bar, which this engine does not
/// model — it decides when a player dies, not what they hit. A replay that was
/// submitted was passing at the end, and passing is the state a storyboard
/// author writes for, so `Passing` fires and `Failing` never does. Stated here
/// rather than guessed at in three places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fires {
    HitSound(HitSoundMatch),
    Passing,
    Failing,
    /// A name this parser could not read. It fires on nothing — a trigger
    /// nobody can understand should stay silent rather than go off on every
    /// note, which is the failure that would be noticed last.
    Unreadable,
}

impl HitSoundMatch {
    /// Whether this sound is one this trigger was waiting for.
    pub fn matches(self, hit: &Sounded) -> bool {
        self.set.is_none_or(|set| set == hit.set)
            && self.addition_set.is_none_or(|set| set == hit.addition_set)
            && self.addition == hit.addition.filter(|_| self.addition.is_some())
            && self.custom.is_none_or(|n| n == hit.custom)
    }
}

/// A body of commands waiting for something to happen.
#[derive(Debug, Clone, PartialEq)]
pub struct Trigger {
    pub fires: Fires,
    /// The window it listens in. A sound outside it fires nothing.
    pub start_ms: f64,
    pub end_ms: f64,
    /// Triggers sharing a group take each other over rather than stacking.
    /// Kept as written; see [`Storyboard::fired`] for what is done with it.
    pub group: i32,
    /// Times inside are stated from the moment it fires, like a loop's.
    pub body: Vec<Command>,
}

/// A picture, and everything that happens to it.
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    pub layer: Layer,
    pub origin: Origin,
    /// As written in the file, with the separators it was written with.
    pub path: String,
    pub x: f32,
    pub y: f32,
    /// Present when the line was an `Animation` rather than a `Sprite`.
    pub animation: Option<Animation>,
    pub commands: Vec<Command>,
    /// Bodies that have not happened yet — see [`Storyboard::fired`].
    pub triggers: Vec<Trigger>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Animation {
    pub frames: u32,
    pub frame_ms: f64,
    /// Whether it stops on the last frame instead of starting over.
    pub once: bool,
}

/// The map's background video, from an `[Events]` line of type `1`.
#[derive(Debug, Clone, PartialEq)]
pub struct Video {
    pub path: String,
    pub start_ms: f64,
    pub offset: (f32, f32),
}

impl Storyboard {
    /// The same storyboard with every trigger turned into ordinary commands.
    ///
    /// A trigger is a body waiting for something the storyboard cannot see. The
    /// sounds a play actually made are handed in here, from the side that
    /// decided to make them, and each one inside a trigger's window lays that
    /// trigger's body down from the moment it sounded — the same unrolling a
    /// loop gets, at times nobody could know in advance.
    ///
    /// Firing again while a body is still running lays down a second copy on
    /// top of the first. osu! restarts the body instead, and a group number
    /// says which triggers take each other over; both come out the same wherever
    /// the later copy sets the same properties as the earlier, which is the
    /// ordinary case, and this engine reads the last command that applies.
    /// Groups are kept in the model and not yet acted on.
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
                    // Not a sound but a state, and the state is "passing" from
                    // the first moment the trigger is listening.
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

/// Everything read out of `[Events]` and the `.osb`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Storyboard {
    pub sprites: Vec<Sprite>,
    pub video: Option<Video>,
}

/// A sprite as it is at one moment: everything a drawer needs and nothing it
/// has to work out.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawn<'a> {
    pub layer: Layer,
    pub origin: Origin,
    pub path: &'a str,
    /// Which picture of an animation, already wrapped or held. Meaningless
    /// unless `animated` — a still sprite is its own file, not frame nought of
    /// it.
    pub frame: u32,
    pub animated: bool,
    pub x: f32,
    pub y: f32,
    pub scale: (f32, f32),
    /// Radians, clockwise.
    pub rotation: f32,
    pub colour: [u8; 3],
    pub alpha: f32,
    pub flip: (bool, bool),
    pub additive: bool,
}

impl Sprite {
    /// When this sprite has anything to say: the first command's start to the
    /// last one's end.
    ///
    /// A sprite is not on screen for the length of the song — it exists for as
    /// long as something is happening to it, and a storyboard with four
    /// thousand sprites has perhaps thirty alive at once. This is what makes
    /// that true.
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

    /// What this sprite looks like at `time_ms`, or `None` when it is not out.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn at(&self, time_ms: f64) -> Option<Drawn<'_>> {
        let (from, to) = self.alive()?;
        if time_ms < from || time_ms > to {
            return None;
        }

        // Every property starts at what the sprite line said, or at what a
        // picture drawn plainly would be, and is then overwritten by whichever
        // commands have started.
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
                // A switch holds for exactly as long as its command runs —
                // except an instant one, which the game keeps for good. A
                // storyboard flips a sprite with `P,0,t,,H` and expects it to
                // stay flipped, and a switch that ended the moment it began
                // would be a picture that never turned over at all.
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
                    // Before the first command of a kind, the sprite is held
                    // at that command's *starting* value rather than at the
                    // default — a sprite faded in from nought at minute two is
                    // invisible at minute one, not fully lit.
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

/// Which kinds of command have already had their say, so the "hold at the
/// first one's start" rule applies to the first of each kind and no other.
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

/// How far into a command a moment is, in the command's own time.
fn progress(command: &Command, time_ms: f64) -> f64 {
    let length = command.end_ms - command.start_ms;
    if length <= 0.0 {
        return 1.0;
    }
    ((time_ms - command.start_ms) / length).clamp(0.0, 1.0)
}

/// `along` runs nought to one; the curve is applied here so that every kind of
/// command gets the same one.
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
    /// Every sprite that is out at `time_ms`, in the order they are drawn:
    /// by layer, and within a layer by the order the file listed them.
    #[must_use]
    pub fn at(&self, time_ms: f64) -> Vec<Drawn<'_>> {
        let mut out: Vec<Drawn<'_>> = self
            .sprites
            .iter()
            .filter_map(|sprite| sprite.at(time_ms))
            .filter(|drawn| drawn.alpha > 0.0)
            .collect();
        // A stable sort, so the file's own order survives inside a layer.
        out.sort_by_key(|drawn| drawn.layer);
        out
    }

    /// Whether there is anything here at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty() && self.video.is_none()
    }

    /// Add another storyboard's sprites after this one's.
    ///
    /// The `.osb` is read first and the difficulty's own `[Events]` second, so
    /// what a difficulty says is drawn over what the set says.
    pub fn absorb(&mut self, other: Storyboard) {
        self.sprites.extend(other.sprites);
        if self.video.is_none() {
            self.video = other.video;
        }
    }
}
