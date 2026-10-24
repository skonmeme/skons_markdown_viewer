//! User settings and their rules (STANDARD §4.5, §5, §7, §8 and §9).

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::style::automatic_margin;

/// Name of the built-in font set. It cannot be edited, renamed or deleted.
pub const DEFAULT_FONT_SET: &str = "Default";

/// Korean font of the built-in font set.
pub const DEFAULT_KOREAN_FONT: &str = "Apple SD Gothic Neo";

/// Base name for new custom font sets ("Custom", "Custom 2", ...).
const NEW_FONT_SET_NAME: &str = "Custom";

pub const FONT_SIZE_MIN: f64 = 8.0;
pub const FONT_SIZE_MAX: f64 = 48.0;
pub const FONT_SIZE_DEFAULT: f64 = 16.0;

pub const LINE_HEIGHT_MIN: f64 = 1.0;
pub const LINE_HEIGHT_MAX: f64 = 2.5;
pub const LINE_HEIGHT_DEFAULT: f64 = 1.6;

pub const FIXED_MARGIN_MIN: f64 = 0.0;
pub const FIXED_MARGIN_MAX: f64 = 480.0;
pub const FIXED_MARGIN_DEFAULT: f64 = 64.0;
/// Amount added or removed by the Increase/Decrease Margins commands.
pub const MARGIN_STEP: f64 = 16.0;

/// Page zoom levels offered by Zoom In and Zoom Out.
pub const ZOOM_LEVELS: [f64; 13] = [
    0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0,
];

/// All persisted user settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Custom font sets. The built-in Default set is not stored.
    pub font_sets: Vec<FontSet>,
    /// Name of the active font set.
    pub active_font_set: String,
    pub color_mode: ColorMode,
    /// Text color used in [`ColorMode::Custom`].
    pub text_color: Rgb,
    /// Background color used in [`ColorMode::Custom`].
    pub background_color: Rgb,
    pub margin_mode: MarginMode,
    /// Margin in CSS pixels used in [`MarginMode::Fixed`].
    pub fixed_margin: f64,
    /// Page zoom factor shared by all windows.
    pub zoom: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            font_sets: Vec::new(),
            active_font_set: DEFAULT_FONT_SET.to_owned(),
            color_mode: ColorMode::Automatic,
            text_color: Rgb::new(0x1f, 0x23, 0x28),
            background_color: Rgb::new(0xff, 0xff, 0xff),
            margin_mode: MarginMode::Automatic,
            fixed_margin: FIXED_MARGIN_DEFAULT,
            zoom: 1.0,
        }
    }
}

/// A named combination of fonts and text metrics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FontSet {
    pub name: String,
    /// Font family for Latin script and everything not covered by `korean`.
    /// `None` selects the system font.
    pub latin: Option<String>,
    /// Font family for Hangul. `None` selects the system font.
    pub korean: Option<String>,
    /// Font family for code. `None` selects the system monospaced font.
    pub monospace: Option<String>,
    /// Base font size in CSS pixels.
    pub size: f64,
    /// Line height as a multiple of the font size.
    pub line_height: f64,
}

impl Default for FontSet {
    fn default() -> Self {
        Self {
            name: DEFAULT_FONT_SET.to_owned(),
            latin: None,
            korean: None,
            monospace: None,
            size: FONT_SIZE_DEFAULT,
            line_height: LINE_HEIGHT_DEFAULT,
        }
    }
}

impl FontSet {
    fn normalized(mut self) -> Self {
        self.name = self.name.trim().to_owned();
        for family in [&mut self.latin, &mut self.korean, &mut self.monospace] {
            *family = family
                .take()
                .map(|f| f.trim().to_owned())
                .filter(|f| !f.is_empty());
        }
        self.size = clamp_or(self.size, FONT_SIZE_MIN, FONT_SIZE_MAX, FONT_SIZE_DEFAULT);
        self.line_height = clamp_or(
            self.line_height,
            LINE_HEIGHT_MIN,
            LINE_HEIGHT_MAX,
            LINE_HEIGHT_DEFAULT,
        );
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorMode {
    /// Built-in light and dark palettes that follow the system appearance.
    #[default]
    Automatic,
    /// The user's text and background colors.
    Custom,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarginMode {
    /// Margins derived from the window width.
    #[default]
    Automatic,
    /// A margin chosen by the user.
    Fixed,
}

/// Reasons a font set name is rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameError {
    Empty,
    Reserved,
    Duplicate,
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "The name cannot be empty.",
            Self::Reserved => "The name “Default” is reserved for the built-in font set.",
            Self::Duplicate => "A font set with this name already exists.",
        })
    }
}

