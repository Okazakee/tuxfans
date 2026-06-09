#[derive(Clone, Debug)]
pub struct SystemSensors {
    pub cpu_temp: Option<f64>,
    pub gpu_temp: Option<f64>,
}

pub fn read_all_sensors() -> SystemSensors {
    SystemSensors {
        cpu_temp: read_hwmon_temp("k10temp"),
        gpu_temp: read_hwmon_temp("amdgpu"),
    }
}

fn read_hwmon_temp(name: &str) -> Option<f64> {
    let dir = std::path::Path::new("/sys/class/hwmon");
    for entry in std::fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let hwmon_name = std::fs::read_to_string(entry.path().join("name")).ok()?;
        if hwmon_name.trim() == name {
            let val = std::fs::read_to_string(entry.path().join("temp1_input")).ok()?;
            return val.trim().parse::<f64>().ok().map(|v| v / 1000.0);
        }
    }
    None
}
