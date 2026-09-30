use dossier_sim::{GameState, Part};

const FRAME_MS: f64 = 1000.0 / 60.0;
const RISE_FRAMES: f64 = 4.0;
const FALL_FRAMES: f64 = 6.0;
const STEP_MS: f64 = 5.0;
const FILL_LONGEST_MS: f64 = 10_000.0;
const FILL_LEAD_MS: f64 = 1_800.0;
const TAIL_MS: f64 = 5_000.0;
const BULGE_MS: f64 = 150.0;
const BULGE_FROM: f32 = 1.2;
const BULGE_TO: f32 = 0.8;
const BURST_MS: f64 = 120.0;
const BURST_ABOVE: f32 = 0.9;
const SHOW_AFTER_MS: f64 = 800.0;

#[derive(Debug)]
pub(super) struct HealthShow {
    from_ms: f64,
    shown: Vec<f32>,
    filling: (f64, f64),
    bulges: Vec<f64>,
    bursts: Vec<f64>,
    shows: Vec<f64>,
    rate: f64,
}

impl HealthShow {
    pub(super) fn of(state: &GameState) -> Option<Self> {
        let objects = &state.timeline().objects;
        let first = objects.first()?.start_ms;
        state.health_at(first)?;
        let last = objects
            .iter()
            .map(|object| object.end_ms)
            .fold(first, f64::max);
        let rate = state.playback_rate().max(0.01);

        let lead = (FILL_LEAD_MS.max(state.difficulty().preempt_ms()) - first).max(0.0);
        let from_ms = if first >= FILL_LONGEST_MS {
            first - FILL_LONGEST_MS
        } else {
            -lead
        };
        let until = last + TAIL_MS;
        let (shown, filled) = followed(
            |time_ms| f64::from(state.health_at(time_ms).unwrap_or(1.0)),
            (from_ms, first, until),
            rate,
        );

        let (mut bulges, mut bursts) = (Vec::new(), Vec::new());
        if let Some(judge) = state.judge() {
            for event in judge.events() {
                if event.result.is_miss() || event.part == Part::SliderTick {
                    continue;
                }
                bulges.push(event.time_ms);
                if state.health_at(event.time_ms).unwrap_or(0.0) > BURST_ABOVE {
                    bursts.push(event.time_ms);
                }
            }
        }
        bulges.sort_by(f64::total_cmp);
        bursts.sort_by(f64::total_cmp);
        let shows = state
            .timeline()
            .breaks
            .iter()
            .filter(|(from, to)| to - from >= SHOW_AFTER_MS)
            .map(|&(_, to)| to)
            .collect();

        Some(Self {
            from_ms,
            shown,
            filling: (from_ms, filled),
            bulges,
            bursts,
            shows,
            rate,
        })
    }

    pub(super) fn shown_at(&self, time_ms: f64) -> f32 {
        let at = (time_ms - self.from_ms) / STEP_MS;
        if at <= 0.0 {
            return 0.0;
        }
        let below = at.floor() as usize;
        let Some(&low) = self.shown.get(below) else {
            return self.shown.last().copied().unwrap_or(1.0);
        };
        let high = self.shown.get(below + 1).copied().unwrap_or(low);
        low + (high - low) * (at - at.floor()) as f32
    }

    pub(super) fn marker_scale(&self, time_ms: f64) -> f32 {
        let pulse = BULGE_MS * self.rate;
        let (fill_from, fill_to) = self.filling;
        if time_ms >= fill_from && time_ms < fill_to {
            let phase = ((time_ms - fill_from) % pulse) / pulse;
            return BULGE_FROM + (BULGE_TO - BULGE_FROM) * phase as f32;
        }
        let last = |times: &[f64]| {
            let at = times.partition_point(|when| *when <= time_ms);
            at.checked_sub(1).map(|at| times[at])
        };
        let settled = last(&self.bulges).map_or(fill_to, |bulge| bulge.max(fill_to));
        if last(&self.shows).is_some_and(|show| show > settled) {
            return 1.0;
        }
        match last(&self.bulges) {
            Some(bulge) if time_ms - bulge < pulse => {
                let share = ((time_ms - bulge) / pulse) as f32;
                BULGE_FROM + (BULGE_TO - BULGE_FROM) * share
            }
            _ if time_ms < fill_from => 1.0,
            _ => BULGE_TO,
        }
    }

    pub(super) fn burst_at(&self, time_ms: f64) -> Option<f32> {
        let at = self.bursts.partition_point(|when| *when <= time_ms);
        let burst = self.bursts[..at].last()?;
        let share = (time_ms - burst) / (BURST_MS * self.rate);
        (share < 1.0).then_some(share as f32)
    }
}

