use std::path::PathBuf;

use dossier_native::{gallery, lang::Lang, ui};
use iced::Size;

fn main() -> Result<(), String> {
    std::env::set_var("ICED_TEST_BACKEND", "wgpu");
    let mut args = std::env::args().skip(1);
    let name = args.next().unwrap_or_else(|| "main-prefs".to_owned());
    let out = args.next().map(PathBuf::from).unwrap_or_else(|| std::env::temp_dir().join("dossier-gpu"));
    let (_, mut main) = gallery::main_states(Lang::Ru).into_iter()
        .find(|(candidate, _)| candidate == &name)
        .ok_or_else(|| format!("Unknown gallery frame: {name}"))?;
    let size = Size::new(980.0, 720.0);
    main.width = size.width;
    main.height = size.height;
    let action = args.next();
    let shot = if let Some(action) = action {
        use iced::{Event, Point, mouse, window};
        ui::set_auto_scale(ui::auto_scale_for(1080.0));
        let factor = ui::scale_of(main.settings.ui_scale);
        let viewport = ui::viewport_at(size, 1.0, factor);
        main.width = viewport.width;
        main.height = viewport.height;
        let backdrop = ui::backdrop_handle();
        let mut screen = iced_test::Simulator::with_size(dossier_native::settings(), size, ui::scaled(gallery::main_frame(&main, &backdrop), factor));
        let coords: Vec<f32> = args.map(|arg| arg.parse().expect("coordinate")).collect();
        if action == "scroll-drag" && coords.len() == 5 {
            screen.point_at(Point::new(450.0, 400.0));
            screen.simulate([Event::Mouse(mouse::Event::WheelScrolled { delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -coords[0] } })]);
            let start = std::time::Instant::now();
            for n in 0..60 {
                screen.simulate([Event::Window(window::Event::RedrawRequested(start + std::time::Duration::from_millis(n * 16)))]);
            }
            screen.point_at(Point::new(coords[1], coords[2]));
            screen.simulate([Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))]);
            let position = Point::new(coords[3], coords[4]);
            screen.point_at(position);
            screen.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
        } else if action == "drag" && coords.len() == 4 {
            screen.point_at(Point::new(coords[0], coords[1]));
            screen.simulate([Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))]);
            let position = Point::new(coords[2], coords[3]);
            screen.point_at(position);
            screen.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
        } else if action == "hover" && coords.len() == 2 {
            let position = Point::new(coords[0], coords[1]);
            screen.point_at(position);
            screen.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
            screen.simulate([Event::Window(window::Event::RedrawRequested(std::time::Instant::now()))]);
            std::thread::sleep(std::time::Duration::from_millis(700));
        } else if action == "scroll" {
            screen.point_at(Point::new(coords.get(1).copied().unwrap_or(450.0), coords.get(2).copied().unwrap_or(400.0)));
            screen.simulate([Event::Mouse(mouse::Event::WheelScrolled { delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -coords.first().copied().unwrap_or(350.0) } })]);
        } else { return Err("Use drag x y to-x to-y, hover x y, scroll pixels [x y], or scroll-drag pixels x y to-x to-y".into()); }
        let start = std::time::Instant::now();
        for n in 0..60 {
            screen.simulate([Event::Window(window::Event::RedrawRequested(start + std::time::Duration::from_millis(n * 16)))]);
        }
        screen.snapshot(&dossier_native::theme::theme()).map_err(|e| format!("{e:?}"))?
    } else {
        gallery::snapshot_main(&main, size).map_err(|error| format!("{error:?}"))?
    };
    let file = gallery::write_snapshot(&shot, &out.join(format!("{name}-ru-RU-980")))?;
    println!("{}", file.display());
    Ok(())
}
