use dossier_beatmap::Point;
use dossier_sim::{GameState, Judgement, Part, TimedKind, TimedObject};
use tiny_skia::{FillRule, Paint, Pixmap, Rect, Shader, Stroke, Transform};

use crate::layout::Layout;
use crate::skin::{blend, lighten, Skin};
use crate::text::{Align, Label};

mod format;

mod paint;
use paint::rounded_rect;

mod hud;
mod keys;
mod objects;
mod overlay;
mod scoreboard;
use keys::KeyTrack;

const SNAKE_SHARE_OF_APPROACH: f64 = 1.0 / 3.0;

const TICK_FADE_MS: f64 = 150.0;

const TICK_FIRST_LEAD: f64 = 0.66;

const TICK_REPEAT_LEAD_MS: f64 = 200.0;

const HIDDEN_FADE_IN: f64 = 0.4;
const HIDDEN_FADE_OUT: f64 = 0.3;

const HIT_FADE_MS: f64 = 240.0;
const MISS_FADE_MS: f64 = 100.0;

const NUMBER_FADE_MS: f64 = HIT_FADE_MS / 4.0;

const BALL_CORE_SCALE: f32 = 0.34;

const ARROW_SCALE: f32 = 0.52;

const ARROW_FADE_MS: f64 = 120.0;

const ARROW_LOOP_MS: f64 = 300.0;
const ARROW_LOOP_FROM: f32 = 1.3;
const ARROW_STRUCK_TO: f32 = 1.4;

const ARROW_REACH: f64 = 0.12;

const WARNING_MS: f64 = 900.0;

const WARNING_EXIT_MS: f64 = 130.0;

const WARNING_SIZE: f64 = 0.8;

const ARROW_ROUNDING: f32 = 0.22;

const WARNING_ROWS: [f64; 2] = [42.0, 342.0];

const WARNING_REST: f32 = 0.42;
const WARNING_BEAT: f32 = 0.58;

const WARNING_SWELL: f32 = 0.10;

const WARNING_ENTRY_MS: f64 = 150.0;

const SPINNER_RADIUS: f64 = 180.0;
const SPINNER_CORE: f64 = 12.0;
const SPINNER_DOT: f64 = 20.0;

const BOARD_LEFT: f64 = 0.022;

const BOARD_STEP: f64 = 0.067;
const BOARD_TEXT: f64 = 0.0245;

const BOARD_WIDTH: f64 = 0.262;

const BOARD_CARD_FILL: f32 = 0.92;

const BOARD_ROWS: usize = 5;

const BOARD_RADIUS: f32 = 0.30;

const BOARD_DARK_SPLIT: f32 = 0.46;
const BOARD_DARK_LEFT: f32 = 0.78;
const BOARD_DARK_RIGHT: f32 = 0.42;

const BOARD_DARK_LEFT_COVER: f32 = 0.84;
const BOARD_DARK_RIGHT_COVER: f32 = 0.58;

const BOARD_DARK_KNEE: f32 = 0.86;

const BOARD_FACE: f32 = 0.72;

const BOARD_RANK_COLUMN: f32 = 0.62;

const BOARD_GROW: f32 = 0.55;

const BOARD_RANK_LIFT: f32 = 0.35;

const BOARD_RING: f32 = 0.05;
const BOARD_GLOW: f32 = 0.06;

const BOARD_CARD_LIFT: f32 = 0.16;

const BOARD_RIVAL_DIM: f32 = 0.35;

const SPIN_SWAP_MS: f64 = 260.0;


const SPIN_READOUT_SIZE: f64 = 0.026;






const SPINNER_BONUS_STEP: u32 = 1000;

const SPINNER_BONUS_MS: f64 = 800.0;
const SPINNER_BONUS_FROM: f64 = 2.0 * SPIN_SPRITE;
const SPINNER_BONUS_TO: f64 = 1.28 * SPIN_SPRITE;
const SPINNER_BONUS_GLYPH: f64 = 46.0;

