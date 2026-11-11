//! Folding for the code editor, engine-free: which lines a set of closed folds hides, and the map
//! between a line of the text and the row it is drawn on.
//!
//! A **fold range** is `start..end` over lines: the header is `start`, which stays drawn, and lines
//! `start + 1..end` are what a closed fold hides. The host says where folds can be (an indented
//! block, a braced one); the player closes and opens them. A host that wants a block's closing
//! line to stay drawn gives `end` as that line, and one that wants it folded away gives the line
//! after it.
//!
//! [`FoldMap`] is rebuilt when a fold opens or closes and answers four questions at the speed of a
//! short list: is a line hidden, which row is it on, which line is a row, and which closed fold
//! hides a line.

use std::ops::Range;

/// The closed folds of a text, as the lines they hide.
///
/// ```
/// use bevy_gooey::FoldMap;
/// // Lines 1 to 3 are folded into line 0, and line 6 into line 5.
/// let map = FoldMap::new([0..4, 5..7]);
/// assert!(map.is_hidden(2) && !map.is_hidden(4));
/// // Line 4 is the next row after the header, line 7 the next after the second fold.
/// assert_eq!(map.row_of(4), 1);
/// assert_eq!(map.line_at(2), 5);
/// assert_eq!(map.rows(10), 6);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FoldMap {
    /// The hidden lines, as sorted, disjoint, half-open ranges.
    hidden: Vec<Range<usize>>,
    /// The headers, parallel to the ranges they hide, for [`FoldMap::header_of`].
    headers: Vec<usize>,
}

impl FoldMap {
    /// The map of these closed folds. Folds inside another closed fold, and overlapping ones, are
    /// merged into the larger; a fold that hides nothing (`end <= start + 1`) is no fold.
    pub fn new(closed: impl IntoIterator<Item = Range<usize>>) -> Self {
        let mut folds: Vec<(usize, Range<usize>)> = closed
            .into_iter()
            .filter(|fold| fold.end > fold.start + 1)
            .map(|fold| (fold.start, fold.start + 1..fold.end))
            .collect();
        folds.sort_by_key(|(_, hidden)| hidden.start);
        let mut map = Self::default();
        for (header, hidden) in folds {
            match map.hidden.last_mut() {
                Some(last) if hidden.start <= last.end => {
                    last.end = last.end.max(hidden.end);
                }
                _ => {
                    map.hidden.push(hidden);
                    map.headers.push(header);
                }
            }
        }
        map
    }

    /// Whether no fold is closed.
    pub fn is_empty(&self) -> bool {
        self.hidden.is_empty()
    }

    /// The hidden ranges, ascending.
    pub fn hidden(&self) -> &[Range<usize>] {
        &self.hidden
    }

    /// Whether a closed fold hides `line`.
    pub fn is_hidden(&self, line: usize) -> bool {
        self.header_of(line).is_some()
    }

    /// The header of the closed fold that hides `line`, `None` for a line that shows.
    pub fn header_of(&self, line: usize) -> Option<usize> {
        let at = self.hidden.partition_point(|range| range.end <= line);
        self.hidden
            .get(at)
            .filter(|range| range.contains(&line))
            .map(|_| self.headers[at])
    }

    /// How many lines are hidden before `line`.
    fn hidden_before(&self, line: usize) -> usize {
        self.hidden
            .iter()
            .take_while(|range| range.start < line)
            .map(|range| range.end.min(line) - range.start)
            .sum()
    }

    /// The row a line is drawn on: its place among the lines that show. A hidden line is on the
    /// row of the header that hides it.
    pub fn row_of(&self, line: usize) -> usize {
        let line = self.header_of(line).unwrap_or(line);
        line - self.hidden_before(line)
    }

    /// The line drawn on `row`.
    pub fn line_at(&self, row: usize) -> usize {
        let mut line = row;
        for range in &self.hidden {
            if line >= range.start {
                line += range.end - range.start;
            } else {
                break;
            }
        }
        line
    }

    /// How many rows `lines` lines are drawn on.
    pub fn rows(&self, lines: usize) -> usize {
        lines - self.hidden_before(lines)
    }

    /// How many lines the folds hide in all.
    pub fn hidden_lines(&self) -> usize {
        self.hidden.iter().map(|range| range.end - range.start).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_and_lines_agree_round_a_fold() {
        let map = FoldMap::new([3..8, 10..12]);
        // Lines 4..8 and 11 are hidden.
        for line in 0..20 {
            let row = map.row_of(line);
            if !map.is_hidden(line) {
                assert_eq!(map.line_at(row), line, "line {line} on row {row}");
            }
        }
        assert_eq!(map.rows(20), 20 - 4 - 1);
        assert_eq!(map.row_of(8), 4);
        assert_eq!(map.line_at(4), 8);
        assert_eq!(map.header_of(5), Some(3));
        assert_eq!(map.header_of(11), Some(10));
        assert_eq!(map.header_of(8), None);
        // A hidden line is on its header's row.
        assert_eq!(map.row_of(6), map.row_of(3));
    }

    #[test]
    fn nested_and_overlapping_folds_merge_and_empty_folds_are_none() {
        let map = FoldMap::new([0..10, 2..5, 8..14, 20..21, 30..30]);
        assert_eq!(map.hidden(), &[1..14]);
        assert_eq!(map.header_of(13), Some(0));
        assert!(FoldMap::new([4..5]).is_empty());
        assert_eq!(FoldMap::default().line_at(7), 7);
    }
}
