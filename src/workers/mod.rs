use lsp_server::RequestId;
use lsp_types::MessageType;
use std::path::PathBuf;

mod ingest;
mod process;
mod report;

pub use ingest::run as ingest;
pub use process::run as process;
pub use report::run as report;

use crate::coverage::Trace;

pub enum Trigger {
    DocDiag(RequestId, PathBuf),
    WorkDiag(RequestId),
    WorkDiagRefresh(RequestId),
    Write(PathBuf),
    Open(PathBuf),
}

pub enum Report {
    Plain(PathBuf, Vec<Trace>),
    Message(MessageType, String),
}
