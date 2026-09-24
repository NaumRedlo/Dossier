use std::path::{Path, PathBuf};
use std::process::Stdio;

use dossier_beatmap::Point;
use dossier_exhibit::{Clip, Reason};
use dossier_render::Scene;
use dossier_sim::GameState;

use crate::video;

const CROSSFADE_S: f64 = 0.4;

const EDGE_FADE_S: f64 = 0.6;

pub type Hitsounds<'a> = dyn Fn(&video::Plan, usize) -> Option<PathBuf> + 'a;

pub fn render(
    scene: &Scene<'_>,
    state: &GameState,
    clips: &[Clip],
    settings: &video::Settings,
    scratch: Option<&Path>,
    hitsounds: &Hitsounds<'_>,
) -> Result<(), String> {
    let span = state.span_ms();
    let rate = state.playback_rate();
    let fail_at_ms = state.ending().map(|end| end.time_ms);
    if clips.is_empty() {
        return Err("no clips to render".to_owned());
    }
    let Some(scratch) = scratch else {
        return Err("a reel needs a scratch directory to build its clips in".to_owned());
    };

    let mut parts: Vec<Part> = Vec::with_capacity(clips.len());
    for (index, clip) in clips.iter().enumerate() {
        let path = scratch.join(format!("clip-{index}.mp4"));
        let mut one = video::Settings {
            out: path.clone(),
            from_ms: Some(clip.span.from_ms),
            to_ms: Some(clip.span.to_ms),
            hitsounds: None,
            ..clone_settings(settings)
        };

        if slows_into_a_mistake(clip) {
            if let Some((at, focus)) = first_mistake(state, clip.span.from_ms, clip.span.to_ms) {
                one.slow_at_ms = Some(at);
                one.slow_focus = Some(focus);
            }
        }
        let plan = video::Plan::new(span, rate, &one, fail_at_ms)?;
        one.hitsounds = hitsounds(&plan, index);

        let reason = clip.reason.describe();
        eprintln!(
            "[{}/{}] {} — {reason}",
            index + 1,
            clips.len(),
            stamp(clip.span.from_ms),
        );
        settings
            .events
            .clip(index + 1, clips.len(), clip.span.from_ms, &reason);
        video::encode(scene, span, rate, &one, fail_at_ms)?;
        parts.push(Part {
            path,
            seconds: plan.video_seconds,
            sound: one.audio.is_some() || one.hitsounds.is_some(),
        });
    }

    stitch(&parts, settings)
}

struct Part {
    path: PathBuf,
    seconds: f64,
    sound: bool,
}

