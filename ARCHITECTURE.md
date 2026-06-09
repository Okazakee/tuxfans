# TUXEDO Fan & EC Architecture

Technical notes for `tuxfans`, reverse-engineered from `tuxedo-drivers-dkms`
source v4.22.2 and direct EC probing on Uniwill-backed TUXEDO hardware.

## Driver Interface

`tuxfans` talks to the kernel through `/dev/tuxedo_io`, the character device
created by the `tuxedo_io` module. The module exposes a generic IOCTL interface
and dispatches to the matching backend, including Uniwill WMI on supported
machines.

On the supported Uniwill path, fan speed and EC profile data are not exposed as
standard fan hwmon entries. Standard hwmon still provides temperature sensors,
but fan reads and writes must go through `tuxedo_io`.

Relevant modules observed in the driver stack:

- `tuxedo_io`: main userspace IOCTL entry point
- `tuxedo_compatibility_check`: shared DMI/platform helpers
- `uniwill_wmi`: Uniwill WMI backend
- `tuxedo_keyboard`: unrelated to fan control, but commonly loaded alongside
  the same driver stack

## IOCTL Encoding

IOCTL command numbers use the Linux kernel `_IOC` encoding:

```text
bits 30-31: direction (0=NONE, 1=WRITE, 2=READ)
bits 16-29: size
bits  8-15: type/magic
bits  0- 7: command number
```

The Uniwill IOCTL constants use these magic values:

```text
MAGIC_READ_UW  = 0xEF
MAGIC_WRITE_UW = 0xF0
```

**Encoding note (verified on AMD Gen10):** The size field in the ioctl encoding
varies. Write commands and most read commands use `sizeof(int32_t*)=8` on
x86_64. Fan speed *reads* (`R_UW_FANSPEED`, `R_UW_FANSPEED2`) use
`sizeof(int32_t)=4`. Using size=8 for fan speed reads triggers a kernel oops
in the tuxedo_io driver (observed on TUXEDO InfinityBook Pro AMD Gen10,
BIOS N.1.21A21, tuxedo-drivers-dkms).

### Effective Ioctl Values

| Constant | Value (size) | Notes |
|----------|-------------|-------|
| `R_UW_FANSPEED` | `0x8004EF10` (4) | Size=4 required — size=8 crashes kernel |
| `R_UW_FANSPEED2` | `0x8004EF11` (4) | Size=4 required — size=8 crashes kernel |
| `R_UW_MODE_ENABLE` | `0x8008EF15` (8) | Works with both sizes |
| `W_UW_FANSPEED` | `0x4008F010` (8) | |
| `W_UW_FANSPEED2` | `0x4008F011` (8) | |
| `W_UW_FANAUTO` | `0x0000F014` (0) | No data |
| `W_UW_PERF_PROF` | `0x4008F018` (8) | |

## Uniwill IOCTLs

### Reads

| Command | Value | Returns |
|---------|-------|---------|
| `R_UW_HW_IF_STR` | `0x8108EF00` | Hardware interface string |
| `R_UW_MODEL_ID` | `0x8008EF01` | Model identifier |
| `R_UW_FANSPEED` | `0x8004EF10` | Fan 1 speed, 0-100% (size=4 required) |
| `R_UW_FANSPEED2` | `0x8004EF11` | Fan 2 speed, 0-100% (size=4 required) |
| `R_UW_FAN_TEMP` | `0x8004EF12` | Fan 1 EC temperature sensor |
| `R_UW_FAN_TEMP2` | `0x8004EF13` | Fan 2 EC temperature sensor |
| `R_UW_MODE` | `0x8008EF14` | Current fan/profile mode |
| `R_UW_MODE_ENABLE` | `0x8008EF15` | Manual mode toggle |
| `R_UW_FANS_OFF_AVAILABLE` | `0x8008EF16` | Whether fans can turn off |
| `R_UW_FANS_MIN_SPEED` | `0x8008EF17` | Minimum fan speed percent |

### Writes

| Command | Value | Action |
|---------|-------|--------|
| `W_UW_FANSPEED` | `0x4008F010` | Set fan 1 speed, 0-100% |
| `W_UW_FANSPEED2` | `0x4008F011` | Set fan 2 speed, 0-100% |
| `W_UW_MODE` | `0x4008F012` | Write fan mode register |
| `W_UW_MODE_ENABLE` | `0x4008F013` | Enable or disable manual mode (required before fan writes) |
| `W_UW_FANAUTO` | `0x0000F014` | Revert to EC automatic fan control |
| `W_UW_PERF_PROF` | `0x4008F018` | Set automatic EC fan profile |

