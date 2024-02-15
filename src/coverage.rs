use std::{collections::HashMap, fs::File, sync::Arc};

use serde::Deserialize;

use crate::Error;

#[derive(Deserialize)]
pub struct Coverage {
    pub traces: HashMap<String, Arc<[Trace]>>,
}

#[derive(Deserialize)]
pub struct Trace {
    pub line: u32,
    pub address: Vec<usize>,
    pub length: usize,
    pub stats: Stats,
    pub fn_name: Option<String>,
}

#[derive(Deserialize)]
pub struct Stats {
    #[serde(rename = "Line")]
    pub line: usize,
}

impl Coverage {
    pub fn load(package: &str) -> Result<Self, Error> {
        let file = File::open(format!("./target/tarpaulin/{package}-coverage.json"))?;
        let coverage = serde_json::from_reader(file)?;

        Ok(coverage)
    }
}
