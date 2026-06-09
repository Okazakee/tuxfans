use std::cell::RefCell;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use crate::config::{interpolate, ControlMode, FanConfig};
use crate::tuxedo::TuxedoIO;

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
                    "{}. Install tuxedo-drivers-dkms or fix /dev/tuxedo_io permissions.",
                    e
                )),
            },
        }
    }

    pub fn apply_profile(&self, mode: ControlMode) -> Result<(), String> {
        if let Some(profile) = mode.ec_profile_value() {
            let io = TuxedoIO::open()?;
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
        let config = self.config.borrow().clone();

        loop {
            if let Some(temp) = crate::sensors::read_all_sensors().cpu_temp {
                let f1 = interpolate(temp, &config.fan1);
                let f2 = interpolate(temp, &config.fan2);
                io.set_fan1_speed(f1).ok();
                io.set_fan2_speed(f2).ok();
            }
            thread::sleep(Duration::from_secs(2));
        }
    }
}
