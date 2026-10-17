use std::collections::HashMap;

use dossier_sim::mania::judge::{Event, Judge, Part, ResultKind};
use dossier_sim::mania::stable::{TraceEvent, TraceKind};
use dossier_sim::mania::Playback;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};

use crate::{Align, Camera, Font, Label, Layout, Picture};

const REFERENCE_HEIGHT: f32 = 1080.0;
const SCROLL_MS: f64 = 520.0;
const LINE_AT: f32 = 0.86;
const LANE: f32 = 104.0;
const NOTE_H: f32 = 34.0;
const LEAD_MS: f64 = 1500.0;
const TAIL_MS: f64 = 2500.0;
const SHOWN_MS: f64 = 420.0;
const FLASH_MS: f64 = 140.0;
const GRADES: [ResultKind; 6] = [ResultKind::Perfect, ResultKind::Great, ResultKind::Good, ResultKind::Ok, ResultKind::Meh, ResultKind::Miss];
const WEIGHTS: [f64; 6] = [300.0, 300.0, 200.0, 100.0, 50.0, 0.0];
const THIN: f32 = 3.0;

#[derive(Debug, Default)]
struct Fate {
    gone_at: Option<f64>,
    grabs: Vec<(f64, f64)>,
    missed: bool,
}

pub struct Mania<'a> {
    playback: &'a Playback,
    judge: &'a Judge,
    fates: HashMap<usize, Fate>,
    hits: Vec<Vec<f64>>,
    stable: bool,
    bare: bool,
    font: Option<Font>,
    background: Option<Pixmap>,
    title: String,
    version: String,
    player: String,
    first_ms: f64,
    last_ms: f64,
}

fn rgb(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color::from_rgba8(r, g, b, (a.clamp(0.0, 1.0) * 255.0) as u8)
}

fn grade_colour(kind: ResultKind) -> Color {
    match kind {
        ResultKind::Perfect => rgb(232, 246, 255, 1.0),
        ResultKind::Great => rgb(255, 209, 92, 1.0),
        ResultKind::Good => rgb(107, 224, 123, 1.0),
        ResultKind::Ok => rgb(90, 169, 242, 1.0),
        ResultKind::Meh => rgb(165, 139, 216, 1.0),
        ResultKind::Miss => rgb(240, 90, 90, 1.0),
    }
}

fn grade_name(kind: ResultKind, stable: bool) -> &'static str {
    match (kind, stable) {
        (ResultKind::Perfect, true) => "MAX",
        (ResultKind::Great, true) => "300",
        (ResultKind::Good, true) => "200",
        (ResultKind::Ok, true) => "100",
        (ResultKind::Meh, true) => "50",
        (ResultKind::Perfect, false) => "PERFECT",
        (ResultKind::Great, false) => "GREAT",
        (ResultKind::Good, false) => "GOOD",
        (ResultKind::Ok, false) => "OK",
        (ResultKind::Meh, false) => "MEH",
        (ResultKind::Miss, _) => "MISS",
    }
}

fn column_colour(column: u8, columns: u8) -> (u8, u8, u8) {
    let mirrored = column.min(columns - 1 - column);
    if columns % 2 == 1 && column == columns / 2 {
        (255, 206, 84)
    } else if mirrored % 2 == 0 {
        (236, 238, 248)
    } else {
        (104, 190, 245)
    }
}

fn fill(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, colour: Color) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let mut paint = Paint::default();
    paint.set_color(colour);
    let rect = if w < 2.0 || h < 2.0 {
        paint.anti_alias = false;
        Rect::from_xywh(x.round(), y.round(), w.round().max(1.0), h.round().max(1.0))
    } else {
        Rect::from_xywh(x, y, w, h)
    };
    if let Some(rect) = rect {
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

fn rounded(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, colour: Color) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    if w < THIN || h < THIN {
        fill(pixmap, x, y, w, h, colour);
        return;
    }
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let mut path = PathBuilder::new();
    path.move_to(x + r, y);
    path.line_to(x + w - r, y);
    path.quad_to(x + w, y, x + w, y + r);
    path.line_to(x + w, y + h - r);
    path.quad_to(x + w, y + h, x + w - r, y + h);
    path.line_to(x + r, y + h);
    path.quad_to(x, y + h, x, y + h - r);
    path.line_to(x, y + r);
    path.quad_to(x, y, x + r, y);
    path.close();
    if let Some(path) = path.finish() {
        let mut paint = Paint::default();
        paint.set_color(colour);
        paint.anti_alias = true;
        pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }
}

