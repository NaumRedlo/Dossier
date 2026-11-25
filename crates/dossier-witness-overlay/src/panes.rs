use std::collections::HashMap;
use std::ffi::c_void;

use dossier_hud::hold::{Chord, Keys};
use dossier_hud::Sprite;

const POPUP: u32 = 0x8000_0000;
const LAYERED: u32 = 0x0008_0000;
const TRANSPARENT: u32 = 0x0000_0020;
const TOPMOST: u32 = 0x0000_0008;
const NO_ACTIVATE: u32 = 0x0800_0000;
const TOOL_WINDOW: u32 = 0x0000_0080;
const HIDE: i32 = 0;
const SHOW_NO_ACTIVATE: i32 = 4;
const BY_ALPHA: u32 = 2;
const REMOVE: u32 = 1;
const ABOVE: u32 = 3;
const ON_TOP: isize = -1;
const KEEP_PLACE: u32 = 0x0001 | 0x0002 | 0x0010 | 0x0200;
const LOOKED_AT_MOST: usize = 4096;
const TAB: i32 = 0x09;
const OTHERS: [i32; 5] = [0x10, 0x11, 0x12, 0x5B, 0x5C];
const SHIFT: i32 = 0x10;
const CTRL: i32 = 0x11;
const ALT: i32 = 0x12;
const WINDOWS: [i32; 2] = [0x5B, 0x5C];
const HIDE_KEY: i32 = 0x48;
const BOARD_KEY: i32 = 0x44;
const PROFILE_KEY: i32 = 0x50;
const BIGGER: [i32; 2] = [0xBB, 0x6B];
const SMALLER: [i32; 2] = [0xBD, 0x6D];
const LEFT_BUTTON: i32 = 0x01;
const RIGHT_BUTTON: i32 = 0x02;
const EX_STYLE: i32 = -20;
const NO_SIZE_NO_ORDER_NO_ACTIVATE: u32 = 0x0001 | 0x0004 | 0x0010;
const KEPT_IN_SIGHT: i32 = 48;
const LAYOUT_FILE: &str = "witness-overlay-layout.txt";

#[repr(C)]
#[derive(Default)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Size {
    wide: i32,
    high: i32,
}

#[repr(C)]
#[derive(Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct Blend {
    operation: u8,
    flags: u8,
    alpha: u8,
    format: u8,
}

#[repr(C)]
#[derive(Default)]
struct Posted {
    window: isize,
    message: u32,
    first: usize,
    second: isize,
    time: u32,
    point: Point,
}

#[repr(C)]
#[derive(Default)]
struct Picture {
    size: u32,
    wide: i32,
    high: i32,
    planes: u16,
    bits: u16,
    compression: u32,
    image: u32,
    across: i32,
    down: i32,
    used: u32,
    important: u32,
    colours: [u32; 3],
}

#[link(name = "user32")]
extern "system" {
    fn CreateWindowExW(extended: u32, class: *const u16, title: *const u16, style: u32, x: i32, y: i32, wide: i32, high: i32, parent: isize, menu: isize, instance: isize, given: *const c_void) -> isize;
    fn DestroyWindow(window: isize) -> i32;
    fn ShowWindow(window: isize, how: i32) -> i32;
    fn UpdateLayeredWindow(window: isize, onto: isize, at: *const Point, size: *const Size, from: isize, origin: *const Point, key: u32, blend: *const Blend, flags: u32) -> i32;
    fn GetDC(window: isize) -> isize;
    fn ReleaseDC(window: isize, context: isize) -> i32;
    fn PeekMessageW(posted: *mut Posted, window: isize, least: u32, most: u32, how: u32) -> i32;
    fn TranslateMessage(posted: *const Posted) -> i32;
    fn DispatchMessageW(posted: *const Posted) -> isize;
    fn GetForegroundWindow() -> isize;
    fn GetAsyncKeyState(key: i32) -> i16;
    fn GetClientRect(window: isize, rect: *mut Rect) -> i32;
    fn ClientToScreen(window: isize, point: *mut Point) -> i32;
    fn EnumWindows(each: extern "system" fn(isize, isize) -> i32, given: isize) -> i32;
    fn GetWindowThreadProcessId(window: isize, pid: *mut u32) -> u32;
    fn IsWindowVisible(window: isize) -> i32;
    fn SetProcessDPIAware() -> i32;
    fn GetWindow(window: isize, relation: u32) -> isize;
    fn GetCursorPos(point: *mut Point) -> i32;
    fn GetWindowLongPtrW(window: isize, index: i32) -> isize;
    fn SetWindowLongPtrW(window: isize, index: i32, value: isize) -> isize;
    fn SetWindowPos(window: isize, after: isize, x: i32, y: i32, wide: i32, high: i32, flags: u32) -> i32;
}

