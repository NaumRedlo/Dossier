use dossier_audio::{Kit, SamplePack, SampleSet, Track, Voice};
use dossier_beatmap::{sound_bits, Beatmap, HitObject, SampleSet as MapSet};
use dossier_sim::{GameState, Part};

const COMBO_BREAK_THRESHOLD: u32 = 20;

pub fn build(
    state: &GameState,
    beatmap: &Beatmap,
    at_video: impl Fn(f64) -> f64,
    video_seconds: f64,
    kit: Kit,
    pack: SamplePack,
    layering: bool,
) -> Track {
    let mut track = Track::new(video_seconds, kit).with_samples(pack);
    let Some(judge) = state.judge() else {
        return track;
    };

    let mut combo_before = 0u32;
    for event in judge.events() {
        let run = combo_before;
        combo_before = event.combo_after;

        if event.result.is_miss() {
            if event.part.breaks_combo() && run >= COMBO_BREAK_THRESHOLD {
                track.strike_with(Voice::Miss, at_video(event.time_ms), SampleSet::Normal, 1.0);
            }
            continue;
        }
        let Some(object) = beatmap.objects.get(event.object_index) else {
            continue;
        };

        let edge = slider_edge(state, event.object_index, event.part, event.time_ms);
        let balance = balance_of(object.pos.x as f32);
        for voice in voices_for(event.part, object, edge, layering) {
            let (set, index, volume) = bank_for(beatmap, object, voice, edge);
            track.strike_panned(
                voice,
                at_video(event.time_ms),
                set,
                index,
                volume * voice_level(voice),
                balance,
            );
        }
    }
    sustained(state, beatmap, &at_video, &mut track);
    track
}

fn sustained(
    state: &GameState,
    beatmap: &Beatmap,
    at_video: &impl Fn(f64) -> f64,
    track: &mut Track,
) {
    let Some(judge) = state.judge() else {
        return;
    };

    let mut played = vec![false; beatmap.objects.len()];
    for event in judge.events() {
        if !event.result.is_miss() {
            if let Some(slot) = played.get_mut(event.object_index) {
                *slot = true;
            }
        }
    }

    for (index, object) in beatmap.objects.iter().enumerate() {
        if !played.get(index).copied().unwrap_or(false) {
            continue;
        }
        let Some(timed) = state.timeline().objects.get(index) else {
            continue;
        };
        let span = (at_video(timed.start_ms), at_video(timed.end_ms));

        match &object.kind {
            dossier_beatmap::ObjectKind::Slider(_) => {
                let (set, bank, volume) = bank_for(beatmap, object, Voice::Normal, None);
                let level = voice_level(Voice::Slide);
                track.sustain(Voice::Slide, span, set, bank, volume * level, |_| 1.0);
                if object.hit_sound & sound_bits::WHISTLE != 0 {
                    let (set, bank, volume) = bank_for(beatmap, object, Voice::Whistle, None);
                    let level = voice_level(Voice::SlideWhistle);
                    track.sustain(Voice::SlideWhistle, span, set, bank, volume * level, |_| {
                        1.0
                    });
                }
            }
            dossier_beatmap::ObjectKind::Spinner { .. } => {
                let needed = dossier_sim::required_spins(state.difficulty(), timed.duration_ms());
                let (set, bank, volume) = bank_for(beatmap, object, Voice::Normal, None);
                let held = timed.end_ms - timed.start_ms;
                track.sustain(Voice::Spin, span, set, bank, volume, |seconds| {
                    if needed <= 0.0 {
                        return SPIN_BASE_RATE;
                    }

                    let at = timed.start_ms + f64::from(seconds as f32) * 1000.0;
                    let turned = dossier_sim::spinner_rotations(
                        state.cursor_track(),
                        timed.start_ms,
                        at.min(timed.start_ms + held),
                    );
                    let progress = (turned / needed) as f32;
                    (SPIN_BASE_RATE + progress * SPIN_RATE_RATIO).min(SPIN_MAX_RATE)
                });
            }
            _ => {}
        }
    }
}

const SPIN_BASE_RATE: f32 = 20_000.0 / 44_100.0;
const SPIN_RATE_RATIO: f32 = 40_000.0 / 44_100.0;
const SPIN_MAX_RATE: f32 = 100_000.0 / 44_100.0;

