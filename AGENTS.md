# AGENTS.md

## 1. Overview

tuxfans is a fan curve controller for TUXEDO laptops. It talks directly to the embedded controller via `/dev/tuxedo_io` using raw ioctl calls. No TUXEDO Control Center required. Written in Rust (edition 2021). The project is split into a library crate (`tuxfans_lib`) and a CLI binary (`tuxfans`). A GTK4 GUI binary is planned behind a `gui` Cargo feature but not yet implemented.

## 2. Repository Structure

```
src/
  lib.rs            # tuxfans_lib crate root — re-exports config, controller, sensors, tuxedo
  main.rs           # CLI binary entry point — all CLI logic lives here
  config.rs         # FanConfig, ControlMode, FanCurve, CurvePoint, interpolation, serde
  controller.rs     # FanController — profiles, daemon loop, sensor reading, safety net
  tuxedo.rs         # /dev/tuxedo_io IOCTL layer — TuxedoIO struct and raw ioctl calls
  sensors.rs        # hwmon probing — k10temp, amdgpu, coretemp, etc.
scripts/
  release.sh        # version bump, tag, push script (Bash)
packaging/
  homebrew/Formula/ # Homebrew formula for personal tap
  aur/              # Arch Linux PKGBUILD and .SRCINFO
udev/
  99-tuxfans.rules  # udev rule for /dev/tuxedo_io permissions (MODE=0666)
.github/
  workflows/
    publish.yml     # CI — cargo test, crates.io publish, Homebrew and AUR update on v* tag
```

- New Rust modules go in `src/` and are re-exported from `lib.rs`.
- CLI subcommand handlers go in `src/main.rs` — never in library modules.
- Library modules contain domain logic only (`config`, `controller`, `tuxedo`, `sensors`).
- Packaging files (`packaging/`) are not part of the Rust workspace.
- The `target/` directory is gitignored and never committed.
- Nothing except the Cargo workspace manifest (`Cargo.toml`) and metadata files belongs at the repo root.

## 5. Commands and Workflows

### Rust

```bash
cargo build              # debug build
cargo build --release    # release build (LTO + strip enabled)
cargo test               # run unit tests (inline #[cfg(test)] only)
cargo install --path .   # install from source
cargo publish            # publish to crates.io (CI handles this on tag push)
```

There is no formatter configured. There is no linter beyond `cargo check`. Run `cargo test` before any commit.

There is no `rustfmt.toml` and there is no plan to add one. The style rules in Section 6 are the sole authority.

### Release

```bash
bash scripts/release.sh 0.3.0   # bumps Cargo.toml version, commits, tags, pushes
```

## 6. Code Formatting

No formatter is configured — no `rustfmt.toml` exists. The code style below is the observed convention and must be followed exactly.

### Rust

#### Indentation

4 spaces. No tabs anywhere.

```rust
pub fn read_all_sensors() -> SystemSensors {
    SystemSensors {
        cpu_temp: read_first_sensor(&["k10temp", "coretemp", "cpu_thermal", "acpitz"]),
        gpu_temp: read_first_sensor(&["amdgpu", "nvidia", "i915", "radeon"]),
    }
}
```

#### Line length

Observed p95 is 72 characters. Keep lines under ~90 characters. The longest line in the codebase is 146 (a multiline string literal in `service_unit`).

#### Brace placement

Opening brace on the same line — K&R style — for all constructs (`fn`, `struct`, `enum`, `impl`, `match`, `if`, `loop`, `for`, `unsafe`).

```rust
fn check_mark(ok: bool) -> &'static str {
    if ok { "\u{2713}" } else { "\u{2717}" }
}
```

#### Blank lines — top-level definitions

1 blank line between top-level functions and structs. 0 blank lines between groups of related constants.

```rust
const FILTER_WINDOW: usize = 13;
const FILTER_MEDIAN_KEEP: usize = 7;
const SAFETY_FAN_HIGH: u8 = 40;

pub struct FanController {
    pub config: Rc<RefCell<FanConfig>>,
}
```

#### Blank lines — methods inside impl

1 blank line between methods.

```rust
impl ControlMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Quiet => "Quiet",
            Self::Performance => "Performance",
            Self::Overboost => "Overboost",
            Self::Custom => "Custom",
        }
    }

    pub fn ec_profile_value(self) -> Option<u8> {
        match self {
            Self::Quiet => Some(0x01),
            Self::Performance => Some(0x02),
            Self::Overboost => Some(0x03),
            Self::Custom => None,
        }
    }
}
```