impl Settings {
    /// Parses persisted settings. Invalid input yields the defaults.
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str::<Settings>(json)
            .unwrap_or_default()
            .normalized()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("settings are always serializable")
    }

    /// Clamps values into their valid ranges and repairs font set names.
    pub fn normalized(mut self) -> Self {
        let mut font_sets: Vec<FontSet> = Vec::new();
        for set in self.font_sets.drain(..).map(FontSet::normalized) {
            let rejected = set.name.is_empty()
                || set.name.eq_ignore_ascii_case(DEFAULT_FONT_SET)
                || font_sets
                    .iter()
                    .any(|s| s.name.eq_ignore_ascii_case(&set.name));
            if !rejected {
                font_sets.push(set);
            }
        }
        self.font_sets = font_sets;
        if self.font_set(&self.active_font_set).is_none() {
            self.active_font_set = DEFAULT_FONT_SET.to_owned();
        }
        self.fixed_margin = clamp_or(
            self.fixed_margin,
            FIXED_MARGIN_MIN,
            FIXED_MARGIN_MAX,
            FIXED_MARGIN_DEFAULT,
        );
        self.zoom = clamp_or(
            self.zoom,
            ZOOM_LEVELS[0],
            ZOOM_LEVELS[ZOOM_LEVELS.len() - 1],
            1.0,
        );
        self
    }

    /// The built-in Default font set (STANDARD §7.2).
    pub fn default_font_set() -> FontSet {
        FontSet {
            korean: Some(DEFAULT_KOREAN_FONT.to_owned()),
            ..FontSet::default()
        }
    }

    /// Looks up a font set, including the built-in Default set.
    pub fn font_set(&self, name: &str) -> Option<FontSet> {
        if name == DEFAULT_FONT_SET {
            return Some(Self::default_font_set());
        }
        self.font_sets.iter().find(|s| s.name == name).cloned()
    }

    /// Names of all font sets, starting with the built-in Default set.
    pub fn font_set_names(&self) -> Vec<String> {
        std::iter::once(DEFAULT_FONT_SET.to_owned())
            .chain(self.font_sets.iter().map(|s| s.name.clone()))
            .collect()
    }

    pub fn active_font_set(&self) -> FontSet {
        self.font_set(&self.active_font_set)
            .unwrap_or_else(Self::default_font_set)
    }

    pub fn is_default_font_set_active(&self) -> bool {
        self.active_font_set == DEFAULT_FONT_SET
    }

    /// Selects a font set by name. Unknown names select the Default set.
    pub fn select_font_set(&mut self, name: &str) {
        self.active_font_set = match self.font_set(name) {
            Some(set) => set.name,
            None => DEFAULT_FONT_SET.to_owned(),
        };
    }

    /// Adds a copy of the active font set under a new unique name, makes it
    /// active and returns its name.
    pub fn duplicate_active_font_set(&mut self) -> String {
        let name = (1..)
            .map(|n| match n {
                1 => NEW_FONT_SET_NAME.to_owned(),
                n => format!("{NEW_FONT_SET_NAME} {n}"),
            })
            .find(|candidate| self.validate_name(candidate, None).is_ok())
            .expect("an unused name always exists");
        let set = FontSet {
            name: name.clone(),
            ..self.active_font_set()
        };
        self.font_sets.push(set);
        self.active_font_set = name.clone();
        name
    }

    /// Checks whether `name` may be used for a font set, ignoring the set
    /// currently called `current` (used when renaming).
    pub fn validate_name(&self, name: &str, current: Option<&str>) -> Result<(), NameError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(NameError::Empty);
        }
        if name.eq_ignore_ascii_case(DEFAULT_FONT_SET) {
            return Err(NameError::Reserved);
        }
        let duplicate = self
            .font_sets
            .iter()
            .any(|s| s.name.eq_ignore_ascii_case(name) && Some(s.name.as_str()) != current);
        if duplicate {
            return Err(NameError::Duplicate);
        }
        Ok(())
    }

    /// Renames a custom font set.
    pub fn rename_font_set(&mut self, current: &str, new_name: &str) -> Result<(), NameError> {
        if current == DEFAULT_FONT_SET {
            return Err(NameError::Reserved);
        }
        self.validate_name(new_name, Some(current))?;
        let new_name = new_name.trim().to_owned();
        if let Some(set) = self.font_sets.iter_mut().find(|s| s.name == current) {
            set.name = new_name.clone();
            if self.active_font_set == current {
                self.active_font_set = new_name;
            }
        }
        Ok(())
    }

    /// Deletes a custom font set. Deleting the active set activates Default.
    pub fn delete_font_set(&mut self, name: &str) {
        self.font_sets.retain(|s| s.name != name);
        if self.active_font_set == name {
            self.active_font_set = DEFAULT_FONT_SET.to_owned();
        }
    }

    /// Edits the active font set. Returns `false` (and changes nothing) when
    /// the built-in Default set is active.
    pub fn edit_active_font_set(&mut self, edit: impl FnOnce(&mut FontSet)) -> bool {
        let active = self.active_font_set.clone();
        let Some(set) = self.font_sets.iter_mut().find(|s| s.name == active) else {
            return false;
        };
        edit(set);
        set.name = active;
        *set = set.clone().normalized();
        true
    }

    /// Margin in CSS pixels for a viewport of the given width.
    pub fn effective_margin(&self, viewport_width: f64) -> f64 {
        match self.margin_mode {
            MarginMode::Automatic => automatic_margin(viewport_width),
            MarginMode::Fixed => self.fixed_margin.min(viewport_width * 0.25),
        }
    }

    /// Increases (positive `steps`) or decreases the margin by multiples of
    /// [`MARGIN_STEP`], starting from the margin currently shown (in whole
    /// points), and switches to [`MarginMode::Fixed`] (STANDARD §5.2).
    pub fn step_margin(&mut self, viewport_width: f64, steps: i32) {
        let current = self.effective_margin(viewport_width).round();
        self.fixed_margin =
            (current + f64::from(steps) * MARGIN_STEP).clamp(FIXED_MARGIN_MIN, FIXED_MARGIN_MAX);
        self.margin_mode = MarginMode::Fixed;
    }

    pub fn zoom_in(&mut self) {
        if let Some(level) = ZOOM_LEVELS.iter().find(|&&z| z > self.zoom + 1e-6) {
            self.zoom = *level;
        }
    }

    pub fn zoom_out(&mut self) {
        if let Some(level) = ZOOM_LEVELS.iter().rev().find(|&&z| z < self.zoom - 1e-6) {
            self.zoom = *level;
        }
    }

    pub fn can_zoom_in(&self) -> bool {
        self.zoom < ZOOM_LEVELS[ZOOM_LEVELS.len() - 1] - 1e-6
    }

    pub fn can_zoom_out(&self) -> bool {
        self.zoom > ZOOM_LEVELS[0] + 1e-6
    }
}

