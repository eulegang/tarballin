use std::{fmt::write, io::BufRead, path::Path};

pub struct Ignore {
    patterns: Vec<Pattern>,
}

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

    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read(path)?;
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

        Ok(Ignore { patterns })
    }

    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    pub fn len(&self) -> usize {
        self.patterns.len()
    }
}

impl Pattern {
    pub fn matches(&self, path: &Path) -> bool {
        match self {
            Pattern::Plain(pat) => path.to_str() == Some(pat.as_str()),
        }
    }
}
