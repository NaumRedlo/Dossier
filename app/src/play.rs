use dossier_sim::GameState;

#[derive(serde::Serialize)]
pub struct Scene {
    pub from_ms: f64,
    pub to_ms: f64,

    pub starts: Vec<f32>,

    pub parts: Vec<[f64; 2]>,
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

const LEAD_MS: f64 = 1200.0;

fn window_of(starts: &[f32], seconds: f64, from_ms: f64) -> (f64, f64) {
    let window = seconds.max(1.0) * 1000.0;
    let busy = busiest(starts, window);
    ((busy - LEAD_MS).max(from_ms), busy + window)
}

pub fn pieces(starts: &[f32], seconds: f64, from_ms: f64, most: usize) -> Vec<[f64; 2]> {
    let mut left: Vec<f32> = starts.to_vec();
    let mut cut = Vec::new();
    while cut.len() < most && !left.is_empty() {
        let (from, to) = window_of(&left, seconds, from_ms);
        cut.push([from, to]);
        left.retain(|start| {
            let at = f64::from(*start);
            at < from || at > to
        });
    }
    cut.sort_by(|a, b| a[0].total_cmp(&b[0]));
    cut
}

pub fn narrow(scene: &mut Scene, seconds: f64, most: usize) {
    scene.parts = pieces(&scene.starts, seconds, scene.from_ms, most);
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
        parts: Vec::new(),
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

    #[test]
    fn several_pieces_come_from_different_parts_of_the_map() {
        let mut starts = Vec::new();
        for cluster in [10_000.0f32, 60_000.0, 120_000.0] {
            for step in 0..30 {
                starts.push(cluster + step as f32 * 60.0);
            }
        }
        let cut = pieces(&starts, 2.0, 0.0, 3);
        assert_eq!(cut.len(), 3);
        for pair in cut.windows(2) {
            assert!(pair[0][1] <= pair[1][0], "pieces overlap: {pair:?}");
        }
        assert!(cut[0][0] < 12_000.0 && cut[1][0] > 55_000.0 && cut[2][0] > 115_000.0);
    }

    #[test]
    fn a_map_shorter_than_the_asking_gives_what_it_has_and_stops() {
        let starts = [1000.0f32, 1100.0, 1200.0];
        let cut = pieces(&starts, 6.0, 0.0, 4);
        assert_eq!(cut.len(), 1);
        assert!(cut[0][0] <= 1000.0);
    }

    #[test]
    fn a_map_with_no_objects_asks_for_no_pieces() {
        assert!(pieces(&[], 6.0, 0.0, 3).is_empty());
    }
}
