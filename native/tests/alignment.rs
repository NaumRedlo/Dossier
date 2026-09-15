use dossier_native::gallery;
use dossier_native::lang::Lang;
use iced::{Rectangle, Size};
use iced_test::Simulator;

fn frame_of(name: &str, lang: Lang) -> Simulator<'static, dossier_native::Message> {
    let (_, flow) = gallery::states(lang).into_iter().find(|(n, _)| n == name).expect("a staged state");
    let backdrop = dossier_native::ui::backdrop_handle();
    let flow: &'static _ = Box::leak(Box::new(flow));
    let backdrop: &'static _ = Box::leak(Box::new(backdrop));
    Simulator::with_size(dossier_native::settings(), Size::new(980.0, 720.0), gallery::frame(flow, backdrop))
}

fn bounds(ui: &mut Simulator<'static, dossier_native::Message>, label: &str) -> Rectangle {
    ui.find(label).unwrap_or_else(|_| panic!("no text {label:?}")).bounds()
}

fn middle(r: Rectangle) -> f32 {
    r.y + r.height / 2.0
}

fn same(a: f32, b: f32, what: &str) {
    assert!((a - b).abs() <= 0.75, "{what}: {a} against {b}");
}

#[test]
fn the_brand_and_the_headline_sit_on_the_axis_of_the_card() {
    for lang in Lang::ALL {
        let mut ui = frame_of("folder-stable", lang);
        let word = bounds(&mut ui, "Dossier");
        let head = if lang == Lang::En { bounds(&mut ui, "Setting up") } else { bounds(&mut ui, "Настройка") };
        let count = if lang == Lang::En { bounds(&mut ui, "· 2 of 4") } else { bounds(&mut ui, "· 2 из 4") };
        same(middle(head), middle(count), "headline and its count share a middle");
        let axis = 980.0 / 2.0;
        let headline_span = (head.x, count.x + count.width);
        let headline_centre = (headline_span.0 + headline_span.1) / 2.0;
        assert!((headline_centre - axis).abs() < 12.0, "headline centre {headline_centre} is off the axis {axis}");
        let brand_centre = word.x + word.width / 2.0;
        assert!(brand_centre > axis, "the word sits to the right of the axis, the letter to the left: {brand_centre}");
    }
}

#[test]
fn every_ledger_name_starts_on_the_same_line() {
    for lang in Lang::ALL {
        let mut ui = frame_of("device", lang);
        let names: Vec<&str> = match lang {
            Lang::En => vec!["Language", "osu! folder", "The bot"],
            Lang::Ru => vec!["Язык", "Папка osu!", "Бот"],
        };
        let lefts: Vec<f32> = names.iter().map(|n| bounds(&mut ui, n).x).collect();
        for left in &lefts {
            same(*left, lefts[0], "ledger names share a left edge");
        }
        let rows: Vec<f32> = names.iter().map(|n| middle(bounds(&mut ui, n))).collect();
        same(rows[1] - rows[0], 28.0, "ledger rows are 28 px apart");
    }
}

#[test]
fn the_reason_the_path_and_the_ledger_hang_off_one_left_edge() {
    for lang in Lang::ALL {
        let mut ui = frame_of("folder-stable", lang);
        let (why, ledger, path) = match lang {
            Lang::En => (bounds(&mut ui, "Found on this device."), bounds(&mut ui, "Language"), bounds(&mut ui, "~/osu")),
            Lang::Ru => (bounds(&mut ui, "Нашлась на этом устройстве."), bounds(&mut ui, "Язык"), bounds(&mut ui, "~/osu")),
        };
        same(ledger.x - why.x, 26.0, "a ledger name sits one glyph and one gap in from the content edge");
        same(path.x - why.x, 12.0, "the path sits one control inset in from the content edge");
    }
}

#[test]
fn the_three_tiles_line_up() {
    for lang in Lang::ALL {
        let mut ui = frame_of("folder-stable", lang);
        let values = ["14", "187"];
        let a = bounds(&mut ui, values[0]);
        let b = bounds(&mut ui, values[1]);
        same(a.y, b.y, "tile values share a top");
        let (la, lb) = match lang {
            Lang::En => (bounds(&mut ui, "skins"), bounds(&mut ui, "replays")),
            Lang::Ru => (bounds(&mut ui, "скинов"), bounds(&mut ui, "реплеев")),
        };
        same(la.y, lb.y, "tile labels share a top");
        same(la.x - a.x, 0.0, "a tile's label starts under its value");
    }
}

#[test]
fn the_buttons_of_a_row_share_a_middle() {
    for lang in Lang::ALL {
        let mut ui = frame_of("folder-stable", lang);
        let (back, add, go) = match lang {
            Lang::En => (bounds(&mut ui, "Back"), bounds(&mut ui, "Add another…"), bounds(&mut ui, "Use this")),
            Lang::Ru => (bounds(&mut ui, "Назад"), bounds(&mut ui, "Добавить ещё…"), bounds(&mut ui, "Взять эту")),
        };
        same(middle(back), middle(add), "quiet buttons share a middle");
        same(middle(add), middle(go), "the primary button shares it too");
        assert!(go.x + go.width > add.x + add.width, "the primary button is the rightmost");
    }
}

#[test]
fn the_waiting_line_and_the_code_stack_under_the_caption() {
    for lang in Lang::ALL {
        let mut ui = frame_of("bot-waiting", lang);
        let (cap, code, open, waiting) = match lang {
            Lang::En => (bounds(&mut ui, "Code · 10 minutes"), bounds(&mut ui, "K7QN-M4XZ"), bounds(&mut ui, "Open Telegram"), bounds(&mut ui, "Waiting for Telegram…")),
            Lang::Ru => (bounds(&mut ui, "Код · 10 минут"), bounds(&mut ui, "K7QN-M4XZ"), bounds(&mut ui, "Открыть Telegram"), bounds(&mut ui, "Жду Telegram…")),
        };
        same(cap.x, code.x, "the code starts under its caption");
        assert!((open.x - cap.x - 12.0).abs() < 1.0, "the button label is one inset in: {} against {}", open.x, cap.x);
        assert!(waiting.x > cap.x, "the waiting line follows its dot");
        assert!(cap.y < code.y && code.y < open.y && open.y < waiting.y, "caption, code, button, waiting — in that order");
    }
}