#### Blank lines — after imports

1 blank line between the last import and the first definition.

```rust
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use crate::config::{interpolate, ControlMode, FanConfig};
use crate::tuxedo::TuxedoIO;

const FILTER_WINDOW: usize = 13;
```

#### Blank lines — end of file

Exactly 1 trailing newline. No trailing blank lines.

#### Trailing whitespace

None. All lines end at the last printable character.

#### Quote style

Double quotes for string literals.

```rust
eprintln!("Device not found: /dev/tuxedo_io\n→ Install the tuxedo drivers and reboot.");
```

#### Spacing — operators

Single space around `=`, `+`, `-`, `*`, `/`, `==`, `!=`, `<`, `>`, `&&`, `||`. No space around `::`.

```rust
let drop = last - target;
if drop > FALLING_LIMIT_PCT {
    return last - FALLING_LIMIT_PCT;
}
```

No space around `.` method calls, no space in `::` paths.

#### Spacing — inside brackets

No space inside parentheses, brackets, or angle brackets — `f(x)`, not `f( x )`.

```rust
let mut buf: VecDeque<f64> = VecDeque::with_capacity(FILTER_WINDOW);
```

```rust
buf.push_back(temp);
```

#### Spacing — after commas

One space after commas in argument lists, array elements, and derive macros.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanConfig { ... }
```

```rust
let (t1, s1) = (pts[i].temp, pts[i].speed);
let (t2, s2) = (pts[i + 1].temp, pts[i + 1].speed);
```

#### Trailing commas

Trailing comma on the last element of multi-line struct literals and multi-line array/vec elements. No trailing comma on the last parameter in a multi-line function call.

```rust
CurvePoint {
    temp: 40.0,
    speed: 0.0,
},
```

```rust
points: vec![
    CurvePoint { temp: 40.0, speed: 0.0 },
    CurvePoint { temp: 55.0, speed: 20.0 },
],
```

```rust
format!(
    r#"[Unit]
..."#,
    exe
)
```

#### Line continuation

Implicit via open bracket — never backslash.

```rust
let script = format!(
    "echo '{}' > {} && udevadm control --reload-rules && udevadm trigger",
    UDEV_RULE, UDEV_RULE_DST
);
```

#### Import block formatting

One import per line. `std::` imports listed first, then a blank line, then external or internal crate imports. Use `{}` to group multiple items from the same module path.

```rust
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;

use crate::config::{interpolate, ControlMode, FanConfig};
```

### Ruby (Homebrew formula only)

Indentation: 2 spaces. Double quotes. 1 blank line between methods. No trailing commas in method argument lists.

### Bash (scripts/)

Indentation: 2 spaces. `set -euo pipefail` at top. Double-quoted variable expansions. Double brackets in conditionals.

## 7. Naming Conventions

### Rust

#### Variables and function arguments

`snake_case`.

```rust
let pid: i32 = match pid_str.trim().parse() { ... };
```

#### Functions and methods

`snake_case`. CLI subcommand handlers are prefixed with `cmd_`. Daemon lifecycle functions are prefixed with `daemon_`. IO read operations are prefixed with `read_`, write operations with `set_`. Interpolation uses bare names.

```rust
fn cmd_status() { ... }
fn cmd_profile(args: &[String]) { ... }
fn daemon_start() { ... }
fn daemon_stop_systemd() { ... }
fn read_fan1_speed(&self) -> Result<u8, String> { ... }
fn set_fan1_speed(&self, percent: u8) -> Result<(), String> { ... }
fn interpolate(temp: f64, curve: &FanCurve) -> u8 { ... }
```

#### Structs

`PascalCase`.

```rust
pub struct FanConfig { ... }
pub struct FanCurve { ... }
pub struct CurvePoint { ... }
pub struct SystemSensors { ... }
pub struct SensorReadings { ... }
```

#### Enums

`PascalCase`. Variants are `PascalCase`.

```rust
pub enum ControlMode {
    Quiet,
    Performance,
    Overboost,
    Custom,
}
```

#### Constants

`SCREAMING_SNAKE_CASE`. IOCTL read constants are prefixed `R_UW_`, write constants `W_UW_`.

```rust
const R_UW_FANSPEED: u64 = 0x8004EF10;
const W_UW_FANSPEED: u64 = 0x4008F010;
const SAFETY_FAN_HIGH: u8 = 40;
const FILTER_WINDOW: usize = 13;
```

#### Module files

`snake_case`: `config.rs`, `controller.rs`, `tuxedo.rs`, `sensors.rs`, `main.rs`, `lib.rs`.

#### Test functions

`snake_case` with descriptive names joined by underscores: `interpolate_handles_empty_curve`, `config_defaults_to_custom_mode`.

## 8. Type Annotations

### Rust

All public function signatures have explicit return types. All function parameters have explicit types. No type elision on return types.

```rust
pub fn interpolate(temp: f64, curve: &FanCurve) -> u8 { ... }
pub fn read_sensors(&self) -> SensorReadings { ... }
pub fn read_fan1_speed(&self) -> Result<u8, String> { ... }
```

Private helper functions also have explicit return types.

```rust
fn read_first_sensor(names: &[&str]) -> Option<f64> { ... }
fn speed_to_percent(speed: f64) -> u8 { ... }
```

The canonical error return type is `Result<(), String>`. Read operations use `Result<u8, String>`. There is no custom error enum or `thiserror`. Use `String` for all error messages.

`Option<f64>` is used for values that may be unavailable (sensor readings). `Option<bool>` for optional boolean state. Never import `Option` or `Result` explicitly — use them directly from the prelude.

No type checker is enforced beyond `cargo check`. No `clippy` config exists.

## 9. Imports

### Rust

**Order:** `std::` imports first, then a blank line, then external crates, then `crate::` internal imports.

```rust
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;

