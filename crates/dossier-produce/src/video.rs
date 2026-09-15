use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use dossier_render::{Layout, Scene};
use tiny_skia::Pixmap;

#[derive(Debug, Clone)]
pub struct Backdrop {
    pub path: std::path::PathBuf,

    pub start_ms: f64,

    pub dim: f32,
}

pub struct Settings {
    pub out: std::path::PathBuf,
    pub fps: f64,
    pub size: (u32, u32),

    pub from_ms: Option<f64>,
    pub to_ms: Option<f64>,
    pub ffmpeg: String,
    pub crf: u32,

    pub preset: String,

    pub threads: Option<usize>,

    pub encoder_threads: Option<usize>,

    pub audio: Option<std::path::PathBuf>,

    pub video: Option<Backdrop>,

    pub hitsounds: Option<std::path::PathBuf>,

    pub music_level: f32,
    pub hitsound_level: f32,

    pub events: crate::events::Events,

    pub slow_at_ms: Option<f64>,

    pub slow_focus: Option<dossier_beatmap::Point>,
}

fn fail_tail_ms() -> f64 {
    dossier_render::FAIL_ANIMATION_MS + dossier_render::FAIL_EMPTY_MS
}

const FAIL_STEPS: usize = 10;

const FAIL_FLOOR: f64 = 0.08;

const SLOW_STEPS: usize = 8;

const SLOW_FLOOR: f64 = 0.25;

const SLOW_SPAN_MS: f64 = 700.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioSync {
    pub seek_seconds: f64,

    pub delay_seconds: f64,
    pub tempo: f64,
}

impl AudioSync {
    pub fn new(from_ms: f64, rate: f64) -> Self {
        Self {
            seek_seconds: (from_ms / 1000.0).max(0.0),

            delay_seconds: (-from_ms / 1000.0 / rate).max(0.0),
            tempo: rate,
        }
    }