fn lies_above(window: isize, other: isize) -> bool {
    let mut next = window;
    for _ in 0..LOOKED_AT_MOST {
        next = unsafe { GetWindow(next, ABOVE) };
        if next == 0 {
            return false;
        }
        if next == other {
            return true;
        }
    }
    false
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(context: isize) -> isize;
    fn DeleteDC(context: isize) -> i32;
    fn CreateDIBSection(context: isize, picture: *const Picture, usage: u32, bits: *mut *mut u8, section: isize, offset: u32) -> isize;
    fn SelectObject(context: isize, object: isize) -> isize;
    fn DeleteObject(object: isize) -> i32;
}

struct Search {
    pid: u32,
    window: isize,
    area: i64,
}

extern "system" fn largest(window: isize, given: isize) -> i32 {
    let search = unsafe { &mut *(given as *mut Search) };
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(window, &mut pid) };
    if pid == search.pid && unsafe { IsWindowVisible(window) } != 0 {
        if let Some((_, _, wide, high)) = area(window) {
            let area = i64::from(wide) * i64::from(high);
            if area > search.area {
                (search.window, search.area) = (window, area);
            }
        }
    }
    1
}

pub fn game_window(pid: u32) -> Option<isize> {
    let mut search = Search { pid, window: 0, area: 0 };
    unsafe { EnumWindows(largest, &mut search as *mut Search as isize) };
    (search.area > 0).then_some(search.window)
}

pub fn area(window: isize) -> Option<(i32, i32, i32, i32)> {
    let mut rect = Rect::default();
    let mut corner = Point::default();
    if unsafe { GetClientRect(window, &mut rect) } == 0 || unsafe { ClientToScreen(window, &mut corner) } == 0 {
        return None;
    }
    (rect.right > 0 && rect.bottom > 0).then_some((corner.x, corner.y, rect.right, rect.bottom))
}

pub fn chord() -> Chord {
    let down = |key: i32| unsafe { GetAsyncKeyState(key) } < 0;
    Chord {
        ctrl_shift: down(CTRL) && down(SHIFT) && !down(ALT) && !WINDOWS.into_iter().any(down),
        hide: down(HIDE_KEY),
        board: down(BOARD_KEY),
        profile: down(PROFILE_KEY),
        bigger: BIGGER.into_iter().any(down),
        smaller: SMALLER.into_iter().any(down),
    }
}

pub fn keys(window: isize) -> Keys {
    let down = |key: i32| unsafe { GetAsyncKeyState(key) } < 0;
    Keys { key: down(TAB), other: OTHERS.into_iter().any(down), front: unsafe { GetForegroundWindow() } == window }
}

struct Pane {
    window: isize,
    memory: isize,
    bitmap: isize,
    bits: *mut u8,
    size: (u32, u32),
    at: (i32, i32),
    pixels: Vec<u8>,
    base: (i32, i32),
    cross: Option<(i32, i32, i32)>,
    shown: bool,
}

impl Pane {
    fn new(screen: isize) -> Option<Pane> {
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let title: Vec<u16> = "Dossier Witness\0".encode_utf16().collect();
        let window = unsafe { CreateWindowExW(LAYERED | TRANSPARENT | TOPMOST | NO_ACTIVATE | TOOL_WINDOW, class.as_ptr(), title.as_ptr(), POPUP, 0, 0, 1, 1, 0, 0, 0, std::ptr::null()) };
        if window == 0 {
            return None;
        }
        let memory = unsafe { CreateCompatibleDC(screen) };
        Some(Pane { window, memory, bitmap: 0, bits: std::ptr::null_mut(), size: (0, 0), at: (0, 0), pixels: Vec::new(), base: (0, 0), cross: None, shown: false })
    }

    fn sized(&mut self, wide: u32, high: u32) -> bool {
        if self.size == (wide, high) && !self.bits.is_null() {
            return true;
        }
        let picture = Picture { size: 40, wide: wide as i32, high: -(high as i32), planes: 1, bits: 32, ..Picture::default() };
        let mut bits = std::ptr::null_mut();
        let bitmap = unsafe { CreateDIBSection(self.memory, &picture, 0, &mut bits, 0, 0) };
        if bitmap == 0 || bits.is_null() {
            return false;
        }
        unsafe { SelectObject(self.memory, bitmap) };
        if self.bitmap != 0 {
            unsafe { DeleteObject(self.bitmap) };
        }
        (self.bitmap, self.bits, self.size) = (bitmap, bits, (wide, high));
        self.pixels.clear();
        true
    }

