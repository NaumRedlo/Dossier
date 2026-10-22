pub fn output_named(technology: u32) -> String {
    match technology {
        0 => "VGA".to_owned(),
        4 => "DVI".to_owned(),
        5 => "HDMI".to_owned(),
        6 | 11 | 13 | 0x8000_0000 => "built in".to_owned(),
        10 => "DisplayPort".to_owned(),
        15 => "Miracast".to_owned(),
        16 | 17 => "indirect".to_owned(),
        other => format!("output {other}"),
    }
}

pub fn colour_named(supported: bool, enabled: bool) -> &'static str {
    match (supported, enabled) {
        (_, true) => "HDR on",
        (true, false) => "HDR off",
        (false, false) => "no HDR",
    }
}

pub fn refresh(numerator: u32, denominator: u32) -> String {
    if denominator == 0 {
        return "an unknown rate".to_owned();
    }
    let hertz = f64::from(numerator) / f64::from(denominator);
    format!("{} Hz", (hertz * 100.0).round() / 100.0)
}

pub fn window_named(style: u32, extended: u32, window: (i32, i32, i32, i32), monitor: (i32, i32, i32, i32)) -> String {
    let (x, y, right, bottom) = window;
    let (left, top, edge, foot) = monitor;
    let kind = if style & 0x8000_0000 != 0 {
        "a borderless window"
    } else if style & 0x00C0_0000 == 0x00C0_0000 {
        "a window with a title bar"
    } else {
        "a window without a title bar"
    };
    let covers = if window == monitor {
        "covers its monitor exactly"
    } else if x <= left && y <= top && right >= edge && bottom >= foot {
        "covers its monitor and reaches past it"
    } else {
        "does not cover its whole monitor"
    };
    let topmost = if extended & 0x0000_0008 != 0 { ", kept on top" } else { "" };
    format!("the game's window is {kind}{topmost}, {}x{} at {x}, {y}, and {covers} ({}x{} at {left}, {top})", right - x, bottom - y, edge - left, foot - top)
}

pub fn osu_layers(said: &[(String, String)]) -> Vec<String> {
    said.iter()
        .filter(|(program, _)| program.to_ascii_lowercase().ends_with("osu!.exe"))
        .map(|(program, flags)| format!("compatibility flags for {program}: {}", flags.trim()))
        .collect()
}

#[cfg(windows)]
mod read {
    use std::ffi::c_void;

