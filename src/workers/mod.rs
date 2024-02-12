use std::path::PathBuf;

mod ingest;
mod process;
mod report;

pub use ingest::ingest;
pub use process::process;
pub use report::report;

enum Trigger {
    Write(PathBuf),
    Open(PathBuf),
}
