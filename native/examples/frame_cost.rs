use std::time::{Duration, Instant};

use dossier_native::gallery;
use dossier_native::lang::Lang;
use dossier_native::main_screen::Message;
use iced_test::core::renderer::Headless;
use iced_test::core::{mouse, renderer, theme::Base};
use iced_test::runtime::user_interface::{Cache, UserInterface};

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[samples.len() / 2]
}

fn ms(took: Duration) -> String {
    format!("{:>7.2}", took.as_secs_f64() * 1000.0)
}

fn main() {
    let rounds: usize = std::env::args().nth(1).and_then(|asked| asked.parse().ok()).unwrap_or(30).max(1);
    let (width, height) = std::env::args()
        .nth(2)
        .and_then(|asked| asked.split_once('x').and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?))))
        .unwrap_or((1440.0, 900.0));
    let size = iced::Size::new(width, height);
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().expect("the font system");
        for bytes in dossier_native::theme::FONTS {
            fonts.load_font(std::borrow::Cow::Borrowed(bytes));
        }
    }
    let settings = dossier_native::settings();
    let mut paint = iced_test::futures::futures::executor::block_on(iced_test::renderer::Renderer::new(settings.default_font, settings.default_text_size, Some(std::env::var("FRAME_COST_BACKEND").as_deref().unwrap_or("tiny-skia"))))
        .expect("a headless renderer");
    let theme = dossier_native::theme::theme();
    let style = renderer::Style { text_color: theme.base().text_color };
    let backdrop = dossier_native::ui::backdrop_handle();

    let mut rows = Vec::new();
    for (name, mut main) in gallery::main_states(Lang::Ru) {
        let (mut tick, mut view, mut layout, mut draw, mut open, mut present) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let start = main.now;
        let mut cache = Cache::default();
        for round in 0..=rounds + 2 {
            let took = Instant::now();
            let _ = main.update(Message::Tick(start + Duration::from_millis(16 * (round as u64 + 1))));
            let ticked = took.elapsed();
            let took = Instant::now();
            let element = gallery::main_frame(&main, &backdrop);
            let viewed = took.elapsed();
            let took = Instant::now();
            let mut ui = UserInterface::build(element, size, cache, &mut paint);
            let laid = took.elapsed();
            let took = Instant::now();
            ui.draw(&mut paint, &theme, &style, mouse::Cursor::Unavailable);
            let drawn = took.elapsed();
            let took = Instant::now();
            let _ = paint.screenshot(iced::Size::new((width * 2.0) as u32, (height * 2.0) as u32), 2.0, iced::Color::BLACK);
            let presented = took.elapsed();
            cache = ui.into_cache();
            let element = gallery::main_frame(&main, &backdrop);
            let took = Instant::now();
            let mut fresh = UserInterface::build(element, size, Cache::default(), &mut paint);
            fresh.draw(&mut paint, &theme, &style, mouse::Cursor::Unavailable);
            let opened = took.elapsed();
            drop(fresh);
            if round > 2 {
                open.push(opened);
                present.push(presented);
                tick.push(ticked);
                view.push(viewed);
                layout.push(laid);
                draw.push(drawn);
            }
        }
        rows.push((name, median(tick), median(view), median(layout), median(draw), median(open), median(present)));
    }
    rows.sort_by(|a, b| b.6.cmp(&a.6));
    println!("{:<30} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>8}   ms, median of {rounds}, {width}x{height} at 2x", "screen", "tick", "view", "layout", "draw", "frame", "open", "shot");
    for (name, tick, view, layout, draw, open, present) in rows {
        println!("{name:<30} {} {} {} {} {} {} {}", ms(tick), ms(view), ms(layout), ms(draw), ms(tick + view + layout + draw), ms(open), ms(present));
    }
}
