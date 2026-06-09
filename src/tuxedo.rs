use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;

const R_UW_FANSPEED: u64 = 0x8004EF10;
const R_UW_FANSPEED2: u64 = 0x8004EF11;
const R_UW_MODE_ENABLE: u64 = 0x8008EF15;
const W_UW_FANSPEED: u64 = 0x4008F010;
const W_UW_FANSPEED2: u64 = 0x4008F011;
const W_UW_FANAUTO: u64 = 0x0000F014;
const W_UW_PERF_PROF: u64 = 0x4008F018;

pub struct TuxedoIO {
    fd: File,
}

impl TuxedoIO {
    pub fn open() -> Result<Self, String> {
        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tuxedo_io")
            .map_err(|e| format!("Cannot open /dev/tuxedo_io: {}", e))?;
        Ok(Self { fd })
    }

    pub fn read_fan1_speed(&self) -> Result<u8, String> {
        self.read_ioctl(R_UW_FANSPEED)
    }

    pub fn read_fan2_speed(&self) -> Result<u8, String> {
        self.read_ioctl(R_UW_FANSPEED2)
    }

    pub fn read_mode_enable(&self) -> Result<bool, String> {
        let val = self.read_ioctl(R_UW_MODE_ENABLE)?;
        Ok(val != 0)
    }

    pub fn set_fan1_speed(&self, percent: u8) -> Result<(), String> {
        self.write_ioctl(W_UW_FANSPEED, percent)
    }

    pub fn set_fan2_speed(&self, percent: u8) -> Result<(), String> {
        self.write_ioctl(W_UW_FANSPEED2, percent)
    }

    pub fn set_auto(&self) -> Result<(), String> {
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), W_UW_FANAUTO as libc::c_ulong) };
        if ret < 0 {
            return Err(format!(
                "ioctl W_UW_FANAUTO failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    pub fn set_performance_profile(&self, profile: u8) -> Result<(), String> {
        self.write_ioctl(W_UW_PERF_PROF, profile)
    }

    fn read_ioctl(&self, req: u64) -> Result<u8, String> {
        let mut val: i32 = 0;
        let ret = unsafe {
            libc::ioctl(
                self.fd.as_raw_fd(),
                req as libc::c_ulong,
                &mut val as *mut i32 as *mut libc::c_void,
            )
        };
        if ret < 0 {
            return Err(format!(
                "ioctl read failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(val as u8)
    }

    fn write_ioctl(&self, req: u64, val: u8) -> Result<(), String> {
        let arg: i32 = val as i32;
        let ret = unsafe {
            libc::ioctl(
                self.fd.as_raw_fd(),
                req as libc::c_ulong,
                &arg as *const i32 as *mut libc::c_void,
            )
        };
        if ret < 0 {
            return Err(format!(
                "ioctl write failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
}
