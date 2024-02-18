#[derive(Debug, PartialEq)]
pub struct LineSlice {
    pub start: usize,
    pub begin: usize,
    pub end: usize,
}

impl LineSlice {
    pub fn build(slice: &[u8]) -> Vec<LineSlice> {
        let mut start = 0;
        let mut begin = 0;
        let mut pre = true;
        let mut end;

        let mut lines = Vec::new();

        for i in 0..slice.len() {
            if slice[i] == b'\n' {
                if pre {
                    begin = i;
                }

                end = i;
                pre = true;

                let slice = LineSlice { start, begin, end };
                lines.push(slice);

                start = i + 1;
                begin = i + 1;

                continue;
            }

            if pre && !matches!(slice[i], b' ' | b'\t' | b'\r') {
                pre = false;
                begin = i;
            }
        }

        lines
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_slice() {
        const CONTENT: &str = r#"
fn main() {
    println!("Hello World!");
}
"#;

        assert_eq!(
            LineSlice::build(CONTENT.as_bytes()),
            vec![
                LineSlice {
                    start: 0,
                    begin: 0,
                    end: 0
                },
                LineSlice {
                    start: 1,
                    begin: 1,
                    end: 12
                },
                LineSlice {
                    start: 13,
                    begin: 17,
                    end: 42
                },
                LineSlice {
                    start: 43,
                    begin: 43,
                    end: 44
                },
            ]
        );
    }
}
