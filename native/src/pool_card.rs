use std::collections::HashMap;
use std::path::{Path, PathBuf};

use iced::advanced::{layout, renderer, widget::Tree, Layout};
use iced::advanced::renderer::{Headless, Renderer as _};
use iced::widget::{column, container, image, row, text, Space};
use iced::{Background, Border, Color, Element, Length, Rectangle, Size};

use crate::lang::Words;
use crate::pools::{Pool, Slot};
use crate::theme::{self, FAINT, INK, MUTED};

const WIDTH: f32 = 1080.0;
const TILE_WIDTH: f32 = 328.0;
const TILE_HEIGHT: f32 = 204.0;
const GAP: f32 = 12.0;
const PADDING: f32 = 36.0;
const COLUMNS: usize = 3;
const MAX_PIXELS: u64 = 32_000_000;
const BACKING_HIGH: f32 = 300.0;

fn size(pool: &Pool) -> Result<Size, String> {
    let rows = pool.slots.len().max(1).div_ceil(COLUMNS);
    let height = 232.0 + rows as f32 * (TILE_HEIGHT + GAP);
    if (WIDTH * 2.0) as u64 * (height * 2.0) as u64 > MAX_PIXELS {
        return Err("Pool image is too large".into());
    }
    Ok(Size::new(WIDTH, height))
}

fn line<'a>(value: String, font: iced::Font, size: f32, colour: Color) -> Element<'a, ()> {
    container(text(value).font(font).size(size).color(colour).wrapping(text::Wrapping::None).width(Length::Fill)).width(Length::Fill).clip(true).into()
}

fn tile<'a>(slot: &'a Slot, at: usize, words: &'a Words, covers: &'a HashMap<String, image::Handle>) -> Element<'a, ()> {
    let cover: Element<'a, ()> = match slot.hash.as_ref().and_then(|hash| covers.get(hash)) {
        Some(handle) => image(crate::crops::fitted(handle, TILE_WIDTH - 24.0, 82.0, 8.0)).content_fit(iced::ContentFit::Fill).width(TILE_WIDTH - 24.0).height(82.0).into(),
        None => container(crate::ui::fine_hatch()).width(TILE_WIDTH - 24.0).height(82.0).clip(true).style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.018))),
            border: Border { radius: 8.0.into(), ..Border::default() },
            ..container::Style::default()
        }).into(),
    };
    let stars = match slot.measure {
        Some(measure) => text(format!("{} ★", crate::community_screen::decimal(words, measure.stars as f32, 2))).font(theme::MONO_BOLD).size(18.0).color(crate::dossier::star_colour(measure.stars as f32)),
        None => text("—").font(theme::MONO).size(18.0).color(FAINT),
    };
    let heading = row![text((at + 1).to_string()).font(theme::MONO_BOLD).size(14).color(INK), text(slot.mods.code()).font(theme::MONO_BOLD).size(12).color(MUTED), Space::new().width(Length::Fill), stars].spacing(10).align_y(iced::Center);
    let title = if slot.is_empty() { words.t("pool-slot-empty") } else if slot.title.trim().is_empty() { slot.hash.clone().unwrap_or_default() } else { slot.title.clone() };
    let subtitle = if slot.is_empty() { words.t("pool-slot-need") } else { slot.version.clone() };
    let mut bottom = row![line(subtitle, theme::SANS, 11.5, FAINT)].spacing(12).align_y(iced::Center);
    if let Some(measure) = slot.measure {
        bottom = bottom.push(text(format!("{:.0} BPM", measure.bpm)).font(theme::MONO).size(10.5).color(MUTED))
            .push(text(crate::pools::clock(measure.length_ms)).font(theme::MONO).size(10.5).color(MUTED));
    }
    container(column![heading, cover, line(title, theme::SANS_SEMI, 15.0, INK), line(slot.artist.clone(), theme::SANS, 11.5, MUTED), bottom].spacing(6))
        .padding(12).width(TILE_WIDTH).height(TILE_HEIGHT).clip(true).style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgb(0.073, 0.034, 0.043))),
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.09), width: 1.0, radius: 12.0.into() },
            ..container::Style::default()
        }).into()
}

