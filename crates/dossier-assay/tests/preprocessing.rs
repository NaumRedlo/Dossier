use dossier_assay::preprocessing::{
    difficulty_objects, MIN_DELTA_TIME, NORMALISED_DIAMETER, NORMALISED_RADIUS,
};
use dossier_assay::slider_parts;
use dossier_beatmap::Beatmap;
use dossier_replay::{bits, Mods};
use dossier_sim::Timeline;

fn corpus() -> Vec<(String, Beatmap)> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let text = std::fs::read_to_string(dir.join("expected.json")).expect("the corpus");
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid json");
    parsed["maps"]
        .as_array()
        .expect("maps")
        .iter()
        .map(|entry| {
            let id = entry["beatmap_id"].as_u64().expect("an id");
            let path = dir.join("maps").join(format!("{id}.osu"));
            let map = Beatmap::parse(&std::fs::read_to_string(path).expect("read")).expect("parse");
            (
                format!("{} ({id})", entry["title"].as_str().unwrap_or("?")),
                map,
            )
        })
        .collect()
}

fn interesting() -> Vec<(&'static str, Mods)> {
    vec![
        ("NM", Mods::new(0)),
        ("HR", Mods::new(bits::HARD_ROCK)),
        ("EZ", Mods::new(bits::EASY)),
        ("DT", Mods::new(bits::DOUBLE_TIME)),
        ("HT", Mods::new(bits::HALF_TIME)),
    ]
}

#[test]
fn nothing_in_the_corpus_produces_a_figure_that_is_not_a_number() {
    for (title, map) in corpus() {
        for (name, mods) in interesting() {
            for object in difficulty_objects(&map, mods) {
                let at = format!("{title} {name} #{}", object.index);
                for (what, value) in [
                    ("jump", object.jump_distance),
                    ("lazy jump", object.lazy_jump_distance),
                    ("minimum jump", object.minimum_jump_distance),
                    ("travel", object.travel_distance),
                    ("lazy travel", object.lazy_travel_distance),
                    ("delta", object.delta_time),
                    ("travel time", object.travel_time),
                    ("minimum jump time", object.minimum_jump_time),
                ] {
                    assert!(value.is_finite(), "{at}: {what} is {value}");
                    assert!(value >= 0.0, "{at}: {what} is negative ({value})");
                }
                if let Some(angle) = object.angle {
                    assert!(
                        angle.is_finite() && (0.0..=std::f64::consts::PI).contains(&angle),
                        "{at}: angle {angle} is not an angle"
                    );
                }
            }
        }
    }
}

#[test]
fn no_two_objects_are_ever_closer_together_than_the_floor() {
    for (title, map) in corpus() {
        for object in difficulty_objects(&map, Mods::new(0)) {
            assert!(
                object.adjusted_delta_time >= MIN_DELTA_TIME,
                "{title} #{}: {} ms",
                object.index,
                object.adjusted_delta_time
            );
            assert!(object.last_object_end_delta_time >= MIN_DELTA_TIME);
            assert!(object.minimum_jump_time >= MIN_DELTA_TIME);
        }
    }
}

#[test]
fn a_slider_starts_where_the_slider_is() {
    for (_, map) in corpus() {
        let timeline = Timeline::build(&map, Mods::new(0));
        for object in &timeline.objects {
            let parts = slider_parts(&map, object);
            let Some(head) = parts.first() else { continue };
            assert!(
                (head.pos.x - object.pos.x).abs() < 0.001
                    && (head.pos.y - object.pos.y).abs() < 0.001,
                "head at {:?}, slider at {:?}",
                head.pos,
                object.pos
            );
        }
    }
}

#[test]
fn distances_are_measured_against_a_circle_of_one_size_on_every_map() {
    for (title, map) in corpus() {
        let timeline = Timeline::build(&map, Mods::new(0));
        let radius = timeline.difficulty.circle_radius();
        let scaling = NORMALISED_RADIUS / radius;
        let mut checked = 0;

        for object in difficulty_objects(&map, Mods::new(0)) {
            let here = &timeline.objects[object.index];
            let before = &timeline.objects[object.index - 1];
            if here.is_spinner() || before.is_spinner() {
                continue;
            }
            let raw = (here.pos.x - before.pos.x).hypot(here.pos.y - before.pos.y);
            assert!(
                (object.jump_distance - raw * scaling).abs() < 0.001,
                "{title} #{}: {} against {}",
                object.index,
                object.jump_distance,
                raw * scaling
            );
            checked += 1;
        }
        assert!(checked > 100, "{title}: only {checked} jumps to look at");
    }
}

#[test]
fn the_shortest_reading_of_a_jump_off_a_slider_is_the_one_taken() {
    for (title, map) in corpus() {
        for object in difficulty_objects(&map, Mods::new(0)) {
            assert!(
                object.minimum_jump_distance <= object.lazy_jump_distance + 0.001,
                "{title} #{}: minimum {} over lazy {}",
                object.index,
                object.minimum_jump_distance,
                object.lazy_jump_distance
            );
        }
    }
}

#[test]
fn a_slider_followed_lazily_never_travels_further_than_its_path() {
    for (title, map) in corpus() {
        let timeline = Timeline::build(&map, Mods::new(0));
        let radius = timeline.difficulty.circle_radius();
        for object in difficulty_objects(&map, Mods::new(0)) {
            if !object.is_slider {
                assert_eq!(object.lazy_travel_distance, 0.0);
                continue;
            }
            let path = match &timeline.objects[object.index].kind {
                dossier_sim::TimedKind::Slider { path, slides, .. } => {
                    path.length() * f64::from(*slides)
                }
                _ => continue,
            };

            let normalised_path = path * NORMALISED_DIAMETER / (radius * 2.0);
            assert!(
                object.lazy_travel_distance <= normalised_path + 1.0,
                "{title} #{}: lazy {} over path {normalised_path}",
                object.index,
                object.lazy_travel_distance
            );
        }
    }
}