fn fates(playback: &Playback, judge: &Judge, trace: Option<&[TraceEvent]>) -> HashMap<usize, Fate> {
    let mut out: HashMap<usize, Fate> = HashMap::new();
    for event in judge.events() {
        let fate = out.entry(event.object_index).or_default();
        match event.part {
            Part::Tap | Part::Hold | Part::Tail if event.result == ResultKind::Miss => fate.missed = true,
            Part::Tap | Part::Hold | Part::Tail => fate.gone_at = Some(event.time_ms),
            Part::Head if event.result == ResultKind::Miss => fate.missed = true,
            Part::Head | Part::HoldBreak => {}
        }
    }
    match trace {
        Some(trace) => {
            let mut open: HashMap<usize, f64> = HashMap::new();
            for step in trace {
                match step.kind {
                    TraceKind::HeadPressed | TraceKind::Regrabbed => {
                        open.insert(step.object_index, step.time_ms);
                    }
                    _ => {
                        if let Some(from) = open.remove(&step.object_index) {
                            out.entry(step.object_index).or_default().grabs.push((from, step.time_ms));
                        }
                    }
                }
            }
        }
        None => {
            for event in judge.events().iter().filter(|event| event.part == Part::Head && event.result != ResultKind::Miss) {
                let released = playback
                    .input
                    .events()
                    .iter()
                    .find(|key| key.column == event.column && !key.pressed && key.time_ms as f64 >= event.time_ms)
                    .map_or(f64::INFINITY, |key| key.time_ms as f64);
                out.entry(event.object_index).or_default().grabs.push((event.time_ms, released));
            }
        }
    }
    out
}

impl<'a> Mania<'a> {
    pub fn new(playback: &'a Playback, judge: &'a Judge, trace: Option<&[TraceEvent]>) -> Self {
        let notes = (0..playback.timeline.columns()).filter_map(|column| playback.timeline.notes(column)).flatten();
        let (first_ms, last_ms) = notes.fold((f64::INFINITY, f64::NEG_INFINITY), |(first, last), note| (first.min(note.start_ms), last.max(note.end_ms)));
        let mut hits = vec![Vec::new(); usize::from(playback.timeline.columns())];
        for event in judge.events().iter().filter(|event| event.result != ResultKind::Miss && event.part != Part::HoldBreak) {
            if let Some(column) = hits.get_mut(usize::from(event.column)) {
                column.push(event.time_ms);
            }
        }
        Self {
            playback,
            judge,
            fates: fates(playback, judge, trace),
            hits,
            stable: trace.is_some(),
            bare: false,
            font: None,
            background: None,
            title: String::new(),
            version: String::new(),
            player: String::new(),
            first_ms: if first_ms.is_finite() { first_ms } else { 0.0 },
            last_ms: if last_ms.is_finite() { last_ms } else { 0.0 },
        }
    }

    pub fn titled(mut self, title: String, version: String, player: String) -> Self {
        (self.title, self.version, self.player) = (title, version, player);
        self
    }

    pub fn bare(mut self) -> Self {
        self.bare = true;
        self
    }

    pub fn with_font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    pub fn over(mut self, background: Option<Pixmap>) -> Self {
        self.background = background;
        self
    }

    pub fn span_ms(&self) -> (f64, f64) {
        (self.first_ms - LEAD_MS, self.last_ms + TAIL_MS)
    }

    fn label(&self, pixmap: &mut Pixmap, text: &str, x: f32, y: f32, size: f32, colour: Color, align: Align) {
        if let Some(font) = &self.font {
            font.draw(pixmap, Label { text, x, y, size, colour, align });
        }
    }

