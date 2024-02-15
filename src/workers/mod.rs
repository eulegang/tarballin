use lsp_server::RequestId;
use std::path::PathBuf;

mod ingest;
mod process;
mod report;

pub use ingest::ingest;
pub use process::process;
pub use report::report;

enum Trigger {
    DocDiag(RequestId, PathBuf),
    WorkDiag(RequestId),
    WorkDiagRefresh(RequestId),
    Write(PathBuf),
    Open(PathBuf),
}
