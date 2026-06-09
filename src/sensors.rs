use std::path::Path;

#[derive(Clone, Debug)]
pub struct SystemSensors {
    pub cpu_temp: Option<f64>,
    pub gpu_temp: Option<f64>,
}

pub fn read_all_sensors() -> SystemSensors {
    SystemSensors {
        cpu_temp: read_first_sensor(&["k10temp", "coretemp", "cpu_thermal", "acpitz"]),
        gpu_temp: read_first_sensor(&["amdgpu", "nvidia", "i915", "radeon"]),
    }
}

fn read_first_sensor(names: &[&str]) -> Option<f64> {
    for name in names {
        if let Some(temp) = read_hwmon_temp(name) {
            return Some(temp);
        }
    }
    None
}

fn read_hwmon_temp(name: &str) -> Option<f64> {
    let dir = Path::new("/sys/class/hwmon");
    for entry in std::fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let name_file = entry.path().join("name");
        let hwmon_name = std::fs::read_to_string(&name_file).ok()?;
        if hwmon_name.trim() == name {
            let temp_file = entry.path().join("temp1_input");
            let val = std::fs::read_to_string(&temp_file).ok()?;
            return val.trim().parse::<f64>().ok().map(|v| v / 1000.0);
        }
    }
    None
}
