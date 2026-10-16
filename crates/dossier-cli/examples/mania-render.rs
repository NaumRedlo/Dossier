use dossier_beatmap::Beatmap;
use dossier_render::{Align, Font, Label};
use dossier_replay::Replay;
use dossier_sim::mania::{
    judge::{Event, Judge, Part, ResultKind},
    stable::{TraceEvent, TraceKind},
    Playback,
};
use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const FPS: f64 = 60.0;
const SCROLL_MS: f64 = 520.0;
const LINE_Y: f32 = 930.0;
const LANE: f32 = 104.0;
const NOTE_H: f32 = 34.0;
const LEAD_MS: f64 = 1500.0;
const TAIL_MS: f64 = 2500.0;
const GRADES: [ResultKind; 6] = [
    ResultKind::Perfect,
    ResultKind::Great,
    ResultKind::Good,
    ResultKind::Ok,
    ResultKind::Meh,
    ResultKind::Miss,
];

struct Fate {
    gone_at: Option<f64>,
    grabs: Vec<(f64, f64)>,
    missed: bool,
}

struct Show<'a> {
    playback: &'a Playback,
    judge: &'a Judge,
    fates: HashMap<usize, Fate>,
    stable: bool,
    font: Font,
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
    if let Some(rect) = Rect::from_xywh(x, y, w, h) {
        let mut paint = Paint::default();
        paint.set_color(colour);
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

fn rounded(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, colour: Color) {
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
        let fate = out.entry(event.object_index).or_insert(Fate {
            gone_at: None,
            grabs: Vec::new(),
            missed: false,
        });
        match event.part {
            Part::Tap | Part::Hold | Part::Tail => {
                if event.result == ResultKind::Miss {
                    fate.missed = true;
                } else {
                    fate.gone_at = Some(event.time_ms);
                }
            }
            Part::Head if event.result == ResultKind::Miss => fate.missed = true,
            Part::Head | Part::HoldBreak => {}
        }
    }
    let mut open: HashMap<usize, f64> = HashMap::new();
    let close = |out: &mut HashMap<usize, Fate>, object: usize, from: f64, to: f64| {
        if let Some(fate) = out.get_mut(&object) {
            fate.grabs.push((from, to));
        }
    };
    if let Some(trace) = trace {
        for step in trace {
            match step.kind {
                TraceKind::HeadPressed | TraceKind::Regrabbed => {
                    open.insert(step.object_index, step.time_ms);
                }
                _ => {
                    if let Some(from) = open.remove(&step.object_index) {
                        close(&mut out, step.object_index, from, step.time_ms);
                    }
                }
            }
        }
    } else {
        let input = &playback.input;
        for event in judge.events() {
            if event.part != Part::Head || event.result == ResultKind::Miss {
                continue;
            }
            let released = input
                .events()
                .iter()
                .find(|key| {
                    key.column == event.column && !key.pressed && key.time_ms as f64 >= event.time_ms
                })
                .map_or(f64::INFINITY, |key| key.time_ms as f64);
            close(&mut out, event.object_index, event.time_ms, released);
        }
    }
    out
}