pub fn view<'a>(pool: &'a Pool, words: &'a Words, covers: &'a HashMap<String, image::Handle>, backing: Option<&image::Handle>) -> Element<'a, ()> {
    let name = if pool.name.trim().is_empty() { words.t("pool-untitled") } else { pool.name.clone() };
    let header = row![image(crate::ui::letter_red().clone()).width(22).height(26), text("Dossier").font(theme::SANS_SEMI).size(18).color(INK)].spacing(12).align_y(iced::Center);
    let title = container(text(name).font(theme::SANS_SEMI).size(30).color(INK).wrapping(text::Wrapping::None).width(Length::Fill)).height(40).clip(true);
    let frame = text(words.t(pool.frame.key())).font(theme::SANS).size(12).color(FAINT);
    let mut grid = column![].spacing(GAP);
    for (offset, slots) in pool.slots.chunks(COLUMNS).enumerate() {
        let mut band = row![].spacing(GAP);
        for (at, slot) in slots.iter().enumerate() {
            band = band.push(tile(slot, offset * COLUMNS + at, words, covers));
        }
        grid = grid.push(band);
    }
    let balance = match pool.balance() {
        Some(crate::pools::Balance::Heavy(skill, percent)) => format!("{}  {percent}%", words.t(skill.outweighs_key())),
        Some(crate::pools::Balance::Even) => words.t("pool-even"),
        None => words.t("pool-image-no-measures"),
    };
    let footer = container(row![line(balance, theme::SANS, 12.0, FAINT), text(pool.fingerprint()).font(theme::MONO).size(12).color(FAINT)].spacing(16).align_y(iced::Center)).padding(iced::Padding::ZERO.top(8));
    let page = container(column![header, title, frame, grid, footer].spacing(16)).padding(PADDING).width(Length::Fill).height(Length::Fill);
    let ground = container(match pool.backdrop.as_ref().filter(|chosen| chosen.on_card).zip(backing) {
        Some((chosen, picture)) => {
            let base = theme::GROUND.into_rgba8();
            Element::from(column![image(crate::crops::sunk(picture, WIDTH, BACKING_HIGH, chosen.dim, [base[0], base[1], base[2]])).content_fit(iced::ContentFit::Fill).width(Length::Fill).height(BACKING_HIGH), Space::new().height(Length::Fill)])
        }
        None => Space::new().into(),
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| container::Style { background: Some(Background::Color(theme::GROUND)), ..container::Style::default() });
    iced::widget::stack![ground, page].into()
}

pub fn render(pool: &Pool, words: &Words, covers: &HashMap<String, image::Handle>, backing: Option<&image::Handle>) -> Result<Vec<u8>, String> {
    let size = size(pool)?;
    let settings = crate::gallery::settings_once();
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(settings.default_font, settings.default_text_size, Some("tiny-skia"))).ok_or("Could not create image renderer")?;
    let mut element = crate::ui::fading(1.0, || view(pool, words, covers, backing));
    let mut tree = Tree::new(&element);
    let node = element.as_widget_mut().layout(&mut tree, &renderer, &layout::Limits::new(size, size));
    let viewport = Rectangle::with_size(size);
    renderer.reset(viewport);
    element.as_widget().draw(&tree, &mut renderer, &theme::theme(), &renderer::Style { text_color: INK }, Layout::new(&node), iced::mouse::Cursor::Unavailable, &viewport);
    let physical = Size::new((size.width * 2.0) as u32, (size.height * 2.0) as u32);
    let pixels = renderer.screenshot(physical, 2.0, theme::GROUND);
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, physical.width, physical.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|why| why.to_string())?;
        writer.write_image_data(&pixels).map_err(|why| why.to_string())?;
    }
    Ok(bytes)
}

pub fn covers(pool: &Pool, paths: &HashMap<String, PathBuf>, cached: &HashMap<String, image::Handle>) -> HashMap<String, image::Handle> {
    pool.slots.iter().filter_map(|slot| {
        let hash = slot.hash.as_ref()?;
        let fresh = paths.get(hash).and_then(|path| crate::main_screen::decoded(path, 608, Some((608, 164))));
        fresh.or_else(|| cached.get(hash).cloned()).map(|image| (hash.clone(), image))
    }).collect()
}

pub fn file_name(pool: &Pool) -> String {
    let name = crate::maps::tidy(&pool.name);
    format!("{}.png", if name.is_empty() { "pool".into() } else { name })
}

