import contextlib
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from typing import Iterator, Optional

from dossier.log import get_logger

logger = get_logger("machine")

BATTERY_FLOOR = 15

BATTERY_ABORT = 10

IDLE_SECONDS = 300

LOW_POWER = 1

@dataclass(frozen=True)
class Capacity:

    take: bool
    reason: str
    threads: int = 0
    encoder_threads: int = 0

    code: str = ""

    detail: str = ""

    polite: bool = False

def _run(args: tuple[str, ...]) -> str:
    try:
        done = subprocess.run(args, capture_output=True, text=True, timeout=5)
    except (OSError, subprocess.SubprocessError) as exc:
        logger.warning("could not ask the machine (%s): %s", args[0], exc)
        return ""
    return done.stdout

def parse_battery(text: str) -> tuple[bool, int]:
    on_battery = "'Battery Power'" in text
    found = re.search(r"(\d+)%", text)
    return on_battery, int(found.group(1)) if found else 100

def parse_power_mode(text: str) -> int:
    found = re.search(r"^\s*powermode\s+(\d+)", text, re.MULTILINE)
    return int(found.group(1)) if found else 0

def parse_idle_seconds(text: str) -> float:
    found = re.search(r'"HIDIdleTime"\s*=\s*(\d+)', text)
    return int(found.group(1)) / 1e9 if found else 0.0

def parse_thermal_pressure(text: str) -> bool:
    found = re.search(r"CPU_Speed_Limit\s*=\s*(\d+)", text)
    if found:
        return int(found.group(1)) < 100
    return bool(re.search(r"warning level\s*=?\s*[1-9]", text))

