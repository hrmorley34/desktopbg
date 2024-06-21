use ::wallpaper;
use dunce::canonicalize;
use std::path::PathBuf;

use crate::bucket::{bucket_random, Bucket};
use crate::utils::{display_short_path, list_bucket_files};

pub fn set_wallpaper_from_buckets(bgdir: &PathBuf, buckets: &Vec<Bucket>) -> Result<(), String> {
    let old_p = wallpaper::get()
        .ok()
        .map(PathBuf::from)
        .and_then(|p| canonicalize(p).ok());
    let old = old_p.as_ref();

    let check_not_old = |p: &PathBuf| old != Some(p);

    if cfg!(debug_assertions) {
        // print in debug mode
        old.map_or_else(
            || println!("Old?: ???"),
            |p| println!("Old: {}", display_short_path(p, &bgdir)),
        );
        list_bucket_files(&buckets, &bgdir, &check_not_old);
    }

    let new = bucket_random(&buckets, &check_not_old).map_err(|_| "Failed to select image.")?;
    let new_s = new.to_str().ok_or("Failed to convert path to string.")?;
    wallpaper::set_from_path(new_s).map_err(|e| format!("Failed to set wallpaper: {}", e))?;

    if cfg!(debug_assertions) {
        println!("New: {}", display_short_path(&new, &bgdir));
    }

    Ok(())
}
