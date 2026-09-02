//! Whether this computer should be rendering at all, and how hard.
//!
//! Ported from `client/dossier/machine.py`, thresholds and all. Every number
//! here was measured on a real machine rather than chosen, and the comments say
//! which — a policy of round numbers is a policy nobody checked.
//!
//! The shape is deliberate: [`decide`] is pure, so it can be tested without a
//! laptop in a particular mood, and everything platform-specific is a reader
//! that hands it facts.

use crate::bot::Capacity;

/// Below this, on battery, nothing is taken.
///
/// Not zero: somebody who lent a machine did not lend the walk home with a dead
/// phone-charger of a laptop.
const BATTERY_FLOOR: u8 = 15;

/// Below this, a render already running is abandoned.
const BATTERY_ABORT: u8 = 10;

/// Silence this long means nobody is at the keyboard.
const IDLE_SECONDS: f64 = 300.0;

/// What the machine said, before any policy is applied.
#[derive(Debug, Clone, Copy)]
pub struct Reading {
    pub on_battery: bool,
    pub percent: u8,
    /// macOS's `powermode`: 1 is low power.
    pub low_power: bool,
    pub idle_seconds: f64,
    pub hot: bool,
    pub cores: u32,
}

/// The policy itself, given what the machine said.
///
/// Order matters: the reasons to refuse are asked before the questions about
/// how hard to work, because a refusal makes the rest moot.
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
        return refuse("машина в режиме энергосбережения", "low-power", None);
    }
    if said.on_battery && said.percent < BATTERY_FLOOR {
        return refuse(
            &format!("от батареи, {}%", said.percent),
            "battery",
            Some(said.percent.to_string()),
        );
    }

    let busy = said.idle_seconds < IDLE_SECONDS;
    let cores = said.cores.max(1);
    let (mut threads, mut reason) = if said.on_battery {
        // Half the machine: measured at 566% of a 1200% ceiling, for 1.2× the
        // time of an unrestricted render. The restraint is for heat in a closed
        // bag, not for charge.
        (cores / 2, format!("от батареи, {}%", said.percent))
    } else if busy {
        // Four threads costs 2.1× the time and leaves eight cores to whoever is
        // using them, which is the trade the whole policy exists to make.
        (4.min(cores), "кто-то за клавиатурой".to_owned())
    } else {
        // Drawing on the performance cores, encoding on what is left. Not
        // `cores - 1` for drawing: the encoder needs its own, and two pools
        // sized as though each had the machine to itself is how both end up
        // waiting on the same cores.
        (cores * 2 / 3, "машина свободна".to_owned())
    };
    threads = threads.max(1);

    if said.hot {
        // A tier down rather than a refusal: the job is already worth doing,
        // and adding to the pressure is the only part worth avoiding. Checked
        // after every branch, including the one where somebody is present — a
        // hot machine under someone's hands is the worst of both.
        threads = (threads / 2).max(1);
        reason.push_str(", и она горячая");
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

/// Whether a render already running should be given up.
///
/// Separate from [`decide`] because it is asked of a machine that has already
/// been told yes, and the answer is allowed to be different: taking a job at
/// fifteen per cent and abandoning it at ten is not a contradiction.
pub fn should_abort(percent: u8, on_battery: bool) -> bool {
    on_battery && percent < BATTERY_ABORT
}

/// Ask this machine, whichever one it is.
pub fn read() -> Reading {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4);
    let mut said = Reading {
        on_battery: false,
        percent: 100,
        low_power: false,
        // Unknown means somebody is here, which is the cautious way round: it
        // costs speed, not somebody's machine.
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

/// What this machine will give right now.
pub fn capacity() -> Capacity {
    decide(read())
}

/// Ask the machine something, and take silence for an answer.
///
/// A reader that cannot run is a fact nobody has, not a reason to refuse work:
/// the policy above treats every unknown as the cautious answer already.
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

    /// `(on battery, percent)` from `pmset -g batt`.
    ///
    /// A desktop reports no battery at all, and a machine whose charge cannot
    /// be read is treated as plugged in: refusing every job because a string
    /// did not match would be a worse failure than taking one.
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

    /// The active `powermode` from `pmset -g`. One is low power.
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

    /// Seconds since the last keypress or gesture, from `ioreg -c IOHIDSystem`.
    ///
    /// Reported in nanoseconds. An unreadable answer means somebody is here.
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

    /// Whether `pmset -g therm` is reporting a speed limit at all.
    ///
    /// It says nothing has been recorded on a cool machine, so anything other
    /// than a hundred per cent of speed is the signal.
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
    /// `(on battery, percent)` from `/sys/class/power_supply/BAT*`.
    ///
    /// Two one-line files rather than a command's prose, which makes this the
    /// easiest of the three to read and the easiest to get subtly wrong:
    /// `status` is `Discharging`, `Charging`, `Full`, `Idle` or `Unknown`, and
    /// only the first means the wall is not helping.
    ///
    /// A desktop and a server both have no `BAT0`, and both should render at
    /// full tilt — which is what "not on battery, a hundred per cent" says.
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

    /// `(on battery, percent)` through PowerShell rather than the API.
    ///
    /// One process rather than a crate that binds half of `kernel32`: this is
    /// asked once a poll, and a spawn is cheaper than a dependency somebody has
    /// to audit.
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
        // `1` is discharging; everything else means the wall is helping.
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

    /// The trade the whole policy exists to make.
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

    /// A refusal is asked before anything else, because it makes the rest moot.
    #[test]
    fn low_power_mode_beats_every_other_answer() {
        let got = decide(Reading {
            low_power: true,
            ..plugged_in()
        });
        assert!(!got.take);
        assert_eq!(got.code, "low-power");
    }

    /// A tier down rather than a refusal — and it applies to every branch,
    /// including the one where somebody is present.
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

    /// The facts come back filled in, and the measurement with them.
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

    /// Taking a job at fifteen per cent and abandoning it at ten is not a
    /// contradiction: they are different questions.
    #[test]
    fn a_render_is_given_up_lower_than_one_is_started() {
        assert!(should_abort(9, true));
        assert!(!should_abort(11, true));
        assert!(!should_abort(2, false), "the wall does not run out");
    }
}

/// What this machine is made of.
///
/// Facts, not a verdict. What they are worth for drawing is measured — see
/// [`crate::bench`] — because a number assembled out of a specification sheet
/// is one nobody checked.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Hardware {
    pub cpu: String,
    pub cores: u32,
    pub memory_gb: Option<f64>,
    /// Hardware encoders `ffmpeg` admits to. Not used yet — the engine encodes
    /// with x264 — but the first thing anybody will ask for, and knowing which
    /// machines could is the difference between asking and guessing.
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

/// Hardware encoders `ffmpeg` offers, by the names it uses for them.
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

/// Everything worth knowing about this machine at once.
///
/// The facts, what the policy will currently allow, and how fast it actually
/// draws. The last of those is the one a farm should sort by: the other two say
/// what a machine *is*, and only the measurement says what it will do.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Profile {
    pub hardware: Hardware,
    pub capacity: Capacity,
    pub speed: Option<crate::bench::Speed>,
    /// Why the speed is missing, when it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub could_not_measure: Option<String>,
}

/// Read the machine, apply the policy, and draw a few frames to see.
///
/// The measurement is small on purpose — a few dozen frames at a modest size,
/// which is well under a second. Anything longer is a benchmark somebody
/// notices, and this runs while they are looking at a settings screen.
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