impl Show<'_> {
    fn draw(&self, pixmap: &mut Pixmap, time: f64) {
        pixmap.fill(rgb(14, 12, 18, 1.0));
        let columns = self.playback.timeline.columns();
        let field = LANE * f32::from(columns);
        let left = (WIDTH as f32 - field) / 2.0;
        fill(pixmap, left - 4.0, 0.0, field + 8.0, HEIGHT as f32, rgb(6, 5, 9, 1.0));
        for column in 1..columns {
            fill(pixmap, left + LANE * f32::from(column) - 1.0, 0.0, 2.0, HEIGHT as f32, rgb(255, 255, 255, 0.05));
        }

        let snapshot = match self.playback.at(time, SCROLL_MS) {
            Ok(snapshot) => snapshot,
            Err(_) => return,
        };
        for column in 0..columns {
            if snapshot.keys & (1 << column) != 0 {
                let (r, g, b) = column_colour(column, columns);
                for step in 0..24 {
                    let k = step as f32 / 24.0;
                    fill(pixmap, left + LANE * f32::from(column), LINE_Y - 300.0 * (1.0 - k), LANE, 300.0 / 24.0 + 1.0, rgb(r, g, b, 0.16 * k));
                }
            }
        }
        fill(pixmap, left, LINE_Y - 2.0, field, 4.0, rgb(255, 255, 255, 0.85));

        let travel = f64::from(LINE_Y);
        for visible in &snapshot.notes {
            let note = visible.note;
            let fate = self.fates.get(&note.object_index);
            if fate.and_then(|f| f.gone_at).is_some_and(|gone| gone <= time) {
                continue;
            }
            let held = fate.is_some_and(|f| f.grabs.iter().any(|&(from, to)| from <= time && time < to));
            let missed = fate.is_some_and(|f| f.missed);
            let (r, g, b) = column_colour(note.column, columns);
            let alpha = if missed { 0.35 } else { 1.0 };
            let x = left + LANE * f32::from(note.column) + 6.0;
            let w = LANE - 12.0;
            let head = if held { 0.0f64.max(visible.head) } else { visible.head };
            let head_y = (travel - head * travel) as f32;
            if note.hold {
                let tail_y = (travel - visible.tail * travel) as f32;
                let top = tail_y.max(-NOTE_H);
                let bottom = head_y.min(HEIGHT as f32 + NOTE_H);
                if bottom > top {
                    let body = if held { 0.75 } else { 0.5 } * alpha;
                    rounded(pixmap, x + 10.0, top, w - 20.0, bottom - top, 6.0, rgb(r, g, b, body));
                    rounded(pixmap, x + 4.0, tail_y - NOTE_H / 2.0, w - 8.0, NOTE_H / 2.0, 6.0, rgb(r, g, b, 0.9 * alpha));
                }
            }
            if head_y > -NOTE_H && head_y < HEIGHT as f32 + NOTE_H {
                rounded(pixmap, x, head_y - NOTE_H, w, NOTE_H, 8.0, rgb(r, g, b, alpha));
            }
        }

        for column in 0..columns {
            let (r, g, b) = column_colour(column, columns);
            let pressed = snapshot.keys & (1 << column) != 0;
            let x = left + LANE * f32::from(column) + 10.0;
            rounded(pixmap, x, LINE_Y + 22.0, LANE - 20.0, 60.0, 12.0, rgb(r, g, b, if pressed { 0.95 } else { 0.18 }));
        }

        self.draw_feedback(pixmap, time, left, field);
        self.draw_panel(pixmap, time, left + field + 60.0);
        self.draw_header(pixmap, time, left - 96.0);
    }

    fn fitted(&self, text: &str, size: f32, room: f32) -> String {
        if self.font.width(text, size) <= room {
            return text.to_owned();
        }
        let mut kept: Vec<char> = text.chars().collect();
        while !kept.is_empty() {
            kept.pop();
            let candidate = format!("{}…", kept.iter().collect::<String>().trim_end());
            if self.font.width(&candidate, size) <= room {
                return candidate;
            }
        }
        String::new()
    }

    fn draw_feedback(&self, pixmap: &mut Pixmap, time: f64, left: f32, field: f32) {
        let events = self.judge.events();
        let end = events.partition_point(|e: &Event| e.time_ms <= time);
        let centre = left + field / 2.0;
        if let Some(last) = end.checked_sub(1).map(|i| events[i]) {
            let age = time - last.time_ms;
            if age < 420.0 && last.part != Part::HoldBreak && last.part != Part::Head {
                let fade = (1.0 - age / 420.0) as f32;
                let pop = 1.0 + 0.25 * (1.0 - (age / 90.0).min(1.0)) as f32;
                let mut colour = grade_colour(last.result);
                colour.set_alpha(fade);
                self.font.draw(pixmap, Label {
                    text: grade_name(last.result, self.stable),
                    x: centre,
                    y: 560.0,
                    size: 64.0 * pop,
                    colour,
                    align: Align::Centre,
                });
            }
        }
        if let Some(combo) = self.judge.combo_at(time) {
            if combo.current > 0 {
                self.font.draw(pixmap, Label {
                    text: &combo.current.to_string(),
                    x: centre,
                    y: 440.0,
                    size: 92.0,
                    colour: rgb(255, 255, 255, 0.92),
                    align: Align::Centre,
                });
            }
        }
    }

    fn draw_panel(&self, pixmap: &mut Pixmap, time: f64, x: f32) {
        let counts = self.judge.counts_at(time);
        let total = counts.total();
        let weight = [300.0, 300.0, 200.0, 100.0, 50.0, 0.0];
        let earned: f64 = GRADES.iter().zip(weight).map(|(&k, w)| f64::from(counts.get(k)) * w).sum();
        let accuracy = if total == 0 { 100.0 } else { earned / (300.0 * f64::from(total)) * 100.0 };
        self.font.draw(pixmap, Label {
            text: &format!("{accuracy:.2}%"),
            x,
            y: 330.0,
            size: 58.0,
            colour: rgb(255, 255, 255, 1.0),
            align: Align::Left,
        });
        if let Some(combo) = self.judge.combo_at(time) {
            self.font.draw(pixmap, Label {
                text: &format!("{}x", combo.maximum),
                x,
                y: 380.0,
                size: 30.0,
                colour: rgb(255, 255, 255, 0.6),
                align: Align::Left,
            });
        }
        for (row, kind) in GRADES.iter().enumerate() {
            let y = 450.0 + row as f32 * 52.0;
            rounded(pixmap, x, y - 30.0, 10.0, 36.0, 4.0, grade_colour(*kind));
            self.font.draw(pixmap, Label {
                text: grade_name(*kind, self.stable),
                x: x + 26.0,
                y,
                size: 32.0,
                colour: grade_colour(*kind),
                align: Align::Left,
            });
            self.font.draw(pixmap, Label {
                text: &counts.get(*kind).to_string(),
                x: x + 300.0,
                y,
                size: 32.0,
                colour: rgb(255, 255, 255, 0.9),
                align: Align::Right,
            });
        }
    }

    fn draw_header(&self, pixmap: &mut Pixmap, time: f64, room: f32) {
        self.font.draw(pixmap, Label {
            text: &self.fitted(&self.title, 34.0, room),
            x: 48.0,
            y: 70.0,
            size: 34.0,
            colour: rgb(255, 255, 255, 0.95),
            align: Align::Left,
        });
        self.font.draw(pixmap, Label {
            text: &self.fitted(&self.version, 28.0, room),
            x: 48.0,
            y: 110.0,
            size: 28.0,
            colour: rgb(255, 255, 255, 0.75),
            align: Align::Left,
        });
        self.font.draw(pixmap, Label {
            text: &self.fitted(&self.player, 26.0, room),
            x: 48.0,
            y: 150.0,
            size: 26.0,
            colour: rgb(255, 255, 255, 0.55),
            align: Align::Left,
        });
        let span = (self.last_ms - self.first_ms).max(1.0);
        let done = ((time - self.first_ms) / span).clamp(0.0, 1.0) as f32;
        fill(pixmap, 0.0, HEIGHT as f32 - 6.0, WIDTH as f32, 6.0, rgb(255, 255, 255, 0.08));
        fill(pixmap, 0.0, HEIGHT as f32 - 6.0, WIDTH as f32 * done, 6.0, rgb(255, 255, 255, 0.7));
    }
}

