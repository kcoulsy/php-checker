use std::cmp::Ordering;

/// Represents a single in-file edit returned by a fixable rule.
#[derive(Clone, Debug)]
pub struct TextEdit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

impl TextEdit {
    pub fn new(start: usize, end: usize, replacement: impl Into<String>) -> Self {
        assert!(start <= end, "text edit start must not exceed end");
        Self {
            start,
            end,
            replacement: replacement.into(),
        }
    }
}

/// Applies a sequence of edits to `source` and returns the updated text.
pub fn apply_text_edits(source: &str, edits: &[TextEdit]) -> String {
    let mut sorted = edits.to_vec();
    sorted.sort_by(|a, b| match a.start.cmp(&b.start) {
        Ordering::Equal => a.end.cmp(&b.end),
        ordering => ordering,
    });

    let mut result = String::with_capacity(source.len());
    let mut cursor = 0;
    for edit in sorted {
        if cursor > edit.start {
            panic!("overlapping edits are not supported");
        }

        result.push_str(&source[cursor..edit.start]);
        result.push_str(&edit.replacement);
        cursor = edit.end;
    }

    result.push_str(&source[cursor..]);
    result
}

/// Expands the range defined by `start`/`end` to cover the entire line it sits on.
pub fn covering_line_range(source: &str, start: usize, end: usize) -> (usize, usize) {
    let start = line_start(source, start);
    let end = line_end(source, end);
    (start, end)
}

fn line_start(source: &str, idx: usize) -> usize {
    let idx = idx.min(source.len());
    match source[..idx].rfind('\n') {
        Some(pos) => pos + 1,
        None => 0,
    }
}

fn line_end(source: &str, idx: usize) -> usize {
    let mut pos = idx.min(source.len());
    while pos < source.len() {
        let ch = source.as_bytes()[pos];
        pos += 1;
        if ch == b'\n' {
            break;
        }
    }
    pos
}