    fn fitted(&self, text: &str, size: f32, room: f32) -> String {
        let Some(font) = &self.font else {
            return String::new();
        };
        if font.width(text, size) <= room {
            return text.to_owned();
        }
        let mut kept: Vec<char> = text.chars().collect();
        while kept.pop().is_some() {
            let candidate = format!("{}…", kept.iter().collect::<String>().trim_end());
            if font.width(&candidate, size) <= room {
                return candidate;
            }
        }
        String::new()
    }

    fn draw(&self, pixmap: &mut Pixmap, time: f64) {
        let (width, height) = (pixmap.width() as f32, pixmap.height() as f32);
        let s = height / REFERENCE_HEIGHT;
        match &self.background {
            Some(background) if background.width() == pixmap.width() && background.height() == pixmap.height() => {
                pixmap.draw_pixmap(0, 0, background.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
            }
            _ => pixmap.fill(rgb(14, 12, 18, 1.0)),
        }
        let columns = self.playback.timeline.columns();
        let lane = LANE * s;
        let field = lane * f32::from(columns);
        let left = (width - field) / 2.0;
        let line_y = height * LINE_AT;
        fill(pixmap, left - 4.0 * s, 0.0, field + 8.0 * s, height, rgb(6, 5, 9, 0.92));
        for column in 1..columns {
            fill(pixmap, left + lane * f32::from(column) - s, 0.0, 2.0 * s, height, rgb(255, 255, 255, 0.05));
        }
        let Ok(snapshot) = self.playback.at(time, SCROLL_MS) else {
            return;
        };
        for column in 0..columns {
            if snapshot.keys & (1 << column) != 0 {
                let (r, g, b) = column_colour(column, columns);
                let reach = 300.0 * s;
                for step in 0..24 {
                    let k = step as f32 / 24.0;
                    fill(pixmap, left + lane * f32::from(column), line_y - reach * (1.0 - k), lane, reach / 24.0 + 1.0, rgb(r, g, b, 0.16 * k));
                }
            }
        }
        fill(pixmap, left, line_y - 2.0 * s, field, 4.0 * s, rgb(255, 255, 255, 0.85));

        let travel = f64::from(line_y);
        let note_h = NOTE_H * s;
        for visible in &snapshot.notes {
            let note = visible.note;
            let fate = self.fates.get(&note.object_index);
            if fate.and_then(|fate| fate.gone_at).is_some_and(|gone| gone <= time) {
                continue;
            }
            let held = fate.is_some_and(|fate| fate.grabs.iter().any(|&(from, to)| from <= time && time < to));
            let alpha = if fate.is_some_and(|fate| fate.missed) { 0.35 } else { 1.0 };
            let (r, g, b) = column_colour(note.column, columns);
            let x = left + lane * f32::from(note.column) + 6.0 * s;
            let w = lane - 12.0 * s;
            let head = if held { visible.head.max(0.0) } else { visible.head };
            let head_y = (travel - head * travel) as f32;
            if note.hold {
                let tail_y = (travel - visible.tail * travel) as f32;
                let top = tail_y.max(-note_h);
                let bottom = head_y.min(height + note_h);
                if bottom > top {
                    let body = if held { 0.75 } else { 0.5 } * alpha;
                    rounded(pixmap, x + 10.0 * s, top, w - 20.0 * s, bottom - top, 6.0 * s, rgb(r, g, b, body));
                    rounded(pixmap, x + 4.0 * s, tail_y - note_h / 2.0, w - 8.0 * s, note_h / 2.0, 6.0 * s, rgb(r, g, b, 0.9 * alpha));
                }
            }
            if head_y > -note_h && head_y < height + note_h {
                rounded(pixmap, x, head_y - note_h, w, note_h, 8.0 * s, rgb(r, g, b, alpha));
            }
        }

        for column in 0..columns {
            let (r, g, b) = column_colour(column, columns);
            let pressed = snapshot.keys & (1 << column) != 0;
            let x = left + lane * f32::from(column) + 10.0 * s;
            let flash = self.hits.get(usize::from(column)).and_then(|times| {
                let end = times.partition_point(|at| *at <= time);
                end.checked_sub(1).map(|at| time - times[at])
            });
            if let Some(age) = flash.filter(|age| *age < FLASH_MS) {
                let k = (1.0 - age / FLASH_MS) as f32;
                rounded(pixmap, x - 6.0 * s, line_y - 18.0 * s, lane - 8.0 * s, 36.0 * s, 14.0 * s, rgb(r, g, b, 0.55 * k));
            }
            rounded(pixmap, x, line_y + 22.0 * s, lane - 20.0 * s, 60.0 * s, 12.0 * s, rgb(r, g, b, if pressed { 0.95 } else { 0.18 }));
        }

        self.draw_feedback(pixmap, time, left + field / 2.0, s);
        if !self.bare {
            self.draw_panel(pixmap, time, left + field + 60.0 * s, s);
            self.draw_header(pixmap, time, left - 96.0 * s, s);
        }
    }

    fn draw_feedback(&self, pixmap: &mut Pixmap, time: f64, centre: f32, s: f32) {
        let events = self.judge.events();
        let end = events.partition_point(|event: &Event| event.time_ms <= time);
        if let Some(last) = end.checked_sub(1).map(|at| events[at]) {
            let age = time - last.time_ms;
            if age < SHOWN_MS && last.part != Part::HoldBreak && last.part != Part::Head {
                let fade = (1.0 - age / SHOWN_MS) as f32;
                let pop = 1.0 + 0.25 * (1.0 - (age / 90.0).min(1.0)) as f32;
                let mut colour = grade_colour(last.result);
                colour.set_alpha(fade);
                self.label(pixmap, grade_name(last.result, self.stable), centre, 560.0 * s, 64.0 * s * pop, colour, Align::Centre);
            }
        }
        if let Some(combo) = self.judge.combo_at(time).filter(|combo| combo.current > 0) {
            self.label(pixmap, &combo.current.to_string(), centre, 440.0 * s, 92.0 * s, rgb(255, 255, 255, 0.92), Align::Centre);
        }
    }

    fn draw_panel(&self, pixmap: &mut Pixmap, time: f64, x: f32, s: f32) {
        let counts = self.judge.counts_at(time);
        let total = counts.total();
        let accuracy = match self.judge.score_at(time) {
            Some(score) => score.accuracy * 100.0,
            None if total == 0 => 100.0,
            None => GRADES.iter().zip(WEIGHTS).map(|(&kind, weight)| f64::from(counts.get(kind)) * weight).sum::<f64>() / (300.0 * f64::from(total)) * 100.0,
        };
        self.label(pixmap, &format!("{accuracy:.2}%"), x, 330.0 * s, 58.0 * s, rgb(255, 255, 255, 1.0), Align::Left);
        if let Some(combo) = self.judge.combo_at(time) {
            self.label(pixmap, &format!("{}x", combo.maximum), x, 380.0 * s, 30.0 * s, rgb(255, 255, 255, 0.6), Align::Left);
        }
        for (row, kind) in GRADES.iter().enumerate() {
            let y = (450.0 + row as f32 * 52.0) * s;
            rounded(pixmap, x, y - 30.0 * s, 10.0 * s, 36.0 * s, 4.0 * s, grade_colour(*kind));
            self.label(pixmap, grade_name(*kind, self.stable), x + 26.0 * s, y, 32.0 * s, grade_colour(*kind), Align::Left);
            self.label(pixmap, &counts.get(*kind).to_string(), x + 300.0 * s, y, 32.0 * s, rgb(255, 255, 255, 0.9), Align::Right);
        }
    }

    fn draw_header(&self, pixmap: &mut Pixmap, time: f64, room: f32, s: f32) {
        let x = 48.0 * s;
        self.label(pixmap, &self.fitted(&self.title, 34.0 * s, room), x, 70.0 * s, 34.0 * s, rgb(255, 255, 255, 0.95), Align::Left);
        self.label(pixmap, &self.fitted(&self.version, 28.0 * s, room), x, 110.0 * s, 28.0 * s, rgb(255, 255, 255, 0.75), Align::Left);
        self.label(pixmap, &self.fitted(&self.player, 26.0 * s, room), x, 150.0 * s, 26.0 * s, rgb(255, 255, 255, 0.55), Align::Left);
        let (width, height) = (pixmap.width() as f32, pixmap.height() as f32);
        let span = (self.last_ms - self.first_ms).max(1.0);
        let done = ((time - self.first_ms) / span).clamp(0.0, 1.0) as f32;
        fill(pixmap, 0.0, height - 6.0 * s, width, 6.0 * s, rgb(255, 255, 255, 0.08));
        fill(pixmap, 0.0, height - 6.0 * s, width * done, 6.0 * s, rgb(255, 255, 255, 0.7));
    }
}

impl Picture for Mania<'_> {
    fn draw_into(&self, pixmap: &mut Pixmap, time_ms: f64, _: &Layout, _: Option<Camera>) {
        self.draw(pixmap, time_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dossier_beatmap::Beatmap;
    use dossier_replay::{GameMode, HitCounts, Keys, Mods, Replay, ReplayFrame};

    fn playback(objects: &str, edges: &[(i64, u32)]) -> Playback {
        let map = Beatmap::parse(&format!("osu file format v14\n[General]\nMode:3\n[Difficulty]\nCircleSize:4\nOverallDifficulty:5\n[HitObjects]\n{objects}")).unwrap();
        let frames: Vec<ReplayFrame> = edges.iter().map(|&(time_ms, keys)| ReplayFrame { time_ms, x: keys as f32, y: 0.0, keys: Keys(0) }).collect();
        let replay = Replay {
            mode: GameMode::Mania,
            game_version: 20_250_603,
            beatmap_hash: String::new(),
            player: "test".into(),
            replay_hash: String::new(),
            hits: HitCounts::default(),
            score: 0,
            max_combo: 0,
            perfect_combo: false,
            mods: Mods::new(0),
            life_bar: String::new(),
            timestamp_ticks: 0,
            online_score_id: 0,
            target_practice_accuracy: None,
            frames,
            rng_seed: None,
            score_info: None,
        };
        Playback::new(&map, &replay).unwrap()
    }

    #[test]
    fn a_hit_note_leaves_the_field_and_a_held_body_stops_at_the_line() {
        let playback = playback("64,192,1000,1,0\n192,192,1000,128,0,3000:0:0:0:0:", &[(1000, 3), (2000, 2), (3000, 0)]);
        let (judge, trace) = playback.trace_stable_with(playback.stable_rules(5.0).unwrap()).unwrap();
        let mania = Mania::new(&playback, &judge, Some(&trace));
        assert_eq!(mania.span_ms(), (1000.0 - LEAD_MS, 3000.0 + TAIL_MS));
        assert_eq!(mania.fates[&0].gone_at, Some(1000.0), "the tap is gone once it is hit");
        assert_eq!(mania.fates[&1].grabs.first().map(|grab| grab.0), Some(1000.0), "the hold is held from its head");
        let (width, height) = (640u32, 360u32);
        let mut pixmap = Pixmap::new(width, height).unwrap();
        mania.draw_into(&mut pixmap, 2000.0, &Layout::new(width, height), None);
        let line_y = (height as f32 * LINE_AT) as u32 - 6;
        let lane = LANE * height as f32 / REFERENCE_HEIGHT;
        let body_x = ((width as f32 - lane * 4.0) / 2.0 + lane * 1.5) as u32;
        let at = pixmap.pixel(body_x, line_y).unwrap();
        assert!(at.red() > 60, "the held body reaches the line: {at:?}");
        let tap_x = ((width as f32 - lane * 4.0) / 2.0 + lane * 0.5) as u32;
        let empty = pixmap.pixel(tap_x, line_y - 20).unwrap();
        assert!(empty.red() < 40, "the hit tap is not drawn any more: {empty:?}");
    }
}
