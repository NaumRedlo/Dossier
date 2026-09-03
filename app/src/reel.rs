//! Several chosen spans of one play, rendered and put end to end.
//!
//! Not [`dossier_produce::reel`], which is the bot's reel: that one takes the
//! clips [`dossier_exhibit`] *chose* and dissolves them into each other. These
//! spans were chosen by a person on a timeline, no scorer picked them, and
//! `Clip` is the wrong shape to claim otherwise.
//!
//! So the pieces are drawn by the same render path a whole replay goes through
//! and then concatenated without re-encoding — every part comes out of one
//! encoder with one set of settings, which is exactly the case `-c copy` is
//! for. Hard cuts, then. The dissolve is the produce crate's, and reaching it
//! from here means teaching it to take spans rather than chooser's clips.

use std::path::{Path, PathBuf};

use crate::draw::{self, Asked, Told};

/// A span of the play, in map milliseconds.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct Span {
    pub from_ms: f64,
    pub to_ms: f64,
}

/// `file 'name'` — the concat demuxer's own quoting: a single quote inside is
/// closed, escaped and reopened. The names here are ours, but a scratch
/// directory sits under a path somebody else named.
fn listed(path: &Path) -> String {
    format!(
        "file '{}'\n",
        path.display().to_string().replace('\'', "'\\''")
    )
}

/// Draw every span and join them into one file. Returns where it went.
pub fn build(
    asked: &Asked<'_>,
    spans: &[Span],
    told: &Told,
    step: &dyn Fn(usize, usize),
) -> Result<PathBuf, String> {
    if spans.is_empty() {
        return Err("нечего собирать: на ленте нет ни одного куска".to_owned());
    }
    let scratch = std::env::temp_dir().join(format!("dossier-reel-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).map_err(|why| why.to_string())?;

    let mut parts = Vec::with_capacity(spans.len());
    for (index, span) in spans.iter().enumerate() {
        step(index, spans.len());
        let out = scratch.join(format!("part-{index}.mp4"));
        let one = Asked {
            out: out.clone(),
            from_ms: Some(span.from_ms),
            to_ms: Some(span.to_ms),
            ..clone_asked(asked)
        };
        draw::draw(&one, told)?;
        parts.push(out);
    }

    // Один кусок — это и есть готовый файл, склеивать нечего.
    if parts.len() == 1 {
        std::fs::rename(&parts[0], &asked.out)
            .or_else(|_| std::fs::copy(&parts[0], &asked.out).map(|_| ()))
            .map_err(|why| why.to_string())?;
        let _ = std::fs::remove_dir_all(&scratch);
        return Ok(asked.out.clone());
    }

    let list = scratch.join("parts.txt");
    std::fs::write(&list, parts.iter().map(|p| listed(p)).collect::<String>())
        .map_err(|why| why.to_string())?;

    let done = std::process::Command::new("ffmpeg")
        .args(["-y", "-f", "concat", "-safe", "0", "-i"])
        .arg(&list)
        .args(["-c", "copy"])
        .arg(&asked.out)
        .output()
        .map_err(|why| format!("ffmpeg не запустился: {why}"))?;
    let _ = std::fs::remove_dir_all(&scratch);
    if !done.status.success() {
        let said = String::from_utf8_lossy(&done.stderr);
        let tail: Vec<&str> = said.lines().rev().take(3).collect();
        return Err(format!("склеить не вышло: {}", tail.join(" · ")));
    }
    Ok(asked.out.clone())
}

/// `Asked` holds borrows and cannot derive `Clone`; this is the two fields a
/// part overrides and everything else as it was.
fn clone_asked<'a>(asked: &Asked<'a>) -> Asked<'a> {
    Asked {
        replay: asked.replay,
        songs: asked.songs,
        map: asked.map,
        out: asked.out.clone(),
        size: asked.size,
        fps: asked.fps,
        from_ms: asked.from_ms,
        to_ms: asked.to_ms,
        background: asked.background,
        storyboard: asked.storyboard,
        bare: asked.bare,
        mute: asked.mute,
        skin: asked.skin.clone(),
        events: asked.events,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_with_a_quote_cannot_end_the_list_entry() {
        let said = listed(Path::new("/tmp/it's here/part-0.mp4"));
        assert_eq!(said, "file '/tmp/it'\\''s here/part-0.mp4'\n");
    }

    #[test]
    fn an_empty_reel_says_so_rather_than_writing_nothing() {
        let told = Told::default();
        let asked = Asked {
            replay: Path::new("x.osr"),
            songs: None,
            map: None,
            out: PathBuf::from("out.mp4"),
            size: (640, 360),
            fps: 30.0,
            from_ms: None,
            to_ms: None,
            background: false,
            storyboard: false,
            bare: false,
            mute: true,
            skin: None,
            events: false,
        };
        assert!(build(&asked, &[], &told, &|_, _| {}).is_err());
    }
}
