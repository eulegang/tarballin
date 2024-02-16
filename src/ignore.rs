use std::{io::BufRead, path::Path};

#[derive(Default, PartialEq, Eq, Debug)]
pub struct Ignore {
    patterns: Vec<Pattern>,
}

#[derive(PartialEq, Eq, Debug)]
enum Pattern {
    Plain(String),
}

impl Ignore {
    pub fn matches(&self, path: &Path) -> bool {
        for pat in &self.patterns {
            if pat.matches(path) {
                return true;
            }
        }
        false
    }

    fn parse(content: &[u8]) -> std::io::Result<Self> {
        let mut patterns = Vec::new();
        for line in content.lines() {
            let line = line?;

            let content = if let Some((pre, _)) = line.split_once('#') {
                pre
            } else {
                &line
            };

            if content.trim().is_empty() {
                continue;
            }

            patterns.push(Pattern::Plain(content.trim().to_string()));
        }

        Ok(Self { patterns })
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read(path)?;
        Self::parse(&content)
    }

    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

// yes I know I'm the worst
impl std::ops::AddAssign for Ignore {
    fn add_assign(&mut self, rhs: Self) {
        self.patterns.extend(rhs.patterns)
    }
}

impl Pattern {
    pub fn matches(&self, path: &Path) -> bool {
        match self {
            Pattern::Plain(pat) => path.to_str() == Some(pat.as_str()),
        }
    }
}

#[test]
fn test_parse() {
    const CONTENT: &[u8] = b"/src/main.rs";

    let ignore = Ignore::parse(CONTENT).unwrap();

    assert_eq!(
        ignore,
        Ignore {
            patterns: vec![Pattern::Plain("/src/main.rs".to_string())]
        }
    );
}
