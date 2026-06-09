use std::process::Command;
use std::thread;
use std::time::Duration;

use tuxfans_lib::config::{ControlMode, FanConfig};
use tuxfans_lib::controller::FanController;
use tuxfans_lib::tuxedo::TuxedoIO;

const UDEV_RULE_DST: &str = "/etc/udev/rules.d/99-tuxfans.rules";
const UDEV_RULE: &str =
    "SUBSYSTEM==\"tuxedo_io\", KERNEL==\"tuxedo_io\", MODE=\"0666\"";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("");

    match cmd {
        "onboard" => cmd_onboard(),
        "status" => {
            ensure_device(false);
            cmd_status();
        }
        "profile" => {
            ensure_device(false);
            cmd_profile(&args);
        }
        "config" => cmd_config(&args),
        "daemon" => cmd_daemon(&args),
        "daemon-run" => {
            ensure_device(true);
            cmd_daemon_run();
        }
        "test" => {
            ensure_device(true);
            cmd_test(&args);
        }
        _ => {
            print_usage();
            ensure_device(false);
        }
    }
}

// =====================================================================
// onboard
// =====================================================================

fn cmd_onboard() {
    println!("tuxfans setup\n");

    let has_dev = std::path::Path::new("/dev/tuxedo_io").exists();
    println!("  /dev/tuxedo_io  {}", check_mark(has_dev));
    if !has_dev {
        println!("    → Install the tuxedo drivers and reboot.\n");
        std::process::exit(1);
    }

    let has_access = TuxedoIO::open().is_ok();
    println!("  read/write      {}", check_mark(has_access));
    if has_access {
        println!("    → Already accessible.\n");
    } else {
        println!("    → Installing udev rule for persistent access...");
        install_udev_rule();
        println!();
    }

    let sensors = tuxfans_lib::sensors::read_all_sensors();
    println!(
        "  CPU temp  {}",
        check_mark(sensors.cpu_temp.is_some())
    );
    println!(
        "  GPU temp  {}",
        check_mark(sensors.gpu_temp.is_some())
    );
    println!();

    println!("Done.");
}

// =====================================================================
// health check
// =====================================================================

fn ensure_device(exit_on_fail: bool) {
    if !std::path::Path::new("/dev/tuxedo_io").exists() {
        eprintln!("Device not found: /dev/tuxedo_io\n→ Install the tuxedo drivers and reboot.");
        if exit_on_fail {
            std::process::exit(1);
        }
        return;
    }

    if TuxedoIO::open().is_ok() {
        return;
    }

    eprintln!("Device permission denied. Installing udev rule...");
    install_udev_rule();

    if exit_on_fail {
        std::process::exit(1);
    }
}

fn install_udev_rule() {
    let script = format!(
        "echo '{}' > {} && udevadm control --reload-rules && udevadm trigger",
        UDEV_RULE, UDEV_RULE_DST
    );

    let success = run_privileged(&script);

    if success {
        eprintln!("→ udev rule installed. You may need to re-plug or reboot.");
    } else {
        eprintln!("→ Privilege escalation failed. Run manually: sudo chmod 666 /dev/tuxedo_io");
    }
}

fn run_privileged(script: &str) -> bool {
    for cmd in &["sudo", "pkexec"] {
        match Command::new(cmd).args(["sh", "-c", script]).status() {
            Ok(s) if s.success() => return true,
            Ok(_) => continue,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => continue,
        }
    }
    false
}

fn check_mark(ok: bool) -> &'static str {
    if ok { "\u{2713}" } else { "\u{2717}" }
}

// =====================================================================
// status
// =====================================================================

fn cmd_status() {
    let ctrl = FanController::init();
    let s = ctrl.read_sensors();

    println!("CPU temp  {}", fmt_temp(s.cpu_temp));
    println!("GPU temp  {}", fmt_temp(s.gpu_temp));
    println!(
        "EC control {}",
        match s.ec_auto {
            Some(true) => "auto".to_string(),
            Some(false) => "manual".to_string(),
            None => "--".to_string(),
        }
    );
    println!("Profile   {}", ctrl.config.borrow().active_mode.label());
    println!("Config    {}", FanConfig::config_path().display());

    if let Some(err) = &s.device_error {
        eprintln!("\nDevice: {}", err);
    }
}

// =====================================================================
// profile
// =====================================================================

