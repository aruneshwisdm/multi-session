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
