//! Reading `[Events]` and `.osb` into sprites and commands.
//!
//! ## Two files, one storyboard
//!
//! `[Events]` in the `.osu` belongs to the one difficulty; a sibling `.osb`
//! belongs to the whole set. Both are read and the difficulty's own is added
//! last, which is the order the game draws them in.
//!
//! ## Indentation is syntax
//!
//! A command line is indented, and how deep says what it belongs to: one level
//! is a command on the sprite above, two levels is a command inside the loop
//! above. The indent character is a space or an underscore and the two are
//! interchangeable, which is why the ordinary line trim the rest of this crate
//! uses cannot be borrowed here — it would flatten a loop's body into the
//! sprite and run every command once, at the wrong time.
//!
//! ## What is not read
//!
//! **Triggers** (`T,HitSoundClap,…`) are parsed far enough to be skipped
//! whole, body and all. They fire on things the storyboard cannot know by
//! itself — a hit sound, passing, failing — and a trigger expanded on a guess
//! is a sprite that appears when nothing happened. Skipping one loses an
//! effect; guessing invents one.
//!
//! **Sample lines** (`5,…`) name a sound rather than a picture, and belong
//! with the audio rather than here.
//!
//! **Chained parameters.** A command carries one starting group and at most
//! one ending group. Sequences of three or more groups on one line are not a
//! thing the legacy decoder reads either.

use std::collections::HashMap;

use super::{Animation, Change, Command, Layer, Origin, Sprite, Storyboard, Switch, Video};

/// Nothing here refuses a file — a storyboard is decoration, and a map whose
/// decoration has one bad line should still be rendered. The error type exists
/// so the reason can be told to somebody who asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub text: String,
}

/// Read a storyboard out of `.osb` text, or out of a whole `.osu`.
///
/// Either way only `[Events]` and `[Variables]` are looked at, so the same
/// function reads both and a caller does not have to cut the section out
/// first.
#[must_use]
pub fn parse(text: &str) -> Storyboard {
    parse_reporting(text).0
}

/// The same, and the lines that could not be read.
#[must_use]
pub fn parse_reporting(text: &str) -> (Storyboard, Vec<ParseError>) {
    let variables = variables(text);
    let mut out = Storyboard::default();
    let mut errors = Vec::new();
    let mut section = String::new();
    // Where the commands being read now are going, and — while a loop is open
    // — what has been collected for it.
    let mut open: Option<OpenLoop> = None;

    for (index, raw) in text.lines().enumerate() {
        let line = without_comment(raw);
        if line.trim().is_empty() {
            continue;
        }
        if let Some(name) = section_header(line.trim()) {
            section = name;
            continue;
        }
        if section != "events" {
            continue;
        }

        let depth = indent(line);
        let body = expand(line.trim_start_matches([' ', '_', '\t']), &variables);
        let fields: Vec<&str> = body.split(',').map(str::trim).collect();

        if depth == 0 {
            close_loop(&mut open, &mut out);
            match object(&fields) {
                Ok(Some(Read::Sprite(sprite))) => out.sprites.push(sprite),
                Ok(Some(Read::Video(video))) => {
                    // The first one wins, the way the background does: a map
                    // with two video lines is showing the first.
                    if out.video.is_none() {
                        out.video = Some(video);
                    }
                }
                Ok(None) => {}
                Err(()) => errors.push(ParseError {
                    line: index + 1,
                    text: raw.trim().to_owned(),
                }),
            }
            continue;
        }

        // A command, on the sprite above. Without one there is nothing for it
        // to happen to.
        if out.sprites.is_empty() {
            errors.push(ParseError {
                line: index + 1,
                text: raw.trim().to_owned(),
            });
            continue;
        }

        if depth == 1 {
            close_loop(&mut open, &mut out);
            match fields.first().copied().unwrap_or("") {
                "L" => open = Some(OpenLoop::begin(&fields)),
                // A trigger and everything indented under it. `skipping` eats
                // the body without a sprite ever hearing about it.
                "T" => open = Some(OpenLoop::skipping()),
                _ => match command(&fields) {
                    Ok(cmd) => push(&mut out, cmd),
                    Err(()) => errors.push(ParseError {
                        line: index + 1,
                        text: raw.trim().to_owned(),
                    }),
                },
            }
            continue;
        }

        // Deeper: the body of whatever is open.
        match (&mut open, command(&fields)) {
            (Some(loop_), Ok(cmd)) => loop_.body.push(cmd),
            (None, Ok(cmd)) => push(&mut out, cmd),
            (_, Err(())) => errors.push(ParseError {
                line: index + 1,
                text: raw.trim().to_owned(),
            }),
        }
    }
    close_loop(&mut open, &mut out);
    (out, errors)
}

