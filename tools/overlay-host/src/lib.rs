use dossier_hud::{Context, Sprite, Stage};
use dossier_overlay::{Client, View, Viewport};
use std::time::Instant;

pub enum Update {
    Unchanged,
    Hide,
    Draw(Sprite),
}

#[derive(Default)]
pub struct Scene {
    view: Option<View>,
    context: Context,
    stage: Stage,
    size: (u32, u32),
    visible: bool,
    supplied: Option<dossier_hud::context::Packet>,
}

impl Scene {
    pub fn supply(&mut self, packet: Option<dossier_hud::context::Packet>) {
        self.supplied = packet;
    }

    pub fn update(
        &mut self,
        view: Option<&View>,
        pid: u32,
        width: u32,
        height: u32,
        now: Instant,
    ) -> Update {
        let view = view.filter(|view| view.pid == pid && view.client == Client::Stable);
        let Some(view) =
            view.filter(|_| width > 0 && height > 0 && width <= 16384 && height <= 16384)
        else {
            let visible = self.visible;
            *self = Self::default();
            return if visible {
                Update::Hide
            } else {
                Update::Unchanged
            };
        };
        let changed = self
            .view
            .as_ref()
            .is_none_or(|old| old.pid != view.pid || old.snapshot != view.snapshot);
        let resized = self.size != (width, height);
        let mut context = self
            .supplied
            .as_ref()
            .and_then(|packet| packet.context(view, dossier_hud::context::now_ms()))
            .unwrap_or_default();
        dossier_hud::told(view, &mut context);
        let context_changed = self.context != context;
        self.context = context;
        if self.view.is_none() {
            self.stage = Stage::settled(Some(view), &self.context);
        }
        let moving = self.stage.moving(Some(view), &self.context);
        self.stage.advance(Some(view), &self.context, now);
        self.view = Some(view.clone());
        self.size = (width, height);
        if !changed && !resized && !moving && !context_changed {
            return Update::Unchanged;
        }
        let viewport = Viewport {
            width,
            height,
            scale: 1.0,
        };
        let sprites = dossier_hud::sprites(Some(view), &self.context, &self.stage, viewport);
        match composite(sprites, viewport) {
            Some(sprite) => {
                self.visible = true;
                Update::Draw(sprite)
            }
            None => {
                let visible = std::mem::take(&mut self.visible);
                if visible {
                    Update::Hide
                } else {
                    Update::Unchanged
                }
            }
        }
    }
}

