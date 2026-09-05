use crate::bot::Capacity;

const BATTERY_FLOOR: u8 = 15;

const BATTERY_ABORT: u8 = 10;

const IDLE_SECONDS: f64 = 300.0;

#[derive(Debug, Clone, Copy)]
pub struct Reading {
    pub on_battery: bool,
    pub percent: u8,

    pub low_power: bool,
    pub idle_seconds: f64,
    pub hot: bool,
    pub cores: u32,
}

pub fn decide(said: Reading) -> Capacity {
    let refuse = |reason: &str, code: &str, detail: Option<String>| Capacity {
        take: false,
        reason: reason.to_owned(),
        code: code.to_owned(),
        detail,
        threads: 0,
        polite: false,
    };
    if said.low_power {
        return refuse("В режиме энергосбережения", "low-power", None);
    }
    if said.on_battery && said.percent < BATTERY_FLOOR {
        return refuse(
            &format!("От батареи, {}%", said.percent),
            "battery",
            Some(said.percent.to_string()),
        );
    }

    let busy = said.idle_seconds < IDLE_SECONDS;
    let cores = said.cores.max(1);
    let (mut threads, mut reason) = if said.on_battery {
        (cores / 2, format!("От батареи, {}%", said.percent))
    } else if busy {
        (4.min(cores), "Используется".to_owned())
    } else {
        (cores * 2 / 3, "Свободно".to_owned())
    };
    threads = threads.max(1);

    if said.hot {
        threads = (threads / 2).max(1);
        reason.push_str(", под перегревом");
    }
    Capacity {
        take: true,
        reason,
        code: if said.on_battery {
            "battery"
        } else if busy {
            "busy"
        } else {
            "idle"
        }
        .to_owned(),
        detail: None,
        threads,
        polite: busy,
    }
}

pub fn should_abort(percent: u8, on_battery: bool) -> bool {
    on_battery && percent < BATTERY_ABORT
}

pub fn read() -> Reading {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4);
    let mut said = Reading {
        on_battery: false,
        percent: 100,
        low_power: false,

        idle_seconds: 0.0,
        hot: false,
        cores,
    };
    #[cfg(target_os = "macos")]
    {
        let (on_battery, percent) = mac::battery();
        said.on_battery = on_battery;
        said.percent = percent;
        said.low_power = mac::low_power();
        said.idle_seconds = mac::idle_seconds();
        said.hot = mac::hot();
    }
    #[cfg(target_os = "linux")]
    {
        let (on_battery, percent) = linux::battery();
        said.on_battery = on_battery;
        said.percent = percent;
    }
    #[cfg(target_os = "windows")]
    {
        let (on_battery, percent) = windows::battery();
        said.on_battery = on_battery;
        said.percent = percent;
    }
    said
}

pub fn capacity() -> Capacity {
    decide(read())
}

#[allow(dead_code)]
fn ask(program: &str, args: &[&str]) -> String {
    std::process::Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|done| done.status.success())
        .map(|done| String::from_utf8_lossy(&done.stdout).into_owned())
        .unwrap_or_default()
}

#[cfg(target_os = "macos")]
mod mac {
    use super::ask;

    pub fn battery() -> (bool, u8) {
        let said = ask("pmset", &["-g", "batt"]);
        let on_battery = said.contains("'Battery Power'");
        let percent = said
            .split('%')
            .next()
            .and_then(|before| {
                let digits: String = before
                    .chars()
                    .rev()
                    .take_while(char::is_ascii_digit)
                    .collect();
                digits.chars().rev().collect::<String>().parse().ok()
            })
            .unwrap_or(100);
        (on_battery, percent)
    }

    pub fn low_power() -> bool {
        ask("pmset", &["-g"])
            .lines()
            .filter_map(|line| {
                let mut words = line.split_whitespace();
                (words.next()? == "powermode").then(|| words.next()?.parse::<u32>().ok())?
            })
            .next()
            == Some(1)
    }