    const ACTIVE_PATHS: u32 = 2;
    const TARGET_NAME: u32 = 2;
    const ADVANCED_COLOUR: u32 = 9;
    const KEY_READ: u32 = 0x2_0019;
    const ANY: u32 = 0xFFFF;
    const STRING: u32 = 1;
    const EXPANDABLE: u32 = 2;
    const NUMBER: u32 = 4;
    const NEAREST: u32 = 2;
    const STYLE: i32 = -16;
    const EXTENDED_STYLE: i32 = -20;
    const ADAPTERS: &str = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}";
    const LAYERS: &str = "Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers";

    fn machine() -> isize {
        0x8000_0002u32 as i32 as isize
    }

    fn user() -> isize {
        0x8000_0001u32 as i32 as isize
    }

    #[repr(C)]
    struct Version {
        size: u32,
        major: u32,
        minor: u32,
        build: u32,
        platform: u32,
        service: [u16; 128],
    }

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct MonitorInfo {
        size: u32,
        monitor: Rect,
        work: Rect,
        flags: u32,
        device: [u16; 32],
    }

    #[repr(C)]
    struct TargetName {
        kind: u32,
        size: u32,
        adapter_low: u32,
        adapter_high: u32,
        id: u32,
        flags: u32,
        technology: u32,
        maker: u16,
        product: u16,
        connector: u32,
        name: [u16; 64],
        path: [u16; 128],
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn RtlGetVersion(version: *mut Version) -> i32;
    }

    #[link(name = "advapi32")]
    extern "system" {
        fn RegGetValueW(key: isize, below: *const u16, value: *const u16, flags: u32, kind: *mut u32, data: *mut c_void, size: *mut u32) -> i32;
        fn RegOpenKeyExW(key: isize, below: *const u16, options: u32, access: u32, opened: *mut isize) -> i32;
        fn RegEnumValueW(key: isize, index: u32, name: *mut u16, name_size: *mut u32, reserved: *mut u32, kind: *mut u32, data: *mut u8, size: *mut u32) -> i32;
        fn RegCloseKey(key: isize) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetDisplayConfigBufferSizes(flags: u32, paths: *mut u32, modes: *mut u32) -> i32;
        fn QueryDisplayConfig(flags: u32, path_count: *mut u32, paths: *mut [u32; 18], mode_count: *mut u32, modes: *mut [u32; 16], topology: *mut u32) -> i32;
        fn DisplayConfigGetDeviceInfo(packet: *mut c_void) -> i32;
        fn GetWindowLongW(window: isize, which: i32) -> i32;
        fn GetWindowRect(window: isize, rect: *mut Rect) -> i32;
        fn MonitorFromWindow(window: isize, fallback: u32) -> isize;
        fn GetMonitorInfoW(monitor: isize, info: *mut MonitorInfo) -> i32;
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn narrow(text: &[u16]) -> String {
        let end = text.iter().position(|unit| *unit == 0).unwrap_or(text.len());
        String::from_utf16_lossy(&text[..end])
    }

    fn value(key: isize, below: &str, name: &str) -> Option<String> {
        let (below, name) = (wide(below), wide(name));
        let mut kind = 0u32;
        let mut size = 0u32;
        if unsafe { RegGetValueW(key, below.as_ptr(), name.as_ptr(), ANY, &mut kind, std::ptr::null_mut(), &mut size) } != 0 || size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize + 2];
        if unsafe { RegGetValueW(key, below.as_ptr(), name.as_ptr(), ANY, &mut kind, data.as_mut_ptr().cast(), &mut size) } != 0 {
            return None;
        }
        match kind {
            STRING | EXPANDABLE => {
                let units: Vec<u16> = data[..size as usize].chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
                Some(narrow(&units))
            }
            NUMBER if size >= 4 => Some(u32::from_le_bytes([data[0], data[1], data[2], data[3]]).to_string()),
            _ => None,
        }
    }

    fn values(key: isize, below: &str) -> Vec<(String, String)> {
        let below = wide(below);
        let mut opened = 0isize;
        if unsafe { RegOpenKeyExW(key, below.as_ptr(), 0, KEY_READ, &mut opened) } != 0 {
            return Vec::new();
        }
        let mut found = Vec::new();
        for index in 0..4096 {
            let mut name = vec![0u16; 1024];
            let mut name_size = name.len() as u32;
            let mut data = vec![0u8; 2048];
            let mut size = data.len() as u32;
            let mut kind = 0u32;
            let said = unsafe { RegEnumValueW(opened, index, name.as_mut_ptr(), &mut name_size, std::ptr::null_mut(), &mut kind, data.as_mut_ptr(), &mut size) };
            if said == 259 {
                break;
            }
            if said != 0 || kind != STRING {
                continue;
            }
            let units: Vec<u16> = data[..size as usize].chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
            found.push((narrow(&name), narrow(&units)));
        }
        unsafe { RegCloseKey(opened) };
        found
    }

    fn windows() -> String {
        let mut version = Version { size: std::mem::size_of::<Version>() as u32, major: 0, minor: 0, build: 0, platform: 0, service: [0; 128] };
        unsafe { RtlGetVersion(&mut version) };
        let current = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
        let update = value(machine(), current, "UBR").map_or_else(String::new, |update| format!(".{update}"));
        let named = value(machine(), current, "DisplayVersion").map_or_else(String::new, |named| format!(" ({named})"));
        let edition = value(machine(), current, "EditionID").map_or_else(String::new, |edition| format!(" {edition}"));
        let name = if version.build >= 22000 { "Windows 11" } else { "Windows 10" };
        format!("system: {name}{edition}{named}, build {}.{}.{}{update}", version.major, version.minor, version.build)
    }

    fn adapters() -> Vec<String> {
        (0..16)
            .filter_map(|at| {
                let below = format!("{ADAPTERS}\\{at:04}");
                let name = value(machine(), &below, "DriverDesc")?;
                let driver = value(machine(), &below, "DriverVersion").unwrap_or_else(|| "unknown".to_owned());
                let date = value(machine(), &below, "DriverDate").map_or_else(String::new, |date| format!(" of {date}"));
                Some(format!("system: graphics {name}, driver {driver}{date}"))
            })
            .collect()
    }

    fn monitors() -> Vec<String> {
        let (mut path_count, mut mode_count) = (0u32, 0u32);
        if unsafe { GetDisplayConfigBufferSizes(ACTIVE_PATHS, &mut path_count, &mut mode_count) } != 0 {
            return vec!["system: the monitors could not be read".to_owned()];
        }
        let mut paths = vec![[0u32; 18]; path_count as usize];
        let mut modes = vec![[0u32; 16]; mode_count as usize];
        if unsafe { QueryDisplayConfig(ACTIVE_PATHS, &mut path_count, paths.as_mut_ptr(), &mut mode_count, modes.as_mut_ptr(), std::ptr::null_mut()) } != 0 {
            return vec!["system: the monitors could not be read".to_owned()];
        }
        paths.truncate(path_count as usize);
        modes.truncate(mode_count as usize);
        paths
            .iter()
            .map(|path| {
                let source = modes.get(path[3] as usize).filter(|mode| mode[0] == 1);
                let size = source.map_or_else(|| "an unknown size".to_owned(), |mode| format!("{}x{} at {}, {}", mode[4], mode[5], mode[7] as i32, mode[8] as i32));
                let mut target = TargetName { kind: TARGET_NAME, size: std::mem::size_of::<TargetName>() as u32, adapter_low: path[5], adapter_high: path[6], id: path[7], flags: 0, technology: 0, maker: 0, product: 0, connector: 0, name: [0; 64], path: [0; 128] };
                let named = unsafe { DisplayConfigGetDeviceInfo((&mut target as *mut TargetName).cast()) } == 0;
                let name = if named { narrow(&target.name) } else { String::new() };
                let name = if name.is_empty() { "a monitor".to_owned() } else { name };
                let mut colour = [ADVANCED_COLOUR, 32, path[5], path[6], path[7], 0, 0, 0];
                let colour_read = unsafe { DisplayConfigGetDeviceInfo(colour.as_mut_ptr().cast()) } == 0;
                let colour = if colour_read { super::colour_named(colour[5] & 1 != 0, colour[5] & 2 != 0) } else { "HDR unknown" };
                format!("system: {name} over {}, {size}, {}, {colour}", super::output_named(path[9]), super::refresh(path[12], path[13]))
            })
            .collect()
    }

    fn settings() -> Vec<String> {
        let mut said = Vec::new();
        let mut seen = |label: &str, key: isize, below: &str, name: &str| {
            said.push(format!("system: {label}: {}", value(key, below, name).unwrap_or_else(|| "not set".to_owned())));
        };
        seen("full screen optimizations (GameDVR_FSEBehaviorMode)", user(), "System\\GameConfigStore", "GameDVR_FSEBehaviorMode");
        seen("full screen optimizations honoured (GameDVR_DXGIHonorFSEWindowsCompatible)", user(), "System\\GameConfigStore", "GameDVR_DXGIHonorFSEWindowsCompatible");
        seen("game mode (AutoGameModeEnabled)", user(), "Software\\Microsoft\\GameBar", "AutoGameModeEnabled");
        seen("graphics settings for all games (DirectXUserGlobalSettings)", user(), "Software\\Microsoft\\DirectX\\UserGpuPreferences", "DirectXUserGlobalSettings");
        seen("hardware-accelerated GPU scheduling (HwSchMode)", machine(), "SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers", "HwSchMode");
        let mut layers = super::osu_layers(&values(user(), LAYERS));
        layers.extend(super::osu_layers(&values(machine(), LAYERS)));
        if layers.is_empty() {
            said.push("system: compatibility flags for osu!.exe: none".to_owned());
        }
        said.extend(layers.into_iter().map(|line| format!("system: {line}")));
        let preferences = values(user(), "Software\\Microsoft\\DirectX\\UserGpuPreferences");
        for (program, chosen) in preferences.iter().filter(|(program, _)| program.to_ascii_lowercase().ends_with("osu!.exe")) {
            said.push(format!("system: graphics settings for {program}: {chosen}"));
        }
        said
    }

    pub fn lines() -> Vec<String> {
        let mut said = vec![windows()];
        said.extend(adapters());
        said.extend(monitors());
        said.extend(settings());
        said
    }

    pub fn window(window: isize) -> Option<String> {
        let mut rect = Rect::default();
        if unsafe { GetWindowRect(window, &mut rect) } == 0 {
            return None;
        }
        let monitor = unsafe { MonitorFromWindow(window, NEAREST) };
        let mut info = MonitorInfo { size: std::mem::size_of::<MonitorInfo>() as u32, monitor: Rect::default(), work: Rect::default(), flags: 0, device: [0; 32] };
        if monitor == 0 || unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
            return None;
        }
        let style = unsafe { GetWindowLongW(window, STYLE) } as u32;
        let extended = unsafe { GetWindowLongW(window, EXTENDED_STYLE) } as u32;
        let edges = |rect: Rect| (rect.left, rect.top, rect.right, rect.bottom);
        Some(super::window_named(style, extended, edges(rect), edges(info.monitor)))
    }

    pub fn current_monitors() -> Vec<String> {
        monitors()
    }
}

