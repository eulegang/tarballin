use std::{collections::HashMap, fs::File};

use serde::Deserialize;

use crate::Error;

#[derive(Deserialize)]
pub struct Coverage {
    pub traces: HashMap<String, Vec<Trace>>,
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
    pub fn load(package: &str) -> Result<Self, Error> {
        let file = File::open(format!("./target/tarpaulin/{package}-coverage.json"))?;
        let coverage = serde_json::from_reader(file)?;

        Ok(coverage)
    }
}
