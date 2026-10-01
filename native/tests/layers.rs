use iced::widget::{canvas, container, stack, Space};
use iced::{mouse, Background, Color, Element, Length, Padding, Point, Rectangle, Renderer, Size, Theme};
use iced_test::Simulator;

const WIDTH: f32 = 200.0;
const HEIGHT: f32 = 100.0;

struct Fill {
    colour: Color,
    from: f32,
    to: f32,
}

impl canvas::Program<()> for Fill {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::new(bounds.width * self.from, 0.0), Size::new(bounds.width * (self.to - self.from), bounds.height), self.colour);
        vec![frame.into_geometry()]
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Made {
    Meshes,
    Quads,
}

fn layer(made: Made, colour: Color, from: f32, to: f32) -> Element<'static, ()> {
    match made {
        Made::Meshes => canvas(Fill { colour, from, to }).width(Length::Fill).height(Length::Fill).into(),
        Made::Quads => {
            let padding = Padding { left: WIDTH * from, right: WIDTH * (1.0 - to), ..Padding::ZERO };
            container(
                container(Space::new()).width(Length::Fill).height(Length::Fill).style(move |_| container::Style { background: Some(Background::Color(colour)), ..container::Style::default() }),
            )
            .padding(padding)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        }
    }
}

fn scene(made: Made) -> Element<'static, ()> {
    let wash = |colour: Color| layer(Made::Quads, colour, 0.0, 1.0);
    stack![
        wash(Color::from_rgb(0.2, 0.2, 0.2)),
        layer(made, Color::from_rgba(1.0, 0.0, 0.0, 0.5), 0.0, 1.0),
        wash(Color::from_rgba(0.0, 0.0, 1.0, 0.5)),
        layer(made, Color::from_rgba(0.0, 1.0, 0.0, 0.5), 0.0, 0.5),
        layer(made, Color::from_rgba(1.0, 1.0, 0.0, 0.5), 0.5, 1.0),
        wash(Color::from_rgba(1.0, 1.0, 1.0, 0.25)),
        layer(made, Color::from_rgba(0.0, 0.0, 0.0, 0.5), 0.0, 1.0),
        layer(made, Color::from_rgba(0.0, 1.0, 1.0, 0.5), 0.5, 1.0),
        wash(Color::from_rgba(1.0, 0.0, 1.0, 0.25)),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn pixels(made: Made) -> (u32, Vec<u8>) {
    let stem = std::env::temp_dir().join(format!("dossier-layers-{}-{}", std::process::id(), if made == Made::Meshes { "meshes" } else { "quads" }));
    let mut sim = Simulator::with_size(dossier_native::settings(), Size::new(WIDTH, HEIGHT), scene(made));
    let shot = sim.snapshot(&dossier_native::theme::theme()).expect("a frame");
    let file = dossier_native::gallery::write_snapshot(&shot, &stem).expect("written");
    let bytes = std::fs::read(&file).expect("the frame");
    let _ = std::fs::remove_file(&file);
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().expect("a png");
    let mut found = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut found).expect("pixels");
    found.truncate(info.buffer_size());
    (info.width, found)
}

#[test]
fn meshes_in_stacked_layers_land_where_quads_of_the_same_order_would() {
    let (width, meshes) = pixels(Made::Meshes);
    let (_, quads) = pixels(Made::Quads);
    assert_eq!(meshes.len(), quads.len());
    let at = |pixels: &[u8], x: u32, y: u32| {
        let start = ((y * width + x) * 4) as usize;
        [pixels[start], pixels[start + 1], pixels[start + 2]]
    };
    let (y, left, right) = (HEIGHT as u32, WIDTH as u32 / 2, WIDTH as u32 * 3 / 2);
    assert_ne!(at(&quads, left, y), at(&quads, right, y), "the two halves differ in the reference");
    let worst = meshes
        .chunks(4)
        .zip(quads.chunks(4))
        .map(|(a, b)| a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0))
        .max()
        .unwrap_or(0);
    assert!(worst <= 4, "a mesh pixel is {worst} away from the quad drawn in the same order");
    for (x, label) in [(left, "left"), (right, "right")] {
        let (a, b) = (at(&meshes, x, y), at(&quads, x, y));
        assert!(a.iter().zip(b).all(|(a, b)| a.abs_diff(b) <= 4), "{label} half: {a:?} against {b:?}");
    }
}
