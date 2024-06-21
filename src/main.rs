#![cfg_attr(debug_assertions, windows_subsystem = "console")]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window for release build

use std::{env, process::ExitCode};

mod bucket;
mod utils;
mod wallpaper;
use crate::bucket::{get_buckets_from_dir, DESKTOPBG, LOCKSCREENBG};
use crate::utils::convert_bgdir;
use crate::wallpaper::set_wallpaper_from_buckets;

#[cfg(feature = "lockscreen")]
mod lockscreen;
#[cfg(feature = "lockscreen")]
use crate::lockscreen::set_lockscreen_from_buckets;

#[cfg(feature = "display")]
mod display;
#[cfg(feature = "display")]
use crate::display::set_wallpaper_multi_from_buckets;

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

fn run_command(command: Command, execname: &str) -> Result<(), Option<String>> {
    match command {
        Command::Help => {
            print_help(execname);
            Ok(())
        }
        Command::BadArguments => {
            eprintln!("Invalid arguments");
            print_help(execname);
            Err(None)
        }
        Command::SetWallpaper(bgdir) => {
            let bgdir = convert_bgdir(bgdir)?;
            let buckets = get_buckets_from_dir(&bgdir, DESKTOPBG)?;

            Ok(set_wallpaper_from_buckets(&bgdir, &buckets)?)
        }

        #[cfg(feature = "lockscreen")]
        Command::SetLockscreen(bgdir) => {
            let bgdir = convert_bgdir(bgdir)?;
            let buckets = get_buckets_from_dir(&bgdir, LOCKSCREENBG)?;

            Ok(set_lockscreen_from_buckets(&bgdir, &buckets)?)
        }

        #[cfg(feature = "display")]
        Command::SetWallpaperMulti(bgdir) => {
            let bgdir = convert_bgdir(bgdir)?;
            let buckets = get_buckets_from_dir(&bgdir, DESKTOPBG)?;

            Ok(set_wallpaper_multi_from_buckets(&bgdir, &buckets)?)
        }
    }
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
            [] => Command::Help,
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

    run_command(command, &args[0]).map_or_else(
        |e| {
            if let Some(e) = e {
                eprintln!("{}", e);
            }
            ExitCode::FAILURE
        },
        |_| ExitCode::SUCCESS,
    )
}
