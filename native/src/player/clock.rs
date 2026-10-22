use std::time::{Duration, Instant};

const HEARD_FOR: f64 = 150.0;
const SNAP: f64 = 200.0;
const SETTLE: Duration = Duration::from_millis(700);
const FOLLOW: f64 = 0.02;
const SLEW: f64 = 0.08;

#[derive(Debug, Clone, Copy, Default)]
pub struct Pace {
    from_ms: f64,
    rate: f64,
    frame_ms: f64,
    began: Option<Instant>,
    paused_at: Option<Instant>,
    resumed_at: Option<Instant>,
    held: Duration,
    pub(super) offset: f64,
    synced: bool,
    far_since: Option<Instant>,
    pub(super) bias: f64,
    last: Option<Instant>,
    gap_ms: f64,
    tick_ms: f64,
}

impl Pace {
    pub fn new(from_ms: i64, rate: f32, shown_fps: f64, held: bool, now: Instant) -> Pace {
        Pace { from_ms: from_ms as f64, rate: rate as f64, frame_ms: 1000.0 / shown_fps.max(1.0), paused_at: held.then_some(now), ..Pace::default() }
    }

    pub fn pause(&mut self, now: Instant) {
        if self.paused_at.is_none() {
            self.paused_at = Some(now);
        }
    }

    pub fn resume(&mut self, now: Instant) {
        if let Some(since) = self.paused_at.take() {
            if self.began.is_some_and(|began| since >= began) {
                self.held += now.saturating_duration_since(since);
            }
            self.resumed_at = Some(now);
            self.last = None;
        }
    }

    fn wall(&self, now: Instant) -> Option<f64> {
        let began = self.began?;
        let now = self.paused_at.map_or(now, |paused| paused.max(began));
        Some(self.from_ms + now.saturating_duration_since(began).saturating_sub(self.held).as_secs_f64() * 1000.0 * self.rate)
    }

    pub fn at(&mut self, now: Instant, heard: Option<(f64, Instant)>, sound: bool, ready: bool) -> Option<f64> {
        if self.paused_at.is_some() {
            return self.wall(now).map(|wall| wall + self.offset + self.bias);
        }
        if self.began.is_none() {
            if !ready || (sound && heard.is_none()) {
                return None;
            }
            self.began = Some(now);
        }
        if let Some(before) = self.last.replace(now) {
            let gap = now.saturating_duration_since(before).as_secs_f64() * 1000.0;
            if gap > 0.5 && gap < 50.0 {
                if (gap - self.gap_ms).abs() < 1.5 {
                    self.tick_ms = (gap + self.gap_ms) / 2.0;
                }
                self.gap_ms = gap;
            }
        }
        let wall = self.wall(now)?;
        if let Some((at, stamp)) = heard.filter(|(_, stamp)| self.resumed_at.is_none_or(|resumed| *stamp >= resumed)) {
            let since = now.saturating_duration_since(stamp).as_secs_f64() * 1000.0 * self.rate;
            if since <= HEARD_FOR {
                let off = at + since - (wall + self.offset);
                if !self.synced {
                    self.offset += off;
                    self.synced = true;
                } else if off.abs() <= SNAP {
                    self.far_since = None;
                    self.offset += (off * FOLLOW).clamp(-SLEW, SLEW);
                } else if now.saturating_duration_since(*self.far_since.get_or_insert(now)) >= SETTLE {
                    self.offset += off;
                    self.far_since = None;
                }
            }
        }
        Some(wall + self.offset + self.bias)
    }

    pub fn shown(&mut self, at: f64, frame_at: f64) {
        let tick = self.tick_ms;
        let shown_for = self.frame_ms / self.rate.max(0.01);
        if tick <= 0.0 || tick > shown_for * 1.03 {
            return;
        }
        let ticks = shown_for / tick;
        if (ticks - ticks.round()).abs() > 0.03 {
            return;
        }
        let late = (at - frame_at) / self.rate.max(0.01);
        if late < tick * 0.25 || late > tick * 0.75 {
            self.bias = (self.bias + (tick * 0.5 - late) * self.rate).clamp(-tick * self.rate, tick * self.rate);
        }
    }
}

