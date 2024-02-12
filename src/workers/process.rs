use std::path::PathBuf;

use crossbeam_channel::{Receiver, Sender};
use tracing::{error, info_span};

use crate::coverage::{Coverage, Trace};

use super::Trigger;

pub fn process(
    package: String,
    resent: &Sender<Trigger>,
    rx: &Receiver<Trigger>,
    tx: &Sender<(PathBuf, Vec<Trace>)>,
) {
    let _span = info_span!("process worker").entered();

    for trigger in rx.iter() {
        match trigger {
            Trigger::Write(_) => todo!(),
            Trigger::Open(path) => {
                let mut coverage = match Coverage::load(&package) {
                    Ok(c) => c,
                    Err(error) => {
                        error!(%error, "failed to load coverage report");
                        continue;
                    }
                };

                let Some(traces) = coverage.traces.remove(&path.display().to_string()) else {
                    error!(file = %path.display(), "file written that was not in coverage report");
                    continue;
                };

                let Ok(_) = tx.send((path, traces)) else {
                    break;
                };
            }
        }
    }
}
