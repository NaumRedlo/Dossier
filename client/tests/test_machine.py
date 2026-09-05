import sys
import types

from dossier import machine
from dossier.machine import (
    BATTERY_ABORT, BATTERY_FLOOR, decide, parse_battery,
    parse_idle_seconds, parse_power_mode, parse_thermal_pressure, should_abort,
)

CORES = 12

ON_BATTERY = (
    "Now drawing from 'Battery Power'\n"
    " -InternalBattery-0 (id=21430371)\t69%; discharging; 9:33 remaining present: true\n"
)
ON_AC = (
    "Now drawing from 'AC Power'\n"
    " -InternalBattery-0 (id=21430371)\t67%; charging; 1:00 remaining present: true\n"
)
DESKTOP = "Now drawing from 'AC Power'\n"
COOL = (
    "Note: No thermal warning level has been recorded\n"
    "Note: No performance warning level has been recorded\n"
)

def take(**over):
    args = dict(on_battery=False, percent=100, power_mode=0,
                idle_seconds=9999, hot=False, cores=CORES)
    args.update(over)
    return decide(**args)

def test_the_real_output_of_pmset_is_understood():
    assert parse_battery(ON_BATTERY) == (True, 69)
    assert parse_battery(ON_AC) == (False, 67)

def test_a_machine_with_no_battery_is_not_a_machine_to_spare():
    on_battery, percent = parse_battery(DESKTOP)
    assert on_battery is False and percent == 100
    assert take(on_battery=on_battery, percent=percent).take

def test_idle_time_arrives_in_nanoseconds():
    assert parse_idle_seconds('"HIDIdleTime" = 5600000000') == 5.6

def test_an_unreadable_idle_time_means_somebody_is_here():
    assert parse_idle_seconds("nothing of the sort") == 0.0
    assert take(idle_seconds=parse_idle_seconds("")).polite

def test_a_cool_machine_reports_no_pressure():
    assert parse_thermal_pressure(COOL) is False
    assert parse_thermal_pressure("CPU_Speed_Limit = 70") is True
    assert parse_thermal_pressure("CPU_Speed_Limit = 100") is False

def test_the_energy_mode_is_read_from_the_active_profile():
    assert parse_power_mode(" powermode            0\n sleep    1\n") == 0
    assert parse_power_mode(" powermode            1\n") == 1

def test_the_battery_floor_is_a_floor_not_a_target():
    assert not take(on_battery=True, percent=BATTERY_FLOOR - 1).take
    assert take(on_battery=True, percent=BATTERY_FLOOR).take
    assert take(on_battery=True, percent=BATTERY_FLOOR + 1).take

def test_a_full_battery_is_no_help_if_the_operator_asked_for_low_power():
    assert not take(power_mode=1, percent=100).take
    assert not take(power_mode=1, on_battery=False).take

def test_a_refusal_says_why():
    assert "энергосбережения" in take(power_mode=1).reason
    assert "14%" in take(on_battery=True, percent=14).reason

def test_a_render_already_running_gives_the_job_back_before_the_battery_dies():
    assert should_abort(BATTERY_ABORT - 1, on_battery=True)
    assert not should_abort(BATTERY_ABORT, on_battery=True)

    assert not should_abort(2, on_battery=False)

def test_an_idle_plugged_in_machine_is_ours():
    got = take()
    assert got.threads == 8 and got.encoder_threads == 4
    assert got.threads + got.encoder_threads == CORES, "the two pools share one machine"
    assert not got.polite

def test_a_hand_on_the_trackpad_gets_out_of_the_way():
    got = take(idle_seconds=3)
    assert got.threads == 4 and got.encoder_threads == 2
    assert got.polite, "nice costs nothing idle and decides who yields under load"

def test_on_battery_it_takes_half_the_machine():
    got = take(on_battery=True, percent=69)
    assert got.threads == 6 and got.encoder_threads == 3

def test_heat_drops_a_tier_rather_than_refusing():
    cool, hot = take(), take(hot=True)
    assert hot.take
    assert hot.threads < cool.threads and hot.encoder_threads < cool.encoder_threads
    assert "горячая" in hot.reason

def test_heat_counts_even_with_somebody_at_the_keyboard():
    assert take(idle_seconds=3, hot=True).threads < take(idle_seconds=3).threads

def test_no_tier_ever_asks_for_no_threads_at_all():
    for cores in (1, 2, 3, 4):
        for over in ({}, {"hot": True}, {"on_battery": True, "percent": 50},
                     {"on_battery": True, "percent": 50, "hot": True}):
            got = take(cores=cores, **over)
            assert got.threads >= 1 and got.encoder_threads >= 1, (cores, over)

def test_a_refusal_carries_no_thread_counts_to_act_on():
    refused = take(power_mode=1)
    assert refused.take is False
    assert (refused.threads, refused.encoder_threads) == (0, 0)

def test_a_refusal_says_why_as_a_word_as_well_as_a_sentence():
    flat = take(power_mode=1)
    assert flat.code == "low-power" and flat.reason

    low = take(on_battery=True, percent=9)
    assert low.code == "battery"
    assert low.detail == "9", "the number is separate so the phrasing can differ"

