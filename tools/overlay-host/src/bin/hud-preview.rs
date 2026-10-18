use base64::{Engine, engine::general_purpose::STANDARD};
use dossier_hud::{Lang, Stage, sample, sprites};
use dossier_overlay::Viewport;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Read, Write};
use std::time::Instant;

#[derive(Deserialize)]
struct Request {
    id: u64,
    width: u32,
    height: u32,
    #[serde(default)]
    scene: Option<String>,
    #[serde(default)]
    pid: u32,
    #[serde(default)]
    started_ms: u64,
}
#[derive(Serialize)]
struct Plate {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    png: String,
}
#[derive(Serialize)]
struct Response {
    id: u64,
    plates: Vec<Plate>,
    status: String,
    map: Option<String>,
}

fn render(
    request: Request,
    stage: &mut Stage,
    tail: &mut dossier_overlay_host::lazer::Tail,
    logs: &std::path::Path,
) -> io::Result<Response> {
    if request.width == 0 || request.height == 0 || request.width > 8192 || request.height > 8192 {
        return Err(io::Error::other(
            "preview size must be within 1..8192 pixels",
        ));
    }
    let Some(scene) = &request.scene else {
        let state = tail.poll(logs, request.pid, request.started_ms, Instant::now());
        return Ok(Response {
            id: request.id,
            plates: Vec::new(),
            status: state
                .as_ref()
                .and_then(|state| state.screen.clone())
                .unwrap_or_else(|| {
                    if state.is_some() {
                        "Ожидание событий lazer"
                    } else {
                        "Ожидание журнала lazer"
                    }
                    .into()
                }),
            map: state.and_then(|state| state.map),
        });
    };
    let (_, view, context) = sample::frames(Lang::Ru)
        .into_iter()
        .find(|(name, _, _)| *name == scene)
        .ok_or_else(|| io::Error::other("unknown preview scene"))?;
    stage.advance(Some(&view), &context, Instant::now());
    let viewport = Viewport {
        width: request.width,
        height: request.height,
        scale: 1.0,
    };
    let plates = sprites(Some(&view), &context, stage, viewport)
        .into_iter()
        .map(|mut sprite| {
            // PNG stores straight alpha; the HUD's rasterizer returns premultiplied pixels.
            for pixel in sprite.pixels.chunks_exact_mut(4) {
                let alpha = u32::from(pixel[3]);
                if alpha != 0 {
                    for channel in &mut pixel[..3] {
                        *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
                    }
                }
            }
            Plate {
                x: sprite.x,
                y: sprite.y,
                width: sprite.width,
                height: sprite.height,
                png: STANDARD.encode(sample::png(sprite.width, sprite.height, &sprite.pixels)),
            }
        })
        .collect();
    Ok(Response {
        id: request.id,
        plates,
        status: "Тестовые данные".into(),
        map: None,
    })
}

fn main() -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut stage = Stage::default();
    let mut tail = dossier_overlay_host::lazer::Tail::default();
    let logs = std::env::var_os("DOSSIER_LAZER_LOGS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join("Library/Application Support/osu/logs")
        });
    loop {
        let mut line = Vec::new();
        let count = (&mut input).take(4097).read_until(b'\n', &mut line)?;
        if count == 0 {
            return Ok(());
        }
        if count > 4096 || !line.ends_with(b"\n") {
            return Err(io::Error::other("invalid preview request"));
        }
        let request = serde_json::from_slice(&line)?;
        serde_json::to_writer(&mut output, &render(request, &mut stage, &mut tail, &logs)?)?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_protocol_uses_runtime_events_and_never_sample_plates() {
        let dir = std::env::temp_dir().join(format!("dossier-live-render-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("1000.runtime.log"), "Game-wide working beatmap updated to Actual artist - Actual map [Hard]\nOsuScreenStack#1 entered SoloPlayer#2\n").unwrap();
        let request: Request = serde_json::from_str(
            r#"{"id":1,"width":1280,"height":720,"pid":7,"started_ms":1000000}"#,
        )
        .unwrap();
        let frame = render(
            request,
            &mut Stage::default(),
            &mut dossier_overlay_host::lazer::Tail::default(),
            &dir,
        )
        .unwrap();
        assert!(frame.plates.is_empty());
        assert_eq!(frame.status, "Игровой экран");
        assert_eq!(
            frame.map.as_deref(),
            Some("Actual artist - Actual map [Hard]")
        );
        let request: Request =
            serde_json::from_str(r#"{"id":2,"width":1280,"height":720}"#).unwrap();
        let frame = render(
            request,
            &mut Stage::default(),
            &mut dossier_overlay_host::lazer::Tail::default(),
            &dir,
        )
        .unwrap();
        assert!(frame.plates.is_empty());
        assert!(frame.map.is_none());
        assert_eq!(frame.status, "Ожидание журнала lazer");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn every_scene_renders_at_retina_size_and_invalid_input_fails() {
        for (scene, _, _) in sample::frames(Lang::Ru) {
            let frame = render(
                Request {
                    id: 42,
                    width: 2560,
                    height: 1440,
                    scene: Some(scene.into()),
                    pid: 0,
                    started_ms: 0,
                },
                &mut Stage::default(),
                &mut dossier_overlay_host::lazer::Tail::default(),
                std::path::Path::new("/nonexistent"),
            )
            .unwrap();
            assert_eq!(frame.id, 42);
            assert!(!frame.plates.is_empty());
            for plate in frame.plates {
                assert!(
                    STANDARD
                        .decode(plate.png)
                        .unwrap()
                        .starts_with(b"\x89PNG\r\n\x1a\n")
                );
            }
        }
        assert!(
            render(
                Request {
                    id: 1,
                    width: 0,
                    height: 720,
                    scene: Some("play".into()),
                    pid: 0,
                    started_ms: 0
                },
                &mut Stage::default(),
                &mut dossier_overlay_host::lazer::Tail::default(),
                std::path::Path::new("/nonexistent")
            )
            .is_err()
        );
        assert!(
            render(
                Request {
                    id: 1,
                    width: 1280,
                    height: 720,
                    scene: Some("unknown".into()),
                    pid: 0,
                    started_ms: 0
                },
                &mut Stage::default(),
                &mut dossier_overlay_host::lazer::Tail::default(),
                std::path::Path::new("/nonexistent")
            )
            .is_err()
        );
    }
}
