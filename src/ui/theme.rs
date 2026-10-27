use iced::{Color, Font};

pub const FONT: Font = Font::with_name("Google Sans Code");
pub const BRAND_FONT: Font = Font::with_name("Jomhuria");
pub const LINE_HEIGHT: f32 = 20.0;
pub const INSET: f32 = 20.0;
pub const VERTICAL_INSET: f32 = 15.0;
pub const TYPING_MAX_WIDTH: f32 = 700.0;
pub const VERTICAL_CLEARANCE: f32 = 125.0;

pub fn alpha(mut color: Color, opacity: f32) -> Color {
    color.a *= opacity.clamp(0.0, 1.0);
    color
}

pub const GRAPH_RAW: Color = PENDING;
pub const GRAPH_GRID: Color = CONTROL_BACKGROUND;
pub const GRAPH_WPM: Color = BLUE;
pub const GRAPH_ERROR: Color = INCORRECT;

pub fn typing_height(line_height: f32) -> f32 {
    // Round upward so fractional font sizes never lose a row to rounding.
    (4.0 * line_height + 2.0 * VERTICAL_INSET).ceil()
}

pub fn minimum_window_height(line_height: f32) -> f32 {
    (typing_height(line_height) + 2.0 * VERTICAL_CLEARANCE).max(400.0)
}

// Cargo validates the JSON and compiles its role mappings into native colors.
include!(concat!(env!("OUT_DIR"), "/dracula.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use typerr::layout::{TextLayout, Viewport};

    #[test]
    fn configured_sizes_fit_four_whole_rows_at_minimum_window_height() {
        let target: Vec<char> = "word ".repeat(100).chars().collect();
        for hundredths in 800..=4800 {
            let line_height = hundredths as f32 / 100.0 * 1.25;
            let height = typing_height(line_height);
            let available = minimum_window_height(line_height) - 2.0 * VERTICAL_CLEARANCE;
            assert!(height <= available);
            let layout = TextLayout::new(&target, 100.0, 10.0, line_height);
            let mut viewport = Viewport::default();
            viewport.resize(&layout, height - 2.0 * VERTICAL_INSET, 0);
            assert_eq!(viewport.visible_lines(layout.lines.len()).len(), 4);
        }
        assert_eq!(typing_height(24.0 * 1.25), 150.0);
        assert_eq!(minimum_window_height(24.0 * 1.25), 400.0);
        assert_eq!(minimum_window_height(48.0 * 1.25), 520.0);
    }
}