const SPIN_BOX: (f64, f64) = (640.0, 480.0);

const SPIN_SPRITE: f64 = 0.625;

const SPIN_LIFT: f64 = 8.0;

const SPIN_TOP: f64 = 29.0;
const SPIN_MIDDLE: f64 = SPIN_TOP + 219.0;
const SPIN_CLEAR_AT: f64 = SPIN_TOP + 115.0;
const SPIN_BONUS_AT: f64 = SPIN_TOP + 299.0;
const SPIN_WORD_AT: f64 = SPIN_TOP + 335.0;

const SPIN_RPM_X: f64 = -87.0;
const SPIN_RPM_Y: f64 = 445.0;
const SPIN_SPM_X: f64 = 80.0;
const SPIN_SPM_Y: f64 = 448.0;
const SPIN_SPM_FACE: f64 = 0.9;
const SPIN_SPM_GLYPH: f64 = 46.0;
const SPIN_RPM_RISE: f64 = 50.0;

const SPIN_BARS: u32 = 10;

const SPIN_SETTLED: f64 = 0.8;
const SPIN_GROW: f64 = 0.2;

const SPIN_APPROACH_WIDE: f64 = 1.86;
const SPIN_APPROACH_GONE: f64 = 0.1;

const SPIN_GLOW_COLOUR: (u8, u8, u8) = (3, 151, 255);

const SPIN_WORD_OUT_MS: f64 = 400.0;
const SPIN_WORD_HUSH_MS: f64 = 300.0;

const SPIN_CLEAR_IN_MS: f64 = 400.0;
const SPIN_CLEAR_OUT_MS: f64 = 50.0;
const SPIN_CLEAR_DROP_MS: f64 = 240.0;
const SPIN_CLEAR_REST_MS: f64 = 160.0;


const SHAKE_MS: f64 = 120.0;

const VERDICT_FADE_IN_MS: f64 = 120.0;
const VERDICT_HOLD_MS: f64 = 500.0;
const VERDICT_FADE_OUT_MS: f64 = 600.0;
const VERDICT_MS: f64 = VERDICT_HOLD_MS + VERDICT_FADE_OUT_MS;

const VERDICT_INK_SHARE: f64 = 0.4;

const VERDICT_WIDTH_SHARE: f64 = 0.5;

const VERDICT_TEXT_SCALE: f64 = 0.75;

const MISS_DRIFT_FROM: f64 = -5.0;
const MISS_DRIFT_BY: f64 = 80.0;

const SECTION_MIN_BREAK_MS: f64 = 2880.0;

const SECTION_PASS_HEALTH: f32 = 0.5;

const SECTION_FADE_FROM_MS: f64 = 1280.0;
const SECTION_FADE_TO_MS: f64 = 1480.0;

const LIGHTING_FADE_IN_MS: f64 = 200.0;
const LIGHTING_HOLD_MS: f64 = 400.0;
const LIGHTING_FADE_OUT_MS: f64 = 1000.0;
const LIGHTING_MS: f64 = LIGHTING_HOLD_MS + LIGHTING_FADE_OUT_MS;
const LIGHTING_GROWTH_MS: f64 = 600.0;
const LIGHTING_FROM: f32 = 0.8;
const LIGHTING_TO: f32 = 1.2;

const AFTERLIFE_MS: f64 = if VERDICT_MS > LIGHTING_MS {
    VERDICT_MS
} else {
    LIGHTING_MS
};

const BREAK_HUD_FADE_MS: f64 = 400.0;

const COMBO_POP_MS: f64 = 300.0;
const COMBO_POP_FROM: f32 = 1.56;
const COMBO_POP_ALPHA: f32 = 0.6;

const COMBO_CATCH_MS: f64 = 160.0;

const COMBO_SMALL_POP_MS: f64 = 100.0;
const COMBO_SMALL_POP_GAIN: f32 = 0.1;
const COMBO_BREAK_PULSE_MS: f64 = 260.0;
const COMBO_BREAK_PULSE_GAIN: f32 = 0.26;

