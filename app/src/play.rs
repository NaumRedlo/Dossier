use dossier_sim::GameState;

#[derive(serde::Serialize)]
pub struct Scene {
    pub from_ms: f64,
    pub to_ms: f64,

    pub starts: Vec<f32>,
    pub summary: crate::look::Judged,
}

fn busiest(starts: &[f32], window: f64) -> f64 {
    let Some(first) = starts.first() else {
        return 0.0;
    };
    let mut best = (f64::from(*first), 0usize);
    let mut low = 0;
    for high in 0..starts.len() {
        while f64::from(starts[high] - starts[low]) > window {
            low += 1;
        }
        if high - low + 1 > best.1 {
            best = (f64::from(starts[low]), high - low + 1);
        }
    }
    best.0
}

fn window_of(starts: &[f32], seconds: f64, from_ms: f64) -> (f64, f64) {
    let window = seconds.max(1.0) * 1000.0;
    let busy = busiest(starts, window);
    ((busy - 1200.0).max(from_ms), busy + window)
}

pub fn narrow(scene: &mut Scene, seconds: f64) {
    let (from, to) = window_of(&scene.starts, seconds, scene.from_ms);

    scene
        .starts
        .retain(|start| f64::from(*start) >= from && f64::from(*start) <= to);
    scene
        .summary
        .marks
        .retain(|mark| mark.ms >= from && mark.ms <= to);
    scene.from_ms = from;
    scene.to_ms = to;
}

pub fn read(
    beatmap: &dossier_beatmap::Beatmap,
    replay: &dossier_replay::Replay,
    state: &GameState,
) -> Result<Scene, String> {
    let summary = crate::look::summarise(beatmap, replay, state)?;
    let timeline = state.timeline();

    let first = timeline.objects.first().map_or(0.0, |o| o.start_ms);
    let from_ms = (first - timeline.difficulty.preempt_ms()).max(0.0);
    let to_ms = timeline
        .objects
        .last()
        .map_or(from_ms, |o| o.end_ms + 1200.0);

    Ok(Scene {
        from_ms,
        to_ms,
        starts: timeline
            .objects
            .iter()
            .map(|object| object.start_ms as f32)
            .collect(),
        summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_busiest_window_is_found_rather_than_the_first_one() {
        let starts = [
            0.0, 2000.0, 4000.0, 6000.0, 20_000.0, 20_200.0, 20_400.0, 20_600.0, 20_800.0,
        ];
        assert_eq!(busiest(&starts, 3000.0), 20_000.0);
    }

    #[test]
    fn a_map_with_nothing_in_it_still_answers() {
        assert_eq!(busiest(&[], 3000.0), 0.0);
    }

    #[test]
    fn the_window_opens_a_beat_before_the_busiest_moment_and_lasts_as_long_as_asked() {
        let starts = [0.0, 100.0, 20_000.0, 20_500.0, 21_000.0];
        assert_eq!(window_of(&starts, 2.0, 0.0), (18_800.0, 22_000.0));
    }

    #[test]
    fn the_window_never_opens_before_the_map_does() {
        let starts = [500.0, 600.0, 700.0];
        let (from, _) = window_of(&starts, 2.0, 400.0);
        assert_eq!(from, 400.0);
    }
}
