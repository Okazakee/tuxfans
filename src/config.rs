use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanConfig {
    #[serde(default)]
    pub active_mode: ControlMode,
    #[serde(default = "default_paired_edit")]
    pub paired_edit: bool,
    pub fan1: FanCurve,
    pub fan2: FanCurve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ControlMode {
    Quiet,
    Performance,
    Overboost,
    #[default]
    Custom,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanCurve {
    pub points: Vec<CurvePoint>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CurvePoint {
    pub temp: f64,
    pub speed: f64,
}

fn speed_to_percent(speed: f64) -> u8 {
    speed.round().clamp(0.0, 100.0) as u8
}

fn default_paired_edit() -> bool {
    true
}

impl FanConfig {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tuxfans")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir).map_err(|e| format!("Cannot create config dir: {}", e))?;
        let path = Self::config_path();
        let contents =
            toml::to_string_pretty(self).map_err(|e| format!("TOML serialization: {}", e))?;
        fs::write(&path, contents).map_err(|e| format!("Cannot write config: {}", e))?;
        Ok(())
    }

    pub fn load_or_default() -> Self {
        let path = Self::config_path();
        match fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!(
                        "Config parse error ({}), using defaults: {}",
                        path.display(),
                        e
                    );
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }
}

impl Default for FanConfig {
    fn default() -> Self {
        Self {
            active_mode: ControlMode::Custom,
            paired_edit: true,
            fan1: FanCurve {
                points: vec![
                    CurvePoint {
                        temp: 40.0,
                        speed: 0.0,
                    },
                    CurvePoint {
                        temp: 55.0,
                        speed: 20.0,
                    },
                    CurvePoint {
                        temp: 70.0,
                        speed: 50.0,
                    },
                    CurvePoint {
                        temp: 85.0,
                        speed: 80.0,
                    },
                    CurvePoint {
                        temp: 95.0,
                        speed: 100.0,
                    },
                ],
            },
            fan2: FanCurve {
                points: vec![
                    CurvePoint {
                        temp: 40.0,
                        speed: 0.0,
                    },
                    CurvePoint {
                        temp: 55.0,
                        speed: 15.0,
                    },
                    CurvePoint {
                        temp: 70.0,
                        speed: 40.0,
                    },
                    CurvePoint {
                        temp: 85.0,
                        speed: 70.0,
                    },
                    CurvePoint {
                        temp: 95.0,
                        speed: 100.0,
                    },
                ],
            },
        }
    }
}

pub fn interpolate(temp: f64, curve: &FanCurve) -> u8 {
    let pts = &curve.points;
    if pts.is_empty() {
        return 0;
    }
    if pts.len() == 1 {
        return speed_to_percent(pts[0].speed);
    }
    if temp <= pts[0].temp {
        return speed_to_percent(pts[0].speed);
    }
    if let Some(last) = pts.last() {
        if temp >= last.temp {
            return speed_to_percent(last.speed);
        }
    }
    for i in 0..pts.len() - 1 {
        let (t1, s1) = (pts[i].temp, pts[i].speed);
        let (t2, s2) = (pts[i + 1].temp, pts[i + 1].speed);
        if temp >= t1 && temp <= t2 {
            if (t2 - t1).abs() < f64::EPSILON {
                return speed_to_percent(s1);
            }
            let t = (temp - t1) / (t2 - t1);
            return speed_to_percent(s1 + t * (s2 - s1));
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn interpolate_handles_empty_curve() {
        assert_eq!(interpolate(60.0, &curve(&[])), 0);
    }

    #[test]
    fn interpolate_clamps_single_point_speed() {
        assert_eq!(interpolate(60.0, &curve(&[(40.0, 125.0)])), 100);
        assert_eq!(interpolate(60.0, &curve(&[(40.0, -5.0)])), 0);
    }

    #[test]
    fn interpolate_uses_endpoints_outside_curve_range() {
        let fan_curve = curve(&[(40.0, 10.0), (80.0, 90.0)]);

        assert_eq!(interpolate(35.0, &fan_curve), 10);
        assert_eq!(interpolate(85.0, &fan_curve), 90);
    }

    #[test]
    fn interpolate_linearly_between_points() {
        let fan_curve = curve(&[(40.0, 10.0), (80.0, 90.0)]);

        assert_eq!(interpolate(60.0, &fan_curve), 50);
    }

    #[test]
    fn interpolate_handles_duplicate_temperatures() {
        let fan_curve = curve(&[(40.0, 10.0), (40.0, 30.0), (80.0, 90.0)]);

        assert_eq!(interpolate(40.0, &fan_curve), 10);
    }

    #[test]
    fn config_defaults_to_custom_mode() {
        assert_eq!(FanConfig::default().active_mode, ControlMode::Custom);
    }

    #[test]
    fn config_defaults_to_paired_editing() {
        assert!(FanConfig::default().paired_edit);
    }

    #[test]
    fn old_config_shape_loads_with_default_mode_and_pairing() {
        let config: FanConfig = toml::from_str(
            r#"
[fan1]
points = [{ temp = 40.0, speed = 0.0 }, { temp = 80.0, speed = 100.0 }]

[fan2]
points = [{ temp = 40.0, speed = 0.0 }, { temp = 80.0, speed = 100.0 }]
"#,
        )
        .expect("old config shape should still load");

        assert_eq!(config.active_mode, ControlMode::Custom);
        assert!(config.paired_edit);
    }
}