use dossier_sim::DANGER_LEVEL as DANGER_FROM;

const DANGER_MAX: f32 = 0.85;

const DANGER_REACH: f32 = 0.30;

const DANGER_BANDS: usize = 24;

pub const FAIL_ANIMATION_MS: f64 = 2500.0;

const FAIL_CLEAR_OF_RELEASE: f32 = 1.0;

pub const FAIL_EMPTY_MS: f64 = 1000.0;

const FAIL_SQUEEZE: f32 = 0.72;

const FAIL_RELEASE_AT: f32 = 0.80;

const FAIL_FLASH_MS: f64 = 1000.0;

const FAIL_FLASH_ALPHA: f32 = 0.30;

const INTRO_FADE_MS: f64 = 450.0;

pub const OUTRO_FADE_MS: f64 = 700.0;

const ERROR_BAR_UR_SIZE: f64 = 0.020;
const ERROR_BAR_UR_GAP: f32 = 0.35;

const ERROR_BAR_SPAN: f64 = 1.0;

const ERROR_BAR_TICKS: usize = 28;
const SHAKE_WIDTH: f64 = 0.22;
const SHAKE_CYCLES: f64 = 3.0;

const TRAIL_STEP_MS: f64 = 1000.0 / 60.0;
const TRAIL_DISJOINT_MS: f64 = 150.0;

const CURSOR_TURN_MS: f64 = 10_000.0;

const TRAIL_CONTINUOUS_MS: f64 = 500.0;

const TRAIL_INTERVAL_SHARE: f32 = 1.0 / 2.5;

#[derive(Debug, Clone)]
struct Annotation {
    colour: usize,

    number: u32,

    resolved_ms: f64,
    missed: bool,

    verdict: Option<Judgement>,

    head_ms: f64,
    head_missed: bool,

    spawn_ms: f64,
    gone_ms: f64,

    ticks_ms: Vec<f64>,

    shakes_ms: Vec<f64>,

    turns: Option<(Turn, Turn)>,
}

#[derive(Debug, Clone, Copy)]
struct Turn {
    at: Point,

    dir: (f64, f64),
}

pub struct Scene<'a> {
    state: &'a GameState,
    skin: Skin,
    annotations: Vec<Annotation>,

    longest_life_ms: f64,

    combo_changes: Vec<(f64, u32)>,

    hidden: bool,

    signature: Option<Signature>,

    leaderboard: crate::leaderboard::Leaderboard,

    pictures: std::collections::HashMap<std::path::PathBuf, Pixmap>,

    keys: KeyTrack,

    bare: bool,

    backdrop: Option<Pixmap>,
    show: Option<crate::storyboard::Show>,
    over_video: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub focus: Point,
    pub closeness: f64,
}

#[derive(Debug, Clone)]
pub struct Signature {
    pub mods: String,

    pub badges: Vec<String>,

    pub client: String,

    pub version: String,
}

