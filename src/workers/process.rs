use std::{collections::HashSet, path::PathBuf};

use crossbeam_channel::{bounded, select, Receiver, SendError, Sender};
use lsp_types::MessageType;
use tracing::{error, info_span};

use crate::{
    coverage::Coverage,
    runner::{runner_thread, Input, Status},
};

use super::{Report, Trigger};

struct State {
    package: String,
    generation: usize,
    coverage: Option<Coverage>,
    interest: HashSet<PathBuf>,
}

#[derive(thiserror::Error, Debug)]
enum ProcessError {
    #[error("{0}")]
    Base(#[from] crate::Error),

    #[error("missing path from coverage {0}")]
    MissingTrace(PathBuf),

    #[error("closed channel")]
    ChannelClose,
}

impl<T> From<SendError<T>> for ProcessError {
    fn from(_: SendError<T>) -> Self {
        ProcessError::ChannelClose
    }
}

pub fn process(package: String, rx: Receiver<Trigger>, tx: Sender<Report>) {
    let _span = info_span!("process worker").entered();

    let (input_tx, input_rx) = bounded(1);
    let (status_tx, status_rx) = bounded(1);

    let handle = std::thread::spawn(|| runner_thread(input_rx, status_tx));

    let coverage = Coverage::load(&package).ok();
    let interest = HashSet::new();
    let mut state = State {
        package,
        generation: 1,
        coverage,
        interest,
    };

    loop {
        let result = select! {
            recv(rx) -> trigger => {
                let Ok(trigger) = trigger else { break; };
                handle_trigger(&mut state, trigger, &input_tx, &tx)
            }

            recv(status_rx) -> status => {
                let Ok(status) = status else { break; };
                handle_status(&mut state, status, &tx)
            }
        };

        if matches!(result, Err(ProcessError::ChannelClose)) {
            break;
        }

        if let Err(error) = result {
            error!(%error, "failed to process input");
        }
    }

    handle.join().unwrap();
}

fn handle_trigger(
    state: &mut State,
    trigger: Trigger,
    input_tx: &Sender<Input>,
    tx: &Sender<Report>,
) -> Result<(), ProcessError> {
    match trigger {
        Trigger::WorkDiagRefresh(_) => todo!(),
        Trigger::Write(_) => {
            input_tx.send(Input::Run)?;
        }

        Trigger::Open(path) => {
            let coverage = Coverage::load(&state.package)?;

            let Some(traces) = coverage.traces.get(&path.display().to_string()) else {
                return Err(ProcessError::MissingTrace(path));
            };

            tx.send(Report::Plain(path, traces.clone()))?;
        }
        Trigger::DocDiag(_, _) => todo!(),
        Trigger::WorkDiag(_) => todo!(),
    }

    Ok(())
}

fn handle_status(
    state: &mut State,
    status: Status,
    tx: &Sender<Report>,
) -> Result<(), ProcessError> {
    match status {
        Status::Success => {
            tracing::debug!("successful coverage found");
            state.generation += 1;
            state.coverage = Coverage::load(&state.package).ok();

            if let Some(cov) = &state.coverage {
                for (path, traces) in &cov.traces {
                    tx.send(Report::Plain(path.into(), traces.clone()))?;
                }
            }
        }
        Status::Failure => {
            tracing::debug!("failed coverage found");
            tx.send(Report::Message(
                MessageType::ERROR,
                "tarpaulin failed to run".to_string(),
            ))?;
        }
        Status::Reset => {
            tracing::debug!("resenting coverage run");
            tx.send(Report::Message(
                MessageType::WARNING,
                "restarting tarpaulin run".to_string(),
            ))?;
        }
        Status::Starting => {
            tracing::debug!("starting coverage run");
            tx.send(Report::Message(
                MessageType::INFO,
                "starting tarpaulin".to_string(),
            ))?;
            state.coverage = None;
            state.interest.clear();
        }
    }

    Ok(())
}