fn stitch(parts: &[Part], settings: &video::Settings) -> Result<(), String> {
    let sound = parts.iter().all(|part| part.sound);
    let total =
        parts.iter().map(|part| part.seconds).sum::<f64>() - CROSSFADE_S * (parts.len() - 1) as f64;
    if total <= 0.0 {
        return Err("the crossfades are longer than the clips they join".to_owned());
    }

    let mut command = crate::quiet(&settings.ffmpeg);
    command.args(["-y", "-loglevel", "error"]);
    for part in parts {
        command.arg("-i").arg(&part.path);
    }
    command.args(["-filter_complex", &graph(parts, total, sound)]);
    command.args(["-map", "[v]"]);
    if sound {
        command.args(["-map", "[a]", "-c:a", "aac", "-b:a", "192k"]);
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
        .stdin(Stdio::null())
        .stderr(Stdio::piped());
    if std::env::var("DOSSIER_FFMPEG_ARGS").is_ok() {
        eprintln!("ffmpeg {:?}", command.get_args().collect::<Vec<_>>());
    }

    eprintln!("   cutting {} clips together, {total:.1}s", parts.len());
    let output = command
        .output()
        .map_err(|error| format!("could not start {}: {error}", settings.ffmpeg))?;
    if output.status.success() {
        let (width, height) = settings.size;
        crate::note!("video {width}x{height} {total:.3}s");
        settings.events.video(width, height, total);
        return Ok(());
    }

    let said = String::from_utf8_lossy(&output.stderr);
    let lines: Vec<&str> = said
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    Err(format!(
        "cutting the reel together failed ({}){}",
        output.status,
        match lines.is_empty() {
            true => String::new(),
            false => format!(":\n{}", lines[lines.len().saturating_sub(6)..].join("\n")),
        }
    ))
}

fn graph(parts: &[Part], total: f64, sound: bool) -> String {
    let mut chain = Vec::new();

    let mut label = "0:v".to_owned();
    let mut so_far = parts[0].seconds;
    for (index, part) in parts.iter().enumerate().skip(1) {
        let next = format!("vx{index}");
        chain.push(format!(
            "[{label}][{index}:v]xfade=transition=fade:duration={CROSSFADE_S}:offset={:.3}[{next}]",
            so_far - CROSSFADE_S
        ));
        so_far += part.seconds - CROSSFADE_S;
        label = next;
    }
    chain.push(format!(
        "[{label}]fade=t=in:st=0:d={EDGE_FADE_S},fade=t=out:st={:.3}:d={EDGE_FADE_S}[v]",
        (total - EDGE_FADE_S).max(0.0)
    ));

    if sound {
        let mut label = "0:a".to_owned();
        for index in 1..parts.len() {
            let next = format!("ax{index}");
            chain.push(format!(
                "[{label}][{index}:a]acrossfade=d={CROSSFADE_S}:c1=tri:c2=tri[{next}]"
            ));
            label = next;
        }
        chain.push(format!(
            "[{label}]afade=t=in:st=0:d={EDGE_FADE_S},afade=t=out:st={:.3}:d={EDGE_FADE_S}[a]",
            (total - EDGE_FADE_S).max(0.0)
        ));
    }

    chain.join(";")
}

fn clone_settings(settings: &video::Settings) -> video::Settings {
    video::Settings {
        out: settings.out.clone(),
        fps: settings.fps,
        size: settings.size,
        from_ms: settings.from_ms,
        to_ms: settings.to_ms,
        ffmpeg: settings.ffmpeg.clone(),
        crf: settings.crf,
        preset: settings.preset.clone(),
        music_level: settings.music_level,
        hitsound_level: settings.hitsound_level,
        threads: settings.threads,
        encoder_threads: settings.encoder_threads,
        audio: settings.audio.clone(),
        video: settings.video.clone(),
        hitsounds: settings.hitsounds.clone(),

        events: settings.events,
        slow_at_ms: settings.slow_at_ms,
        slow_focus: settings.slow_focus,
    }
}

const SLOW_INTO_A_MISTAKE: bool = false;

fn slows_into_a_mistake(clip: &Clip) -> bool {
    SLOW_INTO_A_MISTAKE && about_a_mistake(clip)
}

fn about_a_mistake(clip: &Clip) -> bool {
    let mistake =
        |reason: &Reason| matches!(reason, Reason::Choke { .. } | Reason::Scramble { .. });
    mistake(&clip.reason) || clip.with.as_ref().is_some_and(mistake)
}

fn first_mistake(state: &GameState, from_ms: f64, to_ms: f64) -> Option<(f64, Point)> {
    let judge = state.judge()?;
    let objects = &state.timeline().objects;
    judge
        .events()
        .iter()
        .filter(|event| event.result.is_miss() && event.part.breaks_combo())
        .filter(|event| event.time_ms >= from_ms && event.time_ms <= to_ms)
        .min_by(|a, b| a.time_ms.total_cmp(&b.time_ms))
        .and_then(|event| Some((event.time_ms, objects.get(event.object_index)?.pos)))
}

fn stamp(ms: f64) -> String {
    let total = (ms / 1000.0).max(0.0);
    let minutes = (total / 60.0).floor();
    format!("{minutes:.0}:{:04.1}", total - minutes * 60.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(lengths: &[f64], sound: bool) -> Vec<Part> {
        lengths
            .iter()
            .enumerate()
            .map(|(i, &seconds)| Part {
                path: PathBuf::from(format!("clip-{i}.mp4")),
                seconds,
                sound,
            })
            .collect()
    }

    fn total(parts: &[Part]) -> f64 {
        parts.iter().map(|p| p.seconds).sum::<f64>() - CROSSFADE_S * (parts.len() - 1) as f64
    }

    #[test]
    fn each_dissolve_starts_where_the_stream_so_far_ends() {
        let parts = parts(&[6.0, 6.0, 6.0], true);
        let graph = graph(&parts, total(&parts), true);

        assert!(graph.contains("offset=5.600"), "{graph}");

        assert!(graph.contains("offset=11.200"), "{graph}");
    }

    #[test]
    fn the_reel_fades_out_a_fade_before_it_ends() {
        let parts = parts(&[6.0, 6.0], true);

        let graph = graph(&parts, total(&parts), true);
        assert!(graph.contains("fade=t=out:st=11.000"), "{graph}");
        assert!(graph.contains("afade=t=out:st=11.000"), "{graph}");
    }

    fn a_choke(with: Option<Reason>) -> Clip {
        Clip {
            span: dossier_exhibit::Span {
                from_ms: 1_000.0,
                to_ms: 7_000.0,
            },
            reason: Reason::Choke {
                combo: 400,
                through: 0.6,
            },
            with,
            rank: 0,
            score: 0.9,
        }
    }

    #[test]
    fn a_reel_does_not_slow_into_a_mistake_for_now() {
        assert!(
            about_a_mistake(&a_choke(None)),
            "a choke is still what the dip would be for — only the dip is off"
        );
        assert!(!slows_into_a_mistake(&a_choke(None)));
        assert!(!slows_into_a_mistake(&a_choke(Some(Reason::Peak {
            combo: 300
        }))));
    }

    #[test]
    fn a_silent_reel_builds_no_audio_chain() {
        let parts = parts(&[6.0, 6.0], false);
        let graph = graph(&parts, total(&parts), false);
        assert!(!graph.contains(":a]"), "{graph}");
        assert!(!graph.contains("acrossfade"), "{graph}");
        assert!(graph.ends_with("[v]"), "{graph}");
    }

    #[test]
    fn a_single_clip_still_gets_its_edges() {
        let parts = parts(&[6.0], true);
        let graph = graph(&parts, total(&parts), true);
        assert!(!graph.contains("xfade"), "{graph}");
        assert!(graph.contains("[0:v]fade=t=in:st=0"), "{graph}");
        assert!(graph.contains("[0:a]afade=t=in:st=0"), "{graph}");
    }

    #[test]
    fn uneven_clips_still_line_up() {
        let parts = parts(&[6.0, 6.7, 6.0], true);
        let graph = graph(&parts, total(&parts), true);
        assert!(graph.contains("offset=5.600"), "{graph}");

        assert!(graph.contains("offset=11.900"), "{graph}");
    }
}
