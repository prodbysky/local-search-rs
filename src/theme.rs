use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Default, Debug, Clone, Copy)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn into_raylib(self) -> raylib::color::Color {
        raylib::color::Color::new(self.r, self.g, self.b, 255)
    }
}

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b);
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s
            .strip_prefix('#')
            .ok_or_else(|| serde::de::Error::custom("expected #RRGGBB"))?;

        if s.len() != 6 {
            return Err(serde::de::Error::custom("expected #RRGGBB"));
        }

        let r = u8::from_str_radix(&s[0..2], 16).map_err(serde::de::Error::custom)?;
        let g = u8::from_str_radix(&s[2..4], 16).map_err(serde::de::Error::custom)?;
        let b = u8::from_str_radix(&s[4..6], 16).map_err(serde::de::Error::custom)?;

        Ok(Color { r, g, b })
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    #[default]
    Default,
    CatppuccinLatte,
    CatppuccinMocha,
    Custom(ThemeColors),
}

impl Theme {
    const DEFAULT_COLORS: ThemeColors = ThemeColors {
        background_color: Color::new(0x18, 0x18, 0x18),
        foreground_color: Color::new(0xcc, 0xcc, 0xcc),
        idle_color: Color::new(0x20, 0x20, 0x20),
        hovered_color: Color::new(0x30, 0x30, 0x30),
        clicked_color: Color::new(0x40, 0x40, 0x40),
    };

    const CAT_LATTE_COLORS: ThemeColors = ThemeColors {
        background_color: Color::new(0xef, 0xf1, 0xf5), // BASE
        foreground_color: Color::new(0x4c, 0x4f, 0x69), // TEXT
        idle_color: Color::new(0xdc, 0x8a, 0x78),       // ROSEWATER
        hovered_color: Color::new(0xdd, 0x78, 0x78),    // FLAMINGO
        clicked_color: Color::new(0xea, 0x76, 0xcb),    // PINK
    };

    const CAT_MOCHA_COLORS: ThemeColors = ThemeColors {
        background_color: Color::new(0x1e, 0x1e, 0x2e), // BASE
        foreground_color: Color::new(0xcd, 0xd6, 0xf4), // TEXT
        idle_color: Color::new(0x31, 0x32, 0x44),       // SURFACE 0
        hovered_color: Color::new(0x45, 0x47, 0x5a),    // SURFACE 1
        clicked_color: Color::new(0x58, 0x5b, 0x70),    // SURFACE 2
    };

    pub fn get_all_colors(&self) -> &ThemeColors {
        match self {
            Self::Default => &Self::DEFAULT_COLORS,
            Self::CatppuccinLatte => &Self::CAT_LATTE_COLORS,
            Self::CatppuccinMocha => &Self::CAT_MOCHA_COLORS,
            Self::Custom(c) => &c,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ThemeColors {
    pub background_color: Color,
    pub foreground_color: Color,
    pub idle_color: Color,
    pub hovered_color: Color,
    pub clicked_color: Color,
}
