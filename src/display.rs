use std::option::Option;
use std::path::{Path, PathBuf};
use std::result;
use windows::{
    core::{Result, HSTRING, PCWSTR, PWSTR},
    Win32::{Foundation, System::Com, UI::Shell},
};

use crate::bucket::{bucket_random, Bucket};
use crate::utils::{display_short_path, list_bucket_files};
use crate::wallpaper::set_wallpaper_from_buckets;

pub struct MonitorID {
    monitor_id: PCWSTR,
}

pub struct MonitorInfo {
    index: u32,
    monitor_id: MonitorID,
    rect: Option<Foundation::RECT>,
}

impl MonitorInfo {
    pub fn index(&self) -> u32 {
        self.index
    }
    pub fn monitor_id(&self) -> &MonitorID {
        &self.monitor_id
    }
    #[allow(dead_code)]
    pub fn rect(&self) -> Option<Foundation::RECT> {
        self.rect
    }
    pub fn is_connected(&self) -> bool {
        self.rect.is_some()
    }
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

    fn get_monitor_device_path_count(&self) -> Result<u32> {
        unsafe { self.wallpaper.GetMonitorDevicePathCount() }
    }

    fn get_monitor_device_path_at(&self, monitorindex: u32) -> Result<MonitorID> {
        let monitor_id: PWSTR = unsafe { self.wallpaper.GetMonitorDevicePathAt(monitorindex) }?;
        Ok(MonitorID {
            monitor_id: PCWSTR(monitor_id.as_ptr()),
        })
    }

    fn get_monitor_rect(&self, monitor_id: &MonitorID) -> Result<Option<Foundation::RECT>> {
        let rect = unsafe { self.wallpaper.GetMonitorRECT(monitor_id.monitor_id) };
        rect.map_or_else(
            |e| {
                if e.code() == Foundation::S_FALSE {
                    Ok(None)
                } else {
                    Err(e)
                }
            },
            |r| Ok(Some(r)),
        )
    }

    fn get_monitor_info(&self, monitorindex: u32) -> Result<MonitorInfo> {
        let monitor_id = self.get_monitor_device_path_at(monitorindex)?;
        let rect = self.get_monitor_rect(&monitor_id)?;
        Ok(MonitorInfo {
            index: monitorindex,
            monitor_id,
            rect,
        })
    }

    pub fn set_wallpaper(&self, monitor_id: &MonitorID, wallpaper: &Path) -> Result<()> {
        let wallpaper: HSTRING = wallpaper.into();
        unsafe {
            self.wallpaper
                .SetWallpaper(monitor_id.monitor_id, &wallpaper)
        }
    }

    pub fn get_all_monitors(&self) -> Result<Vec<MonitorInfo>> {
        Ok((0..self.get_monitor_device_path_count()?)
            .filter_map(|i| self.get_monitor_info(i).ok())
            .collect())
    }
}

pub fn get_displaymanager_variables() -> result::Result<(DisplayManager, Vec<MonitorInfo>), String>
{
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

            for monitor in displays.iter() {
                if monitor.is_connected() {
                    let path = bucket_random(&buckets, &predicate)
                        .map_err(|_| "Failed to select image.")?;
                    dm.set_wallpaper(monitor.monitor_id(), &path).map_err(|e| {
                        format!(
                            "Failed to set wallpaper on monitor {}: {}",
                            monitor.index() + 1,
                            e
                        )
                    })?;
                    if cfg!(debug_assertions) {
                        eprintln!(
                            "Set monitor {}: {}",
                            monitor.index() + 1,
                            display_short_path(&path, &bgdir)
                        );
                    } else {
                        eprintln!("Set monitor {}", monitor.index() + 1);
                    }
                } else {
                    eprintln!("Skipping disconnected monitor {}", monitor.index() + 1);
                }
            }
            Ok(())
        },
    )
}