use serde::{Deserialize, Serialize};
use crate::config::{interpolate, ControlMode, FanConfig};
```

**Grouping:** Multiple items from the same module path are grouped with `{}`. Never write separate `use` lines for the same module path.

```rust
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
```

Not:
```rust
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc;
```

**Qualified paths:** Within the library, use `crate::` for internal references. In `main.rs`, use `tuxfans_lib::` for library imports.

```rust
// In controller.rs (library)
use crate::config::{interpolate, ControlMode, FanConfig};

// In main.rs (binary)
use tuxfans_lib::config::{ControlMode, FanConfig};
```

**Wildcard imports:** Never use `use ...::*`. Always import items explicitly.

**Aliases:** None observed. Do not introduce `use ... as ...` aliases.

## 10. Error Handling

### Rust

All fallible operations return `Result<T, String>`. There is no custom error type. Error messages are built with `format!()` and passed directly as `Err(format!(...))`.

```rust
pub fn save(&self) -> Result<(), String> {
    let dir = Self::config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create config dir: {}", e))?;
    let path = Self::config_path();
    let contents =
        toml::to_string_pretty(self).map_err(|e| format!("TOML serialization: {}", e))?;
    fs::write(&path, contents).map_err(|e| format!("Cannot write config: {}", e))?;
    Ok(())
}
```

Use `.ok()` to silently discard errors when the outcome is non-critical (e.g., cleanup operations, informational writes).

```rust
io.set_fan1_speed(f1).ok();
io.set_fan2_speed(f2).ok();
```

Use `match` for explicit error handling where the caller needs to decide.

```rust
match ctrl.apply_profile(mode) {
    Ok(()) => println!("{}", mode.label()),
    Err(e) => {
        eprintln!("Failed: {}", e);
        std::process::exit(1);
    }
}
```

User-facing errors go to stderr via `eprintln!`. Fatal errors call `std::process::exit(1)`. Non-fatal errors return early or log and continue.

Never use `panic!` or bare `unwrap()` for user-facing errors. `.expect()` with a message is acceptable for invariants that indicate a programming bug.

```rust
std::fs::write(&path, &svc).expect("Failed to write service file");
```

The `?` operator is used within functions that return `Result<_, String>`.

Swallowing errors silently is only acceptable in daemon loops where continuing is preferred over crashing — use `.ok()` or `let _ =`.

```rust
let _ = Command::new("systemctl")
    .args(["--user", "daemon-reload"])
    .status();
```

## 11. Comments and Docstrings

### Rust

Doc comments (`///`, `//!`) are **not used** anywhere in the codebase. All comments are plain `//` line comments.

Section separators use a banner pattern with `=` signs:

```rust
// =====================================================================
// status
// =====================================================================
```

Subsection separators use a thinner variant with `---`:

```rust
// --- systemd path ---
```

Inline comments are rare and used only for algorithmic explanations:

```rust
// Critical temperature safety net
if temp >= SAFETY_TEMP_HIGH {
    f1 = f1.max(SAFETY_FAN_HIGH);
}
```

```rust
// Falling speed limiter
f1 = limit_falling(f1, last_fan1);
```

Comment placement: on its own line above the code it describes. Two spaces between `//` and the comment text.