impl<'a> Scene<'a> {
    pub fn new(state: &'a GameState, skin: Skin) -> Self {
        let objects = &state.timeline().objects;
        let window = state.difficulty().hit_window_50();

        let mut annotations = Vec::with_capacity(objects.len());
        let mut colour = 0usize;
        let mut number = 0u32;
        for (index, object) in objects.iter().enumerate() {
            if object.new_combo && index > 0 {
                colour += 1;
                number = 0;
            }
            number += 1;

            let reached = index < state.objects_played();
            let judged = state.judge().filter(|_| reached).and_then(|judge| {
                judge
                    .events_for(index)
                    .find(|e| e.part.counts_for_accuracy())
                    .map(|e| (e.time_ms, e.result == Judgement::Miss))
            });
            let verdict = state.judge().filter(|_| reached).and_then(|judge| {
                judge
                    .events_for(index)
                    .find(|e| e.part.counts_for_accuracy())
                    .map(|e| e.result)
            });
            let (resolved_ms, missed) = match judged {
                Some(pair) => pair,

                None => (object.start_ms + window, false),
            };

            let head = state.judge().filter(|_| reached).and_then(|judge| {
                judge
                    .events_for(index)
                    .find(|e| e.part == Part::SliderHead)
                    .map(|e| (e.time_ms, e.result == Judgement::Miss))
            });
            let (head_ms, head_missed) =
                head.unwrap_or((object.start_ms + window, missed && object.is_slider()));

            let spawn_ms = object.start_ms - state.difficulty().preempt_ms();
            let gone_ms = resolved_ms.max(object.end_ms) + HIT_FADE_MS;

            annotations.push(Annotation {
                colour,
                number,
                resolved_ms,
                missed,
                head_ms,
                head_missed,
                verdict,
                spawn_ms,
                gone_ms,
                ticks_ms: object.tick_times(),
                shakes_ms: state
                    .judge()
                    .map(|judge| {
                        judge
                            .shakes()
                            .iter()
                            .filter(|(at, _)| *at == index)
                            .map(|(_, when)| *when)
                            .collect()
                    })
                    .unwrap_or_default(),
                turns: turns_of(object),
            });
        }

        let longest_life_ms = annotations
            .iter()
            .zip(objects)
            .map(|(a, o)| a.gone_ms - o.start_ms)
            .fold(0.0f64, f64::max)
            + AFTERLIFE_MS;

        let mut combo_changes = Vec::new();
        if let Some(judge) = state.judge() {
            let mut previous = 0u32;
            for event in judge.events() {
                if event.combo_after != previous {
                    combo_changes.push((event.time_ms, event.combo_after));
                    previous = event.combo_after;
                }
            }
        }

        Self {
            state,
            skin,
            annotations,
            longest_life_ms,
            combo_changes,
            hidden: state.mods().contains(dossier_replay::bits::HIDDEN),
            signature: None,
            leaderboard: crate::leaderboard::Leaderboard::default(),
            pictures: std::collections::HashMap::new(),
            keys: KeyTrack::build(state.cursor_track(), state.is_lazer()),
            bare: false,
            backdrop: None,
            show: None,
            over_video: false,
        }
    }

    pub fn signed_by(mut self, replay: &dossier_replay::Replay) -> Self {
        let lazer = replay.lazer_mods();
        let mods = if lazer.is_empty() {
            match replay.mods.to_string() {
                m if m == "NM" => String::new(),
                m => m,
            }
        } else {
            lazer.iter().map(|m| m.acronym.as_str()).collect()
        };
        let badges: Vec<String> = if lazer.is_empty() {
            replay
                .mods
                .acronyms()
                .into_iter()
                .map(str::to_owned)
                .collect()
        } else {
            lazer.iter().map(|m| m.acronym.clone()).collect()
        };
        self.signature = Some(Signature {
            mods,
            badges,
            client: dossier_sim::Ruleset::of_replay(replay).name().to_owned(),
            version: replay.client_version(),
        });
        self
    }

    #[must_use]

    pub fn with_backdrop(mut self, backdrop: Pixmap) -> Self {
        self.backdrop = Some(backdrop);
        self
    }

    #[must_use]
    pub fn with_storyboard(mut self, show: crate::storyboard::Show) -> Self {
        self.show = Some(show);
        self
    }

    #[must_use]
    pub fn over_video(mut self) -> Self {
        self.over_video = true;
        self
    }

    #[must_use]
    pub fn storyboard(&self) -> Option<&crate::storyboard::Show> {
        self.show.as_ref()
    }

    fn draw_storyboard(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout, over: bool) {
        if let Some(show) = &self.show {
            show.draw(pixmap, time_ms, layout, over);
        }
    }

    pub fn bare(mut self) -> Self {
        self.bare = true;
        self
    }

