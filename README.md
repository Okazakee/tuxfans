# tuxfans

Fan curve controller for TUXEDO laptops.

Talks directly to the embedded controller via `/dev/tuxedo_io`. No TCC required.

## Quick start

```bash
# Build
git clone https://github.com/Okazakee/tuxfans
cd tuxfans
cargo build --release
sudo cp target/release/tuxfans /usr/local/bin/

# One-time setup — fixes /dev/tuxedo_io permissions
tuxfans onboard

# Check status
tuxfans status

# Apply a built-in EC profile
tuxfans profile quiet

# Switch to custom curve
tuxfans profile custom

# Run the daemon (applies custom curve every 2 seconds)
tuxfans daemon start
tuxfans daemon status
tuxfans daemon stop
```

## Commands

| Command | |
|---|---|
| `tuxfans` | Usage and device check |
| `tuxfans onboard` | Install udev rule for rootless device access |
| `tuxfans status` | CPU/GPU temp, EC control state, current profile |
| `tuxfans profile [quiet\|performance\|overboost\|custom]` | Apply or show profile |
| `tuxfans config` | Show config |
| `tuxfans config reset` | Reset curves to defaults |
| `tuxfans daemon start\|stop\|status` | systemd user service |
| `tuxfans test [speed] [secs]` | Raw fan test (default 100%, 10s) |

## Config

`~/.config/tuxfans/config.toml`:

```toml
active_mode = "custom"
paired_edit = true

[fan1]
points = [
    { temp = 40.0, speed = 0.0 },
    { temp = 55.0, speed = 20.0 },
    { temp = 70.0, speed = 50.0 },
    { temp = 85.0, speed = 80.0 },
    { temp = 95.0, speed = 100.0 },
]

[fan2]
points = [
    { temp = 40.0, speed = 0.0 },
    { temp = 55.0, speed = 15.0 },
    { temp = 70.0, speed = 40.0 },
    { temp = 85.0, speed = 70.0 },
    { temp = 95.0, speed = 100.0 },
]
```

### Built-in EC profiles

| Profile | EC value | Behavior |
|---|---|---|
| `quiet` | `0x01` | Silent curve, EC automatic |
| `performance` | `0x02` | Balanced curve, EC automatic |
| `overboost` | `0x03` | Aggressive curve, max cooling |

### Custom mode

In `custom` mode the daemon reads CPU temperature from `k10temp` hwmon,
applies a median filter (13 samples, median-of-7) to smooth transient spikes,
interpolates fan speeds from the configured curve points, and writes them
directly to the EC via `W_UW_MODE_ENABLE`.

Safety features hard-coded into the daemon loop:
- **90°C+** → minimum 40% fan regardless of curve
- **80°C+** → minimum 30% fan regardless of curve
- **Falling speed limiter** — max 2% drop per tick when above 20%, preventing
  harsh fan cutoffs

Fan speed reads return 0 on AMD Gen10 hardware (tuxedo_io driver
limitation). Writes and profile switching work correctly. See
[ARCHITECTURE.md](ARCHITECTURE.md) for ioctl details.

## Requirements

- TUXEDO laptop with `tuxedo-drivers-dkms` installed
- Linux
- Rust toolchain (for building from source)

## Architecture

```
src/
  lib.rs          # tuxfans_lib crate
  config.rs       # FanConfig, ControlMode, FanCurve, interpolation
  controller.rs   # FanController — profiles, daemon loop, sensor reading
  tuxedo.rs       # /dev/tuxedo_io IOCTL layer
  sensors.rs      # hwmon probing (k10temp, amdgpu)
  main.rs         # CLI binary
```

The CLI imports `tuxfans_lib`. A future GTK4 binary will use the same library
behind a `gui` Cargo feature.

## License

MIT.
