use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use crate::config::{interpolate, ControlMode, FanConfig};
use crate::tuxedo::TuxedoIO;

const FILTER_WINDOW: usize = 13;
const FILTER_MEDIAN_KEEP: usize = 7;
const SAFETY_FAN_HIGH: u8 = 40;
const SAFETY_FAN_MID: u8 = 30;
const SAFETY_TEMP_HIGH: f64 = 90.0;
const SAFETY_TEMP_MID: f64 = 80.0;
const FALLING_LIMIT_PCT: u8 = 2;
const FALLING_LIMIT_THRESHOLD: u8 = 20;

pub struct SensorReadings {
    pub cpu_temp: Option<f64>,
    pub gpu_temp: Option<f64>,
    pub ec_auto: Option<bool>,
    pub device_error: Option<String>,
}

pub struct FanController {
    pub config: Rc<RefCell<FanConfig>>,
}

impl FanController {
    pub fn init() -> Self {
        Self {
            config: Rc::new(RefCell::new(FanConfig::load_or_default())),
        }
    }

    pub fn read_sensors(&self) -> SensorReadings {
        let sensors = crate::sensors::read_all_sensors();

        match TuxedoIO::open() {
            Ok(io) => SensorReadings {
                cpu_temp: sensors.cpu_temp,
                gpu_temp: sensors.gpu_temp,
                ec_auto: io.read_mode_enable().ok().map(|manual| !manual),
                device_error: None,
            },
            Err(e) => SensorReadings {
                cpu_temp: sensors.cpu_temp,
                gpu_temp: sensors.gpu_temp,
                ec_auto: None,
                device_error: Some(format!(
                    "{}. Install the tuxedo drivers (tuxedo-drivers-dkms or equivalent for your distribution) or fix /dev/tuxedo_io permissions.",
                    e
                )),
            },
        }
    }

    pub fn apply_profile(&self, mode: ControlMode) -> Result<(), String> {
        if let Some(profile) = mode.ec_profile_value() {
            let io = TuxedoIO::open()?;
            io.set_mode_enable(false)?;
            io.set_auto()?;
            io.set_performance_profile(profile)?;
        }
        self.config.borrow_mut().active_mode = mode;
        self.config.borrow().save()?;
        Ok(())
    }

    pub fn save_config(&self) -> Result<(), String> {
        self.config.borrow().save()
    }

    pub fn run_daemon_loop(&self) -> Result<(), String> {
        let io = TuxedoIO::open()?;
        io.set_mode_enable(true)?;

        let config = self.config.borrow().clone();

        let mut buf: VecDeque<f64> = VecDeque::with_capacity(FILTER_WINDOW);
        let mut last_fan1: u8 = 0;
        let mut last_fan2: u8 = 0;

        loop {
            let raw_temp = match crate::sensors::read_all_sensors().cpu_temp {
                Some(t) => t,
                None => {
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            };

            let temp = filter_temp(&mut buf, raw_temp);

            let mut f1 = interpolate(temp, &config.fan1);
            let mut f2 = interpolate(temp, &config.fan2);

            // Critical temperature safety net
            if temp >= SAFETY_TEMP_HIGH {
                f1 = f1.max(SAFETY_FAN_HIGH);
                f2 = f2.max(SAFETY_FAN_HIGH);
            } else if temp >= SAFETY_TEMP_MID {
                f1 = f1.max(SAFETY_FAN_MID);
                f2 = f2.max(SAFETY_FAN_MID);
            }

            // Falling speed limiter
            f1 = limit_falling(f1, last_fan1);
            f2 = limit_falling(f2, last_fan2);

            io.set_fan1_speed(f1).ok();
            io.set_fan2_speed(f2).ok();

            last_fan1 = f1;
            last_fan2 = f2;

            thread::sleep(Duration::from_secs(2));
        }
    }
}

fn filter_temp(buf: &mut VecDeque<f64>, temp: f64) -> f64 {
    buf.push_back(temp);
    if buf.len() > FILTER_WINDOW {
        buf.pop_front();
    }

    if buf.len() < FILTER_MEDIAN_KEEP {
        let sum: f64 = buf.iter().sum();
        return (sum / buf.len() as f64 * 10.0).round() / 10.0;
    }

    let mut sorted: Vec<f64> = buf.iter().copied().collect();
    sorted.sort_by(|a, b| a.total_cmp(b));

    let start = (sorted.len() - FILTER_MEDIAN_KEEP) / 2;
    let sum: f64 = sorted[start..start + FILTER_MEDIAN_KEEP].iter().sum();

    (sum / FILTER_MEDIAN_KEEP as f64 * 10.0).round() / 10.0
}

fn limit_falling(target: u8, last: u8) -> u8 {
    if last > FALLING_LIMIT_THRESHOLD && last > target {
        let drop = last - target;
        if drop > FALLING_LIMIT_PCT {
            return last - FALLING_LIMIT_PCT;
        }
    }
    target
}
