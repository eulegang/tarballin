use std::{
    collections::HashSet,
    fs::File,
    path::{Path, PathBuf},
};

use crossbeam_channel::{bounded, select, Receiver, SendError, Sender};
use lsp_types::MessageType;
use tracing::{debug, error, info_span, trace};

use crate::{
    coverage::Coverage,
    ignore::Ignore,
    runner::{runner_thread, Input, Status},
};

use super::{Report, Trigger};

struct State {
    package: String,
    target: PathBuf,
    generation: usize,
    ignore: Ignore,
    coverage: Option<Coverage>,
    interest: HashSet<PathBuf>,
    workspaces: Vec<PathBuf>,
}

#[derive(thiserror::Error, Debug)]
enum ProcessError {
    #[error("{0}")]
    Base(#[from] crate::Error),

    #[error("missing path from coverage {0}")]
    MissingTrace(PathBuf),

    #[error("failed to read {0}: {1}")]
    FailedRead(PathBuf, std::io::Error),

    #[error("{0}")]
    Eyre(#[from] eyre::Error),

    #[error("closed channel")]
    ChannelClose,
}

impl<T> From<SendError<T>> for ProcessError {
    fn from(_: SendError<T>) -> Self {
        ProcessError::ChannelClose
    }
}

pub fn run(
    package: String,
    target: PathBuf,
    workspaces: Vec<PathBuf>,
    ignore: Ignore,
    rx: Receiver<Trigger>,
    tx: Sender<Report>,
) {
    let _span = info_span!("process worker").entered();

    let (input_tx, input_rx) = bounded(1);
    let (status_tx, status_rx) = bounded(1);

    let handle = {
        let target = target.clone();
        std::thread::spawn(|| runner_thread(target, input_rx, status_tx))
    };

    let mut coverage = None;
    for workspace in &workspaces {
        let mut path = workspace.to_path_buf();
        path.push("target");
        path.push(".tarballin-cache.json");

        let Ok(file) = File::open(path) else { continue };
        let Ok(cov) = serde_json::from_reader(file) else {
            continue;
        };

        debug!("loaded cached coverage");
        coverage = Some(cov);
        break;
    }

    debug!(loaded = coverage.is_some(), "using cached coverage");

    let interest = HashSet::new();
    let mut state = State {
        ignore,
        target,
        package,
        generation: 1,
        coverage,
        interest,
        workspaces,
    };

    if let Some(cov) = &state.coverage {
        for (path, traces) in &cov.traces {
            let path: PathBuf = path.into();
            let result = state.ignore.matches(state.strip_workspaces(&path));
            debug!(?result, "ignore result");

            let Ok(content) =
                std::fs::read(&path).map_err(|e| ProcessError::FailedRead(path.clone(), e))
            else {
                continue;
            };

            let Ok(traces) = result.filter(&content, traces) else {
                continue;
            };

            if tx.send(Report::Plain(path, traces)).is_err() {
                return;
            }
        }
    }

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
            debug!("quiting processing loop");
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
            let coverage = Coverage::load(&state.package, &state.target)?;
            //let path = state.strip_workspaces(path);
            let result = state.ignore.matches(state.strip_workspaces(&path));
            debug!(?result, "ignore result");

            let Some(traces) = coverage.traces.get(&path) else {
                return Err(ProcessError::MissingTrace(path));
            };

            let content =
                std::fs::read(&path).map_err(|e| ProcessError::FailedRead(path.clone(), e))?;

            let traces = result.filter(&content, traces)?;

            tx.send(Report::Plain(path, traces))?;
        }
        Trigger::DocDiag(_, _) => todo!(),
        Trigger::WorkDiag(_) => todo!(),

        Trigger::Exit(id) => {
            trace!("exiting process worker");
            tx.send(Report::Exit(id))?;
            input_tx.send(Input::Exit)?;
            return Err(ProcessError::ChannelClose);
        }
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
            state.coverage = Coverage::load(&state.package, &state.target).ok();
            for workspace in &state.workspaces {
                let _ = cache(&state.package, &state.target, workspace);
            }

            if let Some(cov) = &state.coverage {
                for (path, traces) in &cov.traces {
                    let path: PathBuf = path.into();
                    let result = state.ignore.matches(state.strip_workspaces(&path));
                    debug!(?result, "ignore result");

                    let content = std::fs::read(&path)
                        .map_err(|e| ProcessError::FailedRead(path.clone(), e))?;

                    let traces = result.filter(&content, traces)?;

                    tx.send(Report::Plain(path, traces))?;
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

fn cache(package: &str, target: &Path, workspace: &Path) -> std::io::Result<()> {
    let mut src = target.to_path_buf();
    src.push("tarpaulin");
    src.push(format!("{package}-coverage.json"));

    let mut dst = workspace.to_path_buf();
    dst.push("target");
    dst.push(".tarballin-cache.json");

    std::fs::copy(src, dst)?;

    Ok(())
}

impl State {
    fn strip_workspaces<'a>(&self, path: &'a Path) -> &'a Path {
        for workspace in &self.workspaces {
            if let Ok(p) = path.strip_prefix(workspace) {
                return p;
            }
        }

        path
    }
}
