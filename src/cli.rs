use std::str::FromStr;

#[derive(clap::Parser)]
pub struct Args {
    /// how to connect to an editor
    #[clap(short, long)]
    pub connect: Conn,
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
    assert_eq!(Conn::from_str("-").unwrap(), Conn::Stdio);
}