fn slider_edge(state: &GameState, index: usize, part: Part, time_ms: f64) -> Option<usize> {
    if !matches!(
        part,
        Part::SliderHead | Part::SliderRepeat | Part::SliderTail
    ) {
        return None;
    }
    let object = state.timeline().objects.get(index)?;
    let duration = object.slide_duration_ms()?;
    if duration <= 0.0 {
        return Some(0);
    }
    Some((((time_ms - object.start_ms) / duration).round().max(0.0)) as usize)
}

fn bits_for(object: &HitObject, edge: Option<usize>) -> u8 {
    match (&object.kind, edge) {
        (dossier_beatmap::ObjectKind::Slider(slider), Some(edge)) => slider
            .edge_sounds
            .get(edge)
            .copied()
            .unwrap_or(object.hit_sound),
        _ => object.hit_sound,
    }
}

fn voices_for(part: Part, object: &HitObject, edge: Option<usize>, layering: bool) -> Vec<Voice> {
    match part {
        Part::Slider | Part::Spinner => Vec::new(),

        Part::SpinnerSpin | Part::SpinnerPoints => Vec::new(),
        Part::SpinnerBonus => vec![Voice::Bonus],
        Part::SliderTick => vec![Voice::Tick],
        _ => layered(bits_for(object, edge), layering),
    }
}

fn bank_for(
    beatmap: &Beatmap,
    object: &HitObject,
    voice: Voice,
    edge: Option<usize>,
) -> (SampleSet, u32, f32) {
    let point = beatmap.timing.sample_point_at(object.time_ms);

    let inherited = point
        .filter(|p| p.set_given)
        .map_or(beatmap.sample_set, |p| p.set);

    let sample = object.hit_sample;

    let (edge_normal, edge_addition) = match (&object.kind, edge) {
        (dossier_beatmap::ObjectKind::Slider(slider), Some(edge)) => slider
            .edge_sets
            .get(edge)
            .copied()
            .unwrap_or((sample.normal_set, sample.addition_set)),
        _ => (sample.normal_set, sample.addition_set),
    };
    let sample = dossier_beatmap::HitSample {
        normal_set: if edge_normal != 0 {
            edge_normal
        } else {
            sample.normal_set
        },
        addition_set: if edge_addition != 0 {
            edge_addition
        } else {
            sample.addition_set
        },
        ..sample
    };
    let code = match voice {
        Voice::Normal | Voice::Tick => sample.normal_set,

        _ => {
            if sample.addition_set != 0 {
                sample.addition_set
            } else {
                sample.normal_set
            }
        }
    };
    let set = if code == 0 {
        inherited
    } else {
        MapSet::from_code(code)
    };

    let volume = if sample.volume > 0 {
        f32::from(sample.volume)
    } else {
        point.map_or(100.0, |p| f32::from(p.volume))
    } / 100.0;

    let index = if sample.index > 0 {
        sample.index
    } else {
        point.map_or(1, |p| p.index)
    };

    (convert(set), index.max(1), volume)
}

fn convert(set: MapSet) -> SampleSet {
    match set {
        MapSet::Normal => SampleSet::Normal,
        MapSet::Soft => SampleSet::Soft,
        MapSet::Drum => SampleSet::Drum,
    }
}

fn voice_level(voice: Voice) -> f32 {
    match voice {
        Voice::Normal | Voice::Tick | Voice::Slide => 0.8,
        Voice::Whistle | Voice::Clap | Voice::SlideWhistle => 0.85,
        _ => 1.0,
    }
}

fn balance_of(x: f32) -> f32 {
    const FIELD: f32 = 512.0;
    ((x - FIELD / 2.0) / FIELD).clamp(-1.0, 1.0)
}

fn layered(bits: u8, layering: bool) -> Vec<Voice> {
    let is_layered = bits != 0 && bits & sound_bits::NORMAL == 0;
    let mut voices = Vec::new();
    if layering || !is_layered {
        voices.push(Voice::Normal);
    }
    for (bit, voice) in [
        (sound_bits::FINISH, Voice::Finish),
        (sound_bits::WHISTLE, Voice::Whistle),
        (sound_bits::CLAP, Voice::Clap),
    ] {
        if bits & bit != 0 {
            voices.push(voice);
        }
    }
    voices
}