#[cfg(windows)]
pub use read::{current_monitors, lines, window};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_game_window_is_told_by_its_style_and_how_it_lies_on_its_monitor() {
        let monitor = (0, 0, 1920, 1080);
        assert_eq!(window_named(0x9400_0000, 0x8, monitor, monitor), "the game's window is a borderless window, kept on top, 1920x1080 at 0, 0, and covers its monitor exactly (1920x1080 at 0, 0)");
        assert!(window_named(0x14CF_0000, 0, (100, 100, 1124, 868), monitor).starts_with("the game's window is a window with a title bar, 1024x768 at 100, 100, and does not cover"));
        assert!(window_named(0x8000_0000, 0, (-8, -8, 1928, 1088), monitor).ends_with("covers its monitor and reaches past it (1920x1080 at 0, 0)"));
    }

    #[test]
    fn monitors_name_their_link_rate_and_colour() {
        assert_eq!(output_named(10), "DisplayPort");
        assert_eq!(output_named(5), "HDMI");
        assert_eq!(output_named(0x8000_0000), "built in");
        assert_eq!(refresh(143_981, 1000), "143.98 Hz");
        assert_eq!(refresh(60, 1), "60 Hz");
        assert_eq!(refresh(1, 0), "an unknown rate");
        assert_eq!(colour_named(true, true), "HDR on");
        assert_eq!(colour_named(true, false), "HDR off");
        assert_eq!(colour_named(false, false), "no HDR");
    }

    #[test]
    fn only_the_game_s_compatibility_flags_are_kept() {
        let said = vec![
            ("C:\\Games\\osu!\\osu!.exe".to_owned(), "~ DISABLEDXMAXIMIZEDWINDOWEDMODE HIGHDPIAWARE ".to_owned()),
            ("C:\\Other\\game.exe".to_owned(), "~ RUNASADMIN".to_owned()),
        ];
        assert_eq!(osu_layers(&said), ["compatibility flags for C:\\Games\\osu!\\osu!.exe: ~ DISABLEDXMAXIMIZEDWINDOWEDMODE HIGHDPIAWARE"]);
    }
}
