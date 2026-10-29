//! Shared utilities across the Sayanox Core rendering pipeline.

use std::fmt;

/// An 8-bit per channel sRGB color with alpha (`rgba`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Parse `#rgb`, `#rgba`, `#rrggbb`, or `#rrggbbaa` (leading `#` optional).
    pub fn from_hex(input: &str) -> Option<Self> {
        let hex = input.strip_prefix('#').unwrap_or(input);
        if !hex.is_ascii() {
            return None;
        }
        match hex.len() {
            3 => {
                let r = parse_nibble(hex.as_bytes()[0])?;
                let g = parse_nibble(hex.as_bytes()[1])?;
                let b = parse_nibble(hex.as_bytes()[2])?;
                Some(Self::rgb(r * 17, g * 17, b * 17))
            }
            4 => {
                let r = parse_nibble(hex.as_bytes()[0])?;
                let g = parse_nibble(hex.as_bytes()[1])?;
                let b = parse_nibble(hex.as_bytes()[2])?;
                let a = parse_nibble(hex.as_bytes()[3])?;
                Some(Self::rgba(r * 17, g * 17, b * 17, a * 17))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::rgba(r, g, b, a))
            }
            _ => None,
        }
    }

    /// Look up a CSS named color (ASCII case-insensitive).
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "transparent" => Some(Self::TRANSPARENT),
            "black" => Some(Self::rgb(0, 0, 0)),
            "white" => Some(Self::rgb(255, 255, 255)),
            "red" => Some(Self::rgb(255, 0, 0)),
            "green" => Some(Self::rgb(0, 128, 0)),
            "lime" => Some(Self::rgb(0, 255, 0)),
            "blue" => Some(Self::rgb(0, 0, 255)),
            "yellow" => Some(Self::rgb(255, 255, 0)),
            "cyan" | "aqua" => Some(Self::rgb(0, 255, 255)),
            "magenta" | "fuchsia" => Some(Self::rgb(255, 0, 255)),
            "silver" => Some(Self::rgb(192, 192, 192)),
            "gray" | "grey" => Some(Self::rgb(128, 128, 128)),
            "lightgray" | "lightgrey" => Some(Self::rgb(211, 211, 211)),
            "darkgray" | "darkgrey" => Some(Self::rgb(169, 169, 169)),
            "dimgray" | "dimgrey" => Some(Self::rgb(105, 105, 105)),
            "slategray" | "slategrey" => Some(Self::rgb(112, 128, 144)),
            "darkslategray" | "darkslategrey" => Some(Self::rgb(47, 79, 79)),
            "maroon" => Some(Self::rgb(128, 0, 0)),
            "olive" => Some(Self::rgb(128, 128, 0)),
            "purple" => Some(Self::rgb(128, 0, 128)),
            "rebeccapurple" => Some(Self::rgb(102, 51, 153)),
            "teal" => Some(Self::rgb(0, 128, 128)),
            "navy" => Some(Self::rgb(0, 0, 128)),
            "midnightblue" => Some(Self::rgb(25, 25, 112)),
            "orange" => Some(Self::rgb(255, 165, 0)),
            "orangered" => Some(Self::rgb(255, 69, 0)),
            "coral" => Some(Self::rgb(255, 127, 80)),
            "tomato" => Some(Self::rgb(255, 99, 71)),
            "salmon" => Some(Self::rgb(250, 128, 114)),
            "crimson" => Some(Self::rgb(220, 20, 60)),
            "gold" => Some(Self::rgb(255, 215, 0)),
            "khaki" => Some(Self::rgb(240, 230, 140)),
            "pink" => Some(Self::rgb(255, 192, 203)),
            "hotpink" => Some(Self::rgb(255, 105, 180)),
            "violet" => Some(Self::rgb(238, 130, 238)),
            "indigo" => Some(Self::rgb(75, 0, 130)),
            "slateblue" => Some(Self::rgb(106, 90, 205)),
            "darkslateblue" => Some(Self::rgb(72, 61, 139)),
            "royalblue" => Some(Self::rgb(65, 105, 225)),
            "steelblue" => Some(Self::rgb(70, 130, 180)),
            "dodgerblue" => Some(Self::rgb(30, 144, 255)),
            "cornflowerblue" => Some(Self::rgb(100, 149, 237)),
            "skyblue" => Some(Self::rgb(135, 206, 235)),
            "turquoise" => Some(Self::rgb(64, 224, 208)),
            "seagreen" => Some(Self::rgb(46, 139, 87)),
            "forestgreen" => Some(Self::rgb(34, 139, 34)),
            "brown" => Some(Self::rgb(165, 42, 42)),
            "tan" => Some(Self::rgb(210, 180, 140)),
            "wheat" => Some(Self::rgb(245, 222, 179)),
            "ivory" => Some(Self::rgb(255, 255, 240)),
            "aliceblue" => Some(Self::rgb(240, 248, 255)),
            "whitesmoke" => Some(Self::rgb(245, 245, 245)),
            _ => None,
        }
    }

    /// Alpha-composite `self` (foreground) over `dst` (background).
    pub fn blend_over(self, dst: Self) -> Self {
        match self.a {
            0 => dst,
            255 => self,
            a => {
                let src_a = a as u32;
                let inv_a = 255 - src_a;
                let out_r = (self.r as u32 * src_a + dst.r as u32 * inv_a + 127) / 255;
                let out_g = (self.g as u32 * src_a + dst.g as u32 * inv_a + 127) / 255;
                let out_b = (self.b as u32 * src_a + dst.b as u32 * inv_a + 127) / 255;
                let out_a = src_a + (dst.a as u32 * inv_a + 127) / 255;
                Self::rgba(out_r as u8, out_g as u8, out_b as u8, out_a.min(255) as u8)
            }
        }
    }

    /// Pack into `0x00RRGGBB` as expected by `softbuffer`.
    pub const fn to_0rgb_u32(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a == 255 {
            write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            write!(f, "rgba({}, {}, {}, {})", self.r, self.g, self.b, self.a)
        }
    }
}

fn parse_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_short_and_long_hex_colors() {
        assert_eq!(Color::from_hex("#f0a"), Some(Color::rgb(0xff, 0x00, 0xaa)));
        assert_eq!(Color::from_hex("#1e293b"), Some(Color::rgb(0x1e, 0x29, 0x3b)));
        assert_eq!(Color::from_hex("#ff000080"), Some(Color::rgba(255, 0, 0, 128)));
        assert_eq!(Color::from_hex("#12"), None);
        assert_eq!(Color::from_hex("#zzzzzz"), None);
    }

    #[test]
    fn looks_up_named_colors_and_blends() {
        assert_eq!(Color::from_name("ReD"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(Color::from_name("transparent"), Some(Color::TRANSPARENT));
        assert_eq!(Color::from_name("not-a-color"), None);

        let blended = Color::rgba(255, 0, 0, 128).blend_over(Color::rgb(0, 0, 255));
        assert_eq!(blended, Color::rgb(128, 0, 127));
        assert_eq!(Color::rgb(0x12, 0x34, 0x56).to_0rgb_u32(), 0x0012_3456);
    }
}
