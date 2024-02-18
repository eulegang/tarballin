use std::{io::BufRead, path::Path};

use eyre::ContextCompat;
use glob::{MatchOptions, Pattern};
use tracing::{debug, instrument};
use tree_sitter::{Parser, Query, QueryCursor};
use tree_sitter_rust::language;

use crate::coverage::Trace;

#[derive(Default, PartialEq, Debug)]
pub struct Ignore {
    rules: Vec<Rule>,
}

#[derive(Debug)]
pub enum IgnoreResult<'a> {
    Ignore,
    Apply,
    Partial(&'a [Query]),
}

impl<'a> IgnoreResult<'a> {
    pub fn filter(&self, content: &[u8], traces: &[Trace]) -> eyre::Result<Vec<Trace>> {
        match self {
            IgnoreResult::Ignore => Ok(vec![]),
            IgnoreResult::Apply => Ok(traces.to_vec()),
            IgnoreResult::Partial(queries) => {
                let mut parser = Parser::new();
                parser.set_language(language())?;
                let tree = parser
                    .parse(content, None)
                    .with_context(|| "failed to parse tree")?;

                let node = tree.root_node();

                let mut cur = QueryCursor::new();

                let mut traces = traces.to_vec();
                let mut rm_mark = vec![false; traces.len()];

                for query in queries.iter() {
                    let captures = cur.captures(query, node, content);

                    for (capt, _) in captures {
                        for sub in capt.captures {
                            for (i, trace) in traces.iter().enumerate() {
                                if !rm_mark[i] {
                                    let line = trace.line as usize;
                                    if sub.node.start_position().row <= line
                                        && line <= sub.node.end_position().row
                                    {
                                        rm_mark[i] = true;
                                    }
                                }
                            }
                        }
                    }
                }

                for (i, mark) in rm_mark.iter().enumerate().rev() {
                    if *mark {
                        traces.remove(i);
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
    #[instrument]
    pub fn matches(&self, path: &Path) -> IgnoreResult {
        debug!(path = %path.display(), "checking ignore");
        for pat in &self.rules {
            let result = pat.matches(path);

            if !matches!(result, IgnoreResult::Apply) {
                return result;
            }
        }

        IgnoreResult::Apply
    }

    fn parse(content: &[u8]) -> eyre::Result<Self> {
        let mut rules = Vec::<Rule>::new();
        for line in content.lines() {
            let line = line?;

            if line.starts_with('#') {
                continue;
            }

            let content = &line;

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

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_basic_parse() {
        const CONTENT: &[u8] = b"src/main.rs\n\n";
        Ignore::parse(CONTENT).unwrap();
    }

    #[test]
    fn test_query_parse() {
        const CONTENT: &[u8] =
            b"src/main.rs\n\t((function_item (identifier) @id) (#eq? @id \"main\"))";
        Ignore::parse(CONTENT).unwrap();
    }

    #[test]
    fn test_odds_parse() {
        const CONTENT: &[u8] = b"\t\nsrc/main.rs\n";
        Ignore::parse(CONTENT).unwrap();
    }

    #[test]
    fn test_match_ignore() {
        const CONTENT: &[u8] = b"src/main.rs";

        let ignore = Ignore::parse(CONTENT).unwrap();

        let res = ignore.matches(&PathBuf::from("src/main.rs"));
        assert!(
            matches!(res, IgnoreResult::Ignore),
            "expected {:?} found {:?}",
            IgnoreResult::Ignore,
            res
        );
    }

    #[test]
    fn test_match_apply() {
        const CONTENT: &[u8] = b"src/main.rs";

        let ignore = Ignore::parse(CONTENT).unwrap();

        let res = ignore.matches(&PathBuf::from("src/ignore.rs"));
        assert!(
            matches!(res, IgnoreResult::Apply),
            "expected {:?} found {:?}",
            IgnoreResult::Apply,
            res
        );
    }

    #[test]
    fn test_match_partial() {
        const CONTENT: &[u8] =
            b"src/main.rs\n\t((function_item (identifier) @id) (#eq? @id \"main\"))";

        let ignore = Ignore::parse(CONTENT).unwrap();

        let res = ignore.matches(&PathBuf::from("src/main.rs"));
        assert!(
            matches!(res, IgnoreResult::Partial(_)),
            "expected Partial found {:?}",
            res
        );
    }
}
