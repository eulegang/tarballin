use std::sync::mpsc::Sender;

use crossbeam_channel::Receiver;
use lsp_server::Message;
use lsp_types::{
    notification::Notification as _, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
};
use tracing::{error, info_span, trace};

use super::Trigger;

pub fn ingest(rx: &Receiver<Message>, tx: &Sender<Trigger>) {
    let _span = info_span!("ingestion worker").entered();

    for msg in rx.iter() {
        let _span = info_span!("msg processing", ?msg).entered();

        match msg {
            lsp_server::Message::Request(_) => (),
            lsp_server::Message::Response(_) => (),
            lsp_server::Message::Notification(note) => {
                match note.method.as_ref() {
                    lsp_types::notification::DidSaveTextDocument::METHOD => {
                        trace!("recieved did save");

                        let params = note
                            .extract::<DidSaveTextDocumentParams>(
                                lsp_types::notification::DidSaveTextDocument::METHOD,
                            )
                            .unwrap();

                        let Ok(path) = params.text_document.uri.to_file_path() else {
                            error!(uri = %params.text_document.uri, "not valid local file path");
                            continue;
                        };

                        let Ok(_) = tx.send(Trigger::Write(path)) else {
                            break;
                        };
                    }

                    lsp_types::notification::DidOpenTextDocument::METHOD => {
                        trace!("recieved open");

                        let params = note
                            .extract::<DidOpenTextDocumentParams>(
                                lsp_types::notification::DidSaveTextDocument::METHOD,
                            )
                            .unwrap();

                        let Ok(path) = params.text_document.uri.to_file_path() else {
                            error!(uri = %params.text_document.uri, "not valid local file path");
                            continue;
                        };

                        let Ok(_) = tx.send(Trigger::Open(path)) else {
                            break;
                        };
                    }

                    _ => (),
                };
            }
        }
    }
}