pub fn audition(kit: Kit, pack: SamplePack) -> Track {
    let mut track = Track::new(6.0, kit).with_samples(pack);
    let mut at = 0.3;

    for voice in [
        Voice::Normal,
        Voice::Whistle,
        Voice::Finish,
        Voice::Clap,
        Voice::Tick,
    ] {
        for _ in 0..3 {
            track.strike(voice, at);
            at += 0.28;
        }
        at += 0.35;
    }

    at += 0.3;
    for i in 0..24 {
        let voice = match i % 8 {
            0 => Voice::Finish,
            4 => Voice::Clap,
            2 | 6 => Voice::Whistle,
            _ => Voice::Normal,
        };
        track.strike(voice, at + f64::from(i) * 0.0833);
    }
    track
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_undecorated_note_makes_the_plain_sound() {
        assert_eq!(layered(0, true), vec![Voice::Normal]);
        assert_eq!(layered(sound_bits::NORMAL, true), vec![Voice::Normal]);
    }

    #[test]
    fn a_decoration_goes_over_the_plain_hit_rather_than_instead_of_it() {
        assert_eq!(
            layered(sound_bits::WHISTLE, true),
            vec![Voice::Normal, Voice::Whistle]
        );
        assert_eq!(
            layered(sound_bits::FINISH, true),
            vec![Voice::Normal, Voice::Finish]
        );
        assert_eq!(
            layered(sound_bits::CLAP, true),
            vec![Voice::Normal, Voice::Clap]
        );
    }

    #[test]
    fn a_skin_can_ask_for_the_decoration_on_its_own() {
        assert_eq!(layered(sound_bits::WHISTLE, false), vec![Voice::Whistle]);
        assert_eq!(
            layered(sound_bits::FINISH | sound_bits::CLAP, false),
            vec![Voice::Finish, Voice::Clap]
        );
    }

    #[test]
    fn a_note_that_asks_for_the_plain_hit_outright_keeps_it_either_way() {
        assert_eq!(
            layered(sound_bits::NORMAL | sound_bits::WHISTLE, false),
            vec![Voice::Normal, Voice::Whistle]
        );

        assert_eq!(layered(0, false), vec![Voice::Normal]);
    }

    #[test]
    fn a_note_that_carries_three_sounds_makes_all_three() {
        let all = sound_bits::WHISTLE | sound_bits::FINISH | sound_bits::CLAP;
        assert_eq!(
            layered(all, true),
            vec![Voice::Normal, Voice::Finish, Voice::Whistle, Voice::Clap]
        );
    }
}

#[cfg(test)]
mod banks {
    use super::*;
    fn map(timing_points: &str, object: &str) -> Beatmap {
        Beatmap::parse(&format!(
            "osu file format v14\n\n[TimingPoints]\n{timing_points}\n\n[HitObjects]\n{object}\n"
        ))
        .expect("test map should parse")
    }

    const SOFT_AT_HALF: &str = "0,500,4,2,0,50,1,0";
    const DRUM_LOUD: &str = "0,500,4,3,0,100,1,0";