### Hardware Checks

| Command | Value | Returns |
|---------|-------|---------|
| `R_MOD_VERSION` | `0x8108EC00` | Driver version string |
| `R_HWCHECK_CL` | `0x8008EC05` | 1 when Clevo hardware is detected |
| `R_HWCHECK_UW` | `0x8008EC06` | 1 when Uniwill hardware is detected |

## EC Register Map

The Uniwill backend reads and writes EC RAM for fan control:

| EC Address | Size | Purpose |
|------------|------|---------|
| `0x043e` | 1 byte | Fan 1 EC temperature sensor |
| `0x044f` | 1 byte | Fan 2 EC temperature sensor |
| `0x0741` | 1 byte | Manual fan mode enable flag |
| `0x0751` | 1 byte | Fan mode / automatic profile |
| `0x1804` | 1 byte | Fan 1 speed, 0-100% |
| `0x1809` | 1 byte | Fan 2 speed, 0-100% |

## Automatic Fan Profiles

The EC exposes three automatic fan profiles through register `0x0751`, written
through `W_UW_PERF_PROF`:

| Profile | Value | Behavior |
|---------|-------|----------|
| `quiet` | `0x01` | Silent EC fan curve, lower max speeds |
| `performance` | `0x02` | Balanced high-performance EC curve |
| `overboost` | `0x03` | Aggressive EC fan curve, max cooling |

These profiles affect the EC fan curve only while the EC is in automatic fan
control. They do not define a custom userspace curve.

## Custom Fan Control

Custom mode bypasses the automatic EC profile curve by writing fan speeds
directly with `W_UW_FANSPEED` and `W_UW_FANSPEED2`. Manual mode must be
enabled first via `W_UW_MODE_ENABLE` (value 1), otherwise the EC ignores
the fan speed writes. On exit, call `W_UW_MODE_ENABLE` (value 0) followed
by `W_UW_FANAUTO` to return control to the firmware.

Observed behavior:

- Fan speed writes use percent values from 0 to 100.
- Speed `0` means off when the platform reports fans-off support.
- Very low non-zero values may be clamped by the driver or EC to off/minimum.
- Reads return percent speed, not RPM. On AMD Gen10 (InfinityBook Pro AMD),
  fan speed reads return 0% regardless of actual speed — a tuxedo_io driver
  limitation. Writes and profile switching work correctly.
- `W_UW_FANAUTO` returns control to the EC automatic fan curve.

## Temperature Sources

Fan speed reads are only available through `tuxedo_io`, but temperature input
for the custom curve can come from standard Linux hwmon.

Currently used by `tuxfans`:

| Sensor | hwmon name | File |
|--------|------------|------|
| CPU temperature | `k10temp` | `/sys/class/hwmon/hwmon*/temp1_input` |
| GPU temperature | `amdgpu` | `/sys/class/hwmon/hwmon*/temp1_input` |

Available for future work:

- EC fan temperature sensors via `R_UW_FAN_TEMP` / `R_UW_FAN_TEMP2`.

## tuxfans Control Model

`tuxfans` has two control paths:

1. **EC profile mode**
   - Select `quiet`, `performance`, or `overboost`.
   - Call `W_UW_MODE_ENABLE` (0) to disable manual control, then
     `W_UW_FANAUTO` to return fan control to firmware.
   - Call `W_UW_PERF_PROF` with the selected profile value.

2. **Custom mode**
   - Call `W_UW_MODE_ENABLE` with value 1 to enable manual fan control.
   - Read CPU temperature from hwmon.
   - Interpolate fan speeds from configured curve points.
   - Apply median filter, critical temperature safety net, and falling speed
     limiter.
   - Write fan speeds directly to both fans every loop.
   - On exit: `W_UW_MODE_ENABLE` (0) then `W_UW_FANAUTO`.

## Permissions

`/dev/tuxedo_io` may be root-only depending on the installed driver and udev
rules. `tuxfans` installs a udev rule granting access to the `plugdev` group
(via `tuxfans onboard`). The onboard command uses `pkexec` to write the rule
to `/etc/udev/rules.d/99-tuxfans.rules`.
