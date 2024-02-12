use std::process::{Command, Stdio};

use tracing::trace;

use crate::Error;

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