    #[test]
    fn a_note_that_says_nothing_takes_the_timing_points_bank_and_volume() {
        let beatmap = map(SOFT_AT_HALF, "100,100,1000,1,0");
        let (set, _, volume) = bank_for(&beatmap, &beatmap.objects[0], Voice::Normal, None);
        assert_eq!(set, SampleSet::Soft);
        assert!((volume - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_note_takes_the_timing_points_custom_bank() {
        let beatmap = map("0,500,4,2,3,50,1,0", "100,100,1000,1,0");
        let (_, index, _) = bank_for(&beatmap, &beatmap.objects[0], Voice::Normal, None);
        assert_eq!(index, 3);
    }

    #[test]
    fn a_notes_own_index_overrules_the_timing_points() {
        let beatmap = map("0,500,4,2,3,50,1,0", "100,100,1000,1,0,0:0:5:0:");
        let (_, index, _) = bank_for(&beatmap, &beatmap.objects[0], Voice::Normal, None);
        assert_eq!(index, 5);
    }

    #[test]
    fn saying_nothing_anywhere_means_the_skins_first_bank() {
        let beatmap = map("0,500,4,2,0,50,1,0", "100,100,1000,1,0");
        let (_, index, _) = bank_for(&beatmap, &beatmap.objects[0], Voice::Normal, None);
        assert_eq!(index, 1);
    }

    #[test]
    fn a_notes_own_bank_overrules_the_timing_point() {
        let beatmap = map(SOFT_AT_HALF, "100,100,1000,1,0,3:0:0:0:");
        let (set, _, _) = bank_for(&beatmap, &beatmap.objects[0], Voice::Normal, None);
        assert_eq!(set, SampleSet::Drum);
    }

    #[test]
    fn decorations_have_a_bank_of_their_own() {
        let beatmap = map(DRUM_LOUD, "100,100,1000,1,8,3:2:0:0:");
        let object = &beatmap.objects[0];
        assert_eq!(
            bank_for(&beatmap, object, Voice::Normal, None).0,
            SampleSet::Drum
        );
        assert_eq!(
            bank_for(&beatmap, object, Voice::Clap, None).0,
            SampleSet::Soft
        );
    }

    #[test]
    fn a_decoration_with_no_bank_of_its_own_follows_the_plain_hit() {
        let beatmap = map(SOFT_AT_HALF, "100,100,1000,1,8,3:0:0:0:");
        let object = &beatmap.objects[0];
        assert_eq!(
            bank_for(&beatmap, object, Voice::Clap, None).0,
            SampleSet::Drum
        );
    }

    #[test]
    fn a_green_line_at_no_volume_is_silence_not_a_whisper() {
        let beatmap = map("0,500,4,2,0,0,1,0", "100,100,1000,1,0");
        let (_, _, volume) = bank_for(&beatmap, &beatmap.objects[0], Voice::Normal, None);
        assert_eq!(volume, 0.0, "a zero-volume timing point still sounded");
    }

    #[test]
    fn a_notes_own_volume_overrules_the_timing_points() {
        let beatmap = map(SOFT_AT_HALF, "100,100,1000,1,0,0:0:0:20:");
        let (_, _, volume) = bank_for(&beatmap, &beatmap.objects[0], Voice::Normal, None);
        assert!((volume - 0.2).abs() < 1e-6, "got {volume}");
    }

    #[test]
    fn the_bank_follows_the_section_a_note_falls_in() {
        let beatmap = map(
            "0,500,4,2,0,50,1,0\n5000,-100,4,3,0,90,0,0",
            "100,100,1000,1,0\n200,200,6000,1,0",
        );
        assert_eq!(
            bank_for(&beatmap, &beatmap.objects[0], Voice::Normal, None).0,
            SampleSet::Soft
        );
        assert_eq!(
            bank_for(&beatmap, &beatmap.objects[1], Voice::Normal, None).0,
            SampleSet::Drum,
            "a green line carries sound settings too"
        );
    }

    #[test]
    fn a_note_with_no_sample_field_at_all_still_resolves() {
        let beatmap = map(DRUM_LOUD, "100,100,1000,1,0");
        let (set, _, volume) = bank_for(&beatmap, &beatmap.objects[0], Voice::Normal, None);
        assert_eq!(set, SampleSet::Drum);
        assert!((volume - 1.0).abs() < 1e-6);
    }
}

#[cfg(test)]
mod edge_tests {
    use super::*;
    use dossier_beatmap::ObjectKind;

    fn slider_map(edge_sounds: &str, edge_sets: &str) -> Beatmap {
        let extra = if edge_sets.is_empty() {
            format!(",{edge_sounds}")
        } else {
            format!(",{edge_sounds},{edge_sets}")
        };
        Beatmap::parse(&format!(
            "osu file format v14\n\n\
             [TimingPoints]\n0,500,4,1,0,100,1,0\n\n\
             [HitObjects]\n0,0,1000,2,0,L|100:0,2,100{extra}\n"
        ))
        .expect("test map should parse")
    }

    fn edges_of(map: &Beatmap) -> (Vec<u8>, Vec<(u8, u8)>) {
        match &map.objects[0].kind {
            ObjectKind::Slider(s) => (s.edge_sounds.clone(), s.edge_sets.clone()),
            _ => panic!("that was a slider"),
        }
    }

    #[test]
    fn a_slider_carries_a_sound_for_every_edge() {
        let map = slider_map("0|8|2", "0:0|1:2|0:0");
        let (sounds, sets) = edges_of(&map);
        assert_eq!(sounds, vec![0, 8, 2]);
        assert_eq!(sets, vec![(0, 0), (1, 2), (0, 0)]);
    }

    #[test]
    fn each_edge_makes_its_own_sound() {
        let map = slider_map("0|4|2", "");
        let object = &map.objects[0];
        assert_eq!(
            voices_for(Part::SliderHead, object, Some(0), true),
            vec![Voice::Normal]
        );
        assert_eq!(
            voices_for(Part::SliderRepeat, object, Some(1), true),
            vec![Voice::Normal, Voice::Finish]
        );
        assert_eq!(
            voices_for(Part::SliderTail, object, Some(2), true),
            vec![Voice::Normal, Voice::Whistle]
        );
    }

    #[test]
    fn an_edge_bank_overrules_the_objects_for_that_edge_alone() {
        let map = slider_map("0|4|0", "0:0|1:2|0:0");
        let object = &map.objects[0];
        assert_eq!(
            bank_for(&map, object, Voice::Finish, Some(1)).0,
            SampleSet::Soft,
            "the repeat's addition bank"
        );
        assert_eq!(
            bank_for(&map, object, Voice::Normal, Some(0)).0,
            SampleSet::Normal,
            "the head keeps the timing point's"
        );
    }

    #[test]
    fn a_slider_that_names_no_edges_falls_back_to_its_own_sound() {
        let map = Beatmap::parse(
            "osu file format v14\n\n\
             [TimingPoints]\n0,500,4,1,0,100,1,0\n\n\
             [HitObjects]\n0,0,1000,2,4,L|100:0,2,100\n",
        )
        .expect("test map should parse");
        let object = &map.objects[0];
        for edge in 0..3 {
            assert_eq!(
                voices_for(Part::SliderTail, object, Some(edge), true),
                vec![Voice::Normal, Voice::Finish],
                "edge {edge}"
            );
        }
    }
}

#[cfg(test)]
mod miss_tests {
    use super::*;
    use dossier_replay::{HitCounts, Keys, Mods, Replay, ReplayFrame};

    fn replay(frames: Vec<ReplayFrame>) -> Replay {
        Replay {
            mode: dossier_replay::GameMode::Standard,
            game_version: 20_260_101,
            beatmap_hash: String::new(),
            player: "t".into(),
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
        }
    }

    fn audible(track: &Track) -> bool {
        track.to_pcm().chunks(2).any(|s| {
            let v = i16::from_le_bytes([s[0], *s.get(1).unwrap_or(&0)]);
            v.abs() > 8
        })
    }

    fn circles(n: usize) -> Beatmap {
        let mut body = String::from(
            "osu file format v14\n\n[Difficulty]\nCircleSize:5\nOverallDifficulty:5\n\n[HitObjects]\n",
        );

        for i in 0..n {
            let x = 60 + (i % 8) * 50;
            let y = 60 + (i / 8) * 50;
            body.push_str(&format!("{x},{y},{},1,0\n", 1000 + i * 300));
        }
        Beatmap::parse(&body).unwrap()
    }

    #[test]
    fn a_short_run_of_misses_stays_silent() {
        let map = circles(2);
        let frames = vec![ReplayFrame {
            time_ms: 0,
            x: 0.0,
            y: 0.0,
            keys: Keys(0),
        }];
        let state = GameState::new(&map, &replay(frames));
        let track = build(
            &state,
            &map,
            |map_ms| map_ms / 1000.0,
            5.0,
            dossier_audio::Kit::plain(),
            dossier_audio::SamplePack::default(),
            true,
        );
        assert!(!audible(&track), "two dropped notes are not a lost run");
    }

    #[test]
    fn losing_a_long_run_is_heard() {
        let map = circles(30);
        let mut frames = Vec::new();
        for i in 0..25usize {
            let at = (1000 + i * 300) as i64;
            let (x, y) = ((60 + (i % 8) * 50) as f32, (60 + (i / 8) * 50) as f32);
            frames.push(ReplayFrame {
                time_ms: at - 10,
                x,
                y,
                keys: Keys(0),
            });
            frames.push(ReplayFrame {
                time_ms: at,
                x,
                y,
                keys: Keys(Keys::K1),
            });
            frames.push(ReplayFrame {
                time_ms: at + 10,
                x,
                y,
                keys: Keys(0),
            });
        }
        frames.push(ReplayFrame {
            time_ms: 12_000,
            x: 0.0,
            y: 0.0,
            keys: Keys(0),
        });

        let state = GameState::new(&map, &replay(frames));
        let track = build(
            &state,
            &map,
            |map_ms| map_ms / 1000.0,
            20.0,
            dossier_audio::Kit::plain(),
            dossier_audio::SamplePack::default(),
            true,
        );
        let judge = state.judge().unwrap();
        assert!(
            judge.final_state().max_combo >= COMBO_BREAK_THRESHOLD,
            "the run has to be long enough to count: {}",
            judge.final_state().max_combo
        );

        let pcm = track.to_pcm();
        let frame = |seconds: f64| (seconds * 44_100.0) as usize * 4;
        let after = pcm[frame(8.6)..frame(9.6).min(pcm.len())]
            .chunks_exact(2)
            .map(|s| i16::from_le_bytes([s[0], s[1]]).abs())
            .max()
            .unwrap_or(0);
        assert!(after > 8, "losing it should be heard: {after}");
    }

    #[test]
    fn a_broken_run_is_heard_in_the_skins_own_voice() {
        let dir = std::env::temp_dir().join(format!("dossier-break-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        std::fs::write(dir.join("combobreak.wav"), []).expect("a blank");

        let map = circles(30);
        let mut frames = Vec::new();
        for i in 0..25usize {
            let at = (1000 + i * 300) as i64;
            let (x, y) = ((60 + (i % 8) * 50) as f32, (60 + (i / 8) * 50) as f32);
            frames.push(ReplayFrame {
                time_ms: at - 10,
                x,
                y,
                keys: Keys(0),
            });
            frames.push(ReplayFrame {
                time_ms: at,
                x,
                y,
                keys: Keys(Keys::K1),
            });
            frames.push(ReplayFrame {
                time_ms: at + 10,
                x,
                y,
                keys: Keys(0),
            });
        }
        frames.push(ReplayFrame {
            time_ms: 12_000,
            x: 0.0,
            y: 0.0,
            keys: Keys(0),
        });
        let state = GameState::new(&map, &replay(frames));

        let energy = |pack: dossier_audio::SamplePack| -> u64 {
            let track = build(
                &state,
                &map,
                |map_ms| map_ms / 1000.0,
                20.0,
                dossier_audio::Kit::plain(),
                pack,
                true,
            );
            track
                .to_pcm()
                .chunks_exact(2)
                .map(|s| u64::from(i16::from_le_bytes([s[0], s[1]]).unsigned_abs()))
                .sum()
        };

        let synthesised = energy(dossier_audio::SamplePack::default());
        let blanked = energy(dossier_audio::SamplePack::load(&dir));
        assert!(synthesised > 0, "nothing was heard at all");
        assert!(
            blanked < synthesised,
            "the skin's own combobreak was not read: {blanked} against {synthesised}"
        );
    }
}

#[cfg(test)]
mod held {
    use super::*;
    use dossier_replay::{Keys, Mods, Replay, ReplayFrame};

    fn replay(frames: Vec<ReplayFrame>) -> Replay {
        Replay {
            mode: dossier_replay::GameMode::Standard,
            game_version: 20_260_101,
            beatmap_hash: String::new(),
            player: "tester".into(),
            replay_hash: String::new(),
            hits: Default::default(),
            score: 0,
            max_combo: 0,
            perfect_combo: false,
            mods: Mods::default(),
            life_bar: String::new(),
            timestamp_ticks: 0,
            online_score_id: 0,
            target_practice_accuracy: None,
            frames,
            rng_seed: None,
            score_info: None,
        }
    }

    fn samples_with(names: &[&str]) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "dossier-held-{}-{}-{}",
            names.join("-"),
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");

        let frames = 44_100 / 5;
        let mut data = Vec::with_capacity(frames * 2);
        for n in 0..frames {
            let phase = n as f32 / 44_100.0 * 300.0 * std::f32::consts::TAU;
            data.extend_from_slice(&((phase.sin() * 12_000.0) as i16).to_le_bytes());
        }
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&44_100u32.to_le_bytes());
        wav.extend_from_slice(&88_200u32.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&data);
        for name in names {
            std::fs::write(dir.join(format!("{name}.wav")), &wav).expect("written");
        }
        dir
    }

    fn slider_map(hit_sound: u8) -> Beatmap {
        Beatmap::parse(&format!(
            "osu file format v14\n\n[Difficulty]\nCircleSize:4\nApproachRate:5\n\
             SliderMultiplier:1.4\nSliderTickRate:1\n\n\
             [TimingPoints]\n0,500,4,1,0,100,1,0\n\n\
             [HitObjects]\n100,192,1000,2,{hit_sound},L|240:192,1,140\n"
        ))
        .expect("test map should parse")
    }

    fn held_replay() -> Replay {
        let mut frames = vec![ReplayFrame {
            time_ms: 900,
            x: 100.0,
            y: 192.0,
            keys: Keys(0),
        }];

        for step in 0..=25i64 {
            let at = 1000 + step * 20;
            let along = step as f32 / 25.0;
            frames.push(ReplayFrame {
                time_ms: at,
                x: 100.0 + 140.0 * along,
                y: 192.0,
                keys: Keys(Keys::K1),
            });
        }
        frames.push(ReplayFrame {
            time_ms: 1600,
            x: 240.0,
            y: 192.0,
            keys: Keys(0),
        });
        replay(frames)
    }

    fn loudness(track: &dossier_audio::Track, from: f64, to: f64) -> i16 {
        let pcm = track.to_pcm();
        let frame = |seconds: f64| (seconds * 44_100.0) as usize * 4;
        pcm[frame(from)..frame(to).min(pcm.len())]
            .chunks_exact(2)
            .map(|s| i16::from_le_bytes([s[0], s[1]]).abs())
            .max()
            .unwrap_or(0)
    }

    fn track_for(dir: Option<&std::path::Path>, hit_sound: u8) -> dossier_audio::Track {
        let map = slider_map(hit_sound);
        let state = GameState::new(&map, &held_replay());
        let pack = dir.map_or_else(dossier_audio::SamplePack::default, |d| {
            dossier_audio::SamplePack::load(d)
        });
        build(
            &state,
            &map,
            |map_ms| map_ms / 1000.0,
            4.0,
            dossier_audio::Kit::plain(),
            pack,
            true,
        )
    }

    #[test]
    fn a_slider_being_held_sounds_for_as_long_as_it_lasts() {
        let dir = samples_with(&["normal-sliderslide"]);
        let track = track_for(Some(&dir), 0);

        assert!(loudness(&track, 1.2, 1.4) > 8, "the slide is not sounding");
    }

    #[test]
    fn a_skin_without_the_loop_leaves_the_slider_silent() {
        let track = track_for(None, 0);
        assert_eq!(loudness(&track, 1.2, 1.4), 0, "something was invented");
    }

    #[test]
    fn the_whistle_is_held_alongside_the_slide_rather_than_instead_of_it() {
        let dir = samples_with(&["normal-sliderslide", "normal-sliderwhistle"]);
        let plain = loudness(&track_for(Some(&dir), 0), 1.2, 1.4);
        let whistled = loudness(&track_for(Some(&dir), sound_bits::WHISTLE), 1.2, 1.4);
        assert!(plain > 8, "the slide alone is sounding: {plain}");
        assert!(
            whistled > plain,
            "the whistle replaced the slide instead of joining it: {whistled} against {plain}"
        );
    }

    #[test]
    fn a_note_that_names_its_own_sound_file_is_still_given_the_banked_one() {
        let dir = samples_with(&["normal-hitnormal"]);
        let sounded = |sample: &str| {
            let map = Beatmap::parse(&format!(
                "osu file format v14\n\n[Difficulty]\nCircleSize:4\nApproachRate:5\n\n\
                 [TimingPoints]\n0,500,4,1,0,100,1,0\n\n\
                 [HitObjects]\n100,192,1000,1,0{sample}\n"
            ))
            .expect("test map should parse");
            let state = GameState::new(&map, &held_replay());
            let track = build(
                &state,
                &map,
                |map_ms| map_ms / 1000.0,
                4.0,
                dossier_audio::Kit::plain(),
                dossier_audio::SamplePack::load(&dir),
                true,
            );
            loudness(&track, 1.0, 1.2)
        };

        let plain = sounded("");
        assert!(
            plain > 8,
            "the skin's own hit is not sounding at all: {plain}"
        );
        assert_eq!(
            sounded(",0:0:0:0:map-clap.wav"),
            plain,
            "the note's own file changed what was played"
        );
    }

    #[test]
    fn a_sound_the_skin_blanked_is_silent_rather_than_invented() {
        let map = Beatmap::parse(
            "osu file format v14\n\n[Difficulty]\nCircleSize:4\nApproachRate:5\n\n\
             [TimingPoints]\n0,500,4,1,0,100,1,0\n\n\
             [HitObjects]\n100,192,1000,1,0\n",
        )
        .expect("test map should parse");
        let state = GameState::new(&map, &held_replay());
        let struck = |dir: &std::path::Path| {
            let track = build(
                &state,
                &map,
                |map_ms| map_ms / 1000.0,
                4.0,
                dossier_audio::Kit::plain(),
                dossier_audio::SamplePack::load(dir),
                true,
            );
            loudness(&track, 1.0, 1.2)
        };

        let missing = samples_with(&["nothing"]);
        assert!(struck(&missing) > 8, "a missing sound was not synthesised");

        let blanked = samples_with(&["blanked"]);
        std::fs::write(blanked.join("normal-hitnormal.wav"), []).expect("a blank");
        assert_eq!(struck(&blanked), 0, "something was invented");
    }

    #[test]
    fn a_slider_nobody_played_is_silent() {
        let dir = samples_with(&["normal-sliderslide"]);
        let map = slider_map(0);
        let state = GameState::new(
            &map,
            &replay(vec![
                ReplayFrame {
                    time_ms: 900,
                    x: 0.0,
                    y: 0.0,
                    keys: Keys(0),
                },
                ReplayFrame {
                    time_ms: 2000,
                    x: 0.0,
                    y: 0.0,
                    keys: Keys(0),
                },
            ]),
        );
        let track = build(
            &state,
            &map,
            |map_ms| map_ms / 1000.0,
            4.0,
            dossier_audio::Kit::plain(),
            dossier_audio::SamplePack::load(&dir),
            true,
        );
        assert_eq!(loudness(&track, 1.2, 1.4), 0);
    }
}

pub fn sounded(
    state: &GameState,
    beatmap: &Beatmap,
    layering: bool,
) -> Vec<dossier_beatmap::storyboard::Sounded> {
    use dossier_beatmap::storyboard::{Addition, Sounded};

    let mut out = Vec::new();
    let Some(judge) = state.judge() else {
        return out;
    };
    for event in judge.events() {
        let Some(object) = beatmap.objects.get(event.object_index) else {
            continue;
        };
        if event.result.is_miss() {
            continue;
        }
        let edge = slider_edge(state, event.object_index, event.part, event.time_ms);
        let voices = voices_for(event.part, object, edge, layering);
        let normal = bank_for(beatmap, object, Voice::Normal, edge);
        for voice in voices {
            let addition = match voice {
                Voice::Whistle => Some(Addition::Whistle),
                Voice::Finish => Some(Addition::Finish),
                Voice::Clap => Some(Addition::Clap),
                _ => None,
            };
            let (set, index, _) = bank_for(beatmap, object, voice, edge);
            out.push(Sounded {
                time_ms: event.time_ms,
                set: unconvert(normal.0),
                addition_set: unconvert(set),
                addition,
                custom: index,
            });
        }
    }
    out
}

fn unconvert(set: SampleSet) -> MapSet {
    match set {
        SampleSet::Normal => MapSet::Normal,
        SampleSet::Soft => MapSet::Soft,
        SampleSet::Drum => MapSet::Drum,
    }
}