pub fn save(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    struct Staging(PathBuf);
    impl Drop for Staging { fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); } }
    let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let staged = Staging(parent.join(format!(".dossier-pool-image-{}-{serial}.part", std::process::id())));
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&staged.0).map_err(|why| why.to_string())?;
    file.write_all(bytes).map_err(|why| why.to_string())?;
    file.sync_all().map_err(|why| why.to_string())?;
    drop(file);
    std::fs::rename(&staged.0, path).map_err(|why| why.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::Lang;
    use crate::pools::{Frame, Mod};

    #[test]
    fn exported_cards_include_every_row_keep_a_clear_border_and_render_both_languages() {
        let (pools, _, covers) = crate::gallery::pool_sample();
        let covers: HashMap<_, _> = covers.into_iter().collect();
        for lang in Lang::ALL {
            for (kind, pool) in ["duel", "stage", "free"].into_iter().zip(&pools) {
                let bytes = render(pool, &Words::new(lang), &covers, None).unwrap();
                let decoder = png::Decoder::new(bytes.as_slice());
                let mut reader = decoder.read_info().unwrap();
                let mut pixels = vec![0; reader.output_buffer_size()];
                let info = reader.next_frame(&mut pixels).unwrap();
                let expected = size(pool).unwrap();
                assert_eq!((info.width, info.height), ((expected.width * 2.0) as u32, (expected.height * 2.0) as u32));
                assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] == 255));
                assert!(pixels[..info.width as usize * 4 * 8].chunks_exact(4).all(|pixel| pixel == [13, 5, 8, 255]));
                assert!(pixels[pixels.len() - info.width as usize * 4 * 8..].chunks_exact(4).all(|pixel| pixel == [13, 5, 8, 255]), "last row reaches the edge");
                if let Some(dir) = std::env::var_os("DOSSIER_POOL_IMAGE_REVIEW").map(PathBuf::from) {
                    std::fs::create_dir_all(&dir).unwrap();
                    std::fs::write(dir.join(format!("pool-card-{kind}-{}.png", lang.tag())), bytes).unwrap();
                }
            }
        }
    }

    #[test]
    fn the_pool_backdrop_stands_behind_the_head_of_the_picture_only_when_the_pool_asks_for_it() {
        let (pools, _, covers) = crate::gallery::pool_sample();
        let covers: HashMap<_, _> = covers.into_iter().collect();
        let words = Words::new(Lang::En);
        let decode = |bytes: Vec<u8>| {
            let mut reader = png::Decoder::new(bytes.as_slice()).read_info().unwrap();
            let mut pixels = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut pixels).unwrap();
            (info.width as usize, pixels)
        };
        let white = image::Handle::from_rgba(640, 360, [255u8, 255, 255, 255].repeat(640 * 360));
        let mut pool = pools[0].clone();
        pool.backdrop = Some(crate::pools::Backdrop { from: crate::pools::BackdropFrom::Map("x".into()), dim: 60, blur: false, on_card: true });
        let (width, backed) = decode(render(&pool, &words, &covers, Some(&white)).unwrap());
        let middle = ((BACKING_HIGH as usize / 2) * width + width / 2) * 4;
        assert_eq!(&backed[middle..middle + 4], &[110, 105, 107, 255], "the top shows the picture under the chosen darkening");
        let low = (BACKING_HIGH as usize * 2 + 8) * width * 4 + 8 * 4;
        assert_eq!(&backed[low..low + 4], &[13, 5, 8, 255], "below the head the ground is clean");
        let plain = render(&pools[0], &words, &covers, None).unwrap();
        pool.backdrop.as_mut().unwrap().on_card = false;
        assert_eq!(render(&pool, &words, &covers, Some(&white)).unwrap(), plain, "switched off, the picture is the usual one");
    }

    #[test]
    fn long_labels_and_oversized_covers_cannot_paint_outside_their_tile() {
        let (pools, _, covers) = crate::gallery::pool_sample();
        let mut covers: HashMap<_, _> = covers.into_iter().collect();
        let words = Words::new(Lang::En);
        let decode = |bytes: Vec<u8>| {
            let mut reader = png::Decoder::new(bytes.as_slice()).read_info().unwrap();
            let mut pixels = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut pixels).unwrap();
            (info.width as usize, pixels)
        };
        let (width, before) = decode(render(&pools[0], &words, &covers, None).unwrap());
        let mut pool = pools[0].clone();
        pool.slots[0].title = "A long title ".repeat(60);
        pool.slots[0].artist = "An artist name ".repeat(60);
        pool.slots[0].version = "A difficulty name ".repeat(60);
        let pixels = [255u8, 20, 10, 255].repeat(900 * 100);
        covers.insert(pool.slots[0].hash.clone().unwrap(), image::Handle::from_rgba(900, 100, pixels));
        let (_, after) = decode(render(&pool, &words, &covers, None).unwrap());
        for (at, (before, after)) in before.chunks_exact(4).zip(after.chunks_exact(4)).enumerate() {
            let (x, y) = (at % width, at / width);
            if !(72..728).contains(&x) || !(320..760).contains(&y) {
                assert_eq!(before, after, "content escaped its tile at {x}, {y}");
            }
        }
    }

    #[test]
    fn images_too_large_to_render_are_refused_before_allocating_pixels() {
        let mut pool = Pool::new(Frame::Free, "Large", 0);
        pool.slots = vec![Slot::empty(Mod::Nm); 1000];
        assert!(render(&pool, &Words::new(Lang::En), &HashMap::new(), None).is_err());
    }

    #[test]
    fn missing_local_covers_use_cached_images_and_absent_covers_stay_empty() {
        let (pools, _, images) = crate::gallery::pool_sample();
        let cached: HashMap<_, _> = images.into_iter().take(1).collect();
        let hash = pools[0].slots[0].hash.clone().unwrap();
        let paths = HashMap::from([(hash.clone(), PathBuf::from("/missing/cover.png"))]);
        let found = covers(&pools[0], &paths, &cached);
        assert_eq!(found.len(), 1);
        assert_eq!(found[&hash].id(), cached[&hash].id());
    }

    #[test]
    fn saving_replaces_the_whole_file_and_cleans_up_after_a_failed_rename() {
        let dir = std::env::temp_dir().join(format!("dossier-pool-image-save-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("pool.png");
        std::fs::write(&target, b"broken").unwrap();
        save(&target, b"complete image").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"complete image");
        let blocked = dir.join("folder");
        std::fs::create_dir(&blocked).unwrap();
        assert!(save(&blocked, b"another image").is_err());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        assert_eq!(std::fs::read(&target).unwrap(), b"complete image");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