    pub fn with_leaderboard(mut self, board: crate::leaderboard::Leaderboard) -> Self {
        let mut wanted: Vec<std::path::PathBuf> = Vec::new();
        for entry in &board.rivals {
            wanted.extend(entry.avatar.clone());
            wanted.extend(entry.cover.clone());
        }
        wanted.extend(board.avatar.clone());
        wanted.extend(board.cover.clone());
        for path in wanted {
            if self.pictures.contains_key(&path) {
                continue;
            }
            match std::fs::read(&path)
                .ok()
                .and_then(|bytes| Pixmap::decode_png(&bytes).ok())
            {
                Some(picture) => {
                    self.pictures.insert(path, picture);
                }
                None => eprintln!(
                    "dossier: {} could not be read as a PNG — the row will draw without it",
                    path.display()
                ),
            }
        }
        self.leaderboard = board;
        self
    }

    fn combo_step(&self, time_ms: f64) -> Option<(usize, f64, u32, bool)> {
        let index = self
            .combo_changes
            .partition_point(|(at, _)| *at <= time_ms)
            .checked_sub(1)?;
        let (at, to) = self.combo_changes[index];
        Some((index, at, to, to < self.combo_before(index)))
    }

    fn combo_before(&self, index: usize) -> u32 {
        index
            .checked_sub(1)
            .map_or(0, |earlier| self.combo_changes[earlier].1)
    }

    fn combo_shown(&self, time_ms: f64) -> (u32, f64, bool) {
        let Some((index, at, to, broke)) = self.combo_step(time_ms) else {
            return (0, f64::NEG_INFINITY, false);
        };
        if broke {
            return (to, at, true);
        }
        if time_ms - at >= COMBO_CATCH_MS {
            return (to, at + COMBO_CATCH_MS, false);
        }
        (self.combo_before(index), at, false)
    }

    fn combo_ghost(&self, time_ms: f64) -> Option<(u32, f32, f32)> {
        let (_, at, to, broke) = self.combo_step(time_ms)?;
        if broke {
            return None;
        }
        let share = ((time_ms - at) / COMBO_POP_MS) as f32;
        if !(0.0..1.0).contains(&share) {
            return None;
        }
        Some((
            to,
            COMBO_POP_FROM + (1.0 - COMBO_POP_FROM) * share,
            COMBO_POP_ALPHA * (1.0 - share),
        ))
    }

    fn combo_pulse(&self, time_ms: f64) -> f32 {
        let (_, since, broke) = self.combo_shown(time_ms);
        let age = time_ms - since;
        if age < 0.0 {
            return 1.0;
        }
        if broke {
            if age >= COMBO_BREAK_PULSE_MS {
                return 1.0;
            }
            let progress = (age / COMBO_BREAK_PULSE_MS) as f32;
            return 1.0 + COMBO_BREAK_PULSE_GAIN * (1.0 - progress).powf(2.2);
        }
        let half = COMBO_SMALL_POP_MS / 2.0;
        if age < half {
            let share = (age / half) as f32;
            1.0 + COMBO_SMALL_POP_GAIN * share * share
        } else if age < COMBO_SMALL_POP_MS {
            let share = ((age - half) / half) as f32;
            1.0 + COMBO_SMALL_POP_GAIN * (1.0 - share) * (1.0 - share)
        } else {
            1.0
        }
    }

    fn candidates(&self, time_ms: f64) -> std::ops::Range<usize> {
        let objects = &self.state.timeline().objects;
        let preempt = self.state.difficulty().preempt_ms();
        let first = objects.partition_point(|o| o.start_ms < time_ms - self.longest_life_ms);
        let last = objects.partition_point(|o| o.start_ms - preempt <= time_ms);
        first..last
    }

    pub fn skin(&self) -> &Skin {
        &self.skin
    }

    pub fn frame(&self, time_ms: f64, layout: &Layout) -> Pixmap {
        let mut pixmap = Pixmap::new(layout.width, layout.height)
            .expect("a frame with a zero dimension was requested");
        self.draw_into(&mut pixmap, time_ms, layout, None);
        pixmap
    }