    fn show(&mut self, screen: isize, at: (i32, i32), sprite: &Sprite) -> bool {
        if sprite.width == 0 || sprite.height == 0 || !self.sized(sprite.width, sprite.height) {
            return false;
        }
        if self.shown && self.at == at && self.pixels == sprite.pixels {
            return false;
        }
        if self.pixels != sprite.pixels {
            let into = unsafe { std::slice::from_raw_parts_mut(self.bits, sprite.pixels.len()) };
            for (to, from) in into.chunks_exact_mut(4).zip(sprite.pixels.chunks_exact(4)) {
                to.copy_from_slice(&[from[2], from[1], from[0], from[3]]);
            }
            self.pixels.clone_from(&sprite.pixels);
        }
        let blend = Blend { operation: 0, flags: 0, alpha: 255, format: 1 };
        let done = unsafe { UpdateLayeredWindow(self.window, screen, &Point { x: at.0, y: at.1 }, &Size { wide: sprite.width as i32, high: sprite.height as i32 }, self.memory, &Point::default(), 0, &blend, BY_ALPHA) };
        if !self.shown {
            unsafe { ShowWindow(self.window, SHOW_NO_ACTIVATE) };
        }
        (self.at, self.shown) = (at, true);
        done != 0
    }

    fn grabbable(&self, on: bool) {
        let style = unsafe { GetWindowLongPtrW(self.window, EX_STYLE) };
        let wanted = if on { style & !(TRANSPARENT as isize) } else { style | TRANSPARENT as isize };
        if wanted != style {
            unsafe { SetWindowLongPtrW(self.window, EX_STYLE, wanted) };
        }
    }

    fn crossed(&self, point: (i32, i32)) -> bool {
        let Some((x, y, radius)) = self.cross.filter(|_| self.shown) else {
            return false;
        };
        let (dx, dy) = (point.0 - (self.at.0 + x), point.1 - (self.at.1 + y));
        dx * dx + dy * dy <= radius * radius
    }

    fn holds(&self, point: (i32, i32)) -> bool {
        let (x, y) = (point.0 - self.at.0, point.1 - self.at.1);
        if !self.shown || x < 0 || y < 0 || x >= self.size.0 as i32 || y >= self.size.1 as i32 {
            return false;
        }
        self.pixels.get((y as usize * self.size.0 as usize + x as usize) * 4 + 3).is_some_and(|alpha| *alpha > 0)
    }

    fn hide(&mut self) {
        if self.shown {
            unsafe { ShowWindow(self.window, HIDE) };
            self.shown = false;
        }
    }
}

impl Drop for Pane {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.window);
            DeleteDC(self.memory);
            if self.bitmap != 0 {
                DeleteObject(self.bitmap);
            }
        }
    }
}

pub struct Panes {
    screen: isize,
    held: HashMap<&'static str, Pane>,
    offsets: HashMap<String, (i32, i32)>,
    grabbing: bool,
    drag: Option<(&'static str, (i32, i32))>,
    pressed: (bool, bool),
}

/// What the mouse did to the plates in one look: whether any was moved, and which one, if any, had its cross pressed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Touched {
    pub moved: bool,
    pub closed: Option<&'static str>,
}

fn layout_path() -> std::path::PathBuf {
    std::env::temp_dir().join(LAYOUT_FILE)
}

fn load_layout() -> HashMap<String, (i32, i32)> {
    let Ok(text) = std::fs::read_to_string(layout_path()) else {
        return HashMap::new();
    };
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            Some((parts.next()?.to_owned(), (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?)))
        })
        .collect()
}

fn cursor() -> (i32, i32) {
    let mut point = Point::default();
    unsafe { GetCursorPos(&mut point) };
    (point.x, point.y)
}

impl Default for Panes {
    fn default() -> Self {
        Panes::new()
    }
}

impl Panes {
    pub fn new() -> Panes {
        unsafe { SetProcessDPIAware() };
        Panes { screen: unsafe { GetDC(0) }, held: HashMap::new(), offsets: load_layout(), grabbing: false, drag: None, pressed: (false, false) }
    }

