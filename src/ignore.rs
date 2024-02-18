use std::{io::BufRead, path::Path};

use eyre::ContextCompat;
use glob::{MatchOptions, Pattern};
use tree_sitter::{Parser, Query, QueryCursor};
use tree_sitter_rust::language;

use crate::coverage::Trace;

#[derive(Default, PartialEq, Debug)]
pub struct Ignore {
    rules: Vec<Rule>,
}

pub enum IgnoreResult<'a> {
    Ignore,
    Apply,
    Partial(&'a [Query]),
}

impl<'a> IgnoreResult<'a> {
    pub fn filter(&self, content: &[u8], traces: &[Trace]) -> eyre::Result<Vec<Trace>> {
        match self {
            IgnoreResult::Ignore => Ok(traces.to_vec()),
            IgnoreResult::Apply => Ok(vec![]),
            IgnoreResult::Partial(queries) => {
                let mut parser = Parser::new();
                parser.set_language(language())?;
                let tree = parser
                    .parse(content, None)
                    .with_context(|| "failed to parse tree")?;

                let node = tree.root_node();

                let mut cur = QueryCursor::new();

                let traces = traces.to_vec();

                for query in queries.iter() {
                    let captures = cur.captures(query, node, content);

                    for (capt, _) in captures {
                        for sub in capt.captures {
                            sub.node.start_position();
                        }
                    }
                }

                Ok(traces)
            }
        }
    }
}

#[derive(PartialEq, Debug)]
struct Rule {
    pattern: Pattern,
    queries: Vec<Query>,
}

impl Ignore {
    pub fn matches(&self, path: &Path) -> IgnoreResult {
        for pat in &self.rules {
            let result = pat.matches(path);

            if !matches!(result, IgnoreResult::Ignore) {
                return result;
            }
        }

        IgnoreResult::Ignore
    }

    fn parse(content: &[u8]) -> eyre::Result<Self> {
        let mut rules = Vec::<Rule>::new();
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

            if content.starts_with(' ') || content.starts_with('\t') {
                let Some(rule) = rules.last_mut() else {
                    continue;
                };

                let content = content.trim();

                let query = Query::new(language(), content)?;

                rule.queries.push(query);
            } else {
                let content = content.trim();
                let pattern = Pattern::new(content)?;
                let queries = vec![];

                rules.push(Rule { pattern, queries });
            }
        }

        Ok(Self { rules })
    }

    pub fn load(path: &Path) -> eyre::Result<Self> {
        let content = std::fs::read(path)?;
        Self::parse(&content)
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

// yes I know I'm the worst
impl std::ops::AddAssign for Ignore {
    fn add_assign(&mut self, rhs: Self) {
        self.rules.extend(rhs.rules)
    }
}

impl Rule {
    pub fn matches(&self, path: &Path) -> IgnoreResult {
        let opts = MatchOptions {
            case_sensitive: true,
            require_literal_separator: true,
            require_literal_leading_dot: true,
        };

        if self.pattern.matches_path_with(path, opts) {
            if self.queries.is_empty() {
                IgnoreResult::Ignore
            } else {
                IgnoreResult::Partial(&self.queries)
            }
        } else {
            IgnoreResult::Apply
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
            rules: vec![Rule {
                pattern: Pattern::new("/src/main.rs").unwrap(),
                queries: vec![]
            }]
        }
    );
}