fn font() -> Result<Font, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts");
    let main = std::fs::read(root.join("Nunito-ExtraBold.ttf")).map_err(|e| e.to_string())?;
    let rest = std::fs::read(root.join("MPLUSRounded1c-Regular.ttf")).map_err(|e| e.to_string())?;
    Ok(Font::from_bytes(&main)?.behind(&Font::from_bytes(&rest)?))
}

fn main() -> Result<(), String> {
    let usage = "usage: mania-render <map.osu> <replay.osr> <out.mp4> [--audio <file>]";
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 && !(args.len() == 5 && args[3] == "--audio") {
        return Err(usage.to_owned());
    }
    let map = Beatmap::parse(&std::fs::read_to_string(&args[0]).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let replay = Replay::parse(&std::fs::read(&args[1]).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let audio = args.get(4).map(PathBuf::from);
    let playback = Playback::new(&map, &replay).map_err(|e| e.to_string())?;
    let od = map.difficulty.overall_difficulty;
    let stable = replay.game_version < 30_000_000;
    let (judge, trace) = if stable {
        let rules = playback.stable_rules(od).map_err(|e| e.to_string())?;
        let (judge, trace) = playback.trace_stable_with(rules).map_err(|e| e.to_string())?;
        (judge, Some(trace))
    } else {
        let rules = playback.lazer_rules(od).map_err(|e| e.to_string())?;
        (playback.judge_with(rules).map_err(|e| e.to_string())?, None)
    };
    let fates = fates(&playback, &judge, trace.as_deref());
    let first_ms = map.objects.iter().map(|o| o.time_ms).fold(f64::INFINITY, f64::min);
    let last_ms = map.objects.iter().map(|o| o.end_time_ms()).fold(f64::NEG_INFINITY, f64::max);
    if !first_ms.is_finite() || !last_ms.is_finite() {
        return Err("the map has no notes".to_owned());
    }
    let show = Show {
        playback: &playback,
        judge: &judge,
        fates,
        stable,
        font: font()?,
        title: format!("{} - {}", map.metadata.artist, map.metadata.title),
        version: map.metadata.version.clone(),
        player: replay.player.clone(),
        first_ms,
        last_ms,
    };

    let rate = playback.clock_rate();
    let from_ms = (first_ms - LEAD_MS).max(0.0);
    let to_ms = last_ms + TAIL_MS;
    let frames = ((to_ms - from_ms) / rate / 1000.0 * FPS).ceil() as u64;

    let mut ffmpeg = Command::new("ffmpeg");
    ffmpeg.args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgba", "-s"])
        .arg(format!("{WIDTH}x{HEIGHT}"))
        .args(["-r", &FPS.to_string(), "-i", "-"]);
    if let Some(audio) = &audio {
        ffmpeg.args(["-ss", &format!("{:.3}", from_ms / 1000.0), "-i"]).arg(audio);
        if (rate - 1.0).abs() > 1e-9 {
            ffmpeg.args(["-af", &format!("atempo={rate}")]);
        }
        ffmpeg.args(["-c:a", "aac", "-b:a", "192k", "-shortest"]);
    }
    ffmpeg.args(["-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p"])
        .arg(&args[2])
        .stdin(Stdio::piped());
    let mut child = ffmpeg.spawn().map_err(|e| format!("ffmpeg: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("ffmpeg gave no input pipe")?;

    let workers = std::thread::available_parallelism().map_or(4, |n| n.get()) as u64;
    let started = std::time::Instant::now();
    let mut done = 0u64;
    while done < frames {
        let batch = (frames - done).min(workers * 4);
        let pictures: Vec<Vec<u8>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers)
                .map(|worker| {
                    let show = &show;
                    scope.spawn(move || {
                        let mut out = Vec::new();
                        let mut pixmap = Pixmap::new(WIDTH, HEIGHT).expect("frame");
                        let mut index = worker;
                        while index < batch {
                            let time = from_ms + (done + index) as f64 / FPS * 1000.0 * rate;
                            show.draw(&mut pixmap, time);
                            out.push((index, pixmap.data().to_vec()));
                            index += workers;
                        }
                        out
                    })
                })
                .collect();
            let mut all: Vec<(u64, Vec<u8>)> = handles.into_iter().flat_map(|h| h.join().expect("render thread")).collect();
            all.sort_by_key(|(index, _)| *index);
            all.into_iter().map(|(_, data)| data).collect()
        });
        for picture in pictures {
            stdin.write_all(&picture).map_err(|e| format!("ffmpeg stopped: {e}"))?;
        }
        done += batch;
        if done % (FPS as u64 * 10) < batch {
            eprintln!("{done}/{frames} frames, {:.0}s", started.elapsed().as_secs_f64());
        }
    }
    drop(stdin);
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("ffmpeg exited with {status}"));
    }
    eprintln!("{} written in {:.0}s", args[2], started.elapsed().as_secs_f64());
    Ok(())
}
