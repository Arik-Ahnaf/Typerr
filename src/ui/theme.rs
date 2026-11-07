use iced::{Color, Font};

pub const FONT: Font = Font::with_name("Google Sans Code");
pub const ITALIC_FONT: Font = Font {
    style: iced::font::Style::Italic,
    ..FONT
};
pub const MEDIUM_FONT: Font = Font {
    weight: iced::font::Weight::Medium,
    ..FONT
};
pub const SEMIBOLD_FONT: Font = Font {
    weight: iced::font::Weight::Semibold,
    ..FONT
};
pub const LIGHT_FONT: Font = Font {
    weight: iced::font::Weight::Light,
    ..FONT
};
pub const USER_FONT: Font = Font {
    weight: iced::font::Weight::Semibold,
    style: iced::font::Style::Italic,
    ..FONT
};
pub const SAVE_FONT: Font = Font {
    weight: iced::font::Weight::Bold,
    ..FONT
};
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

pub fn typing_height(line_height: f32) -> f32 {
    // Round upward so fractional font sizes never lose a row to rounding.
    (4.0 * line_height + 2.0 * VERTICAL_INSET).ceil()
}

pub fn minimum_window_height(line_height: f32) -> f32 {
    (typing_height(line_height) + 2.0 * VERTICAL_CLEARANCE).max(400.0)
}

// Cargo validates the JSON and compiles its role mappings into native colors.
#[derive(Debug)]
pub struct Palette {
    pub name: &'static str,
    pub background: Color,
    pub control_background: Color,
    pub correct: Color,
    pub pending: Color,
    pub incorrect: Color,
    pub cyan: Color,
    pub blue: Color,
    pub orange: Color,
}
mod dracula {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/dracula.rs"));
}
mod gruvbox {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/gruvbox.rs"));
}
mod vscode {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/vscode.rs"));
}
static ACTIVE: std::sync::OnceLock<crate::settings::Theme> = std::sync::OnceLock::new();

/// Called once before creating the UI; saving preferences requires a restart.
pub fn initialize(theme: crate::settings::Theme) {
    ACTIVE
        .set(theme)
        .expect("theme initialized only at startup");
}
pub fn palette() -> &'static Palette {
    palette_for(ACTIVE.get().copied().unwrap_or_default())
}
fn palette_for(theme: crate::settings::Theme) -> &'static Palette {
    match theme {
        crate::settings::Theme::Dracula => &dracula::PALETTE,
        crate::settings::Theme::Gruvbox => &gruvbox::PALETTE,
        crate::settings::Theme::VSCode => &vscode::PALETTE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use typerr::layout::{TextLayout, Viewport};

    #[test]
    fn compiled_palettes_preserve_json_role_mappings() {
        for (choice, source) in [
            (
                crate::settings::Theme::Dracula,
                include_str!("../../themes/dracula.json"),
            ),
            (
                crate::settings::Theme::Gruvbox,
                include_str!("../../themes/gruvbox.json"),
            ),
            (
                crate::settings::Theme::VSCode,
                include_str!("../../themes/vscode.json"),
            ),
        ] {
            let json: serde_json::Value = serde_json::from_str(source).unwrap();
            let palette = palette_for(choice);
            assert_eq!(palette.name, choice.name());
            for (role, color) in [
                ("BACKGROUND", palette.background),
                ("CONTROL_BACKGROUND", palette.control_background),
                ("CORRECT", palette.correct),
                ("PENDING", palette.pending),
                ("INCORRECT", palette.incorrect),
                ("CYAN", palette.cyan),
                ("BLUE", palette.blue),
                ("ORANGE", palette.orange),
            ] {
                let key = json["roles"][role].as_str().unwrap();
                let hex = json["colors"][key]
                    .as_str()
                    .or_else(|| json["accents"][key].as_str())
                    .unwrap();
                let rgb = u32::from_str_radix(&hex[1..], 16).unwrap();
                assert_eq!(
                    color,
                    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
                );
            }
        }
    }

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
