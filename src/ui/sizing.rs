//! Discrete native layout presets. Window resizing changes placement within a
//! range; dimensions and typography change only when both breakpoints are met.
use iced::Size;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    #[default]
    Design,
    Medium,
    Large,
    ExtraLarge,
}

impl Preset {
    pub fn for_window(size: Size) -> Self {
        if size.width >= 1800. && size.height >= 1000. {
            Self::ExtraLarge
        } else if size.width >= 1400. && size.height >= 800. {
            Self::Large
        } else if size.width > 1000. && size.height > 500. {
            Self::Medium
        } else {
            Self::Design
        }
    }

    pub fn scale(self) -> f32 {
        match self {
            Self::Design => 1.,
            Self::Medium => 1.25,
            Self::Large => 1.5,
            Self::ExtraLarge => 2.,
        }
    }

    pub fn size(self) -> Size {
        match self {
            Self::Design => Size::new(700., 400.),
            Self::Medium => Size::new(875., 500.),
            Self::Large => Size::new(1050., 600.),
            Self::ExtraLarge => Size::new(1400., 800.),
        }
    }

    /// Large configured typing fonts still need four complete rows and the
    /// normal exterior clearance. Select a smaller preset if necessary.
    pub fn for_typing(size: Size, line_height: f32) -> Self {
        let requested = Self::for_window(size);
        [Self::ExtraLarge, Self::Large, Self::Medium, Self::Design]
            .into_iter()
            .find(|preset| {
                preset.scale() <= requested.scale()
                    && super::theme::minimum_window_height(line_height) * preset.scale()
                        <= size.height
            })
            .unwrap_or(Self::Design)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakpoints_require_both_axes_and_sizes_stay_constant_between_them() {
        for (width, height, expected) in [
            (700., 400., Preset::Design),
            (1000., 500., Preset::Design),
            (1000., 900., Preset::Design),
            (2200., 500., Preset::Design),
            (1001., 501., Preset::Medium),
            (1399., 799., Preset::Medium),
            (1400., 799., Preset::Medium),
            (1399., 800., Preset::Medium),
            (1400., 800., Preset::Large),
            (1799., 999., Preset::Large),
            (1800., 999., Preset::Large),
            (1799., 1000., Preset::Large),
            (1800., 1000., Preset::ExtraLarge),
            (3840., 2160., Preset::ExtraLarge),
        ] {
            let preset = Preset::for_window(Size::new(width, height));
            assert_eq!(preset, expected);
            assert!(preset.size().width <= width && preset.size().height <= height);
        }
    }

    #[test]
    fn large_typing_fonts_keep_four_rows_and_clearance() {
        for font in [8., 16., 24., 48.] {
            for size in [
                Size::<f32>::new(1001., 501.),
                Size::new(1400., 800.),
                Size::new(1920., 1080.),
            ] {
                // 48px needs a 520px minimum window even at the design preset.
                let size = Size::new(
                    size.width,
                    size.height
                        .max(super::super::theme::minimum_window_height(font * 1.25)),
                );
                let preset = Preset::for_typing(size, font * 1.25);
                assert!(
                    super::super::theme::minimum_window_height(font * 1.25) * preset.scale()
                        <= size.height
                );
            }
        }
    }
}