    pub fn show(&mut self, origin: (i32, i32), sprites: &[Sprite]) -> usize {
        self.held.retain(|key, _| sprites.iter().any(|sprite| sprite.key == *key));
        let mut drawn = 0;
        for sprite in sprites {
            if !self.held.contains_key(sprite.key) {
                let Some(pane) = Pane::new(self.screen) else {
                    continue;
                };
                self.held.insert(sprite.key, pane);
            }
            let pane = self.held.get_mut(sprite.key).expect("the pane was just made");
            pane.grabbable(self.grabbing);
            pane.base = (origin.0 + sprite.x, origin.1 + sprite.y);
            pane.cross = sprite.cross;
            let (dx, dy) = self.offsets.get(sprite.key).copied().unwrap_or_default();
            if pane.show(self.screen, (pane.base.0 + dx, pane.base.1 + dy), sprite) {
                drawn += 1;
            }
        }
        drawn
    }

    pub fn hide(&mut self) {
        self.held.values_mut().for_each(Pane::hide);
    }

    /// Whether the mouse is in use, which is when the loop has to run quickly to keep up with it.
    pub fn busy(&self) -> bool {
        self.grabbing || self.drag.is_some()
    }

    fn save_layout(&self) {
        let mut lines: Vec<String> = self.offsets.iter().filter(|(_, at)| **at != (0, 0)).map(|(key, at)| format!("{key} {} {}", at.0, at.1)).collect();
        lines.sort();
        let _ = std::fs::write(layout_path(), lines.join("\n"));
    }

    /// While `on`, the plates take the mouse: the left button drags one, the right button sends it home.
    /// Tells whether any plate was put somewhere else, so that the plates are drawn again.
    pub fn interact(&mut self, on: bool, area: (i32, i32, i32, i32)) -> Touched {
        if on != self.grabbing {
            self.grabbing = on;
            self.held.values().for_each(|pane| pane.grabbable(on));
        }
        if !on {
            self.drag = None;
            self.pressed = (false, false);
            return Touched::default();
        }
        let down = |key: i32| unsafe { GetAsyncKeyState(key) } < 0;
        let (left, right) = (down(LEFT_BUTTON), down(RIGHT_BUTTON));
        let (was_left, was_right) = std::mem::replace(&mut self.pressed, (left, right));
        let at = cursor();
        let under = self.held.iter().find(|(_, pane)| pane.holds(at)).map(|(key, pane)| (*key, pane.at));
        let mut changed = false;
        let mut closed = None;
        if right && !was_right {
            if let Some((key, _)) = under {
                changed = self.offsets.remove(key).is_some();
                if changed {
                    self.save_layout();
                }
            }
        }
        if left && !was_left {
            closed = self.held.iter().find(|(_, pane)| pane.crossed(at)).map(|(key, _)| *key);
            self.drag = under.filter(|_| closed.is_none()).map(|(key, pane_at)| (key, (at.0 - pane_at.0, at.1 - pane_at.1)));
        }
        if left {
            if let Some((key, grip)) = self.drag {
                if let Some(pane) = self.held.get_mut(key) {
                    let (ax, ay, wide, high) = area;
                    let x = (at.0 - grip.0).clamp(ax + KEPT_IN_SIGHT - pane.size.0 as i32, ax + wide - KEPT_IN_SIGHT);
                    let y = (at.1 - grip.1).clamp(ay + KEPT_IN_SIGHT - pane.size.1 as i32, ay + high - KEPT_IN_SIGHT);
                    if (x, y) != pane.at {
                        unsafe { SetWindowPos(pane.window, 0, x, y, 0, 0, NO_SIZE_NO_ORDER_NO_ACTIVATE) };
                        pane.at = (x, y);
                        self.offsets.insert(key.to_owned(), (x - pane.base.0, y - pane.base.1));
                    }
                }
            }
        } else if self.drag.take().is_some() {
            self.save_layout();
        }
        Touched { moved: changed, closed }
    }

    pub fn count(&self) -> usize {
        self.held.values().filter(|pane| pane.shown).count()
    }

    pub fn keep_above(&self, game: isize) -> usize {
        let mut raised = 0;
        for pane in self.held.values().filter(|pane| pane.shown) {
            if lies_above(pane.window, game) && unsafe { SetWindowPos(pane.window, ON_TOP, 0, 0, 0, 0, KEEP_PLACE) } != 0 {
                raised += 1;
            }
        }
        raised
    }

    pub fn pump(&self) {
        let mut posted = Posted::default();
        while unsafe { PeekMessageW(&mut posted, 0, 0, 0, REMOVE) } != 0 {
            unsafe {
                TranslateMessage(&posted);
                DispatchMessageW(&posted);
            }
        }
    }
}

impl Drop for Panes {
    fn drop(&mut self) {
        self.held.clear();
        unsafe { ReleaseDC(0, self.screen) };
    }
}
