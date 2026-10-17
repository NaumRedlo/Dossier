pub mod background;
pub mod elements;
pub mod imported;
mod layout;
mod leaderboard;
pub mod mania;
pub mod mark;
pub mod mods;
mod renderer;
mod skin;
pub mod storyboard;
mod text;

pub use layout::Layout;
pub use leaderboard::{Entry, Leaderboard};
pub use renderer::{Camera, Scene, Signature, FAIL_ANIMATION_MS, FAIL_EMPTY_MS, OUTRO_FADE_MS};
pub use skin::{ArrowShape, Effects, Skin};
pub use text::{Align, Font, Label};

pub use tiny_skia::Pixmap;

pub trait Picture: Sync {
    fn draw_into(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout, camera: Option<Camera>);
}

impl Picture for Scene<'_> {
    fn draw_into(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout, camera: Option<Camera>) {
        Scene::draw_into(self, pixmap, time_ms, layout, camera);
    }
}
