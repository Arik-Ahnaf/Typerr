use iced::{Color, Font};

pub const FONT: Font = Font::with_name("Google Sans Code");
pub const BRAND_FONT: Font = Font::with_name("Jomhuria");
pub const FONT_SIZE: f32 = 16.0;
pub const LINE_HEIGHT: f32 = 20.0;
pub const INSET: f32 = 20.0;
pub const VERTICAL_INSET: f32 = 15.0;
// Cargo validates the JSON and compiles its role mappings into native colors.
include!(concat!(env!("OUT_DIR"), "/dracula.rs"));