Never leave commented-out code. Never use `/* */` block comments.

## 12. Testing

### Rust

**Framework:** `cargo test` (Rust built-in test harness).

**Location:** Tests are inline with `#[cfg(test)] mod tests` inside the source file. There is no separate `tests/` directory.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_handles_empty_curve() {
        assert_eq!(interpolate(60.0, &curve(&[])), 0);
    }
}
```

**Test file naming:** Tests live in the same file as the code they test. Only `src/config.rs` currently has tests.

**Test function naming:** `snake_case` describing the function and behavior: `interpolate_handles_empty_curve`, `config_defaults_to_custom_mode`, `old_config_shape_loads_with_default_mode_and_pairing`.

**Assertions:** Use `assert_eq!(actual, expected)`, `assert!(condition)`. Expected value comes second.

**Test helpers:** Defined as private functions inside the `mod tests` block.

```rust
fn curve(points: &[(f64, f64)]) -> FanCurve {
    FanCurve {
        points: points
            .iter()
            .map(|(temp, speed)| CurvePoint {
                temp: *temp,
                speed: *speed,
            })
            .collect(),
    }
}
```

**Fixtures:** None. Test data is constructed inline.

**CI:** Tests run on every `v*` tag push via `.github/workflows/publish.yml`. There is no CI on branch pushes.

## 13. Git

**Commit prefixes:** No conventional commit prefixes are used. Commit subjects are plain prose summaries.

```
Update Homebrew formula for v0.3.0
```

**Commit scopes:** Not used. No `feat(scope):` or `fix(scope):` formatting.

**Subject length:** Keep under 72 characters. Observed range is 30–96 characters.

**Commit body:** Used rarely (~14% of commits). When present, body is separated from subject by a blank line.

**Branch naming:** Only `main` branch observed. No prefix convention established.

**Merge strategy:** Rebase — no merge commits in history. Do not create merge commits.

**GPG signing:** Not required.

**What to never commit:** The `target/` directory, editor swap files (`*.swp`, `*.swo`), backup files (`*~`), and `.DS_Store` — all in `.gitignore`.

## 14. Dependencies and Tooling

### Rust

**Package manager:** Cargo. `Cargo.lock` is committed (binary crate).

**Add a dependency:**
```bash
cargo add <crate>
```

Or manually edit `Cargo.toml` under `[dependencies]`.

**Key dependencies:**
- `serde` (v1, with `derive` feature) — serialization for config
- `toml` (v0.8) — TOML config file parsing
- `dirs` (v6) — platform config directory
- `libc` (v0.2) — raw ioctl and POSIX syscalls
- `gtk4`, `libadwaita`, `gdk4`, `glib`, `gio` — all optional, behind `gui` feature (not yet implemented)

**Lockfile:** `Cargo.lock` is committed and must be updated when dependencies change.

**Formatter:** None configured. No `rustfmt.toml`.

**Linter:** None configured beyond `cargo check`. No `clippy.toml`.

**Build profile:** `[profile.release]` enables `lto = true` and `strip = true`.

### Ruby (Homebrew)

Single-file formula. No Gemfile. No formatter or linter configured.

### Bash

Single-file release script. No formatter or linter.

## 15. Red Lines

- **Never use tabs for indentation.** Every file in the repo uses spaces only.
- **Never use single quotes for Rust string literals.** All string literals use double quotes.
- **Never omit the blank line between `std::` imports and external/crate imports.** Always one blank line between import groups.
- **Never define CLI logic outside `src/main.rs`.** The library crate contains no argument parsing, no stdout formatting, no `std::process::exit`.
- **Never create a separate `tests/` directory.** All tests are inline `#[cfg(test)] mod tests` in the source file.
- **Never introduce a custom error enum or `thiserror`.** All errors are `Result<T, String>` with `format!()` messages.
- **Never use `unwrap()` on a `Result` where failure is expected at runtime.** Use `match`, `?`, or `.ok()` instead.
- **Never write user-facing output from library code.** `println!` and `eprintln!` belong in `main.rs` only.
- **Never use `///` doc comments.** The codebase uses only `//` line comments.
- **Never commit commented-out code.** Delete dead code before committing.
- **Never create merge commits.** The repo uses rebase-only history.
- **Never commit `target/`, editor swap files, or `.DS_Store`.** All are in `.gitignore`.
- **Never import with `use ...::*`.** Always list imported items explicitly.
- **Never omit return types on public functions.** Every `pub fn` has an explicit return type.