fn composite(sprites: Vec<Sprite>, viewport: Viewport) -> Option<Sprite> {
    let left = sprites.iter().map(|s| s.x.max(0)).min()?;
    let top = sprites.iter().map(|s| s.y.max(0)).min()?;
    let right = sprites
        .iter()
        .map(|s| (i64::from(s.x) + i64::from(s.width)).min(i64::from(viewport.width)))
        .max()?;
    let bottom = sprites
        .iter()
        .map(|s| (i64::from(s.y) + i64::from(s.height)).min(i64::from(viewport.height)))
        .max()?;
    let width = u32::try_from(right - i64::from(left)).ok()?;
    let height = u32::try_from(bottom - i64::from(top)).ok()?;
    let bytes = usize::try_from(u64::from(width) * u64::from(height) * 4).ok()?;
    if bytes == 0 || bytes > 64 * 1024 * 1024 {
        return None;
    }
    let mut pixels = vec![0u8; bytes];
    for sprite in sprites {
        if sprite.pixels.len() != (u64::from(sprite.width) * u64::from(sprite.height) * 4) as usize
        {
            return None;
        }
        let from_x = (i64::from(left) - i64::from(sprite.x)).max(0) as u32;
        let from_y = (i64::from(top) - i64::from(sprite.y)).max(0) as u32;
        let to_x = (right - i64::from(sprite.x)).clamp(0, i64::from(sprite.width)) as u32;
        let to_y = (bottom - i64::from(sprite.y)).clamp(0, i64::from(sprite.height)) as u32;
        for y in from_y..to_y {
            for x in from_x..to_x {
                let source = ((y as usize * sprite.width as usize) + x as usize) * 4;
                let target_x = (i64::from(sprite.x) + i64::from(x) - i64::from(left)) as usize;
                let target_y = (i64::from(sprite.y) + i64::from(y) - i64::from(top)) as usize;
                let target = (target_y * width as usize + target_x) * 4;
                let alpha = u32::from(sprite.pixels[source + 3]);
                for channel in 0..4 {
                    let over = u32::from(sprite.pixels[source + channel]);
                    let under = u32::from(pixels[target + channel]);
                    pixels[target + channel] =
                        (over + (under * (255 - alpha) + 127) / 255).min(255) as u8;
                }
            }
        }
    }
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Some(Sprite {
        key: "scene",
        x: left,
        y: top,
        width,
        height,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dossier_overlay::{Gameplay, Meter, Screen, Snapshot};
    use std::time::Duration;

    #[test]
    fn context_arrival_and_expiry_redraw_an_unchanged_scene() {
        let mut scene = Scene::default();
        let view = view();
        let now = Instant::now();
        scene.update(Some(&view), 7, 1280, 720, now);
        scene.supply(Some(dossier_hud::context::Packet {
            version: 1,
            pid: 7,
            at_ms: dossier_hud::context::now_ms(),
            lang: dossier_hud::Lang::En,
            map_md5: None,
            card: None,
            day: Some(dossier_hud::Day::default()),
        }));
        assert!(matches!(
            scene.update(Some(&view), 7, 1280, 720, now),
            Update::Draw(_)
        ));
        assert!(scene.context.day.is_some());
        scene.supplied.as_mut().unwrap().at_ms = 0;
        assert!(matches!(
            scene.update(Some(&view), 7, 1280, 720, now),
            Update::Draw(_)
        ));
        assert!(scene.context.day.is_none());
    }

    fn view() -> View {
        View {
            client: Client::Stable,
            pid: 7,
            snapshot: Snapshot {
                screen: Screen::Menu,
                beatmap: None,
                gameplay: None,
                watching_replay: None,
                meter: None,
            },
        }
    }

    #[test]
    fn static_scene_is_cached_resize_redraws_and_lost_input_hides_once() {
        let mut scene = Scene::default();
        let view = view();
        let now = Instant::now();
        assert!(matches!(
            scene.update(Some(&view), 7, 1280, 720, now),
            Update::Draw(_)
        ));
        assert!(matches!(
            scene.update(Some(&view), 7, 1280, 720, now),
            Update::Unchanged
        ));
        assert!(matches!(
            scene.update(Some(&view), 7, 1920, 1080, now),
            Update::Draw(_)
        ));
        assert!(matches!(
            scene.update(None, 7, 1280, 720, now),
            Update::Hide
        ));
        assert!(matches!(
            scene.update(None, 7, 1280, 720, now),
            Update::Unchanged
        ));
        assert!(matches!(
            scene.update(Some(&view), 8, 1280, 720, now),
            Update::Unchanged
        ));
        let mut lazer = view.clone();
        lazer.client = Client::Lazer;
        assert!(matches!(
            scene.update(Some(&lazer), 7, 1280, 720, now),
            Update::Unchanged
        ));
    }

    #[test]
    fn translucent_pixels_overlap_without_becoming_opaque_and_are_bgra() {
        let sprite = |x, pixels| Sprite {
            key: "test",
            x,
            y: 0,
            width: 1,
            height: 1,
            pixels,
        };
        let result = composite(
            vec![
                sprite(0, vec![128, 0, 0, 128]),
                sprite(0, vec![0, 0, 128, 128]),
                sprite(2, vec![0, 64, 0, 64]),
            ],
            Viewport {
                width: 3,
                height: 1,
                scale: 1.0,
            },
        )
        .unwrap();
        assert_eq!(result.pixels, [128, 0, 64, 192, 0, 0, 0, 0, 0, 64, 0, 64]);
    }

    #[test]
    fn edges_are_clipped_and_allocation_is_bounded() {
        let sprite = Sprite {
            key: "edge",
            x: -1,
            y: -1,
            width: 2,
            height: 2,
            pixels: vec![255; 16],
        };
        let result = composite(
            vec![sprite],
            Viewport {
                width: 1,
                height: 1,
                scale: 1.0,
            },
        )
        .unwrap();
        assert_eq!(
            (result.x, result.y, result.width, result.height),
            (0, 0, 1, 1)
        );
        assert_eq!(result.pixels, [255; 4]);
        let oversized = Sprite {
            key: "large",
            x: 0,
            y: 0,
            width: 16384,
            height: 16384,
            pixels: vec![],
        };
        assert!(
            composite(
                vec![oversized],
                Viewport {
                    width: 16384,
                    height: 16384,
                    scale: 1.0
                }
            )
            .is_none()
        );
        let mut scene = Scene::default();
        assert!(matches!(
            scene.update(Some(&view()), 7, 16385, 720, Instant::now()),
            Update::Unchanged
        ));
    }

    #[test]
    fn live_scene_includes_pp_and_rate_and_animates_the_break_transition() {
        let mut scene = Scene::default();
        let mut view = view();
        view.snapshot.screen = Screen::Playing;
        view.snapshot.gameplay = Some(Gameplay {
            ruleset: 0,
            time_ms: 1000,
            score: 100,
            combo: 10,
            max_combo: 10,
            accuracy: Some(99.0),
            misses: 0,
            legacy_mods: Some(0),
            mods: vec![],
            unstable_rate: Some(90.0),
            resting: Some(false),
            pp: Some(42.0),
            pp_clean: Some(100.0),
        });
        view.snapshot.meter = Some(Meter {
            shown: true,
            scale: 1.0,
        });
        let now = Instant::now();
        scene.update(Some(&view), 7, 1280, 720, now);
        scene.update(Some(&view), 7, 1280, 720, now + Duration::from_secs(1));
        assert_eq!(
            dossier_hud::plan(Some(&view), &scene.context, &scene.stage)
                .iter()
                .map(|p| p.key)
                .collect::<Vec<_>>(),
            ["rate", "tile", "pp"]
        );
        assert_eq!(scene.stage.unfold, 0.0);
        view.snapshot.gameplay.as_mut().unwrap().resting = Some(true);
        assert!(matches!(
            scene.update(Some(&view), 7, 1280, 720, now + Duration::from_millis(1050)),
            Update::Draw(_)
        ));
        assert!(scene.stage.unfold > 0.0 && scene.stage.unfold < 1.0);
        scene.update(Some(&view), 7, 1280, 720, now + Duration::from_millis(1300));
        assert_eq!(scene.stage.unfold, 1.0);
        view.snapshot.gameplay.as_mut().unwrap().pp = None;
        view.snapshot.gameplay.as_mut().unwrap().unstable_rate = None;
        scene.update(Some(&view), 7, 1280, 720, now + Duration::from_millis(1320));
        assert_eq!(
            dossier_hud::plan(Some(&view), &scene.context, &scene.stage)
                .iter()
                .map(|p| p.key)
                .collect::<Vec<_>>(),
            ["tile"]
        );
        view.snapshot.gameplay.as_mut().unwrap().pp = Some(42.0);
        view.snapshot.watching_replay = Some(true);
        scene.update(Some(&view), 7, 1280, 720, now + Duration::from_millis(1340));
        assert_eq!(
            dossier_hud::plan(Some(&view), &scene.context, &scene.stage)
                .iter()
                .map(|p| p.key)
                .collect::<Vec<_>>(),
            ["tile"]
        );
        view.snapshot.screen = Screen::Menu;
        view.snapshot.gameplay = None;
        scene.update(Some(&view), 7, 1280, 720, now + Duration::from_millis(1400));
        assert!(scene.context.play.is_none());
    }
}
