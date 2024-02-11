use std::{
    collections::HashMap,
    fs::File,
    process::{Command, Stdio},
};

use crossbeam_channel::{Receiver, Sender};
use lsp_server::Message;
use serde::Deserialize;
use tracing::{debug, error, info, info_span, trace};

#[derive(Deserialize)]
pub struct Coverage {
    pub traces: HashMap<String, Vec<Trace>>,
}

#[derive(Deserialize)]
pub struct Trace {
    pub line: u64,
    pub address: Vec<usize>,
    pub length: usize,
    pub fn_name: Option<String>,
}

#[derive(Deserialize)]
pub struct Stats {
    #[serde(rename = "Line")]
    pub line: usize,
}

pub struct Worker {
    handle: std::thread::JoinHandle<()>,
    tx: Sender<()>,
    report: Sender<Message>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    IO(#[from] std::io::Error),

    #[error("failed to run tarpaulin")]
    Failure,

    #[error("{0}")]
    Serde(#[from] serde_json::Error),
}

impl Worker {
    pub fn new(pkg: String, report: crossbeam_channel::Sender<Message>) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(1);

        let handle = std::thread::spawn(move || Worker::run_loop(pkg, rx));

        Worker { handle, tx, report }
    }

    pub fn check(&self) {
        self.tx.send(()).unwrap();
    }

    pub fn dismiss(self) {
        self.handle.join().unwrap();
    }

    fn run_loop(pkg: String, rx: Receiver<()>) {
        let span = info_span!("worker");
        let _guard = span.enter();

        for _ in rx {
            trace!("running tarpaulin");

            let coverage = match Self::run(&pkg) {
                Err(error) => {
                    error!(?error);
                    continue;
                }
                Ok(coverage) => coverage,
            };

            debug!(keys = ?coverage.traces.keys());
        }
    }

    fn run(pkg: &str) -> Result<Coverage, Error> {
        /*
        let mut proc = Command::new("cargo")
            .arg("tarpaulin")
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .spawn()?;
        let status = proc.wait()?;

        if !status.success() {
            return Err(Error::Failure);
        }
        */

        let file = File::open(format!("./target/tarpaulin/{pkg}-coverage.json"))?;
        let coverage = serde_json::from_reader(file)?;

        Ok(coverage)
    }
}

#[test]
fn xyz() {
    let file = File::open(format!("./target/tarpaulin/lsp-tarpaulin-coverage.json")).unwrap();
    let _: Coverage = serde_json::from_reader(file).unwrap();
}
