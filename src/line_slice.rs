#[derive(Debug)]
pub struct LineSlice {
    pub start: usize,
    pub begin: usize,
    pub end: usize,
}

impl LineSlice {
    pub fn build(slice: &[u8]) -> Vec<LineSlice> {
        let mut start = 0;
        let mut begin = 0;
        let mut end = 0;
        let mut pre = true;

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
                end = i + 1;

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
