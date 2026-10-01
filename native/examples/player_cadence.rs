use std::time::{Duration, Instant};

fn main() {
    let path = std::path::PathBuf::from(std::env::args().nth(1).expect("a video"));
    let seconds: f64 = std::env::args().nth(2).and_then(|said| said.parse().ok()).unwrap_or(10.0);
    let tick: f64 = std::env::args().nth(3).and_then(|said| said.parse().ok()).unwrap_or(120.0);
    let ffmpeg = dossier_native::checks::ffmpeg_on_path().expect("ffmpeg");
    let media = dossier_native::videos::probe(&ffmpeg, &path).expect("a probe");
    let mut player = dossier_native::player::Player::open(&ffmpeg, &path, media, dossier_native::player::Manner { level: 1.0, muted: true, rate: 1.0 });
    let began = Instant::now();
    let mut seen: Vec<(f64, i64)> = Vec::new();
    let mut last = -1;
    let mut ticks = 0u64;
    while began.elapsed().as_secs_f64() < seconds {
        let due = began + Duration::from_secs_f64(ticks as f64 / tick);
        if let Some(wait) = due.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
        ticks += 1;
        player.pull(Instant::now());
        let at = player.at_ms();
        if player.ready() && at != last {
            seen.push((began.elapsed().as_secs_f64() * 1000.0, at));
            last = at;
        }
    }
    player.close();
    let first = seen.first().map_or(0.0, |first| first.0);
    let steady: Vec<&(f64, i64)> = seen.iter().filter(|(wall, _)| *wall > first + 1000.0).collect();
    let gaps: Vec<f64> = steady.windows(2).map(|pair| pair[1].0 - pair[0].0).collect();
    let jumps: Vec<i64> = steady.windows(2).map(|pair| pair[1].1 - pair[0].1).collect();
    let mut sorted = gaps.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let pick = |share: f64| sorted.get(((sorted.len() as f64 - 1.0) * share) as usize).copied().unwrap_or(0.0);
    let span = steady.last().map_or(0.0, |last| last.0) - steady.first().map_or(0.0, |first| first.0);
    println!("first frame after {first:.0} ms");
    println!("{} frames in {:.1} s: {:.1} a second", steady.len(), span / 1000.0, steady.len() as f64 * 1000.0 / span.max(1.0));
    println!("gap between frames: median {:.1}, 90% {:.1}, 99% {:.1}, longest {:.1} ms", pick(0.5), pick(0.9), pick(0.99), pick(1.0));
    println!("source frames skipped: {} of {}", jumps.iter().filter(|jump| **jump > 25).count(), jumps.len());
    let drift = steady.last().map_or(0.0, |last| (last.1 - steady[0].1) as f64 - (last.0 - steady[0].0));
    println!("video against the wall clock over the run: {drift:+.0} ms");
    let mut hist = [0usize; 8];
    for gap in &gaps {
        hist[((gap / 8.34).round() as usize).clamp(1, 7)] += 1;
    }
    println!("gaps in display ticks of 8.3 ms: {:?}", &hist[1..]);
}
