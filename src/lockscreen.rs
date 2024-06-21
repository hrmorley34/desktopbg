use dunce::canonicalize;
use futures::executor::block_on;
use std::{io::Error, path::PathBuf};
use windows::{core::HSTRING, Storage::StorageFile, System::UserProfile::LockScreen};

use crate::bucket::{bucket_random, Bucket};
use crate::utils::{display_short_path, list_bucket_files};

pub fn set_lockscreen(path: &PathBuf) -> Result<(), Error> {
    let strref = HSTRING::from(canonicalize(path)?.as_os_str());
    let f = StorageFile::GetFileFromPathAsync(&strref).and_then(block_on)?;
    LockScreen::SetImageFileAsync(&f).and_then(block_on)?;
    Ok(())
}

pub fn set_lockscreen_from_buckets(bgdir: &PathBuf, buckets: &Vec<Bucket>) -> Result<(), String> {
    let predicate = |_: &_| true;
    if cfg!(debug_assertions) {
        // print in debug mode
        list_bucket_files(&buckets, &bgdir, &predicate);
    }

    let new = bucket_random(&buckets, &predicate).map_err(|_| "Failed to select image.")?;
    set_lockscreen(&new).map_err(|e| format!("Failed to set lockscreen: {}", e))?;

    if cfg!(debug_assertions) {
        println!("New: {}", display_short_path(&new, &bgdir));
    }

    Ok(())
}