fn push(out: &mut Storyboard, command: Command) {
    if let Some(sprite) = out.sprites.last_mut() {
        sprite.commands.push(command);
    }
}

/// A loop being collected, or a trigger being thrown away.
struct OpenLoop {
    start_ms: f64,
    count: u32,
    body: Vec<Command>,
    skip: bool,
}

impl OpenLoop {
    fn begin(fields: &[&str]) -> Self {
        Self {
            start_ms: number(fields.get(1)).unwrap_or(0.0),
            // A loop with no count runs once: it is still a sprite doing
            // something, and dropping it loses more than repeating it wrongly.
            count: number(fields.get(2)).unwrap_or(1.0).max(1.0) as u32,
            body: Vec::new(),
            skip: false,
        }
    }

    fn skipping() -> Self {
        Self {
            start_ms: 0.0,
            count: 0,
            body: Vec::new(),
            skip: true,
        }
    }
}

/// Unroll a loop onto the sprite it belongs to.
///
/// The body's times are stated from the loop's own start, and one turn of it
/// lasts as long as its longest command — so the whole thing is laid out
/// `count` times, each shifted by a turn.
fn close_loop(open: &mut Option<OpenLoop>, out: &mut Storyboard) {
    let Some(loop_) = open.take() else { return };
    if loop_.skip || loop_.body.is_empty() {
        return;
    }
    let turn = loop_
        .body
        .iter()
        .map(|c| c.end_ms)
        .fold(0.0_f64, f64::max)
        .max(0.0);
    let Some(sprite) = out.sprites.last_mut() else {
        return;
    };
    for turn_no in 0..loop_.count {
        let shift = loop_.start_ms + f64::from(turn_no) * turn;
        sprite.commands.extend(loop_.body.iter().map(|c| Command {
            easing: c.easing,
            start_ms: c.start_ms + shift,
            end_ms: c.end_ms + shift,
            change: c.change.clone(),
        }));
    }
}

enum Read {
    Sprite(Sprite),
    Video(Video),
}

fn object(fields: &[&str]) -> Result<Option<Read>, ()> {
    let kind = fields.first().copied().unwrap_or("");
    match kind {
        "Sprite" | "4" => Ok(Some(Read::Sprite(sprite(fields, false)?))),
        "Animation" | "6" => Ok(Some(Read::Sprite(sprite(fields, true)?))),
        "Video" | "1" => {
            let path = quoted(fields.get(2).copied().ok_or(())?);
            if path.is_empty() {
                return Err(());
            }
            Ok(Some(Read::Video(Video {
                path,
                start_ms: number(fields.get(1)).ok_or(())?,
                offset: (
                    number(fields.get(3)).unwrap_or(0.0) as f32,
                    number(fields.get(4)).unwrap_or(0.0) as f32,
                ),
            })))
        }
        // Backgrounds, breaks, samples and anything else in the section.
        _ => Ok(None),
    }
}

fn sprite(fields: &[&str], animated: bool) -> Result<Sprite, ()> {
    let path = quoted(fields.get(3).copied().ok_or(())?);
    if path.is_empty() {
        return Err(());
    }
    Ok(Sprite {
        layer: layer(fields.get(1).copied().unwrap_or("")),
        origin: origin(fields.get(2).copied().unwrap_or("")),
        path,
        // Missing coordinates are the middle of the screen, which is where a
        // sprite with none lands in the game.
        x: number(fields.get(4)).unwrap_or(320.0) as f32,
        y: number(fields.get(5)).unwrap_or(240.0) as f32,
        animation: animated.then(|| Animation {
            frames: number(fields.get(6)).unwrap_or(1.0).max(1.0) as u32,
            frame_ms: number(fields.get(7)).unwrap_or(0.0),
            once: matches!(
                fields.get(8).copied().unwrap_or("").trim(),
                "1" | "LoopOnce"
            ),
        }),
        commands: Vec::new(),
    })
}

