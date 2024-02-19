use std::path::PathBuf;

use crossbeam_channel::{Receiver, SendError, Sender};
use lsp_server::{ExtractError, Message, Notification, Request, RequestId};
use lsp_types::notification::{DidOpenTextDocument, DidSaveTextDocument, Exit};
use lsp_types::request::{
    DocumentDiagnosticRequest, Shutdown, WorkspaceDiagnosticRefresh, WorkspaceDiagnosticRequest,
};
use lsp_types::{notification::Notification as _, request::Request as _};
use serde::de::DeserializeOwned;
use tracing::{error, info_span, trace, warn};
use url::Url;

use super::Trigger;

#[derive(thiserror::Error, Debug)]
enum IngestError {
    #[error("{0}")]
    ExtractRequest(#[from] ExtractError<Request>),

    #[error("{0}")]
    ExtractNotification(#[from] ExtractError<Notification>),

    #[error("invalid file url {0}")]
    InvalidFileUrl(Url),

    #[error("SenderClosed (nonfatal)")]
    SenderClosed,
}

impl<T> From<SendError<T>> for IngestError {
    fn from(_: SendError<T>) -> Self {
        IngestError::SenderClosed
    }
}

pub fn run(rx: Receiver<Message>, tx: Sender<Trigger>) {
    let _span = info_span!("ingestion worker").entered();

    for msg in rx.iter() {
        let result = process(msg, &tx);

        if matches!(result, Err(IngestError::SenderClosed)) {
            trace!("quiting ingest loop");
            return;
        }

        if let Err(error) = result {
            error!(%error, "failed to process lsp message")
        }
    }
}

fn extract_request<R, P>(req: Request) -> Result<(RequestId, R::Params), IngestError>
where
    R: lsp_types::request::Request<Params = P>,
    P: DeserializeOwned,
{
    Ok(req.extract(R::METHOD)?)
}

fn extract_notification<N, P>(note: Notification) -> Result<N::Params, IngestError>
where
    N: lsp_types::notification::Notification<Params = P>,
    P: DeserializeOwned,
{
    Ok(note.extract(N::METHOD)?)
}

fn extract_file_url(url: Url) -> Result<PathBuf, IngestError> {
    match url.to_file_path() {
        Ok(path) => Ok(path),
        Err(_) => Err(IngestError::InvalidFileUrl(url)),
    }
}

fn process(msg: Message, tx: &Sender<Trigger>) -> Result<(), IngestError> {
    let _span = info_span!("msg processing", ?msg).entered();

    match msg {
        lsp_server::Message::Response(_) => (),
        lsp_server::Message::Request(req) => match req.method.as_str() {
            DocumentDiagnosticRequest::METHOD => {
                trace!("document diagnostic request");

                let (id, params) = extract_request::<DocumentDiagnosticRequest, _>(req)?;
                let path = extract_file_url(params.text_document.uri)?;

                tx.send(Trigger::DocDiag(id, path))?;
            }

            WorkspaceDiagnosticRequest::METHOD => {
                trace!("workspace diagnostic request");
                tx.send(Trigger::WorkDiag(req.id))?;
            }

            WorkspaceDiagnosticRefresh::METHOD => {
                trace!("workspace diagnostic refresh");
                tx.send(Trigger::WorkDiagRefresh(req.id))?;
            }

            Shutdown::METHOD => {
                trace!("shutdown request");
                tx.send(Trigger::Exit(req.id))?;
            }

            _ => {
                warn!(req.method, "unsupported lsp request method")
            }
        },
        lsp_server::Message::Notification(note) => match note.method.as_ref() {
            DidSaveTextDocument::METHOD => {
                trace!("recieved did save");

                let params = extract_notification::<DidSaveTextDocument, _>(note)?;
                let path = extract_file_url(params.text_document.uri)?;

                tx.send(Trigger::Write(path))?;
            }

            DidOpenTextDocument::METHOD => {
                trace!("recieved open");

                let params = extract_notification::<DidOpenTextDocument, _>(note)?;
                let path = extract_file_url(params.text_document.uri)?;

                tx.send(Trigger::Open(path))?;
            }

            Exit::METHOD => {
                return Err(IngestError::SenderClosed);
            }

            _ => {
                warn!(note.method, "unsupported lsp notification method")
            }
        },
    }

    Ok(())
}
