use std::sync::{Mutex, Once};

use iced::futures::channel::mpsc::Sender;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use crate::lang::{Lang, Words};

const OPEN: &str = "dossier-open";
const QUIT: &str = "dossier-quit";
const SIDE: u32 = 64;
const MARK_SIDE: u32 = 44;
const MARGIN: f32 = 0.235;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Said {
    Show,
    Quit,
}

pub struct Tray {
    _icon: tray_icon::TrayIcon,
    open: MenuItem,
    quit: MenuItem,
    lang: Lang,
}

static OUT: Mutex<Option<Sender<Said>>> = Mutex::new(None);
static HOOKED: Once = Once::new();

fn tell(said: Said) {
    if let Some(out) = OUT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).as_mut() {
        let _ = out.try_send(said);
    }
}

pub fn heard_click(event: &TrayIconEvent) -> Option<Said> {
    if cfg!(target_os = "macos") {
        return None;
    }
    match event {
        TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } | TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } => Some(Said::Show),
        _ => None,
    }
}

pub fn heard_menu(id: &str) -> Option<Said> {
    match id {
        OPEN => Some(Said::Show),
        QUIT => Some(Said::Quit),
        _ => None,
    }
}

fn hook() {
    HOOKED.call_once(|| {
        TrayIconEvent::set_event_handler(Some(|event: TrayIconEvent| {
            if let Some(said) = heard_click(&event) {
                tell(said);
            }
        }));
        MenuEvent::set_event_handler(Some(|event: MenuEvent| {
            if let Some(said) = heard_menu(event.id.as_ref()) {
                tell(said);
            }
        }));
    });
}

