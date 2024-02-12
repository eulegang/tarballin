use std::{
    collections::HashMap,
    fs::File,
    process::{Command, Stdio},
};

use crossbeam_channel::{Receiver, Sender};
use lsp_server::{Message, Notification};
use lsp_types::{
    notification::PublishDiagnostics, Diagnostic, DiagnosticSeverity, Position,
    PublishDiagnosticsParams, Range, Url,
};
use serde::Deserialize;
use tracing::{debug, error, info_span, trace};

use crate::Error;

#[derive(Deserialize)]
pub struct Coverage {
    pub traces: HashMap<String, Vec<Trace>>,
}

#[derive(Deserialize)]
pub struct Trace {
    pub line: u32,
    pub address: Vec<usize>,
    pub length: usize,
    pub stats: Stats,
    pub fn_name: Option<String>,
}

#[derive(Deserialize)]
pub struct Stats {
    #[serde(rename = "Line")]
    pub line: usize,
}

impl Coverage {
    pub fn load(package: &str) -> Result<Self, Error> {
        let file = File::open(format!("./target/tarpaulin/{package}-coverage.json"))?;
        let coverage = serde_json::from_reader(file)?;

        Ok(coverage)
    }
}

pub struct Worker {
    handle: std::thread::JoinHandle<()>,
    tx: Sender<()>,
    report: Sender<Message>,
}

impl Worker {
    pub fn new(pkg: String, report: crossbeam_channel::Sender<Message>) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(1);

        let r = report.clone();
        let handle = std::thread::spawn(move || Worker::run_loop(r, pkg, rx));

        Worker { handle, tx, report }
    }

    pub fn check(&self) {
        self.tx.send(()).unwrap();
    }

    pub fn dismiss(self) {
        self.handle.join().unwrap();
    }

    fn run_loop(report: Sender<Message>, pkg: String, rx: Receiver<()>) {
        let span = info_span!("worker");
        let _guard = span.enter();

        for _ in rx {
            trace!("running tarpaulin");

            let coverage = match Self::run(&pkg) {
                Err(error) => {
                    error!(?error);
                    continue;
                }
                Ok(coverage) => coverage,
            };

            debug!(keys = ?coverage.traces.keys());

            if let Err(error) = Self::send_report(&report, coverage) {
                error!(%error);
            }
        }
    }

    fn run(pkg: &str) -> Result<Coverage, Error> {
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

        trace!("loading report");

        let file = File::open(format!("./target/tarpaulin/{pkg}-coverage.json"))?;
        let coverage = serde_json::from_reader(file)?;

        Ok(coverage)
    }

    fn send_report(report: &Sender<Message>, coverage: Coverage) -> Result<(), Error> {
        for (path, traces) in coverage.traces {
            let content = std::fs::read(&path)?;
            let line_slices = LineSlice::build(&content);

            let mut diag = Vec::new();
            for trace in traces {
                if trace.stats.line == 0 {
                    let line = trace.line.saturating_sub(1);

                    let line_slice = &line_slices[line as usize];

                    diag.push(Diagnostic {
                        range: Range::new(
                            Position {
                                line,
                                character: (line_slice.begin - line_slice.start) as u32,
                            },
                            Position {
                                line,
                                character: (line_slice.end - line_slice.start) as u32,
                            },
                        ),
                        severity: Some(DiagnosticSeverity::WARNING),
                        code: None,
                        code_description: None,
                        source: Some("lsp-tarpaulin".to_string()),
                        message: "not covered by tests".to_string(),
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
            }

            let _ = report.send(Message::Notification(Notification::new(
                <PublishDiagnostics as lsp_types::notification::Notification>::METHOD.to_string(),
                PublishDiagnosticsParams {
                    uri: Url::parse(&format!("file://{path}"))?,
                    diagnostics: diag,
                    version: None,
                },
            )));
        }

        Ok(())
    }
}

#[derive(Debug)]
struct LineSlice {
    start: usize,
    begin: usize,
    end: usize,
}

impl LineSlice {
    pub fn build(slice: &[u8]) -> Vec<LineSlice> {
        let mut start = 0;
        let mut begin = 0;
        let mut end = 0;
        let mut pre = true;

        let mut lines = Vec::new();

        for i in 0..slice.len() {
            if slice[i] == b'\n' {
                if pre {
                    begin = i;
                }

                end = i;
                pre = true;

                let slice = LineSlice { start, begin, end };
                lines.push(slice);

                start = i + 1;
                begin = i + 1;
                end = i + 1;

                continue;
            }

            if pre && !matches!(slice[i], b' ' | b'\t' | b'\r') {
                pre = false;
                begin = i;
            }
        }

        lines
    }
}

#[test]
fn xyz() {
    let file = File::open(format!("./target/tarpaulin/lsp-tarpaulin-coverage.json")).unwrap();
    let _: Coverage = serde_json::from_reader(file).unwrap();
}
