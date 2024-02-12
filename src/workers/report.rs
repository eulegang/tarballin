use std::path::PathBuf;

use crossbeam_channel::{Receiver, Sender};
use lsp_server::{Message, Notification};
use lsp_types::{
    notification::PublishDiagnostics, Diagnostic, DiagnosticSeverity, Position,
    PublishDiagnosticsParams, Range,
};
use tracing::{error, info_span};
use url::Url;

use crate::{coverage::Trace, line_slice::LineSlice};

pub fn report(rx: &Receiver<(PathBuf, Vec<Trace>)>, tx: &Sender<Message>) {
    let _span = info_span!("report").entered();

    for (path, traces) in rx.iter() {
        let content = match std::fs::read(&path) {
            Ok(c) => c,
            Err(error) => {
                error!(%error, "failed to read written file");
                continue;
            }
        };

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

        let uri = match Url::parse(&format!("file://{}", path.display())) {
            Ok(u) => u,
            Err(error) => {
                error!(%error, path = %path.display(), "failed to parse file url");
                continue;
            }
        };

        let Ok(_) = tx.send(Message::Notification(Notification::new(
            <PublishDiagnostics as lsp_types::notification::Notification>::METHOD.to_string(),
            PublishDiagnosticsParams {
                uri,
                diagnostics: diag,
                version: None,
            },
        ))) else {
            break;
        };
    }
}
