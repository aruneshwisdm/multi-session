use alacritty_terminal::vte::ansi::{Color, NamedColor};
use jc_core::theme::{Appearance, PaletteColors, ThemeConfig};

/// Simple RGBA color representation (0.0-1.0 per channel).
/// Framework-agnostic — converted to iced::Color or other types at the GUI layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }
}

/// Parse a hex color string (`#RRGGBB` or `#RRGGBBAA`) to Rgba.
pub fn hex_to_rgba(hex: &str) -> Rgba {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    let a = if hex.len() >= 8 {
        u8::from_str_radix(&hex[6..8], 16).unwrap_or(0xFF)
    } else {
        0xFF
    };
    Rgba::from_rgba8(r, g, b, a)
}

/// Standard terminal palette with 256 ANSI colors plus foreground/background/cursor.
#[derive(Clone)]
pub struct Palette {
    pub foreground: Rgba,
    pub background: Rgba,
    pub cursor: Rgba,
    ansi: [Rgba; 256],
}

impl Default for Palette {
    fn default() -> Self {
        Palette::from(&PaletteColors::default())
    }
}

impl Palette {
    /// Build a palette for the given appearance.
    pub fn for_appearance(appearance: Appearance) -> Self {
        Palette::from(&ThemeConfig::for_appearance(appearance).palette)
    }
}

