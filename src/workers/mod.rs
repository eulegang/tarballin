use lsp_server::RequestId;
use lsp_types::MessageType;
use std::path::PathBuf;

mod ingest;
mod process;
mod report;

pub use ingest::ingest;
pub use process::process;
pub use report::report;

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
