use std::path::Path;
use windows::{
    core::{Result, HSTRING, PCWSTR, PWSTR},
    Win32::{System::Com, UI::Shell},
};

pub struct MonitorID {
    monitor_id: PCWSTR,
}

pub struct DisplayManager {
    wallpaper: Shell::IDesktopWallpaper,
}

impl DisplayManager {
    pub fn create() -> Result<DisplayManager> {
        unsafe { Com::CoInitialize(None) }?;
        let wallpaper: Shell::IDesktopWallpaper =
            unsafe { Com::CoCreateInstance(&Shell::DesktopWallpaper, None, Com::CLSCTX_ALL) }?;
        Ok(DisplayManager { wallpaper })
    }

    pub fn get_monitor_device_path_count(&self) -> Result<u32> {
        unsafe { self.wallpaper.GetMonitorDevicePathCount() }
    }

    pub fn get_monitor_device_path_at(&self, monitorindex: u32) -> Result<MonitorID> {
        let monitor_id: PWSTR = unsafe { self.wallpaper.GetMonitorDevicePathAt(monitorindex) }?;
        Ok(MonitorID {
            monitor_id: PCWSTR(monitor_id.as_ptr()),
        })
    }

    pub fn set_wallpaper(&self, monitor_id: &MonitorID, wallpaper: &Path) -> Result<()> {
        let wallpaper: HSTRING = wallpaper.into();
        unsafe {
            self.wallpaper
                .SetWallpaper(monitor_id.monitor_id, &wallpaper)
        }
    }

    pub fn get_all_monitors(&self) -> Result<Vec<(u32, MonitorID)>> {
        Ok((0..self.get_monitor_device_path_count()?)
            .filter_map(|i| self.get_monitor_device_path_at(i).ok().map(|mid| (i, mid)))
            .collect())
    }
}
