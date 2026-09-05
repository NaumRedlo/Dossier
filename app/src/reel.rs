use std::path::{Path, PathBuf};

use crate::draw::{self, Asked, Told};

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct Span {
    pub from_ms: f64,
    pub to_ms: f64,
}

fn listed(path: &Path) -> String {
    format!(
        "file '{}'\n",
        path.display().to_string().replace('\'', "'\\''")
    )
}

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
        fine: asked.fine.clone(),
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
            fine: crate::draw::Fine::default(),
        };
        assert!(build(&asked, &[], &told, &|_, _| {}).is_err());
    }
}
