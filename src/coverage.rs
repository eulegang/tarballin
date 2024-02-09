use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};
use lsp_server::Message;
use serde::Deserialize;
use tracing::{info, info_span, trace};

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

impl Worker {
    pub fn new(pkg: String, report: crossbeam_channel::Sender<Message>) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(1);

        let handle = std::thread::spawn(move || Worker::run(pkg, rx));

        Worker { handle, tx, report }
    }

    pub fn check(&self) {
        self.tx.send(()).unwrap();
    }

    pub fn dismiss(self) {
        self.handle.join().unwrap();
    }

    fn run(pkg: String, rx: Receiver<()>) {
        let span = info_span!("worker");
        let _guard = span.enter();

        for _ in rx {
            trace!("running tarpaulin");
            info!("todo: check for {pkg}-coverage.json");
        }
    }
}
