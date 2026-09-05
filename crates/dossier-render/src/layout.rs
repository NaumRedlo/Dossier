use dossier_beatmap::{Point, PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};

const PLAYFIELD_HEIGHT_RATIO: f64 = 0.8;

const VERTICAL_SHIFT_OSU: f64 = 8.0;

const MAX_CAMERA_ZOOM: f64 = 1.8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub width: u32,
    pub height: u32,
    scale: f64,
    origin_x: f64,
    origin_y: f64,
}

impl Layout {
    pub fn new(width: u32, height: u32) -> Self {
        let (w, h) = (f64::from(width), f64::from(height));

        let scale = (h * PLAYFIELD_HEIGHT_RATIO / PLAYFIELD_HEIGHT)
            .min(w * PLAYFIELD_HEIGHT_RATIO / PLAYFIELD_WIDTH);
        Self {
            width,
            height,
            scale,
            origin_x: (w - PLAYFIELD_WIDTH * scale) / 2.0,
            origin_y: (h - PLAYFIELD_HEIGHT * scale) / 2.0 + VERTICAL_SHIFT_OSU * scale,
        }
    }

    pub fn map(&self, point: Point) -> (f32, f32) {
        (
            (self.origin_x + point.x * self.scale) as f32,
            (self.origin_y + point.y * self.scale) as f32,
        )
    }

    pub fn length(&self, osu_pixels: f64) -> f32 {
        (osu_pixels * self.scale) as f32
    }

    pub fn scale(&self) -> f64 {
        self.scale
    }

    pub fn focused(&self, focus: Point, closeness: f64) -> Self {
        let closeness = closeness.clamp(0.0, 1.0);
        let zoom = 1.0 + (MAX_CAMERA_ZOOM - 1.0) * closeness;
        let scale = self.scale * zoom;

        let here_x = self.origin_x + focus.x * self.scale;
        let here_y = self.origin_y + focus.y * self.scale;
        let (centre_x, centre_y) = (f64::from(self.width) / 2.0, f64::from(self.height) / 2.0);
        let target_x = here_x + (centre_x - here_x) * closeness;
        let target_y = here_y + (centre_y - here_y) * closeness;
        Self {
            width: self.width,
            height: self.height,
            scale,
            origin_x: target_x - focus.x * scale,
            origin_y: target_y - focus.y * scale,
        }
    }

    pub fn transform(&self) -> tiny_skia::Transform {
        tiny_skia::Transform::from_row(
            self.scale as f32,
            0.0,
            0.0,
            self.scale as f32,
            self.origin_x as f32,
            self.origin_y as f32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_closeness_is_the_plain_layout() {
        let layout = Layout::new(1920, 1080);
        let focus = Point { x: 100.0, y: 300.0 };
        assert_eq!(layout.focused(focus, 0.0), layout);
    }

    #[test]
    fn full_closeness_centres_the_focus_and_magnifies() {
        let layout = Layout::new(1920, 1080);
        let focus = Point { x: 100.0, y: 300.0 };
        let close = layout.focused(focus, 1.0);
        let (x, y) = close.map(focus);
        assert!(
            (x - 960.0).abs() < 0.01 && (y - 540.0).abs() < 0.01,
            "{x},{y}"
        );
        assert!((close.scale() - layout.scale() * MAX_CAMERA_ZOOM).abs() < 1e-9);
    }

    #[test]
    fn the_field_is_centred_horizontally() {
        let layout = Layout::new(1920, 1080);
        let (left, _) = layout.map(Point { x: 0.0, y: 0.0 });
        let (right, _) = layout.map(Point {
            x: PLAYFIELD_WIDTH,
            y: 0.0,
        });
        assert!(
            (left - (1920.0 - right)).abs() < 0.01,
            "margins differ: {left} vs {}",
            1920.0 - right
        );
    }

    #[test]
    fn the_field_keeps_its_aspect_ratio() {
        let layout = Layout::new(1920, 1080);
        let width = layout.length(PLAYFIELD_WIDTH);
        let height = layout.length(PLAYFIELD_HEIGHT);
        assert!((f64::from(width / height) - 4.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn a_narrow_frame_fits_by_width_instead() {
        let layout = Layout::new(400, 1000);
        assert!(layout.length(PLAYFIELD_WIDTH) <= 400.0);
    }

    #[test]
    fn the_centre_of_the_field_lands_near_the_centre_of_the_frame() {
        let layout = Layout::new(1280, 720);
        let (x, y) = layout.map(Point::CENTRE);

        assert!((f64::from(x) - 640.0).abs() < 0.01);
        let below = f64::from(y) - 360.0;
        assert!(
            (below - f64::from(layout.length(VERTICAL_SHIFT_OSU))).abs() < 0.01,
            "{below}px below centre"
        );
    }
}
