use dunce::canonicalize;
use futures::executor::block_on;
use std::{io::Error, path::PathBuf};
use windows::{core::HSTRING, Storage::StorageFile, System::UserProfile::LockScreen};

pub fn set_lock_screen(path: &PathBuf) -> Result<(), Error> {
    let strref = HSTRING::from(canonicalize(path)?.as_os_str());
    let f = StorageFile::GetFileFromPathAsync(&strref).and_then(block_on)?;
    LockScreen::SetImageFileAsync(&f).and_then(block_on)?;
    Ok(())
}