    pub fn idle_seconds() -> f64 {
        let said = ask("ioreg", &["-c", "IOHIDSystem"]);
        said.split("\"HIDIdleTime\"")
            .nth(1)
            .and_then(|after| {
                let digits: String = after
                    .chars()
                    .skip_while(|c| !c.is_ascii_digit())
                    .take_while(char::is_ascii_digit)
                    .collect();
                digits.parse::<f64>().ok()
            })
            .map_or(0.0, |nanos| nanos / 1e9)
    }

    pub fn hot() -> bool {
        ask("pmset", &["-g", "therm"])
            .split("CPU_Speed_Limit")
            .nth(1)
            .and_then(|after| {
                let digits: String = after
                    .chars()
                    .skip_while(|c| !c.is_ascii_digit())
                    .take_while(char::is_ascii_digit)
                    .collect();
                digits.parse::<u32>().ok()
            })
            .is_some_and(|limit| limit < 100)
    }
}

#[cfg(target_os = "linux")]
mod linux {
    pub fn battery() -> (bool, u8) {
        let Ok(supply) = std::fs::read_dir("/sys/class/power_supply") else {
            return (false, 100);
        };
        let mut names: Vec<_> = supply
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.to_uppercase().starts_with("BAT"))
            })
            .collect();
        names.sort();
        for battery in names {
            let percent = std::fs::read_to_string(battery.join("capacity"))
                .ok()
                .and_then(|text| text.trim().parse::<u8>().ok())
                .unwrap_or(100)
                .min(100);
            let status = std::fs::read_to_string(battery.join("status")).unwrap_or_default();
            return (status.trim().eq_ignore_ascii_case("discharging"), percent);
        }
        (false, 100)
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::ask;

    pub fn battery() -> (bool, u8) {
        let said = ask(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "$b = Get-CimInstance Win32_Battery; \
                 if ($b) { \"$($b.BatteryStatus) $($b.EstimatedChargeRemaining)\" } else { '2 100' }",
            ],
        );
        let mut words = said.split_whitespace();
        let status: u32 = words.next().and_then(|w| w.parse().ok()).unwrap_or(2);
        let percent: u8 = words.next().and_then(|w| w.parse().ok()).unwrap_or(100);

        (status == 1, percent.min(100))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plugged_in() -> Reading {
        Reading {
            on_battery: false,
            percent: 100,
            low_power: false,
            idle_seconds: 600.0,
            hot: false,
            cores: 12,
        }
    }

    #[test]
    fn an_idle_machine_on_the_wall_gives_two_thirds_of_itself() {
        let got = decide(plugged_in());
        assert!(got.take);
        assert_eq!(got.threads, 8, "twelve cores, two thirds of them");
        assert!(!got.polite);
        assert_eq!(got.code, "idle");
    }

    #[test]
    fn somebody_at_the_keyboard_keeps_eight_of_twelve_cores() {
        let got = decide(Reading {
            idle_seconds: 3.0,
            ..plugged_in()
        });
        assert!(got.take);
        assert_eq!(got.threads, 4);
        assert!(got.polite, "a render beside somebody working must yield");
    }

    #[test]
    fn a_low_battery_is_a_refusal_and_a_low_one_is_not() {
        let flat = decide(Reading {
            on_battery: true,
            percent: 9,
            ..plugged_in()
        });
        assert!(!flat.take);
        assert_eq!(flat.code, "battery");
        assert_eq!(flat.detail.as_deref(), Some("9"));

        let some = decide(Reading {
            on_battery: true,
            percent: 80,
            ..plugged_in()
        });
        assert!(some.take, "a charged laptop may still render");
        assert_eq!(some.threads, 6, "half the machine");
    }

    #[test]
    fn low_power_mode_beats_every_other_answer() {
        let got = decide(Reading {
            low_power: true,
            ..plugged_in()
        });
        assert!(!got.take);
        assert_eq!(got.code, "low-power");
    }

    #[test]
    fn heat_costs_a_tier_and_says_so() {
        let hot = decide(Reading {
            hot: true,
            ..plugged_in()
        });
        assert!(hot.take);
        assert_eq!(hot.threads, 4, "half of eight");
        assert!(hot.reason.contains("горячая"), "{}", hot.reason);

        let both = decide(Reading {
            hot: true,
            idle_seconds: 1.0,
            ..plugged_in()
        });
        assert_eq!(both.threads, 2, "hot and in use is the worst of both");
    }

    #[test]
    fn a_machine_can_describe_itself() {
        let told = profile();
        assert!(told.hardware.cores >= 1);
        assert!(!told.hardware.os.is_empty());
        assert!(
            told.speed.is_some() || told.could_not_measure.is_some(),
            "neither a speed nor a reason for its absence"
        );
        if let Some(speed) = told.speed {
            assert!(speed.per_thread > 0.0);
        }
    }

    #[test]
    fn a_machine_with_one_core_still_gets_one_thread() {
        for cores in [1, 2, 3] {
            let got = decide(Reading {
                cores,
                ..plugged_in()
            });
            assert!(
                got.threads >= 1,
                "{cores} cores gave {} threads",
                got.threads
            );
        }
    }

    #[test]
    fn a_render_is_given_up_lower_than_one_is_started() {
        assert!(should_abort(9, true));
        assert!(!should_abort(11, true));
        assert!(!should_abort(2, false), "the wall does not run out");
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Hardware {
    pub cpu: String,
    pub cores: u32,
    pub memory_gb: Option<f64>,

    pub encoders: Vec<String>,
    pub os: String,
}

impl Hardware {
    pub fn read() -> Self {
        Self {
            cpu: cpu_name(),
            cores: std::thread::available_parallelism()
                .map(|n| n.get() as u32)
                .unwrap_or(4),
            memory_gb: memory_gb(),
            encoders: hardware_encoders(),
            os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        }
    }
}

fn cpu_name() -> String {
    #[cfg(target_os = "macos")]
    {
        let said = ask("sysctl", &["-n", "machdep.cpu.brand_string"]);
        if !said.trim().is_empty() {
            return said.trim().to_owned();
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(text) = std::fs::read_to_string("/proc/cpuinfo") {
            if let Some(line) = text.lines().find(|l| l.starts_with("model name")) {
                if let Some((_, name)) = line.split_once(':') {
                    return name.trim().to_owned();
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        let said = ask(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_Processor).Name",
            ],
        );
        if !said.trim().is_empty() {
            return said.trim().to_owned();
        }
    }
    "неизвестен".to_owned()
}

fn memory_gb() -> Option<f64> {
    #[cfg(target_os = "macos")]
    {
        let bytes: f64 = ask("sysctl", &["-n", "hw.memsize"]).trim().parse().ok()?;
        return Some(bytes / 1024.0 / 1024.0 / 1024.0);
    }
    #[cfg(target_os = "linux")]
    {
        let text = std::fs::read_to_string("/proc/meminfo").ok()?;
        let line = text.lines().find(|l| l.starts_with("MemTotal:"))?;
        let kb: f64 = line.split_whitespace().nth(1)?.parse().ok()?;
        return Some(kb / 1024.0 / 1024.0);
    }
    #[cfg(target_os = "windows")]
    {
        let bytes: f64 = ask(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
            ],
        )
        .trim()
        .parse()
        .ok()?;
        return Some(bytes / 1024.0 / 1024.0 / 1024.0);
    }
    #[allow(unreachable_code)]
    None
}

fn hardware_encoders() -> Vec<String> {
    let listed = ask("ffmpeg", &["-hide_banner", "-encoders"]);
    let wanted = [
        "h264_videotoolbox",
        "hevc_videotoolbox",
        "h264_nvenc",
        "hevc_nvenc",
        "h264_qsv",
        "h264_vaapi",
        "h264_amf",
    ];
    wanted
        .iter()
        .filter(|name| listed.contains(**name))
        .map(|name| (*name).to_owned())
        .collect()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Profile {
    pub hardware: Hardware,
    pub capacity: Capacity,
    pub speed: Option<crate::bench::Speed>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub could_not_measure: Option<String>,
}

pub fn profile() -> Profile {
    let hardware = Hardware::read();
    let capacity = decide(read());
    let (speed, could_not_measure) =
        match crate::bench::draw_speed((854, 480), 24, capacity.threads.max(1)) {
            Ok(speed) => (Some(speed), None),
            Err(why) => (None, Some(why)),
        };
    Profile {
        hardware,
        capacity,
        speed,
        could_not_measure,
    }
}
