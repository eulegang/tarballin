use std::{
    process::{Child, Command, Stdio},
    time::Duration,
};

use crossbeam_channel::{select, Receiver, Sender};
use tracing::{error, trace};

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("{0}")]
    IO(#[from] std::io::Error),

    #[error("Failed to run tarpaulin")]
    Failure,
}

#[derive(PartialEq, Eq)]
pub enum Input {
    Run,
    Kill,
}

pub enum Status {
    Success,
    Failure,
    Reset,
    Starting,
}

pub fn runner_thread(input: Receiver<Input>, status: Sender<Status>) {
    loop {
        let Ok(w) = input.recv() else {
            return;
        };

        if w != Input::Run {
            continue;
        }

        if status.send(Status::Starting).is_err() {
            return;
        }

        let mut child = match run() {
            Ok(child) => child,
            Err(error) => {
                error!(%error, "failed to run command");
                continue;
            }
        };

        'check: loop {
            let Ok(job_st) = child.try_wait() else {
                break 'check;
            };

            if let Some(st) = job_st {
                trace!(%st, "job completed");
                if st.success() {
                    if status.send(Status::Success).is_err() {
                        return;
                    }
                } else {
                    if status.send(Status::Failure).is_err() {
                        return;
                    }
                }

                break 'check;
            }

            let i = select! {
                recv(input) -> i => {
                    let Ok(i) = i else { return; };
                    i
                }

                default(Duration::from_millis(200)) => {
                    tracing::trace!("command watcher wakeup");
                    continue 'check;
                }
            };

            match i {
                Input::Run => {
                    if status.send(Status::Reset).is_err() {
                        return;
                    }

                    child = match run() {
                        Ok(child) => child,
                        Err(error) => {
                            error!(%error, "failed to run command");
                            continue;
                        }
                    };
                }

                Input::Kill => {
                    let _ = child.kill();
                    break 'check;
                }
            }
        }
    }
}

fn run() -> Result<Child, RunError> {
    trace!("spawning tarpaulin");
    let proc = Command::new("cargo")
        .arg("tarpaulin")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .spawn()?;

    Ok(proc)
}