def decide(*, on_battery: bool, percent: int, power_mode: int,
           idle_seconds: float, hot: bool, cores: int) -> Capacity:
    if power_mode == LOW_POWER:
        return Capacity(False, "машина в режиме энергосбережения", code="low-power")
    if on_battery and percent < BATTERY_FLOOR:
        return Capacity(False, f"от батареи, {percent}%", code="battery",
                        detail=str(percent))

    busy = idle_seconds < IDLE_SECONDS
    if on_battery:

        threads, encoder = max(1, cores // 2), max(1, cores // 4)
        reason = f"от батареи, {percent}%"
    elif busy:

        threads, encoder = 4, 2
        reason = "кто-то за клавиатурой"
    else:

        threads, encoder = max(1, cores * 2 // 3), max(1, cores // 3)
        reason = "машина свободна"

    if hot:

        threads, encoder = max(1, threads // 2), max(1, encoder // 2)
        reason += ", и она горячая"
    return Capacity(True, reason, threads, encoder, polite=busy)

def parse_linux_battery(capacity_text: str, status_text: str) -> tuple[bool, int]:
    try:
        percent = int(capacity_text.strip())
    except ValueError:
        percent = 100
    return status_text.strip().lower() == "discharging", max(0, min(100, percent))

def _linux_battery() -> tuple[bool, int]:
    supply = "/sys/class/power_supply"
    try:
        names = sorted(n for n in os.listdir(supply) if n.upper().startswith("BAT"))
    except OSError:
        return False, 100
    for name in names:
        try:
            with open(os.path.join(supply, name, "capacity"), encoding="ascii") as f:
                capacity_text = f.read()
            with open(os.path.join(supply, name, "status"), encoding="ascii") as f:
                status_text = f.read()
        except OSError:
            continue
        return parse_linux_battery(capacity_text, status_text)
    return False, 100

def _windows_battery() -> tuple[bool, int]:
    import ctypes

    class Status(ctypes.Structure):
        _fields_ = [
            ("ACLineStatus", ctypes.c_ubyte),
            ("BatteryFlag", ctypes.c_ubyte),
            ("BatteryLifePercent", ctypes.c_ubyte),
            ("SystemStatusFlag", ctypes.c_ubyte),
            ("BatteryLifeTime", ctypes.c_ulong),
            ("BatteryFullLifeTime", ctypes.c_ulong),
        ]

    status = Status()
    try:
        if not ctypes.windll.kernel32.GetSystemPowerStatus(ctypes.byref(status)):
            return False, 100
    except (AttributeError, OSError) as exc:
        logger.warning("could not read the power status: %s", exc)
        return False, 100
    percent = status.BatteryLifePercent
    return status.ACLineStatus == 0, 100 if percent == 255 else int(percent)

def _windows_idle_seconds() -> float:
    import ctypes

    class LastInput(ctypes.Structure):
        _fields_ = [("cbSize", ctypes.c_uint), ("dwTime", ctypes.c_ulong)]

    info = LastInput()
    info.cbSize = ctypes.sizeof(LastInput)
    try:
        if not ctypes.windll.user32.GetLastInputInfo(ctypes.byref(info)):
            return IDLE_SECONDS
        ticks = ctypes.windll.kernel32.GetTickCount64()
    except (AttributeError, OSError):
        return IDLE_SECONDS
    return max(0.0, (ticks - info.dwTime) / 1000.0)

@dataclass(frozen=True)
class Limits:

    polite: bool = False

    threads: int = 0

    hours: Optional[tuple[int, int]] = None
    paused: bool = False

    def closed(self, hour: int) -> Optional[str]:
        if self.paused:
            return "приостановлено владельцем"
        if self.hours is not None and not within(self.hours, hour):
            start, end = self.hours
            return f"вне рабочих часов ({start:02d}:00–{end:02d}:00)"
        return None

    def code(self, hour: int) -> str:
        if self.paused:
            return "paused"
        if self.hours is not None and not within(self.hours, hour):
            return "hours"
        return ""

def within(hours: tuple[int, int], hour: int) -> bool:
    start, end = hours
    if start == end:

        return True
    if start < end:
        return start <= hour < end
    return hour >= start or hour < end

def parse_hours(text: str) -> Optional[tuple[int, int]]:
    match = re.match(r"^\s*(\d{1,2})\s*[-–—]\s*(\d{1,2})\s*$", text or "")
    if not match:
        return None
    start, end = int(match.group(1)), int(match.group(2))
    if not (0 <= start <= 24 and 0 <= end <= 24):
        return None
    return start % 24, end % 24

def capacity(cores: int, *, polite: bool = False, ceiling: int = 0) -> Capacity:

    said_busy = 0.0 if polite else None

    if sys.platform == "darwin":
        on_battery, percent = parse_battery(_run(("pmset", "-g", "batt")))
        return _capped(decide(
            on_battery=on_battery,
            percent=percent,
            power_mode=parse_power_mode(_run(("pmset", "-g"))),
            idle_seconds=said_busy if said_busy is not None
            else parse_idle_seconds(_run(("ioreg", "-c", "IOHIDSystem"))),
            hot=parse_thermal_pressure(_run(("pmset", "-g", "therm"))),
            cores=cores,
        ), ceiling)

    if sys.platform == "win32":
        on_battery, percent = _windows_battery()
        return _capped(decide(
            on_battery=on_battery,
            percent=percent,

            power_mode=0,
            idle_seconds=said_busy if said_busy is not None
            else _windows_idle_seconds(),

            hot=False,
            cores=cores,
        ), ceiling)

    on_battery, percent = _linux_battery()
    return _capped(decide(
        on_battery=on_battery,
        percent=percent,
        power_mode=0,

        idle_seconds=said_busy if said_busy is not None else IDLE_SECONDS,
        hot=False,
        cores=cores,
    ), ceiling)

def _capped(got: Capacity, ceiling: int) -> Capacity:
    if ceiling <= 0 or not got.take:
        return got
    return Capacity(
        got.take,
        f"{got.reason}, потолок {ceiling}",
        min(got.threads, ceiling),
        min(got.encoder_threads, ceiling),
        polite=got.polite,
    )

def should_abort(percent: int, on_battery: bool) -> bool:
    return on_battery and percent < BATTERY_ABORT

def wakeful() -> tuple[str, ...]:
    if sys.platform == "darwin":
        caffeinate = "/usr/bin/caffeinate"
        return (caffeinate, "-i", "-m", "-s") if os.access(caffeinate, os.X_OK) else ()

    if sys.platform.startswith("linux"):
        inhibit = shutil.which("systemd-inhibit")
        if inhibit:
            return (
                inhibit,
                "--what=sleep:idle",
                "--who=dossier",
                "--why=rendering a replay",

                "--mode=block",
                "--",
            )
    return ()

_ES_CONTINUOUS = 0x80000000
_ES_SYSTEM_REQUIRED = 0x00000001
_ES_AWAYMODE_REQUIRED = 0x00000040

@contextlib.contextmanager
def awake() -> Iterator[tuple[str, ...]]:
    if sys.platform != "win32":
        yield wakeful()
        return

    import ctypes

    def state(flags: int) -> bool:
        try:
            return bool(ctypes.windll.kernel32.SetThreadExecutionState(flags))
        except (AttributeError, OSError) as exc:
            logger.warning("could not ask Windows to stay awake: %s", exc)
            return False

    held = state(_ES_CONTINUOUS | _ES_SYSTEM_REQUIRED | _ES_AWAYMODE_REQUIRED)
    if not held:
        held = state(_ES_CONTINUOUS | _ES_SYSTEM_REQUIRED)
    try:
        yield ()
    finally:
        if held:
            state(_ES_CONTINUOUS)
