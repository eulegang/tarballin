use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use tracing::debug;

use crate::Error;

#[derive(Deserialize)]
pub struct Coverage {
    pub traces: HashMap<PathBuf, Vec<Trace>>,
}

#[derive(Deserialize, Clone)]
pub struct Trace {
    pub line: u32,
    pub address: Vec<usize>,
    pub length: usize,
    pub stats: Stats,
    pub fn_name: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct Stats {
    #[serde(rename = "Line")]
    pub line: usize,
}

impl Coverage {
    pub fn load(package: &str, target: &Path) -> Result<Self, Error> {
        let mut path = target.to_path_buf();
        path.push("tarpaulin");
        path.push(format!("{package}-coverage.json"));

        debug!(path = %path.display(), "looking for coverage file");

        let file = File::open(path)?;
        let coverage = serde_json::from_reader(file)?;

        Ok(coverage)
    }
}
