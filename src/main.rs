use clap::Parser;
use coverage::Worker;
use lsp_server::Connection;
use lsp_types::{
    notification::Notification, InitializeParams, SaveOptions, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncOptions,
};
use tracing::{info, info_span};

mod cli;
mod coverage;
mod line_slice;
mod mode;
mod runner;
mod workers;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    IO(#[from] std::io::Error),

    #[error("failed to run tarpaulin")]
    Failure,

    #[error("{0}")]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    Parse(#[from] url::ParseError),
}

fn main() {
    let args = cli::Args::parse();
    args.setup_subscriber();

    let span = info_span!("main");
    let _guard = span.enter();

    let (conn, threads) = match args.connect {
        cli::Conn::Stdio => {
            info!("planning on connecting over stdio");

            Connection::stdio()
        }
    };

    let (id, params) = conn.initialize_start().unwrap();

    let init: InitializeParams = serde_json::from_value(params).unwrap();

    let server_capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: None,
                change: None,
                will_save: None,
                will_save_wait_until: None,
                save: Some(lsp_types::TextDocumentSyncSaveOptions::SaveOptions(
                    SaveOptions {
                        include_text: Some(false),
                    },
                )),
            },
        )),

        ..ServerCapabilities::default()
    };

    let initialize_data = serde_json::json!({
        "capabilities": server_capabilities,
        "serverInfo": {
            "name": "lsp-tarpaulin",
            "version": env!("CARGO_PKG_VERSION")
        }

    });

    let pkg = {
        let manifest = cargo_toml::Manifest::from_path("Cargo.toml").unwrap();
        manifest.package().name.clone()
    };

    conn.initialize_finish(id, initialize_data).unwrap();

    let worker = Worker::new(pkg, conn.sender);

    let span = info_span!("recv-loop");
    let _guard = span.enter();

    for msg in conn.receiver.iter() {
        let span = info_span!("msg processing", ?msg);
        let _guard = span.enter();

        match msg {
            lsp_server::Message::Request(_) => (),
            lsp_server::Message::Response(_) => (),
            lsp_server::Message::Notification(note) => {
                if note.method == lsp_types::notification::DidSaveTextDocument::METHOD {
                    worker.check();
                    info!("text document did save!");
                }
            }
        }
    }

    threads.join().unwrap();
    worker.dismiss();
}