fn cmd_profile(args: &[String]) {
    let ctrl = FanController::init();

    match args.get(2).map(String::as_str) {
        Some("quiet") | Some("Quiet") => apply_and_report(&ctrl, ControlMode::Quiet),
        Some("performance") | Some("Performance") => {
            apply_and_report(&ctrl, ControlMode::Performance)
        }
        Some("overboost") | Some("Overboost") => apply_and_report(&ctrl, ControlMode::Overboost),
        Some("custom") | Some("Custom") => apply_and_report(&ctrl, ControlMode::Custom),
        Some(unknown) => {
            eprintln!(
                "Unknown profile: {}\nValid: quiet, performance, overboost, custom",
                unknown
            );
            std::process::exit(1);
        }
        None => {
            println!("{}", ctrl.config.borrow().active_mode.label());
        }
    }
}

fn apply_and_report(ctrl: &FanController, mode: ControlMode) {
    match ctrl.apply_profile(mode) {
        Ok(()) => println!("{}", mode.label()),
        Err(e) => {
            eprintln!("Failed: {}", e);
            std::process::exit(1);
        }
    }
}

// =====================================================================
// config
// =====================================================================

fn cmd_config(args: &[String]) {
    let ctrl = FanController::init();

    match args.get(2).map(String::as_str) {
        Some("reset") => {
            *ctrl.config.borrow_mut() = FanConfig::default();
            match ctrl.save_config() {
                Ok(()) => println!("Config reset to defaults."),
                Err(e) => eprintln!("Save failed: {}", e),
            }
        }
        Some(other) => {
            eprintln!("Unknown subcommand: {}\nValid: reset", other);
            std::process::exit(1);
        }
        None => {
            let cfg = ctrl.config.borrow();
            println!("{}", FanConfig::config_path().display());
            println!("mode    = {}", cfg.active_mode.label());
            println!("paired  = {}", cfg.paired_edit);
            println!("fan1:");
            for (i, pt) in cfg.fan1.points.iter().enumerate() {
                println!("  {:>2}  {:>5.1}C -> {:>5.1}%", i, pt.temp, pt.speed);
            }
            println!("fan2:");
            for (i, pt) in cfg.fan2.points.iter().enumerate() {
                println!("  {:>2}  {:>5.1}C -> {:>5.1}%", i, pt.temp, pt.speed);
            }
        }
    }
}

// =====================================================================
// daemon
// =====================================================================

fn cmd_daemon(args: &[String]) {
    match args.get(2).map(String::as_str) {
        Some("start") => daemon_start(),
        Some("stop") => daemon_stop(),
        Some("status") => daemon_status(),
        _ => daemon_status(),
    }
}

fn has_systemd() -> bool {
    Command::new("systemctl")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn daemon_start() {
    if has_systemd() {
        daemon_start_systemd();
    } else {
        daemon_start_detach();
    }
}

fn daemon_stop() {
    if has_systemd() {
        daemon_stop_systemd();
    } else {
        daemon_stop_detach();
    }
}

fn daemon_status() {
    if has_systemd() {
        daemon_status_systemd();
    } else {
        daemon_status_detach();
    }
}

// --- systemd path ---

fn daemon_start_systemd() {
    let exe = current_exe_path();
    let svc = service_unit(&exe);

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = format!("{}/.config/systemd/user", home);
    let path = format!("{}/tuxfans.service", dir);

    std::fs::create_dir_all(&dir).ok();
    std::fs::write(&path, &svc).expect("Failed to write service file");

    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();
    let _ = Command::new("systemctl")
        .args(["--user", "enable", "--now", "tuxfans"])
        .status();

    println!("Daemon installed and started.");
    println!("Check status: tuxfans daemon status");
}

fn daemon_stop_systemd() {
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "tuxfans"])
        .status();
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "tuxfans"])
        .status();
    println!("Daemon stopped and disabled.");
}

fn daemon_status_systemd() {
    let output = Command::new("systemctl")
        .args(["--user", "is-active", "tuxfans"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let status = String::from_utf8_lossy(&o.stdout).trim().to_string();
            println!("Daemon: {}", status);
        }
        _ => println!("Daemon: inactive"),
    }
}

// --- detached (non-systemd) path ---

fn pid_file() -> String {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        format!("{}/tuxfans.pid", dir)
    } else {
        let uid = unsafe { libc::getuid() };
        format!("/tmp/tuxfans-{}.pid", uid)
    }
}

fn daemon_start_detach() {
    let exe = current_exe_path();

    match unsafe { libc::fork() } {
        -1 => {
            eprintln!("fork failed: {}", std::io::Error::last_os_error());
            std::process::exit(1);
        }
        0 => {
            unsafe { libc::setsid() };
            // daemon already running check
            let status = Command::new(&exe)
                .arg("daemon-run")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            std::process::exit(status.map_or(1, |s| s.code().unwrap_or(1)));
        }
        pid => {
            let path = pid_file();
            std::fs::write(&path, pid.to_string()).expect("Failed to write PID file");
            println!("Daemon started (pid: {})", pid);
            println!("Stop: tuxfans daemon stop");
        }
    }
}

