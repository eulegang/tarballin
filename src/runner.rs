use std::{
    collections::HashSet,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
};

use crossbeam_channel::{select, Receiver, Sender};
use tracing::{error, trace};

use crate::Error;

pub struct Runner {
    agg_handle: JoinHandle<()>,
    run_handle: JoinHandle<()>,
    sender: Sender<PathBuf>,
}

impl Runner {
    pub fn spawn(results: Sender<Option<HashSet<PathBuf>>>) -> Self {
        let (sender, recver) = crossbeam_channel::bounded(1);
        let rcond = Arc::new((Mutex::new(false), Condvar::new()));
        let acond = rcond.clone();
        let (rtx, rrx) = crossbeam_channel::bounded(1);

        let agg_handle = std::thread::spawn(|| agg_loop(acond, rrx, recver, results));
        let run_handle = std::thread::spawn(|| run_loop(rcond, rtx));

        Self {
            agg_handle,
            run_handle,
            sender,
        }
    }

    pub fn join(self) {
        let _ = self.agg_handle.join();
        let _ = self.run_handle.join();
    }

    pub fn saved(&self, path: PathBuf) {
        self.sender.send(path).unwrap();
    }
}

pub fn agg_loop(
    arc: Arc<(Mutex<bool>, Condvar)>,
    signal: Receiver<bool>,
    income: Receiver<PathBuf>,
    results: Sender<Option<HashSet<PathBuf>>>,
) {
    let mut agg = HashSet::new();
    loop {
        select! {
            recv(signal) -> outcome => {
                if let Ok(outcome) = outcome {
                    if outcome {
                        let res = agg;
                        agg = HashSet::new();
                        if results.send(Some(res)).is_err() {
                            return;
                        }
                    } else {
                        agg.clear();
                        if results.send(None).is_err() {
                            return;
                        }
                    }

                }
            }

            recv(income) -> path => {
                if let Ok(path) = path {
                    agg.insert(path);
                }

                {
                    let mut running = arc.0.lock().unwrap();
                    if !*running {
                        *running = true;
                        arc.1.notify_one();
                    }
                }
            }
        }
    }
}

pub fn run_loop(arc: Arc<(Mutex<bool>, Condvar)>, signal: Sender<bool>) {
    loop {
        let mut guard = arc.0.lock().unwrap();

        while !*guard {
            guard = arc.1.wait(guard).unwrap();
        }

        if let Err(error) = run() {
            error!(%error, "failed to run tarpaulin");
            *guard = false;
            if signal.send(false).is_err() {
                return;
            }
        } else {
            *guard = false;
            if signal.send(true).is_err() {
                return;
            }
        }
    }
}

pub fn run() -> Result<(), Error> {
    trace!("spawning tarpaulin");
    let mut proc = Command::new("cargo")
        .arg("tarpaulin")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .spawn()?;

    trace!("waiting tarpaulin");
    let status = proc.wait()?;

    if !status.success() {
        return Err(Error::Failure);
    }

    Ok(())
}
