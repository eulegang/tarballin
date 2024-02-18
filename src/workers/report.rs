use std::path::{Path, PathBuf};

use crossbeam_channel::{Receiver, SendError, Sender};
use lsp_server::{Message, Notification};
use lsp_types::{
    notification::{PublishDiagnostics, ShowMessage},
    Diagnostic, DiagnosticSeverity, MessageType, Position, PublishDiagnosticsParams, Range,
    ShowMessageParams,
};
use tracing::{error, info_span};
use url::Url;

use crate::{coverage::Trace, line_slice::LineSlice};

use super::Report;

#[derive(thiserror::Error, Debug)]
enum ReportError {
    #[error("failed to read {}: \"{}\"", .0.display(), .1)]
    FailedFileRead(PathBuf, std::io::Error),

    #[error("failed to make local file url: \"{0}\"")]
    UrlParseError(#[from] url::ParseError),

    #[error("shutdown sender")]
    SendShutdown,
}

impl<T> From<SendError<T>> for ReportError {
    fn from(_: SendError<T>) -> Self {
        ReportError::SendShutdown
    }
}

pub fn run(rx: Receiver<Report>, tx: Sender<Message>) {
    let _span = info_span!("report").entered();

    for msg in rx.iter() {
        let result = match msg {
            Report::Plain(path, trace) => send_trace(&tx, &path, &trace),
            Report::Message(ty, message) => send_message(&tx, ty, message),
        };

        if matches!(result, Err(ReportError::SendShutdown)) {
            return;
        }

        if let Err(error) = result {
            error!(%error, "failed to send message to client");
        }
    }
}

fn send_trace(tx: &Sender<Message>, path: &Path, traces: &[Trace]) -> Result<(), ReportError> {
    let content =
        std::fs::read(path).map_err(|e| ReportError::FailedFileRead(path.to_path_buf(), e))?;

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

    let uri = Url::parse(&format!("file://{}", path.display()))?;

    tx.send(Message::Notification(Notification::new(
        <PublishDiagnostics as lsp_types::notification::Notification>::METHOD.to_string(),
        PublishDiagnosticsParams {
            uri,
            diagnostics: diag,
            version: None,
        },
    )))?;

    Ok(())
}

fn send_message(
    tx: &Sender<Message>,
    typ: MessageType,
    message: String,
) -> Result<(), ReportError> {
    tx.send(Message::Notification(Notification::new(
        <ShowMessage as lsp_types::notification::Notification>::METHOD.to_string(),
        ShowMessageParams { typ, message },
    )))?;

    Ok(())
}