pub fn events() -> impl iced::futures::Stream<Item = Said> {
    iced::stream::channel(8, async |out: Sender<Said>| {
        hook();
        *OUT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(out);
        std::future::pending::<()>().await;
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn available() -> bool {
    static SEEN: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *SEEN.get_or_init(|| {
        let asked = || -> zbus::Result<bool> {
            let bus = zbus::blocking::Connection::session()?;
            let reply = bus.call_method(Some("org.freedesktop.DBus"), "/org/freedesktop/DBus", Some("org.freedesktop.DBus"), "NameHasOwner", &("org.kde.StatusNotifierWatcher",))?;
            reply.body().deserialize::<bool>()
        };
        asked().unwrap_or(false)
    })
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
pub fn available() -> bool {
    true
}

#[cfg(target_os = "macos")]
pub fn dock(shown: bool) {
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
    let Some(main_thread) = objc2::MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(main_thread);
    let policy = if shown { NSApplicationActivationPolicy::Regular } else { NSApplicationActivationPolicy::Accessory };
    let _ = app.setActivationPolicy(policy);
}

#[cfg(not(target_os = "macos"))]
pub fn dock(_shown: bool) {}

pub fn template(mask: &image::GrayImage) -> Option<(Vec<u8>, u32)> {
    let (width, height) = mask.dimensions();
    let inked = |x: u32, y: u32| mask.get_pixel(x, y)[0] > 16;
    let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
    for y in 0..height {
        for x in 0..width {
            if inked(x, y) {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
            }
        }
    }
    if right < left || bottom < top {
        return None;
    }
    let side = (right - left).max(bottom - top) + 1;
    let pad = (side as f32 * MARGIN).round() as u32;
    let square = side + 2 * pad;
    let mut framed = image::GrayImage::new(square, square);
    let (off_x, off_y) = (pad + (side - (right - left + 1)) / 2, pad + (side - (bottom - top + 1)) / 2);
    for y in top..=bottom {
        for x in left..=right {
            framed.put_pixel(off_x + x - left, off_y + y - top, *mask.get_pixel(x, y));
        }
    }
    let small = image::imageops::resize(&framed, MARK_SIDE, MARK_SIDE, image::imageops::FilterType::Lanczos3);
    Some((small.pixels().flat_map(|level| [0, 0, 0, level[0]]).collect(), MARK_SIDE))
}

fn picture() -> Option<tray_icon::Icon> {
    let (rgba, side) = if cfg!(target_os = "macos") {
        let mask = image::load_from_memory(include_bytes!("../assets/letter-mask.png")).ok()?.to_luma8();
        template(&mask)?
    } else {
        let full = image::load_from_memory(include_bytes!("../assets/icon/dossier-256.png")).ok()?.to_rgba8();
        (image::imageops::resize(&full, SIDE, SIDE, image::imageops::FilterType::Lanczos3).into_raw(), SIDE)
    };
    tray_icon::Icon::from_rgba(rgba, side, side).ok()
}

impl Tray {
    pub fn new(words: &Words) -> Option<Tray> {
        if !available() {
            return None;
        }
        hook();
        let open = MenuItem::with_id(OPEN, words.t("tray-open"), true, None);
        let quit = MenuItem::with_id(QUIT, words.t("tray-quit"), true, None);
        let menu = Menu::with_items(&[&open, &PredefinedMenuItem::separator(), &quit]).ok()?;
        let builder = TrayIconBuilder::new()
            .with_id("dossier")
            .with_tooltip("Dossier")
            .with_menu_on_left_click(cfg!(target_os = "macos"))
            .with_menu(Box::new(menu));
        #[cfg(target_os = "macos")]
        let builder = builder.with_icon_templated(picture()?);
        #[cfg(not(target_os = "macos"))]
        let builder = builder.with_icon(picture()?);
        let icon = builder.build().ok()?;
        Some(Tray { _icon: icon, open, quit, lang: words.lang() })
    }

    pub fn speak(&mut self, words: &Words) {
        if self.lang == words.lang() {
            return;
        }
        self.lang = words.lang();
        self.open.set_text(words.t("tray-open"));
        self.quit.set_text(words.t("tray-quit"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_opens_the_window_or_quits_and_nothing_else_does() {
        assert_eq!(heard_menu(OPEN), Some(Said::Show));
        assert_eq!(heard_menu(QUIT), Some(Said::Quit));
        assert_eq!(heard_menu("something-else"), None);
    }

    #[test]
    fn the_menu_bar_mark_is_the_letter_alone_in_black_on_clear() {
        let mask = image::load_from_memory(include_bytes!("../assets/letter-mask.png")).unwrap().to_luma8();
        let (rgba, side) = template(&mask).expect("a mark");
        assert_eq!((side, rgba.len()), (MARK_SIDE, (MARK_SIDE * MARK_SIDE * 4) as usize));
        assert!(rgba.chunks(4).all(|pixel| pixel[..3] == [0, 0, 0]), "a template image is black and alpha only");
        let alpha = |x: u32, y: u32| rgba[((y * MARK_SIDE + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0, "the corner is clear");
        let inked: Vec<u32> = (0..MARK_SIDE).filter(|y| (0..MARK_SIDE).any(|x| alpha(x, *y) > 128)).collect();
        let tall = inked[inked.len() - 1] - inked[0] + 1;
        assert!((28..=32).contains(&tall), "the letter should stand about 15 of the menu bar's 22 points, not {tall} of {MARK_SIDE} pixels");
    }

    #[test]
    fn a_click_opens_the_window_where_the_click_is_not_the_menu() {
        let click = TrayIconEvent::Click {
            id: tray_icon::TrayIconId::new("dossier"),
            position: tray_icon::dpi::PhysicalPosition::new(0.0, 0.0),
            rect: tray_icon::Rect::default(),
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
        };
        let expected = if cfg!(target_os = "macos") { None } else { Some(Said::Show) };
        assert_eq!(heard_click(&click), expected);
        let right = TrayIconEvent::Click {
            id: tray_icon::TrayIconId::new("dossier"),
            position: tray_icon::dpi::PhysicalPosition::new(0.0, 0.0),
            rect: tray_icon::Rect::default(),
            button: MouseButton::Right,
            button_state: MouseButtonState::Up,
        };
        assert_eq!(heard_click(&right), None, "a right click belongs to the menu");
    }
}
