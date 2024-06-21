use dunce::canonicalize;
use rand::Rng;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::utils::display_path;

static IMAGE_EXTS: [&str; 3] = ["png", "jpg", "jpeg"];

fn has_image_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| IMAGE_EXTS.iter().any(|ext| e.eq_ignore_ascii_case(ext)))
        .unwrap_or(false)
}

pub type PathPredicate<'l> = dyn Fn(&PathBuf) -> bool + 'l;

pub struct Bucket {
    pub path: PathBuf,
    pub weight: i64,
}

impl Bucket {
    pub fn get_contents(&self) -> impl Iterator<Item = PathBuf> {
        WalkDir::new(&self.path)
            .into_iter()
            .filter_map(|p| p.ok())
            .filter(|p| p.file_type().is_file())
            .filter(|p| has_image_ext(p.path()))
            .map(|p| p.into_path())
    }

    pub fn has_contents(&self, predicate: &PathPredicate) -> bool {
        self.get_contents().any(|p| predicate(&p))
    }

    pub fn get_weight(&self, predicate: &PathPredicate) -> i64 {
        if self.has_contents(predicate) {
            self.weight
        } else {
            0
        }
    }

    pub fn get_random(&self, predicate: &PathPredicate) -> Result<PathBuf, ()> {
        let contents: Vec<_> = self.get_contents().filter(predicate).collect();
        let length = contents.len();
        if length <= 0 {
            return Err(());
        }
        let index = rand::thread_rng().gen_range(0..length);
        Ok(contents[index].clone())
    }
}

pub fn weighted_random<'b>(
    buckets: &'b [Bucket],
    predicate: &PathPredicate,
) -> Result<&'b Bucket, ()> {
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

pub fn bucket_random(buckets: &[Bucket], predicate: &PathPredicate) -> Result<PathBuf, ()> {
    weighted_random(buckets, predicate)?.get_random(predicate)
}

#[derive(Deserialize)]
pub struct BucketsFile {
    backgrounds: toml::Table,
}

#[allow(dead_code)]
pub const DESKTOPBG: &str = "desktopbg.toml";
#[allow(dead_code)]
pub const LOCKSCREENBG: &str = "lockscreenbg.toml";

pub fn get_buckets_from_dir(bgdir: &PathBuf, toml: &str) -> Result<Vec<Bucket>, String> {
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