fn clamp_or(value: f64, min: f64, max: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

/// An sRGB color, persisted as `#rrggbb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn parse_hex(text: &str) -> Option<Self> {
        let hex = text.strip_prefix('#')?;
        if hex.len() != 6 || !hex.is_ascii() {
            return None;
        }
        let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        Some(Self::new(channel(0)?, channel(2)?, channel(4)?))
    }

    /// WCAG relative luminance in `0.0..=1.0`.
    pub fn relative_luminance(self) -> f64 {
        let linear = |c: u8| {
            let c = f64::from(c) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(self.r) + 0.7152 * linear(self.g) + 0.0722 * linear(self.b)
    }

    /// Whether the color counts as a dark background (STANDARD §8).
    pub fn is_dark(self) -> bool {
        self.relative_luminance() < 0.18
    }
}

impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse_hex(&text)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid color {text:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_json_gives_defaults() {
        assert_eq!(Settings::from_json("not json"), Settings::default());
        assert_eq!(Settings::from_json("{}"), Settings::default());
    }

    #[test]
    fn json_round_trip() {
        let mut settings = Settings::default();
        settings.duplicate_active_font_set();
        settings.edit_active_font_set(|set| set.korean = Some("Nanum Gothic".into()));
        settings.color_mode = ColorMode::Custom;
        settings.background_color = Rgb::new(0x10, 0x20, 0x30);
        assert_eq!(Settings::from_json(&settings.to_json()), settings);
        assert!(
            settings
                .to_json()
                .contains("\"backgroundColor\":\"#102030\"")
        );
    }

    #[test]
    fn normalization_repairs_values() {
        let json = r#"{
            "fontSets": [
                {"name": " A ", "size": 200, "lineHeight": 0.1, "latin": "  "},
                {"name": "a"},
                {"name": "default"},
                {"name": ""}
            ],
            "activeFontSet": "missing",
            "fixedMargin": -5,
            "zoom": 9
        }"#;
        let settings = Settings::from_json(json);
        assert_eq!(settings.font_set_names(), ["Default", "A"]);
        let set = settings.font_set("A").unwrap();
        assert_eq!(set.size, FONT_SIZE_MAX);
        assert_eq!(set.line_height, LINE_HEIGHT_MIN);
        assert_eq!(set.latin, None);
        assert_eq!(settings.active_font_set, DEFAULT_FONT_SET);
        assert_eq!(settings.fixed_margin, FIXED_MARGIN_MIN);
        assert_eq!(settings.zoom, 3.0);
    }

    #[test]
    fn default_font_set_matches_the_standard() {
        let set = Settings::default_font_set();
        assert_eq!(set.name, DEFAULT_FONT_SET);
        assert_eq!(set.latin, None);
        assert_eq!(set.korean.as_deref(), Some("Apple SD Gothic Neo"));
        assert_eq!(set.monospace, None);
        assert_eq!((set.size, set.line_height), (16.0, 1.6));
    }

    #[test]
    fn default_font_set_is_immutable() {
        let mut settings = Settings::default();
        assert!(!settings.edit_active_font_set(|set| set.size = 20.0));
        assert_eq!(settings.active_font_set(), Settings::default_font_set());
        assert_eq!(
            settings.rename_font_set(DEFAULT_FONT_SET, "X"),
            Err(NameError::Reserved)
        );
    }

    #[test]
    fn font_set_lifecycle() {
        let mut settings = Settings::default();
        assert_eq!(settings.duplicate_active_font_set(), "Custom");
        assert!(settings.edit_active_font_set(|set| set.size = 18.0));
        assert_eq!(settings.duplicate_active_font_set(), "Custom 2");
        assert_eq!(settings.active_font_set().size, 18.0);

        assert_eq!(
            settings.rename_font_set("Custom 2", " custom "),
            Err(NameError::Duplicate)
        );
        assert_eq!(
            settings.rename_font_set("Custom 2", "DEFAULT"),
            Err(NameError::Reserved)
        );
        assert_eq!(
            settings.rename_font_set("Custom 2", "  "),
            Err(NameError::Empty)
        );
        assert_eq!(settings.rename_font_set("Custom 2", " Reading "), Ok(()));
        assert_eq!(settings.active_font_set, "Reading");

        settings.delete_font_set("Reading");
        assert_eq!(settings.active_font_set, DEFAULT_FONT_SET);
        assert_eq!(settings.font_set_names(), ["Default", "Custom"]);

        settings.select_font_set("Custom");
        assert_eq!(settings.active_font_set().size, 18.0);
        settings.select_font_set("nope");
        assert!(settings.is_default_font_set_active());
    }

    #[test]
    fn margin_steps_start_from_effective_margin() {
        let mut settings = Settings::default();
        // The automatic margin at 1000 pt is 16 + 0.35 × 440 = 170.
        settings.step_margin(1000.0, 1);
        assert_eq!(settings.margin_mode, MarginMode::Fixed);
        assert_eq!(settings.fixed_margin, 186.0);
        settings.step_margin(1000.0, -2);
        assert_eq!(settings.fixed_margin, 154.0);
        // A fixed margin above 25 % of the width starts from the capped value.
        settings.fixed_margin = 300.0;
        settings.step_margin(800.0, 1);
        assert_eq!(settings.fixed_margin, 216.0);
        settings.fixed_margin = 8.0;
        settings.step_margin(1000.0, -1);
        assert_eq!(settings.fixed_margin, 0.0);
    }

    #[test]
    fn fixed_margin_is_limited_to_a_quarter_of_the_width() {
        let settings = Settings {
            margin_mode: MarginMode::Fixed,
            fixed_margin: 300.0,
            ..Settings::default()
        };
        assert_eq!(settings.effective_margin(2000.0), 300.0);
        assert_eq!(settings.effective_margin(800.0), 200.0);
    }

    #[test]
    fn zoom_levels() {
        let mut settings = Settings::default();
        settings.zoom_in();
        assert_eq!(settings.zoom, 1.1);
        settings.zoom_out();
        settings.zoom_out();
        assert_eq!(settings.zoom, 0.9);
        settings.zoom = 3.0;
        assert!(!settings.can_zoom_in());
        settings.zoom_in();
        assert_eq!(settings.zoom, 3.0);
    }

    #[test]
    fn colors() {
        assert_eq!(Rgb::parse_hex("#1F2328"), Some(Rgb::new(0x1f, 0x23, 0x28)));
        assert_eq!(Rgb::parse_hex("1f2328"), None);
        assert_eq!(Rgb::parse_hex("#12345"), None);
        assert!(Rgb::new(0x1e, 0x1e, 0x1e).is_dark());
        assert!(!Rgb::new(0xff, 0xff, 0xff).is_dark());
        assert!(!Rgb::new(0xf4, 0xec, 0xd8).is_dark());
    }
}
