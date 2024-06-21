use std::path::{Path, PathBuf};
use std::result;
use windows::{
    core::{Result, HSTRING, PCWSTR, PWSTR},
    Win32::{System::Com, UI::Shell},
};

use crate::bucket::{bucket_random, Bucket};
use crate::utils::list_bucket_files;
use crate::wallpaper::set_wallpaper_from_buckets;

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

pub fn get_displaymanager_variables(
) -> result::Result<(DisplayManager, Vec<(u32, MonitorID)>), String> {
    let dm =
        DisplayManager::create().map_err(|e| format!("Failed to get DisplayManager: {}", e))?;

    let displays = dm
        .get_all_monitors()
        .map_err(|e| format!("Failed to get monitors: {}", e))?;

    Ok((dm, displays))
}

pub fn set_wallpaper_multi_from_buckets(
    bgdir: &PathBuf,
    buckets: &Vec<Bucket>,
) -> result::Result<(), String> {
    get_displaymanager_variables().map_or_else(
        |e| {
            eprintln!("{}", e);
            eprintln!("Falling back to default setter.");
            set_wallpaper_from_buckets(bgdir, buckets)
        },
        |(dm, displays)| {
            let predicate = |_: &_| true;
            if cfg!(debug_assertions) {
                // print in debug mode
                list_bucket_files(&buckets, &bgdir, &predicate);
            }

            for (i, monitor_id) in displays.iter() {
                let path =
                    bucket_random(&buckets, &predicate).map_err(|_| "Failed to select image.")?;
                dm.set_wallpaper(monitor_id, &path)
                    .map_err(|e| format!("Failed to set wallpaper on monitor {}: {}", i + 1, e))?;
                eprintln!("Set monitor {}", i + 1);
            }
            Ok(())
        },
    )
}