    pub fn draw_into(
        &self,
        pixmap: &mut Pixmap,
        time_ms: f64,
        layout: &Layout,
        camera: Option<Camera>,
    ) {
        let focused = camera.map(|c| layout.focused(c.focus, c.closeness));
        let close = focused.as_ref().unwrap_or(layout);

        if let Some(progress) = self.fail_progress(time_ms) {
            let clear = self.fail_clear(progress);
            if clear >= 1.0 {
                self.ground(pixmap);
                return;
            }
            let frozen = self
                .state
                .ending()
                .map_or(time_ms, |end| end.time_ms.min(time_ms));

            let mut field = Pixmap::new(layout.width, layout.height)
                .expect("a frame with a zero dimension was requested");
            let mut overlay = Pixmap::new(layout.width, layout.height)
                .expect("a frame with a zero dimension was requested");

            self.draw_field(&mut field, frozen, layout, layout);
            self.draw_overlay(&mut overlay, frozen, layout);

            let presence = 1.0 - clear;
            self.compose_fail(pixmap, &field, &overlay, progress, presence, layout);
            return;
        }

        let intro = self
            .intro_presence(time_ms)
            .min(self.outro_presence(time_ms));
        if intro < 1.0 {
            let mut frame = Pixmap::new(layout.width, layout.height)
                .expect("a frame with a zero dimension was requested");
            self.draw_play(&mut frame, time_ms, layout, close);
            pixmap.fill(self.skin.background);
            let paint = tiny_skia::PixmapPaint {
                opacity: intro,
                quality: tiny_skia::FilterQuality::Nearest,
                ..Default::default()
            };
            pixmap.draw_pixmap(0, 0, frame.as_ref(), &paint, Transform::identity(), None);
            return;
        }

        match camera {
            Some(camera) if camera.closeness > 0.0 => {
                self.draw_zoomed(
                    pixmap,
                    time_ms,
                    layout,
                    close,
                    1.0 - camera.closeness as f32,
                );
            }
            _ => self.draw_play(pixmap, time_ms, layout, close),
        }
    }

    fn draw_zoomed(
        &self,
        pixmap: &mut Pixmap,
        time_ms: f64,
        layout: &Layout,
        close: &Layout,
        interface: f32,
    ) {
        self.ground(pixmap);
        for index in self.candidates(time_ms) {
            if self.alpha_of(index, time_ms) > 0.0 {
                self.draw_object(pixmap, index, time_ms, close);
            }
        }

        for index in self.candidates(time_ms).rev() {
            self.draw_approach(pixmap, index, time_ms, close);
        }
        self.draw_cursor(pixmap, time_ms, close);
        if interface <= 0.0 {
            return;
        }
        let mut over = Pixmap::new(layout.width, layout.height)
            .expect("a frame with a zero dimension was requested");
        self.draw_verdicts(&mut over, time_ms, layout);
        self.draw_break_warning(&mut over, time_ms, layout);
        self.draw_section(&mut over, time_ms, layout);
        self.draw_overlay(&mut over, time_ms, layout);
        let paint = tiny_skia::PixmapPaint {
            opacity: interface.min(1.0),
            quality: tiny_skia::FilterQuality::Nearest,
            ..Default::default()
        };
        pixmap.draw_pixmap(0, 0, over.as_ref(), &paint, Transform::identity(), None);
    }

