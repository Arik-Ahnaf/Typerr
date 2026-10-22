//! Pure monospaced layout. All lengths are measured logical pixels supplied by UI.
use std::ops::Range;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CaretTarget {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct TextLayout {
    pub lines: Vec<Range<usize>>,
    pub advance: f32,
    pub line_height: f32,
    length: usize,
}

impl TextLayout {
    /// Reserves one cell for a trailing separator and space for the thin caret.
    pub fn new(target: &[char], width: f32, advance: f32, line_height: f32) -> Self {
        assert!(advance.is_finite() && advance > 0.0);
        assert!(line_height.is_finite() && line_height > 0.0);
        let cells = ((width.max(0.0) - 2.0).max(0.0) / advance).floor() as usize;
        let content_cells = cells.saturating_sub(1).max(1);
        let mut lines = Vec::new();
        let mut start = 0;
        while start < target.len() {
            let limit = (start + content_cells).min(target.len());
            let end = if limit == target.len() {
                limit
            } else if target[limit] == ' ' {
                limit + 1
            } else if let Some(space) = target[start..limit].iter().rposition(|c| *c == ' ') {
                start + space + 1
            } else {
                limit
            };
            lines.push(start..end);
            start = end;
        }
        if lines.is_empty() {
            lines.push(0..0);
        }
        Self {
            lines,
            advance,
            line_height,
            length: target.len(),
        }
    }

    pub fn active_line(&self, position: usize) -> usize {
        let anchor = position.min(self.length.saturating_sub(1));
        self.lines
            .iter()
            .position(|line| line.contains(&anchor))
            .unwrap_or(0)
    }

    pub fn caret(&self, position: usize) -> CaretTarget {
        let line = self.active_line(position);
        CaretTarget {
            x: position
                .min(self.length)
                .saturating_sub(self.lines[line].start) as f32
                * self.advance,
            y: line as f32 * self.line_height,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Viewport {
    pub top_line: usize,
    pub rows: usize,
}

impl Viewport {
    pub fn resize(&mut self, layout: &TextLayout, height: f32, position: usize) {
        self.rows = (height.max(0.0) / layout.line_height).floor() as usize;
        self.follow(layout, position);
    }

    pub fn follow(&mut self, layout: &TextLayout, position: usize) {
        self.top_line = layout.active_line(position);
    }

    pub fn scroll(&mut self, lines: isize, total_lines: usize) {
        self.top_line = self
            .top_line
            .saturating_add_signed(lines)
            .min(total_lines.saturating_sub(1));
    }

    pub fn visible_lines(&self, total_lines: usize) -> Range<usize> {
        self.top_line..(self.top_line + self.rows).min(total_lines)
    }

    /// Center the visible block as a whole; lines retain a shared left edge.
    pub fn content_origin(&self, layout: &TextLayout, width: f32, height: f32) -> CaretTarget {
        let visible = self.visible_lines(layout.lines.len());
        let columns = visible
            .clone()
            .map(|i| layout.lines[i].len())
            .max()
            .unwrap_or(0);
        let content_width = columns as f32 * layout.advance + 2.0;
        let content_height = visible.len() as f32 * layout.line_height;
        CaretTarget {
            x: ((width - content_width) / 2.0).max(0.0),
            y: ((height - content_height) / 2.0).max(0.0),
        }
    }

    pub fn opacity(&self, line: usize, active: usize, total: usize) -> f32 {
        let bottom = self.top_line + self.rows;
        if line != active && line + 1 == bottom && bottom < total {
            0.5
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn layout(text: &str, cells: usize) -> TextLayout {
        TextLayout::new(
            &text.chars().collect::<Vec<_>>(),
            cells as f32 * 10.0 + 2.0,
            10.0,
            25.0,
        )
    }

    #[test]
    fn visible_block_stays_centered_after_scroll_and_resize() {
        let l = layout("one two three", 8);
        let mut v = Viewport::default();
        for (width, height) in [(100.0, 100.0), (300.0, 200.0)] {
            v.resize(&l, height, 0);
            let origin = v.content_origin(&l, width, height);
            assert_eq!(origin.x * 2.0 + 82.0, width);
            assert_eq!(origin.y * 2.0 + 50.0, height);
            v.follow(&l, 13);
            let origin = v.content_origin(&l, width, height);
            assert_eq!(origin.x * 2.0 + 52.0, width);
            assert_eq!(origin.y * 2.0 + 25.0, height);
        }
    }

    #[test]
    fn wraps_words_and_keeps_separator_on_previous_line() {
        let l = layout("one two three", 8);
        assert_eq!(l.lines, vec![0..8, 8..13]);
        assert_eq!(l.active_line(7), 0);
        assert_eq!(l.caret(7), CaretTarget { x: 70.0, y: 0.0 });
        assert_eq!(l.active_line(8), 1);
        assert_eq!(l.caret(8), CaretTarget { x: 0.0, y: 25.0 });
        assert_eq!(l.caret(13), CaretTarget { x: 50.0, y: 25.0 });
    }

    #[test]
    fn long_words_and_tiny_width_never_lose_positions() {
        for cells in [0, 1, 3, 9] {
            let l = layout("abcdefghij kl", cells);
            assert_eq!(
                l.lines.iter().flat_map(Clone::clone).collect::<Vec<_>>(),
                (0..13).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn resize_reflows_same_logical_position_and_final_has_no_sentinel() {
        let narrow = layout("one two three", 5);
        let wide = layout("one two three", 30);
        assert_eq!(narrow.active_line(8), 2);
        assert_eq!(wide.active_line(8), 0);
        assert_eq!(wide.caret(13).x, 130.0);
        assert_eq!(wide.lines.len(), 1);
        assert_eq!(layout("", 5).caret(0), CaretTarget::default());
    }

    #[test]
    fn viewport_tracks_active_line_backspace_and_resize() {
        let l = layout("one two six ten end", 5);
        let mut v = Viewport::default();
        v.resize(&l, 74.0, 0);
        assert_eq!(v.rows, 2);
        v.follow(&l, 4);
        assert_eq!(v.top_line, 1);
        v.follow(&l, 3);
        assert_eq!(v.top_line, 0);
        v.scroll(3, l.lines.len());
        assert_eq!(v.top_line, 3);
        v.follow(&l, 4);
        assert_eq!(v.top_line, 1);
        v.scroll(-99, l.lines.len());
        assert_eq!(v.top_line, 0);
        v.resize(&l, 100.0, 8);
        assert_eq!(v.top_line, 2);
        v.follow(&l, 19);
        assert_eq!(v.top_line, l.lines.len() - 1);
    }

    #[test]
    fn only_overflow_preview_fades_and_active_line_never_fades() {
        let mut v = Viewport {
            top_line: 0,
            rows: 2,
        };
        assert_eq!(v.opacity(0, 0, 4), 1.0);
        assert_eq!(v.opacity(1, 0, 4), 0.5);
        assert_eq!(v.opacity(1, 1, 4), 1.0);
        v.top_line = 1;
        assert_eq!(v.opacity(1, 1, 4), 1.0);
        assert_eq!(v.opacity(2, 1, 4), 0.5);
        v.top_line = 2;
        assert_eq!(v.opacity(3, 2, 4), 1.0);
        assert_eq!(v.visible_lines(3), 2..3);
    }
}
