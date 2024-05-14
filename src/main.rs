#![cfg_attr(debug_assertions, windows_subsystem = "console")]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window for release build

use dunce::canonicalize;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::{env, fs, process::ExitCode};
use toml;
use wallpaper;

mod bucket;
use crate::bucket::*;

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn display_short_path(path: &Path, root: &Path) -> String {
    display_path(path.strip_prefix(root).unwrap_or(path))
}

fn list_bucket_files<'b>(buckets: &'b [Bucket], root: &Path, predicate: &PathPredicate) -> () {
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

#[derive(Deserialize)]
struct BucketsFile {
    backgrounds: toml::Table,
}

const DESKTOPBG: &str = "desktopbg.toml";

fn get_buckets_from_dir(bgdir: &PathBuf, toml: &str) -> Result<Vec<Bucket>, String> {
    let bucketsfile = bgdir.join(toml);
    if !bucketsfile.exists() {
        return Err(format!(
            "Cannot find bucketsfile! Looking for: {}",
            display_path(&bucketsfile)
        ));
    }
    let contents =
        fs::read_to_string(bucketsfile).map_err(|err| format!("Cannot open bucketsfile: {err}"))?;
    let bucketsfile: BucketsFile =
        toml::from_str(&contents).map_err(|err| format!("Cannot parse bucketsfile: {err}"))?;

    Ok(bucketsfile
        .backgrounds
        .iter()
        .filter_map(|(dirname, weight)| {
            // `canonicalize` guarantees existence and corrects capitalisation
            let path = canonicalize(bgdir.join(dirname))
                .or_else(|err| {
                    eprintln!("Cannot find bucket {dirname}: {err}. Skipping...");
                    Err(err)
                })
                .ok()?;
            let weight = weight.as_integer().or_else(|| {
                eprintln!("Non-integer weight {weight} for {dirname}. Skipping...");
                None
            })?;
            if weight <= 0 {
                eprintln!("Non-positive weight {weight} for {dirname}. Skipping...");
                None
            } else {
                Some(Bucket { path, weight })
            }
        })
        .collect())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if cfg!(debug_assertions) {
        println!("Args: {}", args.join(", "));
    }
    if args.len() != 2 {
        eprintln!("Expected exactly 1 argument - the directory.");
        return ExitCode::FAILURE;
    }

    let bgdir = &args[1];
    let bgdir = match canonicalize(PathBuf::from(bgdir)) {
        Err(err) => {
            eprintln!("Cannot find directory '{}': {}", bgdir, err);
            return ExitCode::FAILURE;
        }
        Ok(p) => p,
    };

    let buckets: Vec<Bucket> = match get_buckets_from_dir(&bgdir, DESKTOPBG) {
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
        Ok(b) => b,
    };

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

    let new = bucket_random(&buckets, &check_not_old).expect("Failed to select image.");
    new.to_str()
        .and_then(|p| wallpaper::set_from_path(p).ok())
        .expect("Failed to set wallpaper.");

    if cfg!(debug_assertions) {
        println!("New: {}", display_short_path(&new, &bgdir));
    }

    ExitCode::SUCCESS
}