impl From<&PaletteColors> for Palette {
    fn from(palette: &PaletteColors) -> Self {
        let mut ansi = [Rgba::from_rgb8(0, 0, 0); 256];

        // Standard 16 ANSI colors from theme
        ansi[NamedColor::Black as usize] = hex_to_rgba(&palette.black);
        ansi[NamedColor::Red as usize] = hex_to_rgba(&palette.red);
        ansi[NamedColor::Green as usize] = hex_to_rgba(&palette.green);
        ansi[NamedColor::Yellow as usize] = hex_to_rgba(&palette.yellow);
        ansi[NamedColor::Blue as usize] = hex_to_rgba(&palette.blue);
        ansi[NamedColor::Magenta as usize] = hex_to_rgba(&palette.magenta);
        ansi[NamedColor::Cyan as usize] = hex_to_rgba(&palette.cyan);
        ansi[NamedColor::White as usize] = hex_to_rgba(&palette.white);
        // Bright
        ansi[NamedColor::BrightBlack as usize] = hex_to_rgba(&palette.bright_black);
        ansi[NamedColor::BrightRed as usize] = hex_to_rgba(&palette.bright_red);
        ansi[NamedColor::BrightGreen as usize] = hex_to_rgba(&palette.bright_green);
        ansi[NamedColor::BrightYellow as usize] = hex_to_rgba(&palette.bright_yellow);
        ansi[NamedColor::BrightBlue as usize] = hex_to_rgba(&palette.bright_blue);
        ansi[NamedColor::BrightMagenta as usize] = hex_to_rgba(&palette.bright_magenta);
        ansi[NamedColor::BrightCyan as usize] = hex_to_rgba(&palette.bright_cyan);
        ansi[NamedColor::BrightWhite as usize] = hex_to_rgba(&palette.bright_white);

        // 216-color cube (indices 16..232)
        for i in 0..216u8 {
            let r = if i / 36 > 0 { (i / 36) * 40 + 55 } else { 0 };
            let g = if (i / 6) % 6 > 0 { ((i / 6) % 6) * 40 + 55 } else { 0 };
            let b = if i % 6 > 0 { (i % 6) * 40 + 55 } else { 0 };
            ansi[16 + i as usize] = Rgba::from_rgb8(r, g, b);
        }

        // 24-step grayscale (indices 232..256)
        for i in 0..24u8 {
            let v = i * 10 + 8;
            ansi[232 + i as usize] = Rgba::from_rgb8(v, v, v);
        }

        Self {
            foreground: hex_to_rgba(&palette.foreground),
            background: hex_to_rgba(&palette.background),
            cursor: hex_to_rgba(&palette.cursor),
            ansi,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_from_rgb8() {
        let c = Rgba::from_rgb8(255, 0, 128);
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert!((c.b - 128.0 / 255.0).abs() < 1e-6);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn rgba_from_rgba8_with_alpha() {
        let c = Rgba::from_rgba8(0, 0, 0, 127);
        assert_eq!(c.r, 0.0);
        assert!((c.a - 127.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn hex_to_rgba_6_digit() {
        let c = hex_to_rgba("#FF8000");
        assert_eq!(c.r, 1.0);
        assert!((c.g - 128.0 / 255.0).abs() < 1e-6);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn hex_to_rgba_8_digit() {
        let c = hex_to_rgba("#FF800080");
        assert_eq!(c.r, 1.0);
        assert!((c.a - 128.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn hex_to_rgba_without_hash() {
        let c = hex_to_rgba("00FF00");
        assert_eq!(c.r, 0.0);
        assert_eq!(c.g, 1.0);
        assert_eq!(c.b, 0.0);
    }

    #[test]
    fn default_palette_has_nonzero_foreground() {
        let p = Palette::default();
        assert!(p.foreground.r > 0.0 || p.foreground.g > 0.0 || p.foreground.b > 0.0);
    }

    #[test]
    fn palette_resolve_spec_color() {
        let p = Palette::default();
        let rgb = alacritty_terminal::vte::ansi::Rgb { r: 100, g: 200, b: 50 };
        let c = p.resolve(&Color::Spec(rgb));
        assert!((c.r - 100.0 / 255.0).abs() < 1e-6);
        assert!((c.g - 200.0 / 255.0).abs() < 1e-6);
        assert!((c.b - 50.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn palette_resolve_indexed_color() {
        let p = Palette::default();
        let c = p.resolve(&Color::Indexed(232));
        assert!(c.r == c.g && c.g == c.b, "grayscale index 232 should be gray");
    }

    #[test]
    fn palette_resolve_fg_default() {
        let p = Palette::default();
        let fg = p.resolve_fg(&Color::Named(NamedColor::Foreground));
        assert_eq!(fg, p.foreground);
    }

    #[test]
    fn palette_resolve_bg_default() {
        let p = Palette::default();
        let bg = p.resolve_bg(&Color::Named(NamedColor::Background));
        assert_eq!(bg, p.background);
    }

    #[test]
    fn palette_resolve_fg_named_not_foreground() {
        let p = Palette::default();
        let red = p.resolve_fg(&Color::Named(NamedColor::Red));
        assert!(red.r > red.g, "red should have more red than green");
    }

    #[test]
    fn palette_for_dark_appearance() {
        let p = Palette::for_appearance(Appearance::Dark);
        assert!(p.background.r < 0.3, "dark theme background should be dark");
    }

    #[test]
    fn palette_216_color_cube_boundaries() {
        let p = Palette::default();
        let black_cube = p.resolve(&Color::Indexed(16));
        assert_eq!(black_cube.r, 0.0);
        assert_eq!(black_cube.g, 0.0);
        assert_eq!(black_cube.b, 0.0);

        let white_cube = p.resolve(&Color::Indexed(231));
        assert!(white_cube.r > 0.9);
        assert!(white_cube.g > 0.9);
        assert!(white_cube.b > 0.9);
    }

    #[test]
    fn palette_grayscale_ramp_ascending() {
        let p = Palette::default();
        let dark = p.resolve(&Color::Indexed(232));
        let light = p.resolve(&Color::Indexed(255));
        assert!(light.r > dark.r);
    }
}

impl Palette {
    /// Resolve an alacritty Color to Rgba.
    pub fn resolve(&self, color: &Color) -> Rgba {
        match color {
            Color::Named(name) => self.ansi[*name as usize],
            Color::Spec(rgb) => Rgba::from_rgb8(rgb.r, rgb.g, rgb.b),
            Color::Indexed(idx) => self.ansi[*idx as usize],
        }
    }

    /// Resolve foreground color, applying default.
    pub fn resolve_fg(&self, color: &Color) -> Rgba {
        match color {
            Color::Named(NamedColor::Foreground) => self.foreground,
            other => self.resolve(other),
        }
    }

    /// Resolve background color, applying default.
    pub fn resolve_bg(&self, color: &Color) -> Rgba {
        match color {
            Color::Named(NamedColor::Background) => self.background,
            other => self.resolve(other),
        }
    }
}
