use std::collections::HashMap;

use super::{
    Addition, Animation, Change, Command, Fires, HitSoundMatch, Layer, Origin, SampleSet, Sprite,
    Storyboard, Switch, Trigger, Video,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub text: String,
}

#[must_use]
pub fn parse(text: &str) -> Storyboard {
    parse_reporting(text).0
}

#[must_use]
pub fn parse_reporting(text: &str) -> (Storyboard, Vec<ParseError>) {
    let variables = variables(text);
    let mut out = Storyboard::default();
    let mut errors = Vec::new();
    let mut section = String::new();

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

                "T" => open = Some(OpenLoop::trigger(&fields)),
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

struct OpenLoop {
    start_ms: f64,
    count: u32,
    body: Vec<Command>,

    trigger: Option<(Fires, f64, f64, i32)>,
}

impl OpenLoop {
    fn begin(fields: &[&str]) -> Self {
        Self {
            start_ms: number(fields.get(1)).unwrap_or(0.0),

            count: number(fields.get(2)).unwrap_or(1.0).max(1.0) as u32,
            body: Vec::new(),
            trigger: None,
        }
    }

    fn trigger(fields: &[&str]) -> Self {
        Self {
            start_ms: 0.0,
            count: 0,
            body: Vec::new(),
            trigger: Some((
                fires(fields.get(1).copied().unwrap_or("")),
                number(fields.get(2)).unwrap_or(0.0),
                number(fields.get(3)).unwrap_or(f64::INFINITY),
                number(fields.get(4)).unwrap_or(0.0) as i32,
            )),
        }
    }
}

fn fires(name: &str) -> Fires {
    match name {
        "Passing" => return Fires::Passing,
        "Failing" => return Fires::Failing,
        _ => {}
    }
    let Some(mut rest) = name.strip_prefix("HitSound") else {
        return Fires::Unreadable;
    };
    let mut out = HitSoundMatch::default();
    let set = |rest: &mut &str| -> Option<SampleSet> {
        for (word, which) in [
            ("Normal", SampleSet::Normal),
            ("Soft", SampleSet::Soft),
            ("Drum", SampleSet::Drum),
        ] {
            if let Some(after) = rest.strip_prefix(word) {
                *rest = after;
                return Some(which);
            }
        }
        None
    };
    out.set = set(&mut rest);
    out.addition_set = set(&mut rest);
    for (word, which) in [
        ("Whistle", Addition::Whistle),
        ("Finish", Addition::Finish),
        ("Clap", Addition::Clap),
    ] {
        if let Some(after) = rest.strip_prefix(word) {
            rest = after;
            out.addition = Some(which);
            break;
        }
    }
    if !rest.is_empty() {
        out.custom = rest.parse().ok();
    }
    Fires::HitSound(out)
}

fn close_loop(open: &mut Option<OpenLoop>, out: &mut Storyboard) {
    let Some(loop_) = open.take() else { return };
    if loop_.body.is_empty() {
        return;
    }
    if let Some((fires, start_ms, end_ms, group)) = loop_.trigger {
        if let Some(sprite) = out.sprites.last_mut() {
            sprite.triggers.push(Trigger {
                fires,
                start_ms,
                end_ms,
                group,
                body: loop_.body,
            });
        }
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

        _ => Ok(None),
    }
}

fn sprite(fields: &[&str], animated: bool) -> Result<Sprite, ()> {
    let path = quoted(fields.get(3).copied().ok_or(())?);
    if path.is_empty() {
        return Err(());
    }
    Ok(Sprite {
        triggers: Vec::new(),
        layer: layer(fields.get(1).copied().unwrap_or("")),
        origin: origin(fields.get(2).copied().unwrap_or("")),
        path,

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

    let end_ms = match fields.get(3).copied().unwrap_or("") {
        "" => start_ms,
        text => text.parse().map_err(|_| ())?,
    };
    let p = &fields[4.min(fields.len())..];
    let at = |i: usize| number(p.get(i));

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

fn section_header(line: &str) -> Option<String> {
    line.strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .map(|name| name.to_ascii_lowercase())
}

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
