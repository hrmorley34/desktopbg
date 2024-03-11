#![cfg_attr(debug_assertions, windows_subsystem = "console")]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window for release build

use rand::Rng;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::{env, fs, process};
use toml;
use walkdir::WalkDir;
use wallpaper;

static IMAGE_EXTS: [&str; 3] = ["png", "jpg", "jpeg"];

fn has_image_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| IMAGE_EXTS.iter().any(|ext| e.eq_ignore_ascii_case(ext)))
        .unwrap_or(false)
}

type PathPredicate<'l> = dyn Fn(&PathBuf) -> bool + 'l;

struct Bucket {
    path: PathBuf,
    weight: i64,
}

impl Bucket {
    fn get_contents(&self) -> impl Iterator<Item = PathBuf> {
        WalkDir::new(&self.path)
            .into_iter()
            .filter_map(|p| p.ok())
            .filter(|p| p.file_type().is_file())
            .filter(|p| has_image_ext(p.path()))
            .map(|p| p.into_path())
    }

    fn has_contents(&self, predicate: &PathPredicate) -> bool {
        self.get_contents().any(|p| predicate(&p))
    }

    fn get_weight(&self, predicate: &PathPredicate) -> i64 {
        if self.has_contents(predicate) {
            self.weight
        } else {
            0
        }
    }

    fn get_random(&self, predicate: &PathPredicate) -> Result<PathBuf, ()> {
        let contents: Vec<_> = self.get_contents().filter(predicate).collect();
        let length = contents.len();
        if length <= 0 {
            return Err(());
        }
        let index = rand::thread_rng().gen_range(0..length);
        Ok(contents[index].clone())
    }
}

fn weighted_random<'b>(buckets: &'b [Bucket], predicate: &PathPredicate) -> Result<&'b Bucket, ()> {
    let weights: Vec<i64> = buckets.iter().map(|b| b.get_weight(predicate)).collect();
    let total_weight = weights.iter().sum();
    if total_weight <= 0 {
        return Err(());
    }
    let mut index = rand::thread_rng().gen_range(0..total_weight);
    for (b, i) in buckets.iter().zip(weights) {
        index -= i;
        if index < 0 {
            return Ok(b);
        }
    }
    Err(())
}

fn bucket_random(buckets: &[Bucket], predicate: &PathPredicate) -> Result<PathBuf, ()> {
    weighted_random(buckets, predicate)?.get_random(predicate)
}

fn display_path(path: &Path) -> String {
    let s = path.display().to_string();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_owned()
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

fn main() {
    let args: Vec<String> = env::args().collect();
    if cfg!(debug_assertions) {
        println!("Args: {}", args.join(", "));
    }
    if args.len() != 2 {
        eprintln!("Expected exactly 1 argument - the directory.");
        process::exit(1);
    }

    let bgdir = &args[1];
    let bgdir = PathBuf::from(bgdir).canonicalize().unwrap_or_else(|err| {
        eprintln!("Cannot find directory '{}': {}", bgdir, err);
        process::exit(1);
    });

    let bucketsfile = bgdir.join("desktopbg.toml");
    if !bucketsfile.exists() {
        eprintln!(
            "Cannot find bucketsfile! Looking for: {}",
            display_path(&bucketsfile)
        );
        process::exit(1);
    }
    let contents = fs::read_to_string(bucketsfile).unwrap_or_else(|err| {
        eprintln!("Cannot open bucketsfile: {err}");
        process::exit(1);
    });
    let bucketsfile: BucketsFile = toml::from_str(&contents).unwrap_or_else(|err| {
        eprintln!("Cannot parse bucketsfile: {err}");
        process::exit(1);
    });

    let buckets: Vec<Bucket> = bucketsfile
        .backgrounds
        .iter()
        .filter_map(|(dirname, weight)| {
            let path = bgdir
                .join(dirname)
                // `canonicalize` guarantees existence and corrects capitalisation
                .canonicalize()
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
        .collect();

    let old_p = wallpaper::get()
        .ok()
        .map(PathBuf::from)
        .and_then(|p| p.canonicalize().ok());
    let old = old_p.as_deref();

    let check_not_old = |p: &PathBuf| old != Some(p); // !old.is_some_and(|old| p == old);

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
        .expect("failed to set wallpaper");

    if cfg!(debug_assertions) {
        println!("New: {}", display_short_path(&new, &bgdir));
    }
}