fn daemon_stop_detach() {
    let path = pid_file();
    let pid_str = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            println!("No PID file found. Daemon is not running.");
            return;
        }
    };
    let pid: i32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Invalid PID file.");
            return;
        }
    };

    unsafe {
        if libc::kill(pid, libc::SIGTERM) == 0 {
            println!("Daemon stopped (pid: {}).", pid);
        } else {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                println!("Daemon (pid: {}) is not running.", pid);
            } else {
                eprintln!("Failed to stop daemon: {}", err);
            }
        }
    }
    let _ = std::fs::remove_file(&path);
}

fn daemon_status_detach() {
    let path = pid_file();
    let pid_str = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            println!("Daemon: inactive");
            return;
        }
    };
    let pid: i32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            println!("Daemon: inactive");
            return;
        }
    };

    unsafe {
        if libc::kill(pid, 0) == 0 {
            println!("Daemon: active (pid: {})", pid);
        } else {
            println!("Daemon: inactive (stale pid: {})", pid);
            let _ = std::fs::remove_file(&path);
        }
    }
}

fn cmd_daemon_run() {
    let ctrl = FanController::init();

    let active = ctrl.config.borrow().active_mode;
    if active.ec_profile_value().is_some() {
        match ctrl.apply_profile(active) {
            Ok(()) => eprintln!(
                "tuxfans daemon (pid {}): {} profile applied, EC auto.",
                std::process::id(),
                active.label()
            ),
            Err(e) => {
                eprintln!("tuxfans daemon: cannot apply {}: {}", active.label(), e);
                std::process::exit(1);
            }
        }
        loop {
            thread::sleep(Duration::from_secs(60));
        }
    }

    eprintln!(
        "tuxfans daemon (pid {}): custom curve running.",
        std::process::id()
    );
    if let Err(e) = ctrl.run_daemon_loop() {
        eprintln!("tuxfans daemon: {}", e);
        std::process::exit(1);
    }
}

// =====================================================================
// test
// =====================================================================

fn cmd_test(args: &[String]) {
    let speed: u8 = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .filter(|&v| v <= 100)
        .unwrap_or(100);
    let secs: u64 = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .filter(|&v| v > 0)
        .unwrap_or(10);

    let io = match TuxedoIO::open() {
        Ok(io) => io,
        Err(e) => {
            eprintln!("Cannot open /dev/tuxedo_io: {}", e);
            std::process::exit(1);
        }
    };

    println!("tuxfans fan test - {}% both fans, {} seconds", speed, secs);

    io.set_fan1_speed(speed).ok();
    io.set_fan2_speed(speed).ok();

    for t in 1..=secs {
        thread::sleep(Duration::from_secs(1));
        println!(
            "  t+{:>2}s  fan1={:>3}%  fan2={:>3}%",
            t,
            io.read_fan1_speed().unwrap_or(255),
            io.read_fan2_speed().unwrap_or(255),
        );
    }

    io.set_auto().ok();
    println!("Reverted to EC auto.");
}

// =====================================================================
// usage
// =====================================================================

fn print_usage() {
    println!("tuxfans - fan curve controller for TUXEDO laptops\n");
    println!("USAGE:");
    println!("  tuxfans                        Usage and setup check");
    println!("  tuxfans onboard                Fix device permissions");
    println!("  tuxfans status                 Temps and EC state");
    println!("  tuxfans profile [mode]         Apply Quiet|Performance|Overboost|Custom");
    println!("  tuxfans config [reset]         Show or reset config");
    println!("  tuxfans daemon [start|stop]    Manage background daemon");
    println!("  tuxfans test [speed] [secs]    Raw fan test (default: 100%, 10s)");
}

// =====================================================================
// helpers
// =====================================================================

fn fmt_temp(v: Option<f64>) -> String {
    v.map(|t| format!("{:.0}C", t))
        .unwrap_or_else(|| "--".to_string())
}

fn current_exe_path() -> String {
    std::env::current_exe()
        .map(|p| {
            p.to_string_lossy()
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
        })
        .unwrap_or_else(|_| "/usr/local/bin/tuxfans".to_string())
}

fn service_unit(exe: &str) -> String {
    format!(
        r#"[Unit]
Description=tuxfans fan curve daemon
After=multi-user.target

[Service]
Type=simple
ExecStart="{}" daemon-run
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
"#,
        exe
    )
}
