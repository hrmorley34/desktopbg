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

#[cfg(feature = "lockscreen")]
mod lockscreen;
#[cfg(feature = "lockscreen")]
use crate::lockscreen::set_lock_screen;

#[cfg(feature = "display")]
mod display;
#[cfg(feature = "display")]
use crate::display::DisplayManager;

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
#[allow(dead_code)]
const LOCKSCREENBG: &str = "lockscreenbg.toml";

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

fn print_help(execname: &str) -> () {
    println!("Usage:");
    println!("  {execname} help");
    println!("    Print this help and exit.");
    println!("  {execname} PATH");
    println!("  {execname} setwallpaper PATH");
    println!("    Set the desktop background from PATH");

    #[cfg(feature = "lockscreen")]
    {
        println!("  {execname} setlockscreen PATH");
        println!("    Set the lockscreen background from PATH");
    }

    #[cfg(feature = "display")]
    {
        println!("  {execname} setwallpapermulti PATH");
        println!("    Set the desktop background from PATH individually on each monitor");
    }
}

enum Command<'a> {
    Help,
    BadArguments,
    SetWallpaper(&'a str),

    #[cfg(feature = "lockscreen")]
    SetLockscreen(&'a str),

    #[cfg(feature = "display")]
    SetWallpaperMulti(&'a str),
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if cfg!(debug_assertions) {
        println!("Args: {}", args.join(", "));
    }

    let command = if args.iter().any(|a| (a == "-h" || a == "--help")) {
        Command::Help
    } else {
        let argrefs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        match &argrefs[1..] {
            ["help"] => Command::Help,
            ["setwallpaper", path] => Command::SetWallpaper(path),
            ["setwallpaper"] => Command::BadArguments, // prevent matching path only

            #[cfg(feature = "lockscreen")]
            ["setlockscreen", path] => Command::SetLockscreen(path),
            #[cfg(feature = "lockscreen")]
            ["setlockscreen"] => Command::BadArguments, // prevent matching path only

            #[cfg(feature = "display")]
            ["setwallpapermulti", path] => Command::SetWallpaperMulti(path),
            #[cfg(feature = "display")]
            ["setwallpapermulti"] => Command::BadArguments, // prevent matching path only

            [path] => Command::SetWallpaper(path), // compatability with old format
            [..] => Command::BadArguments,
        }
    };

    match command {
        Command::Help => {
            print_help(&args[0]);
            ExitCode::SUCCESS
        }
        Command::BadArguments => {
            eprintln!("Invalid arguments");
            print_help(&args[0]);
            ExitCode::FAILURE
        }
        Command::SetWallpaper(bgdir) => {
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

        #[cfg(feature = "lockscreen")]
        Command::SetLockscreen(bgdir) => {
            let bgdir = match canonicalize(PathBuf::from(bgdir)) {
                Err(err) => {
                    eprintln!("Cannot find directory '{}': {}", bgdir, err);
                    return ExitCode::FAILURE;
                }
                Ok(p) => p,
            };

            let buckets: Vec<Bucket> = match get_buckets_from_dir(&bgdir, LOCKSCREENBG) {
                Err(err) => {
                    eprintln!("{err}");
                    return ExitCode::FAILURE;
                }
                Ok(b) => b,
            };

            let predicate = |_: &_| true;
            if cfg!(debug_assertions) {
                // print in debug mode
                list_bucket_files(&buckets, &bgdir, &predicate);
            }

            let new = bucket_random(&buckets, &predicate).expect("Failed to select image.");
            set_lock_screen(&new).expect("Failed to set wallpaper.");

            if cfg!(debug_assertions) {
                println!("New: {}", display_short_path(&new, &bgdir));
            }

            ExitCode::SUCCESS
        }

        #[cfg(feature = "display")]
        Command::SetWallpaperMulti(bgdir) => {
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

            let dm = match DisplayManager::create() {
                Ok(dm) => dm,
                Err(_) => {
                    eprintln!("Failed to get DisplayManager.");
                    return ExitCode::FAILURE;
                }
            };

            let displays = match dm.get_all_monitors() {
                Ok(displays) => displays,
                Err(_) => {
                    eprintln!("Failed to get monitors.");
                    return ExitCode::FAILURE;
                }
            };

            for (i, monitor_id) in displays.iter() {
                let path = bucket_random(&buckets, &|_| true).expect("Failed to select image.");
                dm.set_wallpaper(monitor_id, &path)
                    .expect("Failed to set wallpaper.");
                eprintln!("Set monitor {}", i + 1);
            }
            ExitCode::SUCCESS
        }
    }
}