fn followed(
    real: impl Fn(f64) -> f64,
    (from_ms, first, until): (f64, f64, f64),
    rate: f64,
) -> (Vec<f32>, f64) {
    let steps = ((until - from_ms) / STEP_MS).ceil().max(1.0) as usize;
    let mut shown = Vec::with_capacity(steps + 1);
    let (mut value, mut filling, mut filled) = (0.0f64, true, first);
    for step in 0..=steps {
        let time_ms = from_ms + step as f64 * STEP_MS;
        let real = real(time_ms);
        if filling {
            let fill = ((time_ms - from_ms) / (first - from_ms).max(1.0)).clamp(0.0, 1.0);
            if fill >= real {
                filling = false;
                filled = time_ms;
                value = real;
            } else {
                value = fill;
            }
        } else {
            let frames = if real > value { RISE_FRAMES } else { FALL_FRAMES };
            value = real + (value - real) * (-STEP_MS / (frames * FRAME_MS * rate)).exp();
        }
        shown.push(value as f32);
    }
    (shown, filled)
}

pub(super) fn tint_for(health: f32) -> tiny_skia::Color {
    let grey = |level: f32| tiny_skia::Color::from_rgba(level, level, level, 1.0);
    if health < 0.2 {
        let share = ((0.2 - health) / 0.2).clamp(0.0, 1.0);
        return tiny_skia::Color::from_rgba(share, 0.0, 0.0, 1.0).unwrap_or(tiny_skia::Color::BLACK);
    }
    if health < 0.5 {
        return grey(1.0 - (0.5 - health) / 0.5).unwrap_or(tiny_skia::Color::WHITE);
    }
    tiny_skia::Color::WHITE
}

#[cfg(test)]
mod tests {
    use super::{followed, tint_for, STEP_MS};

    fn at(shown: &[f32], from_ms: f64, time_ms: f64) -> f32 {
        shown[((time_ms - from_ms) / STEP_MS).round() as usize]
    }

    #[test]
    fn the_bar_fills_from_empty_to_full_by_the_first_note() {
        let (shown, filled) = followed(|_| 1.0, (-1_000.0, 3_000.0, 6_000.0), 1.0);
        assert_eq!(at(&shown, -1_000.0, -1_000.0), 0.0);
        assert!((at(&shown, -1_000.0, 1_000.0) - 0.5).abs() < 0.01);
        assert!((filled - 3_000.0).abs() <= STEP_MS);
        assert_eq!(at(&shown, -1_000.0, 4_000.0), 1.0);
    }

    #[test]
    fn a_loss_slides_down_slower_than_a_gain_climbs_back() {
        let real = |time_ms: f64| {
            if (2_000.0..3_000.0).contains(&time_ms) {
                0.5
            } else {
                1.0
            }
        };
        let (shown, _) = followed(real, (0.0, 1_000.0, 4_000.0), 1.0);
        let fell = at(&shown, 0.0, 2_100.0);
        assert!(fell < 0.99 && fell > 0.51, "a hundred after the loss it showed {fell}");
        let rose = at(&shown, 0.0, 3_100.0);
        assert!(rose > 0.51 && rose < 0.99, "a hundred after the gain it showed {rose}");
        assert!(1.0 - rose < fell - 0.5, "the climb ({rose}) was no quicker than the fall ({fell})");
        assert!(at(&shown, 0.0, 2_900.0) < 0.51, "and the fall does arrive");
    }

    #[test]
    fn the_slide_keeps_its_pace_under_a_rate_mod() {
        let real = |time_ms: f64| if time_ms >= 2_000.0 { 0.5 } else { 1.0 };
        let (plain, _) = followed(real, (0.0, 1_000.0, 3_000.0), 1.0);
        let (fast, _) = followed(real, (0.0, 1_000.0, 3_000.0), 1.5);
        assert!((at(&plain, 0.0, 2_100.0) - at(&fast, 0.0, 2_150.0)).abs() < 0.01);
    }

    #[test]
    fn a_full_bar_is_white_and_a_dying_one_goes_from_black_to_red() {
        assert_eq!(tint_for(0.8), tiny_skia::Color::WHITE);
        let half_way = tint_for(0.35);
        assert!((half_way.red() - 0.7).abs() < 1e-6 && half_way.red() == half_way.blue());
        let edge = tint_for(0.2);
        assert!((edge.red() - 0.4).abs() < 1e-6, "grey on the upper side of a fifth");
        let dying = tint_for(0.1);
        assert!((dying.red() - 0.5).abs() < 1e-6 && dying.green() == 0.0);
        assert_eq!(tint_for(0.0).red(), 1.0);
    }
}