    pub(super) fn ground(&self, pixmap: &mut Pixmap) {
        if self.over_video {
            pixmap.fill(tiny_skia::Color::TRANSPARENT);
            return;
        }
        let Some(backdrop) = &self.backdrop else {
            pixmap.fill(self.skin.background);
            return;
        };
        pixmap.fill(self.skin.background);
        pixmap.draw_pixmap(
            0,
            0,
            backdrop.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }

    fn fail_progress(&self, time_ms: f64) -> Option<f32> {
        let end = self.state.ending()?;
        (time_ms > end.time_ms)
            .then(|| (((time_ms - end.time_ms) / FAIL_ANIMATION_MS).clamp(0.0, 1.0)) as f32)
    }

    fn fail_clear(&self, progress: f32) -> f32 {
        if progress <= FAIL_RELEASE_AT {
            return 0.0;
        }
        let released = (progress - FAIL_RELEASE_AT) / (1.0 - FAIL_RELEASE_AT);
        let t = (released / FAIL_CLEAR_OF_RELEASE).clamp(0.0, 1.0);

        1.0 - (1.0 - t).powi(3)
    }

    fn intro_presence(&self, time_ms: f64) -> f32 {
        let (from, _) = self.state.span_ms();
        let t = ((time_ms - from) / INTRO_FADE_MS).clamp(0.0, 1.0) as f32;
        1.0 - (1.0 - t) * (1.0 - t)
    }

    fn outro_presence(&self, time_ms: f64) -> f32 {
        if self.state.ending().is_some() {
            return 1.0;
        }
        let (_, to) = self.state.span_ms();

        let t = (((time_ms - to) / OUTRO_FADE_MS).clamp(0.0, 1.0)) as f32;
        1.0 - t * t
    }

    fn draw_play(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout, close: &Layout) {
        self.ground(pixmap);

        self.draw_storyboard(pixmap, time_ms, layout, false);
        self.draw_field(pixmap, time_ms, layout, close);
        self.draw_storyboard(pixmap, time_ms, layout, true);
        self.draw_overlay(pixmap, time_ms, layout);
    }

    fn draw_field(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout, close: &Layout) {
        self.draw_follow_points(pixmap, time_ms, close);

        self.draw_lighting(pixmap, time_ms, close);

        for index in self.candidates(time_ms).rev() {
            if self.alpha_of(index, time_ms) > 0.0 {
                self.draw_object(pixmap, index, time_ms, close);
            }
        }

        for index in self.candidates(time_ms).rev() {
            self.draw_approach(pixmap, index, time_ms, close);
        }
        self.draw_verdicts(pixmap, time_ms, layout);
        self.draw_break_warning(pixmap, time_ms, layout);
        self.draw_section(pixmap, time_ms, layout);
        self.draw_cursor(pixmap, time_ms, close);
    }

    fn draw_overlay(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout) {
        if self.bare {
            self.draw_danger(pixmap, time_ms, layout);
            return;
        }
        self.draw_hud(pixmap, time_ms, layout);
        self.draw_danger(pixmap, time_ms, layout);
        self.draw_keys(pixmap, time_ms, layout, self.hud_presence(time_ms));
        self.draw_leaderboard(pixmap, time_ms, layout);
        self.draw_signature(pixmap, layout);
    }

    fn cannot_die(&self) -> bool {
        self.state.mods().contains(dossier_replay::bits::NO_FAIL)
    }
}

fn fade(exit: f32) -> f32 {
    let left = 1.0 - exit;
    left * left
}

fn turns_of(object: &TimedObject) -> Option<(Turn, Turn)> {
    let TimedKind::Slider { path, slides, .. } = &object.kind else {
        return None;
    };
    if *slides < 2 {
        return None;
    }
    let points = path.points();
    let first = points.first()?;
    let second = points.get(1)?;
    let last = points.last()?;
    let before = points.get(points.len().checked_sub(2)?)?;

    Some((
        Turn {
            at: *first,
            dir: unit(second.x - first.x, second.y - first.y),
        },
        Turn {
            at: *last,
            dir: unit(before.x - last.x, before.y - last.y),
        },
    ))
}

fn unit(dx: f64, dy: f64) -> (f64, f64) {
    let length = dx.hypot(dy);
    if length < 1e-9 {
        (1.0, 0.0)
    } else {
        (dx / length, dy / length)
    }
}

fn spin_place(x: f64, y: f64) -> dossier_beatmap::Point {
    dossier_beatmap::Point {
        x: dossier_beatmap::Point::CENTRE.x + x,
        y: dossier_beatmap::Point::CENTRE.y - SPIN_BOX.1 / 2.0 - SPIN_LIFT + y,
    }
}
