use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Shader, Transform};

pub(super) fn rounded_rect(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
) -> Option<tiny_skia::Path> {
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let r = radius.min(width / 2.0).min(height / 2.0).max(0.0);
    let (right, bottom) = (x + width, y + height);
    let mut path = PathBuilder::new();
    path.move_to(x + r, y);
    path.line_to(right - r, y);
    path.quad_to(right, y, right, y + r);
    path.line_to(right, bottom - r);
    path.quad_to(right, bottom, right - r, bottom);
    path.line_to(x + r, bottom);
    path.quad_to(x, bottom, x, bottom - r);
    path.line_to(x, y + r);
    path.quad_to(x, y, x + r, y);
    path.close();
    path.finish()
}

pub(super) fn draw_bar(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    colour: Color,
) {
    if !(width.is_finite() && height.is_finite() && x.is_finite() && y.is_finite()) {
        return;
    }
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let (max_x, max_y) = (pixmap.width() as f32, pixmap.height() as f32);
    let (x0, y0) = (x.max(0.0), y.max(0.0));
    let (x1, y1) = ((x + width).min(max_x), (y + height).min(max_y));
    let (width, height) = (x1 - x0, y1 - y0);
    let (x, y) = (x0, y0);
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let width = width.max(1.0).round();
    let height = height.max(1.0).round();
    let mut paint = Paint::default();
    paint.set_color(colour);
    paint.anti_alias = false;
    if let Some(rect) = Rect::from_xywh(x.round(), y.round(), width, height) {
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

pub(super) fn draw_pill(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    colour: Color,
) {
    if !(x.is_finite() && y.is_finite() && width.is_finite() && height.is_finite()) {
        return;
    }
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let r = (height * 0.5).min(width * 0.5);
    let mut path = PathBuilder::new();
    path.move_to(x + r, y);
    path.line_to(x + width - r, y);
    path.quad_to(x + width, y, x + width, y + r);
    path.line_to(x + width, y + height - r);
    path.quad_to(x + width, y + height, x + width - r, y + height);
    path.line_to(x + r, y + height);
    path.quad_to(x, y + height, x, y + height - r);
    path.line_to(x, y + r);
    path.quad_to(x, y, x + r, y);
    path.close();
    let Some(path) = path.finish() else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(colour);
    paint.anti_alias = true;
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

pub(super) fn pie(
    pixmap: &mut Pixmap,
    cx: f32,
    cy: f32,
    radius: f32,
    share: f32,
    colour: Color,
    widdershins: bool,
) {
    let share = share.clamp(0.0, 1.0);
    if share <= 0.0 || radius <= 0.0 || colour.alpha() <= 0.0 {
        return;
    }

    const SEGMENTS: usize = 40;
    let steps = ((SEGMENTS as f32 * share).ceil() as usize).max(1);
    let mut path = PathBuilder::new();
    path.move_to(cx, cy);
    for step in 0..=steps {
        let along = (step as f32 / SEGMENTS as f32).min(share);

        let angle = along * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let across = if widdershins { -1.0 } else { 1.0 };
        path.line_to(cx + across * radius * angle.cos(), cy + radius * angle.sin());
    }
    path.close();
    let Some(path) = path.finish() else {
        return;
    };
    let paint = Paint {
        shader: Shader::SolidColor(colour),
        anti_alias: true,
        ..Default::default()
    };
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}
