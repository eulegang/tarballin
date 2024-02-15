use clap::Parser;
use crossbeam_channel::bounded;
use lsp_server::Connection;
use lsp_types::InitializeParams;
use tracing::{debug, info, info_span};

mod cli;
mod coverage;
mod ignore;
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

    debug!(%params, "initialization params");

    let init: InitializeParams = serde_json::from_value(params).unwrap();

    let mode = mode::Mode::try_from(&init).unwrap();
    debug!(?mode, "determined mode");
    let capabilities = mode.capabilities();
    debug!(?capabilities, "determined capabilities");
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

    debug!(?initialize_data, "finished initialization");

    conn.initialize_finish(id, initialize_data).unwrap();

    //mode.run(&conn, &pkg);

    let (trigger_tx, trigger_rx) = bounded(8);
    let (report_tx, report_rx) = bounded(8);

    let ingest_handle = std::thread::spawn(move || workers::ingest(conn.receiver, trigger_tx));
    let process_handle = std::thread::spawn(move || workers::process(pkg, trigger_rx, report_tx));
    let report_handle = std::thread::spawn(move || workers::report(report_rx, conn.sender));

    threads.join().unwrap();

    ingest_handle.join().unwrap();
    process_handle.join().unwrap();
    report_handle.join().unwrap();
}