def test_a_machine_that_is_working_says_no_code():
    assert take().code == ""

def test_a_linux_worker_holds_a_sleep_lock_for_as_long_as_the_render(monkeypatch):
    monkeypatch.setattr(machine.sys, "platform", "linux")
    monkeypatch.setattr(machine.shutil, "which", lambda _: "/usr/bin/systemd-inhibit")
    prefix = machine.wakeful()
    assert prefix[0] == "/usr/bin/systemd-inhibit"
    assert "--mode=block" in prefix, "a delay lock buys seconds; a render is minutes"
    assert prefix[-1] == "--", "without it systemd reads our binary as its own option"

def test_a_linux_without_systemd_asks_for_nothing_rather_than_failing(monkeypatch):
    monkeypatch.setattr(machine.sys, "platform", "linux")
    monkeypatch.setattr(machine.shutil, "which", lambda _: None)
    assert machine.wakeful() == ()

def test_the_lid_is_left_alone(monkeypatch):
    monkeypatch.setattr(machine.sys, "platform", "linux")
    monkeypatch.setattr(machine.shutil, "which", lambda _: "/usr/bin/systemd-inhibit")
    assert "handle-lid-switch" not in " ".join(machine.wakeful())

def test_windows_holds_the_flag_and_then_lets_go(monkeypatch):
    monkeypatch.setattr(machine.sys, "platform", "win32")
    asked = []

    class FakeKernel:
        @staticmethod
        def SetThreadExecutionState(flags):
            asked.append(flags)
            return 1

    fake = types.SimpleNamespace(windll=types.SimpleNamespace(kernel32=FakeKernel))
    monkeypatch.setitem(sys.modules, "ctypes", fake)

    with machine.awake() as prefix:
        assert prefix == (), "there is no command to wrap on Windows"
        assert asked and asked[0] & machine._ES_SYSTEM_REQUIRED

    assert asked[-1] == machine._ES_CONTINUOUS, "the flag outlived the render"

def test_windows_falls_back_when_away_mode_is_refused(monkeypatch):
    monkeypatch.setattr(machine.sys, "platform", "win32")
    asked = []

    class FakeKernel:
        @staticmethod
        def SetThreadExecutionState(flags):
            asked.append(flags)
            return 0 if flags & machine._ES_AWAYMODE_REQUIRED else 1

    fake = types.SimpleNamespace(windll=types.SimpleNamespace(kernel32=FakeKernel))
    monkeypatch.setitem(sys.modules, "ctypes", fake)

    with machine.awake():
        pass
    assert machine._ES_CONTINUOUS | machine._ES_SYSTEM_REQUIRED in asked
    assert asked[-1] == machine._ES_CONTINUOUS

def test_a_desktop_can_say_somebody_is_using_it(monkeypatch):
    monkeypatch.setattr(machine.sys, "platform", "linux")
    monkeypatch.setattr(machine, "_linux_battery", lambda: (False, 100))

    alone = machine.capacity(12)
    shared = machine.capacity(12, polite=True)
    assert alone.threads > shared.threads
    assert shared.reason == "кто-то за клавиатурой"
    assert shared.polite, "the engine is told to keep out of the way as well"

def test_a_ceiling_is_a_ceiling(monkeypatch):
    monkeypatch.setattr(machine.sys, "platform", "linux")
    monkeypatch.setattr(machine, "_linux_battery", lambda: (False, 100))

    got = machine.capacity(32, ceiling=3)
    assert got.threads == 3 and got.encoder_threads <= 3
    assert "потолок 3" in got.reason

def test_a_ceiling_never_turns_a_refusal_into_a_job(monkeypatch):
    monkeypatch.setattr(machine.sys, "platform", "linux")
    monkeypatch.setattr(machine, "_linux_battery", lambda: (True, 5))
    assert not machine.capacity(32, ceiling=3).take

def test_a_ceiling_above_the_machine_changes_nothing(monkeypatch):
    monkeypatch.setattr(machine.sys, "platform", "linux")
    monkeypatch.setattr(machine, "_linux_battery", lambda: (False, 100))
    assert machine.capacity(12, ceiling=999).threads == machine.capacity(12).threads

def test_a_window_can_wrap_round_midnight():
    night = machine.parse_hours("22-6")
    assert night == (22, 6)
    assert machine.within(night, 23) and machine.within(night, 3)
    assert not machine.within(night, 12)

def test_an_ordinary_window_is_half_open():
    day = machine.parse_hours("9-18")
    assert machine.within(day, 9) and machine.within(day, 17)
    assert not machine.within(day, 18), "the end hour is not included"

def test_a_typo_costs_the_limit_and_not_the_worker():
    for nonsense in ("", "ночью", "9", "9-", "25-30", "-"):
        assert machine.parse_hours(nonsense) is None, nonsense

def test_a_zero_width_window_is_read_as_no_limit():
    assert machine.within((0, 0), 13)

def test_paused_beats_the_hours():
    limits = machine.Limits(hours=(0, 9), paused=True)
    assert limits.closed(3) == "приостановлено владельцем"

def test_no_limits_means_no_reason_to_refuse():
    assert machine.Limits().closed(13) is None