fn command(fields: &[&str]) -> Result<Command, ()> {
    let kind = fields.first().copied().ok_or(())?;
    let easing = number(fields.get(1)).unwrap_or(0.0) as u8;
    let start_ms = number(fields.get(2)).ok_or(())?;
    // An empty end time is an instant command, which is how a sprite is put
    // somewhere once rather than moved.
    let end_ms = match fields.get(3).copied().unwrap_or("") {
        "" => start_ms,
        text => text.parse().map_err(|_| ())?,
    };
    let p = &fields[4.min(fields.len())..];
    let at = |i: usize| number(p.get(i));
    // The second group repeats the first when it was left out, which is how a
    // sprite is held at a value for a stretch.
    let pair = |i: usize, j: usize| -> Result<(f32, f32), ()> {
        let first = at(i).ok_or(())? as f32;
        Ok((first, at(j).map_or(first, |v| v as f32)))
    };

    let change = match kind {
        "F" => {
            let (a, b) = pair(0, 1)?;
            Change::Fade(a, b)
        }
        "S" => {
            let (a, b) = pair(0, 1)?;
            Change::Scale(a, b)
        }
        "R" => {
            let (a, b) = pair(0, 1)?;
            Change::Rotate(a, b)
        }
        "MX" => {
            let (a, b) = pair(0, 1)?;
            Change::MoveX(a, b)
        }
        "MY" => {
            let (a, b) = pair(0, 1)?;
            Change::MoveY(a, b)
        }
        "M" | "V" => {
            let sx = at(0).ok_or(())? as f32;
            let sy = at(1).ok_or(())? as f32;
            let ex = at(2).map_or(sx, |v| v as f32);
            let ey = at(3).map_or(sy, |v| v as f32);
            if kind == "M" {
                Change::Move(sx, sy, ex, ey)
            } else {
                Change::ScaleVector(sx, sy, ex, ey)
            }
        }
        "C" => {
            let channel = |i: usize| at(i).unwrap_or(0.0).clamp(0.0, 255.0) as u8;
            let from = [channel(0), channel(1), channel(2)];
            let to = if p.len() > 3 {
                [channel(3), channel(4), channel(5)]
            } else {
                from
            };
            if p.len() < 3 {
                return Err(());
            }
            Change::Colour(from, to)
        }
        "P" => Change::Parameter(match p.first().copied().unwrap_or("") {
            "H" => Switch::FlipHorizontally,
            "V" => Switch::FlipVertically,
            "A" => Switch::Additive,
            _ => return Err(()),
        }),
        _ => return Err(()),
    };
    Ok(Command {
        easing,
        start_ms,
        end_ms,
        change,
    })
}

// ── the small readings ───────────────────────────────────────────────────

fn section_header(line: &str) -> Option<String> {
    line.strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .map(|name| name.to_ascii_lowercase())
}

/// `$name=value` lines from `[Variables]`, which `.osb` files use to write a
/// path or a colour once and spend it a thousand times.
fn variables(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut inside = false;
    for raw in text.lines() {
        let line = without_comment(raw).trim();
        if let Some(name) = section_header(line) {
            inside = name == "variables";
            continue;
        }
        if !inside || !line.starts_with('$') {
            continue;
        }
        if let Some((name, value)) = line.split_once('=') {
            out.insert(name.to_owned(), value.to_owned());
        }
    }
    out
}

/// Spend the variables in a line.
///
/// Longest name first, so `$a` cannot eat the front of `$ab` — and a borrowed
/// line when there is nothing to spend, which is every line of most files.
fn expand<'a>(line: &'a str, variables: &HashMap<String, String>) -> std::borrow::Cow<'a, str> {
    if variables.is_empty() || !line.contains('$') {
        return std::borrow::Cow::Borrowed(line);
    }
    let mut names: Vec<&String> = variables.keys().collect();
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    let mut out = line.to_owned();
    for name in names {
        if out.contains(name.as_str()) {
            out = out.replace(name.as_str(), &variables[name]);
        }
    }
    std::borrow::Cow::Owned(out)
}

fn without_comment(raw: &str) -> &str {
    match raw.trim_start().find("//") {
        // Only a line that *begins* with `//` is a comment. A path can hold
        // two slashes, and cutting at the first pair anywhere would take the
        // file name off `"sb//flash.png"`.
        Some(0) => "",
        _ => raw,
    }
}

fn indent(line: &str) -> usize {
    line.chars()
        .take_while(|c| *c == ' ' || *c == '_' || *c == '\t')
        .count()
}

fn quoted(field: &str) -> String {
    field.trim().trim_matches('"').trim().to_owned()
}

fn number(field: Option<&&str>) -> Option<f64> {
    let text = field?.trim();
    if text.is_empty() {
        return None;
    }
    text.parse().ok()
}

fn layer(field: &str) -> Layer {
    match field.trim() {
        "1" | "Fail" => Layer::Fail,
        "2" | "Pass" => Layer::Pass,
        "3" | "Foreground" => Layer::Foreground,
        "4" | "Overlay" => Layer::Overlay,
        _ => Layer::Background,
    }
}

fn origin(field: &str) -> Origin {
    match field.trim() {
        "1" | "TopCentre" => Origin::TopCentre,
        "2" | "TopRight" => Origin::TopRight,
        "3" | "CentreLeft" => Origin::CentreLeft,
        "4" | "Centre" => Origin::Centre,
        "5" | "CentreRight" => Origin::CentreRight,
        "6" | "BottomLeft" => Origin::BottomLeft,
        "7" | "BottomCentre" => Origin::BottomCentre,
        "8" | "BottomRight" => Origin::BottomRight,
        _ => Origin::TopLeft,
    }
}
