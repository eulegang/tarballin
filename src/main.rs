use clap::Parser;
use lsp_server::Connection;
use lsp_types::{notification::Notification, InitializeParams};
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

    let mode = mode::Mode::try_from(&init).unwrap();
    let capabilities = mode.capabilities();
    let initialize_data = serde_json::json!({
        "capabilities": capabilities,
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

    //mode.run(&conn, &pkg);

    let _span = info_span!("recv-loop").entered();

    for msg in conn.receiver.iter() {
        let _span = info_span!("msg processing", ?msg).entered();

        match msg {
            lsp_server::Message::Request(_) => (),
            lsp_server::Message::Response(_) => (),
            lsp_server::Message::Notification(note) => {
                if note.method == lsp_types::notification::DidSaveTextDocument::METHOD {
                    info!("text document did save!");
                }
            }
        }
    }

    threads.join().unwrap();
}
