use dunce::canonicalize;
use std::path::{Path, PathBuf};

use crate::bucket::{Bucket, PathPredicate};

pub fn display_path(path: &Path) -> String {
    path.display().to_string()
}

pub fn display_short_path(path: &Path, root: &Path) -> String {
    display_path(path.strip_prefix(root).unwrap_or(path))
}

pub fn convert_bgdir(bgdir: &str) -> Result<PathBuf, String> {
    let bgdir = canonicalize(PathBuf::from(bgdir))
        .map_err(|err| format!("Cannot find directory '{}': {}", bgdir, err))?;

    if !bgdir.is_dir() {
        Err(format!("Not a directory: {}", display_path(&bgdir)))
    } else {
        Ok(bgdir)
    }
}

pub fn list_bucket_files<'b>(buckets: &'b [Bucket], root: &Path, predicate: &PathPredicate) -> () {
    let get_weight = |bucket: &Bucket| bucket.get_weight(&predicate);

    let weight_total: i64 = buckets.iter().map(get_weight).sum();
    for bucket in buckets {
        println!(
            "{} [{}/{}]",
            display_short_path(&bucket.path, root),
            get_weight(bucket),
            weight_total
        );
        for file in bucket.get_contents() {
            let file_str = display_short_path(&file, &bucket.path);
            if predicate(&file) {
                println!("- {file_str}");
            } else {
                println!("! {file_str}");
            }
        }
    }
}