    pub fn filter(&self) -> Option<String> {
        let mut chain = Vec::new();
        let tempo = ((self.tempo - 1.0).abs() > 1e-9 && (0.5..=2.0).contains(&self.tempo))
            .then_some(self.tempo);
        if self.delay_seconds > 0.0005 {
            let in_the_musics_own_time = self.delay_seconds * tempo.unwrap_or(1.0);
            chain.push(format!(
                "adelay={:.0}:all=1",
                in_the_musics_own_time * 1000.0
            ));
        }
        if let Some(tempo) = tempo {
            chain.push(format!("atempo={tempo:.6}"));
        }
        (!chain.is_empty()).then(|| chain.join(","))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Segment {
    video_seconds: f64,
    map_from_ms: f64,
    map_to_ms: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub from_ms: f64,
    pub to_ms: f64,
    pub frames: u64,
    pub video_seconds: f64,

    pub fail_at_ms: Option<f64>,

    slow_at_ms: Option<f64>,

    schedule: Vec<Segment>,
}

impl Plan {
    pub fn new(
        span: (f64, f64),
        rate: f64,
        settings: &Settings,
        fail_at_ms: Option<f64>,
    ) -> Result<Self, String> {
        let (width, height) = settings.size;
        if width % 2 != 0 || height % 2 != 0 {
            return Err(format!("{width}x{height}: both sides have to be even"));
        }
        if settings.fps <= 0.0 {
            return Err("fps has to be positive".to_owned());
        }
        if rate <= 0.0 {
            return Err("the playback rate has to be positive".to_owned());
        }

        let from_ms = settings.from_ms.unwrap_or(span.0);
        let to_ms = settings.to_ms.unwrap_or(span.1);
        if to_ms <= from_ms {
            return Err(format!(
                "nothing to render between {from_ms}ms and {to_ms}ms"
            ));
        }

        let fail_at_ms = fail_at_ms.filter(|at| *at > from_ms && *at <= to_ms + 1.0);

        let reaches_the_end = to_ms >= span.1 - 1.0;
        let extra_seconds = match fail_at_ms {
            Some(_) => fail_tail_ms() / 1000.0,
            None if reaches_the_end => dossier_render::OUTRO_FADE_MS / 1000.0,
            None => 0.0,
        };

        let map_end = to_ms + extra_seconds * 1000.0 * rate;

        let slow_at = settings
            .slow_at_ms
            .filter(|_| fail_at_ms.is_none())
            .filter(|at| *at > from_ms + SLOW_SPAN_MS * 0.25 && *at < to_ms - SLOW_SPAN_MS * 0.25);

        let schedule = match slow_at {
            None => vec![Segment {
                video_seconds: (map_end - from_ms) / 1000.0 / rate,
                map_from_ms: from_ms,
                map_to_ms: map_end,
            }],
            Some(at) => ramp_schedule(from_ms, to_ms, map_end, rate, at),
        };

        let total_seconds: f64 = schedule.iter().map(|s| s.video_seconds).sum();
        Ok(Self {
            from_ms,
            to_ms,
            frames: (total_seconds * settings.fps).ceil() as u64,
            video_seconds: total_seconds,
            fail_at_ms,
            slow_at_ms: slow_at,
            schedule,
        })
    }

    pub fn map_time_of(&self, index: u64, fps: f64) -> f64 {
        let mut video_ms = (index as f64 / fps) * 1000.0;
        let last = self.schedule.len().saturating_sub(1);
        for (i, segment) in self.schedule.iter().enumerate() {
            let span_ms = segment.video_seconds * 1000.0;
            if video_ms <= span_ms || i == last {
                let fraction = if span_ms > 1e-9 {
                    video_ms / span_ms
                } else {
                    0.0
                };
                return segment.map_from_ms + fraction * (segment.map_to_ms - segment.map_from_ms);
            }
            video_ms -= span_ms;
        }
        self.from_ms
    }

    pub fn video_time_of(&self, map_ms: f64) -> f64 {
        let mut video_offset = 0.0;
        let last = self.schedule.len().saturating_sub(1);
        for (i, segment) in self.schedule.iter().enumerate() {
            let span_map = segment.map_to_ms - segment.map_from_ms;
            if map_ms <= segment.map_to_ms || i == last {
                let fraction = if span_map.abs() > 1e-9 {
                    (map_ms - segment.map_from_ms) / span_map
                } else {
                    0.0
                };
                return video_offset + fraction * segment.video_seconds;
            }
            video_offset += segment.video_seconds;
        }
        video_offset
    }

    pub fn closeness_at(&self, index: u64, fps: f64) -> f64 {
        let Some(at) = self.slow_at_ms else {
            return 0.0;
        };
        let map_ms = self.map_time_of(index, fps);
        let nearness = 1.0 - ((map_ms - at).abs() / SLOW_SPAN_MS).clamp(0.0, 1.0);
        nearness * nearness * (3.0 - 2.0 * nearness)
    }

    fn music_warp(&self) -> Option<Vec<MusicSlice>> {
        if self.schedule.len() < 2 || self.from_ms < 0.0 {
            return None;
        }
        Some(
            self.schedule
                .iter()
                .map(|segment| {
                    let map_span = segment.map_to_ms - segment.map_from_ms;
                    MusicSlice {
                        start_s: (segment.map_from_ms - self.from_ms) / 1000.0,
                        end_s: (segment.map_to_ms - self.from_ms) / 1000.0,
                        tempo: (map_span / 1000.0) / segment.video_seconds,
                    }
                })
                .collect(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MusicSlice {
    start_s: f64,
    end_s: f64,
    tempo: f64,
}

fn atempo_chain(mut tempo: f64) -> String {
    let mut factors = Vec::new();
    while tempo < 0.5 - 1e-9 {
        factors.push(0.5);
        tempo /= 0.5;
    }
    while tempo > 2.0 + 1e-9 {
        factors.push(2.0);
        tempo /= 2.0;
    }
    factors.push(tempo);
    factors
        .iter()
        .map(|factor| format!("atempo={factor:.6}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn ramp_schedule(from_ms: f64, to_ms: f64, map_end: f64, rate: f64, at: f64) -> Vec<Segment> {
    let lo = (at - SLOW_SPAN_MS).max(from_ms);
    let hi = (at + SLOW_SPAN_MS).min(to_ms);
    let mut segments = Vec::with_capacity(2 * SLOW_STEPS + 2);
    let mut push = |map_from: f64, map_to: f64, seg_rate: f64| {
        if map_to - map_from > 1e-6 {
            segments.push(Segment {
                video_seconds: (map_to - map_from) / 1000.0 / seg_rate,
                map_from_ms: map_from,
                map_to_ms: map_to,
            });
        }
    };

    let rate_at = |m: f64| {
        let side = ((m - at).abs() / SLOW_SPAN_MS).clamp(0.0, 1.0);
        rate * (SLOW_FLOOR + (1.0 - SLOW_FLOOR) * side)
    };
    push(from_ms, lo, rate);
    for (start, end) in [(lo, at), (at, hi)] {
        let step = (end - start) / SLOW_STEPS as f64;
        for k in 0..SLOW_STEPS {
            let a = start + step * k as f64;
            let b = start + step * (k as f64 + 1.0);
            push(a, b, rate_at((a + b) / 2.0));
        }
    }
    push(hi, map_end, rate);
    segments
}

pub fn encode(
    scene: &Scene<'_>,
    span: (f64, f64),
    rate: f64,
    settings: &Settings,
    fail_at_ms: Option<f64>,
) -> Result<(), String> {
    let plan = Plan::new(span, rate, settings, fail_at_ms)?;
    let (width, height) = settings.size;
    let total = plan.frames;

    let sync = AudioSync::new(plan.from_ms, rate);

    let stall_at_seconds = plan
        .fail_at_ms
        .map(|at| (at - plan.from_ms) / 1000.0 / rate);
    let warp = plan.music_warp();
    let mut child = spawn(
        settings,
        sync,
        stall_at_seconds,
        warp.as_deref(),
        plan.video_seconds,
        plan.from_ms,
    )?;
    let drained = drain_stderr(&mut child);
    let mut stdin = child
        .stdin
        .take()
        .ok_or("ffmpeg gave us no pipe to write to")?;

    let layout = Layout::new(width, height);
    let workers = settings.threads.unwrap_or_else(default_workers).max(1);
    let started = std::time::Instant::now();

    const OWNED: usize = 2;
    let mut returns = Vec::with_capacity(workers);
    let mut inboxes = Vec::with_capacity(workers);
    for _ in 0..workers {
        let (tx, rx) = std::sync::mpsc::channel::<Frame>();
        for _ in 0..OWNED {
            let frame = Frame::new(width, height, settings.video.is_some())
                .ok_or("could not allocate a frame")?;
            tx.send(frame).map_err(|e| e.to_string())?;
        }
        returns.push(tx);
        inboxes.push(rx);
    }

    let (done_tx, done_rx) = std::sync::mpsc::channel::<(u64, usize, Frame)>();
    let next = std::sync::atomic::AtomicU64::new(0);
    let drawing = std::sync::atomic::AtomicU64::new(0);

    eprintln!(
        "   {:.2}s…{:.2}s of map time{}",
        plan.from_ms / 1000.0,
        plan.to_ms / 1000.0,
        match sync.delay_seconds {
            d if d > 0.0005 => format!(", music held back {d:.2}s"),
            _ => String::new(),
        }
    );
    eprintln!("   {workers} render thread(s), {OWNED} frame buffers each");

    let mut piping = std::time::Duration::ZERO;
    let outcome: Result<(), String> = std::thread::scope(|scope| {
        for (worker, rx) in inboxes.into_iter().enumerate() {
            let (done_tx, next, drawing) = (done_tx.clone(), &next, &drawing);
            let (scene, layout, plan, settings) = (scene, &layout, &plan, settings);
            scope.spawn(move || {
                while let Ok(mut buffer) = rx.recv() {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if index >= total {
                        break;
                    }

                    let mark = std::time::Instant::now();

                    let camera = settings.slow_focus.map(|focus| dossier_render::Camera {
                        focus,
                        closeness: plan.closeness_at(index, settings.fps),
                    });
                    scene.draw_into(
                        &mut buffer.pixmap,
                        plan.map_time_of(index, settings.fps),
                        layout,
                        camera,
                    );
                    let Frame { pixmap, yuv } = &mut buffer;
                    if settings.video.is_some() {
                        let plane = (pixmap.width() * pixmap.height()) as usize;
                        let split = yuv.len() - plane;
                        let (colour, alpha) = yuv.split_at_mut(split);
                        write_alpha(pixmap, alpha);
                        to_yuv420(pixmap, colour);
                    } else {
                        to_yuv420(pixmap, yuv);
                    }
                    drawing.fetch_add(
                        mark.elapsed().as_micros() as u64,
                        std::sync::atomic::Ordering::Relaxed,
                    );

                    if done_tx.send((index, worker, buffer)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(done_tx);

        let returns = returns;

        let mut pending: std::collections::HashMap<u64, (usize, Frame)> =
            std::collections::HashMap::new();
        let mut wanted = 0u64;
        while wanted < total {
            if crate::halt::asked() {
                return Err(format!("{} на {wanted} из {total}", crate::halt::SAID));
            }
            let (index, worker, frame) = match done_rx.recv() {
                Ok(triple) => triple,
                Err(_) => return Err("a render thread stopped early".to_owned()),
            };
            pending.insert(index, (worker, frame));

            while let Some((worker, frame)) = pending.remove(&wanted) {
                let mark = std::time::Instant::now();
                let written = stdin.write_all(&frame.yuv);
                piping += mark.elapsed();

                let _ = returns[worker].send(frame);
                if let Err(error) = written {
                    return Err(format!("ffmpeg stopped after {wanted} frames: {error}"));
                }
                wanted += 1;
                if wanted.is_multiple_of((settings.fps as u64 * 5).max(1)) {
                    report(wanted, total, started, settings.events);
                }
            }
        }
        Ok(())
    });

    let close_progress = || eprintln!();

    if let Err(message) = outcome {
        drop(stdin);

        if crate::halt::was_it(&message) {
            let _ = child.wait();
            close_progress();
            return Err(message);
        }
        let status = child.wait().ok();
        close_progress();
        let said = ffmpeg_said(drained);
        let mut out = message;
        if let Some(status) = status {
            if !status.success() {
                out.push_str(&format!("\n   ffmpeg exited with {status}"));
            }
        }
        if said.is_empty() {
            out.push_str(
                "\n   ffmpeg said nothing at all, which usually means it was killed \
                 rather than that it failed — check the machine for memory and for \
                 room on the filesystem the output is being written to.",
            );
        } else {
            out.push_str(&format!("\n   ffmpeg said: {said}"));
        }
        return Err(out);
    }

    drop(stdin);
    let status = child.wait().map_err(|e| e.to_string())?;
    let said = ffmpeg_said(drained);
    if !status.success() {
        close_progress();

        return Err(if said.is_empty() {
            format!(
                "ffmpeg exited with {status} and said nothing. If that is a signal, \
                 the machine most likely ran out of memory or disk."
            )
        } else {
            format!("ffmpeg exited with {status}\n   ffmpeg said: {said}")
        });
    }
    if !said.is_empty() {
        close_progress();
        crate::note!("ffmpeg warned: {said}");
    }

    let elapsed = started.elapsed().as_secs_f64();
    eprintln!(
        "\r{total} frames in {elapsed:.1}s ({:.0} fps, {:.1}× realtime){:20}",
        total as f64 / elapsed,
        plan.video_seconds / elapsed,
        ""
    );

    crate::note!("video {width}x{height} {:.3}s", plan.video_seconds);
    settings.events.video(width, height, plan.video_seconds);
    let drawing_ms = drawing.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1000.0;
    eprintln!(
        "   {workers} render thread(s): {:.1}ms of drawing per frame across them, \
         {:.1}ms piping, {:.1}ms elapsed",
        drawing_ms / total as f64,
        piping.as_secs_f64() * 1000.0 / total as f64,
        elapsed * 1000.0 / total as f64,
    );
    Ok(())
}

fn spawn(
    settings: &Settings,
    sync: AudioSync,
    stall_at_seconds: Option<f64>,
    music_warp: Option<&[MusicSlice]>,
    video_seconds: f64,
    from_ms: f64,
) -> Result<Child, String> {
    let (width, height) = settings.size;
    let mut command = Command::new(&settings.ffmpeg);

    let mut inputs = 0usize;
    command.args([
        "-y",
        "-loglevel",
        "error",
        "-f",
        "rawvideo",
        "-pixel_format",
        if settings.video.is_some() {
            "yuva420p"
        } else {
            "yuv420p"
        },
        "-video_size",
        &format!("{width}x{height}"),
        "-framerate",
        &format!("{}", settings.fps),
        "-i",
        "-",
    ]);

    let mut music = None;
    if let Some(audio) = &settings.audio {
        command
            .arg("-ss")
            .arg(format!("{:.3}", sync.seek_seconds))
            .arg("-i")
            .arg(audio);
        music = Some(command_input_index(&mut inputs));
    }

    let mut hits = None;
    if let Some(pcm) = &settings.hitsounds {
        command
            .args([
                "-f",
                "s16le",
                "-ar",
                &dossier_audio::SAMPLE_RATE.to_string(),
                "-ac",
                "2",
                "-i",
            ])
            .arg(pcm);
        hits = Some(command_input_index(&mut inputs));
    }

    let mut over_video = None;
    if let Some(film) = &settings.video {
        let seek = (from_ms - film.start_ms) / 1000.0;
        if seek > 0.0 {
            command.arg("-ss").arg(format!("{seek:.3}"));
        }
        command.arg("-i").arg(&film.path);
        let at = command_input_index(&mut inputs);

        let keep = 1.0 - film.dim.clamp(0.0, 1.0);
        let mut chain = format!(
            "[{at}:v]setpts=(PTS-STARTPTS)/{tempo},\
             scale={width}:{height}:force_original_aspect_ratio=increase,\
             crop={width}:{height},setsar=1,\
             colorchannelmixer=rr={keep}:gg={keep}:bb={keep}",
            tempo = sync.tempo,
        );

        let late = (-seek).max(0.0) / sync.tempo;
        if late > 0.000_5 {
            chain.push_str(&format!(",tpad=start_duration={late:.3}"));
        }
        chain.push_str(&format!(",tpad=stop_duration={video_seconds:.3}[bg];"));

        chain.push_str("[bg][0:v]overlay=eof_action=endall:format=auto:alpha=premultiplied[v]");
        over_video = Some(chain);
    }

    let audio = audio_filter(
        music,
        hits,
        &sync,
        stall_at_seconds,
        music_warp,
        video_seconds,
        (settings.music_level, settings.hitsound_level),
    );
    let picture = if over_video.is_some() { "[v]" } else { "0:v" };
    match (&over_video, &audio) {
        (Some(video), None) => {
            command.args(["-filter_complex", video, "-map", picture]);
        }
        (video, Some(_)) => {
            let filter = match video {
                Some(video) => format!("{video};{}", audio.as_deref().unwrap_or_default()),
                None => audio.clone().unwrap_or_default(),
            };
            command.args(["-filter_complex", &filter, "-map", picture, "-map", "[a]"]);
        }
        (None, None) => {}
    }
    if audio.is_some() {
        command.args(["-c:a", "aac", "-b:a", "192k"]);

        command.arg("-shortest");
    }

    if let Some(threads) = settings.encoder_threads {
        command.args(["-threads", &threads.to_string()]);
    }

    command
        .args([
            "-c:v",
            "libx264",
            "-preset",
            &settings.preset,
            "-crf",
            &settings.crf.to_string(),
            "-pix_fmt",
            "yuv420p",
            "-colorspace",
            "bt709",
            "-color_primaries",
            "bt709",
            "-color_trc",
            "bt709",
            "-color_range",
            "tv",
        ])
        .arg(&settings.out)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped());
    if std::env::var("DOSSIER_FFMPEG_ARGS").is_ok() {
        eprintln!("ffmpeg {:?}", command.get_args().collect::<Vec<_>>());
    }
    command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            format!(
                "{} not found. Install it (macOS: brew install ffmpeg, \
                     Debian: apt install ffmpeg) or pass --ffmpeg <path>.",
                settings.ffmpeg
            )
        } else {
            format!("could not start {}: {error}", settings.ffmpeg)
        }
    })
}

const FFMPEG_STDERR_KEPT: usize = 8 * 1024;

fn drain_stderr(child: &mut Child) -> Option<std::thread::JoinHandle<String>> {
    let mut stderr = child.stderr.take()?;
    Some(std::thread::spawn(move || {
        use std::io::Read as _;
        let mut kept = String::new();
        let mut chunk = [0u8; 4096];
        while let Ok(read) = stderr.read(&mut chunk) {
            if read == 0 {
                break;
            }
            kept.push_str(&String::from_utf8_lossy(&chunk[..read]));
            if kept.len() > FFMPEG_STDERR_KEPT {
                let from = kept.len() - FFMPEG_STDERR_KEPT;
                kept = kept[from..].to_owned();
            }
        }
        kept
    }))
}

fn ffmpeg_said(drained: Option<std::thread::JoinHandle<String>>) -> String {
    let Some(handle) = drained else {
        return String::new();
    };
    let text = handle.join().unwrap_or_default();
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    lines.join("; ")
}

fn report(index: u64, total: u64, started: std::time::Instant, events: crate::events::Events) {
    let done = index.max(1);
    let rate = done as f64 / started.elapsed().as_secs_f64();
    let left = (total - done) as f64 / rate.max(0.001);
    eprint!("\r{done}/{total} frames, {rate:.0}/s, {left:.0}s left     ",);
    let _ = std::io::stderr().flush();
    events.progress(done, total, rate, left);
}

pub fn check_output(path: &Path) -> Result<(), String> {
    match path.extension().and_then(|e| e.to_str()) {
        Some(_) => Ok(()),

        None => Err(format!(
            "{}: give the output a file extension so ffmpeg knows the container",
            path.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings {
            out: std::path::PathBuf::from("out.mp4"),
            fps: 60.0,
            size: (1280, 720),
            from_ms: None,
            to_ms: None,
            ffmpeg: "ffmpeg".to_owned(),
            crf: 20,
            preset: "veryfast".to_owned(),
            music_level: 1.0,
            hitsound_level: 1.0,
            threads: None,
            encoder_threads: None,
            audio: None,
            video: None,
            hitsounds: None,
            events: crate::events::Events::wanted(false),
            slow_at_ms: None,
            slow_focus: None,
        }
    }

    #[test]
    fn a_plain_play_renders_one_frame_per_tick_of_the_clock() {
        let plan = Plan::new((0.0, 10_000.0), 1.0, &settings(), None).unwrap();

        let tail = dossier_render::OUTRO_FADE_MS / 1000.0;
        assert_eq!(plan.frames, ((10.0 + tail) * settings().fps).ceil() as u64);
        assert!((plan.video_seconds - (10.0 + tail)).abs() < 1e-9);
    }

    #[test]
    fn atempo_below_a_half_is_reached_by_chaining() {
        assert_eq!(atempo_chain(1.0), "atempo=1.000000");
        assert_eq!(atempo_chain(1.5), "atempo=1.500000");
        assert_eq!(atempo_chain(0.25), "atempo=0.500000,atempo=0.500000");

        let product: f64 = atempo_chain(0.375)
            .split(',')
            .map(|f| f.trim_start_matches("atempo=").parse::<f64>().unwrap())
            .product();
        assert!((product - 0.375).abs() < 1e-6, "{product}");
    }

    #[test]
    fn the_music_is_sliced_to_fill_the_video_it_plays_under() {
        let mut slowed = settings();
        slowed.slow_at_ms = Some(5_000.0);
        let plan = Plan::new((0.0, 10_000.0), 1.0, &slowed, None).unwrap();
        let slices = plan.music_warp().expect("a dip slices the music");

        assert!((slices[0].start_s - 0.0).abs() < 1e-9);
        for pair in slices.windows(2) {
            assert!(
                (pair[0].end_s - pair[1].start_s).abs() < 1e-9,
                "a seam moved"
            );
        }

        let played: f64 = slices.iter().map(|s| (s.end_s - s.start_s) / s.tempo).sum();
        assert!(
            (played - plan.video_seconds).abs() < 1e-6,
            "{played} vs {}",
            plan.video_seconds
        );

        let slowest = slices.iter().map(|s| s.tempo).fold(f64::INFINITY, f64::min);
        assert!(slowest < 0.5, "slowest tempo {slowest}");

        assert!(Plan::new((0.0, 10_000.0), 1.0, &settings(), None)
            .unwrap()
            .music_warp()
            .is_none());
    }

    #[test]
    fn doubletime_packs_more_map_into_the_same_second_of_video() {
        let tail = dossier_render::OUTRO_FADE_MS / 1000.0;
        let plan = Plan::new((0.0, 10_000.0), 1.5, &settings(), None).unwrap();

        assert_eq!(
            plan.frames,
            ((10.0 / 1.5 + tail) * settings().fps).ceil() as u64
        );

        assert!((plan.map_time_of(60, 60.0) - 1500.0).abs() < 1e-9);
    }

    #[test]
    fn halftime_stretches_it_the_other_way() {
        let tail = dossier_render::OUTRO_FADE_MS / 1000.0;
        let plan = Plan::new((0.0, 10_000.0), 0.75, &settings(), None).unwrap();
        assert_eq!(
            plan.frames,
            ((10.0 / 0.75 + tail) * settings().fps).ceil() as u64
        );
    }

    #[test]
    fn the_two_clocks_invert_each_other() {
        let mut slowed = settings();
        slowed.slow_at_ms = Some(5_000.0);
        let fps = 60.0;
        for (rate, settings) in [(1.0, settings()), (1.5, settings()), (1.0, slowed)] {
            let plan = Plan::new((0.0, 10_000.0), rate, &settings, None).unwrap();
            for map_ms in [100.0, 2_000.0, 4_800.0, 5_000.0, 5_200.0, 9_000.0] {
                let video = plan.video_time_of(map_ms);
                let frame = (video * fps).round() as u64;
                let back = plan.map_time_of(frame, fps);
                let a_frame_of_map = rate / fps * 1000.0;
                assert!(
                    (back - map_ms).abs() < 3.0 * a_frame_of_map,
                    "{map_ms}ms → {video}s → {back}ms"
                );
            }
        }
    }

    #[test]
    fn a_slow_moment_dwells_without_ever_running_backwards() {
        let mut slowed = settings();
        slowed.slow_at_ms = Some(5_000.0);
        let plain = Plan::new((0.0, 10_000.0), 1.0, &settings(), None).unwrap();
        let slow = Plan::new((0.0, 10_000.0), 1.0, &slowed, None).unwrap();

        assert!(
            slow.video_seconds > plain.video_seconds + 0.5,
            "{}",
            slow.video_seconds
        );

        let fps = settings().fps;
        let mut previous = f64::NEG_INFINITY;
        for frame in 0..slow.frames {
            let now = slow.map_time_of(frame, fps);
            assert!(now >= previous - 1e-6, "map time went backwards at {frame}");
            previous = now;
        }

        let map_per_video = |from_frame: u64| {
            slow.map_time_of(from_frame + fps as u64, fps) - slow.map_time_of(from_frame, fps)
        };
        let near_the_start = map_per_video(0);
        let slowest = (0..slow.frames.saturating_sub(fps as u64))
            .map(|f| map_per_video(f))
            .fold(f64::INFINITY, f64::min);
        assert!(
            slowest < near_the_start * 0.5,
            "slowest second {slowest} vs the start's {near_the_start}"
        );
    }

    #[test]
    fn an_explicit_span_overrides_the_plays_own() {
        let mut settings = settings();
        settings.from_ms = Some(2_000.0);
        settings.to_ms = Some(3_000.0);
        let plan = Plan::new((0.0, 100_000.0), 1.0, &settings, None).unwrap();
        assert_eq!((plan.from_ms, plan.to_ms), (2_000.0, 3_000.0));
        assert_eq!(plan.frames, 60);
    }

    #[test]
    fn odd_dimensions_are_refused_before_a_single_frame_is_drawn() {
        let mut settings = settings();
        settings.size = (1281, 720);
        assert!(Plan::new((0.0, 1000.0), 1.0, &settings, None).is_err());
    }

    #[test]
    fn an_empty_or_backwards_span_is_refused() {
        let mut settings = settings();
        settings.from_ms = Some(5_000.0);
        settings.to_ms = Some(1_000.0);
        assert!(Plan::new((0.0, 100_000.0), 1.0, &settings, None).is_err());
    }

    #[test]
    fn the_output_needs_an_extension_for_ffmpeg_to_pick_a_container() {
        assert!(check_output(std::path::Path::new("replay.mp4")).is_ok());
        assert!(check_output(std::path::Path::new("replay")).is_err());
    }
}

#[cfg(test)]
mod audio_tests {
    use super::*;

    #[test]
    fn the_track_is_seeked_to_where_the_render_starts() {
        let sync = AudioSync::new(46_000.0, 1.0);
        assert!((sync.seek_seconds - 46.0).abs() < 1e-9);
        assert_eq!(sync.filter(), None, "no stretching at normal speed");
    }

    #[test]
    fn a_lead_in_before_the_song_seeks_to_zero() {
        let sync = AudioSync::new(-800.0, 1.0);
        assert_eq!(sync.seek_seconds, 0.0);
    }

    #[test]
    fn rate_mods_stretch_the_track_to_match() {
        assert_eq!(
            AudioSync::new(0.0, 1.5).filter().as_deref(),
            Some("atempo=1.500000")
        );
        assert_eq!(
            AudioSync::new(0.0, 0.75).filter().as_deref(),
            Some("atempo=0.750000")
        );
    }

    #[test]
    fn a_rate_atempo_cannot_do_in_one_pass_is_refused_rather_than_emitted() {
        assert_eq!(AudioSync::new(0.0, 3.0).filter(), None);
    }
}

fn command_input_index(count: &mut usize) -> usize {
    *count += 1;
    *count
}

struct Frame {
    pixmap: Pixmap,
    yuv: Vec<u8>,
}

impl Frame {
    fn new(width: u32, height: u32, with_alpha: bool) -> Option<Self> {
        let (w, h) = (width as usize, height as usize);
        Some(Self {
            pixmap: Pixmap::new(width, height)?,
            yuv: vec![0; yuv_len(w, h) + if with_alpha { w * h } else { 0 }],
        })
    }
}

fn write_alpha(pixmap: &Pixmap, out: &mut [u8]) {
    for (pixel, slot) in pixmap.pixels().iter().zip(out.iter_mut()) {
        *slot = pixel.alpha();
    }
}

fn yuv_len(width: usize, height: usize) -> usize {
    width * height + 2 * (width / 2) * (height / 2)
}

fn to_yuv420(pixmap: &Pixmap, out: &mut [u8]) {
    const YR: i32 = 11_966;
    const YG: i32 = 40_254;
    const YB: i32 = 4_064;
    const UR: i32 = -6_595;
    const UG: i32 = -22_189;
    const UB: i32 = 28_784;
    const VR: i32 = 28_784;
    const VG: i32 = -26_142;
    const VB: i32 = -2_642;
    const HALF: i32 = 1 << 15;

    let (width, height) = (pixmap.width() as usize, pixmap.height() as usize);
    let src = pixmap.data();
    let (luma, chroma) = out.split_at_mut(width * height);
    let (blues, reds) = chroma.split_at_mut((width / 2) * (height / 2));

    for (row, line) in luma.chunks_exact_mut(width).enumerate() {
        let pixels = &src[row * width * 4..(row + 1) * width * 4];
        for (out, rgba) in line.iter_mut().zip(pixels.chunks_exact(4)) {
            let (r, g, b) = (i32::from(rgba[0]), i32::from(rgba[1]), i32::from(rgba[2]));
            *out = (16 + ((YR * r + YG * g + YB * b + HALF) >> 16)) as u8;
        }
    }

    let half_width = width / 2;
    for pair in 0..height / 2 {
        let top = &src[pair * 2 * width * 4..(pair * 2 + 1) * width * 4];
        let bottom = &src[(pair * 2 + 1) * width * 4..(pair * 2 + 2) * width * 4];
        let u_row = &mut blues[pair * half_width..(pair + 1) * half_width];
        let v_row = &mut reds[pair * half_width..(pair + 1) * half_width];

        for (x, (u, v)) in u_row.iter_mut().zip(v_row.iter_mut()).enumerate() {
            let mut sums = [0i32; 3];
            for row in [top, bottom] {
                for dx in 0..2 {
                    let at = (x * 2 + dx) * 4;
                    sums[0] += i32::from(row[at]);
                    sums[1] += i32::from(row[at + 1]);
                    sums[2] += i32::from(row[at + 2]);
                }
            }
            let (r, g, b) = (sums[0] / 4, sums[1] / 4, sums[2] / 4);
            *u = (128 + ((UR * r + UG * g + UB * b + HALF) >> 16)) as u8;
            *v = (128 + ((VR * r + VG * g + VB * b + HALF) >> 16)) as u8;
        }
    }
}

const MUSIC_DUCK: f32 = 0.18;

const STALL_RATE: u32 = 44_100;

fn pad_to(video_seconds: f64) -> String {
    format!("apad=whole_dur={video_seconds:.3}")
}

fn audio_filter(
    music: Option<usize>,
    hits: Option<usize>,
    sync: &AudioSync,
    stall_at_seconds: Option<f64>,
    music_warp: Option<&[MusicSlice]>,
    video_seconds: f64,
    levels: (f32, f32),
) -> Option<String> {
    let (music_level, hitsound_level) = (levels.0.max(0.0), levels.1.max(0.0));

    let stretched = |index: usize, duck: bool| {
        let level = if duck {
            MUSIC_DUCK * music_level
        } else {
            music_level
        };
        let ducked = if (level - 1.0).abs() > f32::EPSILON {
            format!(",volume={level}")
        } else {
            String::new()
        };

        if let Some(slices) = music_warp {
            let reach = slices.last().map_or(0.0, |s| s.end_s);
            let mut graph = format!(
                "[{index}:a]apad=whole_dur={reach:.3},asplit={}",
                slices.len()
            );
            for k in 0..slices.len() {
                graph.push_str(&format!("[w{k}]"));
            }
            graph.push(';');
            for (k, slice) in slices.iter().enumerate() {
                graph.push_str(&format!(
                    "[w{k}]atrim=start={:.4}:end={:.4},asetpts=PTS-STARTPTS,{}[p{k}];",
                    slice.start_s,
                    slice.end_s,
                    atempo_chain(slice.tempo)
                ));
            }
            let joins: String = (0..slices.len()).map(|k| format!("[p{k}]")).collect();
            graph.push_str(&format!(
                "{joins}concat=n={}:v=0:a=1{ducked},{}[m]",
                slices.len(),
                pad_to(video_seconds)
            ));
            return graph;
        }
        let mut chain = Vec::new();
        if let Some(tempo) = sync.filter() {
            chain.push(tempo);
        }
        if (level - 1.0).abs() > f32::EPSILON {
            chain.push(format!("volume={level}"));
        }

        chain.push(pad_to(video_seconds));
        format!("[{index}:a]{}[m]", chain.join(","))
    };

    let (hit_stage, hit_label) = match hits {
        Some(h) if (hitsound_level - 1.0).abs() > f32::EPSILON => (
            Some(format!("[{h}:a]volume={hitsound_level}[h]")),
            "[h]".to_owned(),
        ),
        Some(h) => (None, format!("[{h}:a]")),
        None => (None, String::new()),
    };

    let mixed = match (music, hits) {
        (Some(m), Some(_)) => {
            let mut graph = stretched(m, true);
            if let Some(stage) = &hit_stage {
                graph.push(';');
                graph.push_str(stage);
            }
            Some(format!(
                "{graph};[m]{hit_label}amix=inputs=2:duration=longest:normalize=0[mix]"
            ))
        }

        (Some(m), None) => Some(format!("{};[m]anull[mix]", stretched(m, false))),
        (None, Some(_)) => Some(match &hit_stage {
            Some(stage) => format!("{stage};[h]anull[mix]"),
            None => format!("{hit_label}anull[mix]"),
        }),
        (None, None) => None,
    }?;

    let Some(stall) = stall_at_seconds.filter(|s| *s > 0.05) else {
        return Some(format!("{mixed};[mix]{}[a]", pad_to(video_seconds)));
    };

    let seconds = dossier_render::FAIL_ANIMATION_MS / 1000.0;
    let step_out = seconds / FAIL_STEPS as f64;
    let mut chunks = String::new();
    let mut labels = String::new();
    let mut source = 0.0f64;
    for k in 0..FAIL_STEPS {
        let rate = (1.0 - (k as f64 + 0.5) / FAIL_STEPS as f64).max(FAIL_FLOOR);
        let take = step_out * rate;
        chunks.push_str(&format!(
            "[s{k}]atrim=start={:.4}:duration={take:.4},asetpts=PTS-STARTPTS,\
             asetrate={STALL_RATE}*{rate:.4},aresample={STALL_RATE}[c{k}];",
            stall + source
        ));
        labels.push_str(&format!("[c{k}]"));
        source += take;
    }

    let splits: String = (0..FAIL_STEPS).map(|k| format!("[s{k}]")).collect();
    let pad = pad_to(video_seconds);
    Some(format!(
        "{mixed};\
         [mix]aresample={STALL_RATE},asplit={}[head0]{splits};\
         [head0]atrim=0:{stall:.3},asetpts=PTS-STARTPTS[head];\
         {chunks}\
         {labels}concat=n={}:v=0:a=1,afade=t=out:st=0:d={seconds:.3}[tail];\
         [head][tail]concat=n=2:v=0:a=1,{pad}[a]",
        FAIL_STEPS + 1,
        FAIL_STEPS,
    ))
}

#[cfg(test)]
mod filter_tests {
    use super::*;

    fn sync(rate: f64) -> AudioSync {
        AudioSync::new(0.0, rate)
    }

    #[test]
    fn music_alone_is_stretched_and_passed_through() {
        let filter = audio_filter(Some(1), None, &sync(1.5), None, None, 10.0, (1.0, 1.0)).unwrap();

        assert!(
            filter.contains("[1:a]atempo=1.500000,apad=whole_dur=10.000[m]"),
            "{filter}"
        );
        assert!(filter.ends_with("[a]"));
    }

    #[test]
    fn the_two_halves_of_the_mix_are_turned_down_apart_from_each_other() {
        let filter =
            audio_filter(Some(1), Some(2), &sync(1.0), None, None, 10.0, (0.3, 1.0)).unwrap();

        assert!(
            filter.contains(&format!("volume={}", MUSIC_DUCK * 0.3)),
            "{filter}"
        );
        assert!(
            filter.contains("[m][2:a]amix"),
            "the hits were touched: {filter}"
        );

        let other =
            audio_filter(Some(1), Some(2), &sync(1.0), None, None, 10.0, (1.0, 0.4)).unwrap();
        assert!(other.contains("[2:a]volume=0.4[h]"), "{other}");
        assert!(other.contains(&format!("volume={MUSIC_DUCK}")), "{other}");
    }

    #[test]
    fn a_silenced_half_is_a_level_rather_than_a_missing_input() {
        let filter =
            audio_filter(Some(1), Some(2), &sync(1.5), None, None, 10.0, (0.0, 1.0)).unwrap();
        assert!(filter.contains("[1:a]atempo=1.500000,volume=0"), "{filter}");
        assert!(filter.contains("apad=whole_dur=10.000[m]"), "{filter}");
        assert!(
            filter.contains("amix=inputs=2"),
            "the mix lost an input: {filter}"
        );
    }

    #[test]
    fn the_natural_levels_leave_the_graph_as_it_was() {
        let filter =
            audio_filter(Some(1), Some(2), &sync(1.0), None, None, 10.0, (1.0, 1.0)).unwrap();
        assert!(filter.contains(&format!("volume={MUSIC_DUCK}")), "{filter}");
        assert_eq!(filter.matches("volume=").count(), 1, "{filter}");
    }

    #[test]
    fn a_short_audio_file_does_not_cut_the_hit_sounds_off_with_it() {
        let filter =
            audio_filter(Some(1), Some(2), &sync(1.5), None, None, 60.0, (1.0, 1.0)).unwrap();
        assert!(filter.contains("duration=longest"), "{filter}");
        assert!(!filter.contains("duration=first"), "{filter}");

        assert!(filter.contains("apad=whole_dur=60.000[m]"), "{filter}");
    }

    #[test]
    fn the_two_streams_are_mixed_without_being_quietened() {
        let filter =
            audio_filter(Some(1), Some(2), &sync(1.0), None, None, 10.0, (1.0, 1.0)).unwrap();
        assert!(filter.contains("normalize=0"), "{filter}");
        assert!(filter.contains("amix=inputs=2"), "{filter}");
    }

    #[test]
    fn hit_sounds_are_never_stretched() {
        let filter =
            audio_filter(Some(1), Some(2), &sync(1.5), None, None, 10.0, (1.0, 1.0)).unwrap();
        assert!(filter.contains("[1:a]atempo"), "{filter}");
        assert!(!filter.contains("[2:a]atempo"), "{filter}");
    }

    #[test]
    fn a_map_with_no_audio_at_all_emits_no_graph() {
        assert!(audio_filter(None, None, &sync(1.0), None, None, 10.0, (1.0, 1.0)).is_none());
    }

    #[test]
    fn hit_sounds_can_stand_alone() {
        let filter = audio_filter(None, Some(1), &sync(1.0), None, None, 10.0, (1.0, 1.0)).unwrap();
        assert_eq!(filter, "[1:a]anull[mix];[mix]apad=whole_dur=10.000[a]");
    }

    #[test]
    fn every_graph_ends_in_silence_measured_to_the_videos_length() {
        for stall in [None, Some(30.0)] {
            let filter =
                audio_filter(Some(1), Some(2), &sync(1.0), stall, None, 10.0, (1.0, 1.0)).unwrap();
            assert!(
                filter.ends_with("apad=whole_dur=10.000[a]"),
                "stall {stall:?}: {filter}"
            );
            assert!(!filter.contains("apad["), "an unbounded pad: {filter}");
        }
    }

    #[test]
    fn a_failed_play_drags_its_audio_down_at_the_stall() {
        let filter = audio_filter(
            Some(1),
            None,
            &sync(1.0),
            Some(12.5),
            None,
            10.0,
            (1.0, 1.0),
        )
        .unwrap();
        assert!(filter.contains("atrim=0:12.500"), "{filter}");

        assert!(filter.contains("asetrate=44100*0.4"), "{filter}");

        assert!(
            filter.find("aresample=44100").unwrap() < filter.find("asetrate").unwrap(),
            "{filter}"
        );
        assert!(filter.contains("afade=t=out"), "{filter}");
        assert!(filter.ends_with("[a]"), "{filter}");
    }

    #[test]
    fn a_play_that_did_not_fail_keeps_its_audio_straight() {
        let filter = audio_filter(Some(1), None, &sync(1.0), None, None, 10.0, (1.0, 1.0)).unwrap();
        assert!(!filter.contains("asetrate"), "{filter}");
    }

    #[test]
    fn a_render_that_starts_before_the_song_holds_the_music_back() {
        let sync = AudioSync::new(-1500.0, 1.0);
        assert_eq!(sync.seek_seconds, 0.0);
        assert!((sync.delay_seconds - 1.5).abs() < 1e-9);
        assert!(sync.filter().unwrap().contains("adelay=1500:all=1"));
    }

    #[test]
    fn a_render_that_starts_inside_the_song_seeks_instead() {
        let sync = AudioSync::new(4000.0, 1.0);
        assert_eq!(sync.seek_seconds, 4.0);
        assert_eq!(sync.delay_seconds, 0.0);
        assert!(sync.filter().is_none(), "nothing to stretch or shift");
    }

    #[test]
    fn the_lead_in_is_measured_in_video_time() {
        let sync = AudioSync::new(-1500.0, 1.5);
        assert!((sync.delay_seconds - 1.0).abs() < 1e-9);
    }

    #[test]
    fn the_music_is_shifted_before_it_is_stretched() {
        let filter = AudioSync::new(-1500.0, 1.5).filter().unwrap();
        assert_eq!(filter, "adelay=1500:all=1,atempo=1.500000");
    }

    #[test]
    fn the_delay_is_not_scaled_when_there_is_no_stretch_to_divide_it() {
        let filter = AudioSync::new(-1500.0, 1.0).filter().unwrap();
        assert_eq!(filter, "adelay=1500:all=1");

        let refused = AudioSync::new(-1500.0, 3.0);
        assert!(refused.filter().unwrap().starts_with("adelay=500:all=1"));
    }
}

fn default_workers() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).max(1))
        .unwrap_or(1)
}

#[cfg(test)]
mod fail_timing {
    use super::*;

    fn settings() -> Settings {
        Settings {
            out: std::path::PathBuf::from("/dev/null"),
            fps: 100.0,
            size: (2, 2),
            from_ms: None,
            to_ms: None,
            ffmpeg: "ffmpeg".into(),
            crf: 20,
            preset: "veryfast".into(),
            music_level: 1.0,
            hitsound_level: 1.0,
            threads: Some(1),
            encoder_threads: Some(1),
            audio: None,
            video: None,
            hitsounds: None,
            events: crate::events::Events::wanted(false),
            slow_at_ms: None,
            slow_focus: None,
        }
    }

    #[test]
    fn the_animation_is_real_time_over_a_frozen_field() {
        let plan = Plan::new((0.0, 2000.0), 1.0, &settings(), Some(1000.0)).expect("a plan");

        let at = |frame: u64| plan.map_time_of(frame, 100.0);
        assert!((at(50) - 500.0).abs() < 1e-9);
        assert!((at(150) - 1500.0).abs() < 1e-9, "straight through the fail");
    }

    #[test]
    fn the_animation_is_room_the_plan_makes_for_itself() {
        let plain = Plan::new((0.0, 2000.0), 1.0, &settings(), None).expect("a plan");
        for rate in [1.0, 1.5] {
            let failed = Plan::new((0.0, 2000.0), rate, &settings(), Some(1000.0)).expect("a plan");
            let base = Plan::new((0.0, 2000.0), rate, &settings(), None).expect("a plan");

            let extra = failed.video_seconds - base.video_seconds;
            let wanted = (fail_tail_ms() - dossier_render::OUTRO_FADE_MS) / 1000.0;
            assert!((extra - wanted).abs() < 1e-9, "at rate {rate}: {extra}");
        }
        assert!(plain.video_seconds > 0.0);
    }

    #[test]
    fn a_finished_play_carries_a_tail_for_its_closing_fade() {
        let settings = settings();
        let plan = Plan::new((0.0, 2000.0), 1.0, &settings, None).expect("a plan");
        let played = (2000.0 - 0.0) / 1000.0;
        assert!(
            (plan.video_seconds - played - dossier_render::OUTRO_FADE_MS / 1000.0).abs() < 1e-9,
            "{}",
            plan.video_seconds
        );
    }

    #[test]
    fn the_tail_covers_every_phase_of_the_ending() {
        assert!(
            (fail_tail_ms() - (dossier_render::FAIL_ANIMATION_MS + dossier_render::FAIL_EMPTY_MS))
                .abs()
                < 1e-9
        );
    }
}

#[cfg(test)]
mod video_tests {
    use super::*;

    fn a_frame(with_alpha: bool) -> Frame {
        Frame::new(4, 2, with_alpha).expect("a frame")
    }

    #[test]
    fn a_frame_over_a_video_carries_an_alpha_plane_and_one_without_does_not() {
        assert_eq!(a_frame(false).yuv.len(), yuv_len(4, 2));
        assert_eq!(a_frame(true).yuv.len(), yuv_len(4, 2) + 4 * 2);
    }

    #[test]
    fn the_alpha_plane_is_what_was_drawn_and_not_what_was_left_behind() {
        let mut frame = a_frame(true);
        frame.pixmap.fill(tiny_skia::Color::TRANSPARENT);
        let mut plane = vec![9u8; 8];
        write_alpha(&frame.pixmap, &mut plane);
        assert!(
            plane.iter().all(|&a| a == 0),
            "an empty frame was not empty"
        );

        frame
            .pixmap
            .fill(tiny_skia::Color::from_rgba8(20, 30, 40, 255));
        write_alpha(&frame.pixmap, &mut plane);
        assert!(
            plane.iter().all(|&a| a == 255),
            "a solid frame was not solid"
        );
    }

    #[test]
    fn the_colours_stay_premultiplied() {
        let mut frame = a_frame(true);
        frame
            .pixmap
            .fill(tiny_skia::Color::from_rgba8(255, 255, 255, 128));
        let pixel = frame.pixmap.pixels()[0];
        assert_eq!(pixel.alpha(), 128);
        assert!(
            pixel.red() <= pixel.alpha(),
            "a premultiplied pixel cannot be brighter than its own alpha"
        );
    }
}
