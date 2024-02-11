use std::{fs::File, path::PathBuf, str::FromStr};

use tracing::Level;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(clap::Parser)]
pub struct Args {
    /// how to connect to an editor
    #[clap(short, long)]
    pub connect: Conn,

    /// override the log location
    #[clap(short, long)]
    pub log: Option<PathBuf>,

    /// override the log level
    #[clap(short, long)]
    pub level: Option<Level>,
}

impl Args {
    pub fn setup_subscriber(&self) {
        match (self.log.as_ref(), self.level.as_ref()) {
            (None, None) => {
                tracing_subscriber::fmt()
                    .with_writer(std::io::stderr)
                    .pretty()
                    .with_ansi(false)
                    .finish()
                    .init();
            }
            (None, Some(_)) => {
                tracing_subscriber::fmt()
                    .with_writer(std::io::stderr)
                    .pretty()
                    .with_ansi(false)
                    .finish()
                    .init();
            }
            (Some(path), None) => {
                let file = File::options().write(true).create(true).open(path).unwrap();

                tracing_subscriber::fmt()
                    .with_writer(file)
                    .pretty()
                    .with_ansi(true)
                    .finish()
                    .init();
            }
            (Some(path), Some(level)) => {
                let file = File::options().write(true).create(true).open(path).unwrap();

                tracing_subscriber::fmt()
                    .with_writer(file)
                    .pretty()
                    .with_ansi(true)
                    .with_max_level(*level)
                    .finish()
                    .init();
            }
        }
    }
}

#[derive(Clone, Default, PartialEq, Debug)]
pub enum Conn {
    #[default]
    Stdio,
}

impl FromStr for Conn {
    type Err = InvalidConnection;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "-" {
            return Ok(Self::Stdio);
        }

        Err(InvalidConnection(s.to_string()))
    }
}

#[derive(Debug)]
pub struct InvalidConnection(String);

impl std::error::Error for InvalidConnection {}

impl std::fmt::Display for InvalidConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid connection specified \"{}\"", self.0)
    }
}

#[test]
fn test_str() {
    assert_eq!(Conn::from_str("-").unwrap(), Conn::Stdio);
}
