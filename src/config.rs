//! Persistent settings, stored with cosmic-config.

use cosmic::cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::cosmic_theme::BlurStrength;
use serde::{Deserialize, Serialize};

use crate::highlight::SyntaxTheme;

/// Stored under `v1/`. The version must be set with this attribute: the config
/// watcher reads it from `Config::VERSION`, which otherwise defaults to 0, so the
/// watcher would see an empty `v0/` and replace the loaded settings with defaults.
#[derive(Clone, CosmicConfigEntry, Debug, Deserialize, PartialEq, Serialize)]
#[version = 1]
#[serde(default)]
pub struct Config {
    /// Whether the content area lets the blurred desktop show through.
    pub translucent: bool,
    /// How translucent the content area is, when `translucent` is set. The value is
    /// turned into an opacity by the theme's alpha map, as COSMIC does for its own
    /// frosted surfaces.
    pub blur_strength: BlurStrength,
    /// Colour theme for code; its light or dark variant follows the desktop.
    pub syntax_theme: SyntaxTheme,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            translucent: true,
            blur_strength: BlurStrength::VeryLow,
            syntax_theme: SyntaxTheme::Cosmic,
        }
    }
}

impl Config {
    /// Loads the settings, falling back to defaults for anything missing or invalid.
    pub fn load(app_id: &str) -> (Option<cosmic_config::Config>, Self) {
        match cosmic_config::Config::new(app_id, Self::VERSION) {
            Ok(handler) => {
                let config = Self::get_entry(&handler).unwrap_or_else(|(errors, config)| {
                    log::warn!("errors loading config: {errors:?}");
                    config
                });
                (Some(handler), config)
            }
            Err(error) => {
                log::warn!("cannot open config: {error}");
                (None, Self::default())
            }
        }
    }

    /// Opacity of the content area's background, between 0 and 1.
    pub fn content_alpha(&self, theme: &cosmic::cosmic_theme::Theme) -> f32 {
        if self.translucent {
            theme.alpha_map.blurred_alpha(self.blur_strength)
        } else {
            1.0
        }
    }
}

/// Number of blur strength levels.
const BLUR_LEVELS: u8 = 14;

/// Highest position of the opacity slider. Positions below it are the blur strengths
/// from most to least translucent; this last one is fully opaque.
pub const MAX_OPACITY_STEP: u8 = BLUR_LEVELS;

impl Config {
    /// Position of the opacity slider for these settings.
    pub fn opacity_step(&self) -> u8 {
        if self.translucent {
            BLUR_LEVELS - 1 - (self.blur_strength as u8).min(BLUR_LEVELS - 1)
        } else {
            MAX_OPACITY_STEP
        }
    }

    /// Applies an opacity slider position. Going opaque keeps the blur strength, so
    /// moving back restores the previous translucency.
    pub fn set_opacity_step(&mut self, step: u8) {
        if step >= MAX_OPACITY_STEP {
            self.translucent = false;
        } else {
            self.translucent = true;
            self.blur_strength = BlurStrength::try_from(BLUR_LEVELS - 1 - step).unwrap_or_default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_stored_settings() {
        // Settings of earlier releases are stored under v1/.
        assert_eq!(Config::VERSION, 1);
    }

    #[test]
    fn default_is_translucent_very_low() {
        let config = Config::default();
        assert!(config.translucent);
        assert_eq!(config.blur_strength, BlurStrength::VeryLow);
    }

    #[test]
    fn opaque_has_full_alpha() {
        let theme = cosmic::cosmic_theme::Theme::default();
        let config = Config {
            translucent: false,
            ..Config::default()
        };
        assert_eq!(config.content_alpha(&theme), 1.0);
    }

    #[test]
    fn translucent_uses_theme_alpha_map() {
        let theme = cosmic::cosmic_theme::Theme::default();
        let config = Config::default();
        let expected = theme.alpha_map.blurred_alpha(BlurStrength::VeryLow);
        assert_eq!(config.content_alpha(&theme), expected);
        assert!(expected < 1.0);
    }

    #[test]
    fn opacity_steps_round_trip() {
        let mut config = Config::default();
        for step in 0..=MAX_OPACITY_STEP {
            config.set_opacity_step(step);
            assert_eq!(config.opacity_step(), step);
        }
    }

    #[test]
    fn higher_step_is_more_opaque() {
        let theme = cosmic::cosmic_theme::Theme::default();
        let mut config = Config::default();
        let alphas: Vec<f32> = (0..=MAX_OPACITY_STEP)
            .map(|step| {
                config.set_opacity_step(step);
                config.content_alpha(&theme)
            })
            .collect();
        assert!(alphas.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(alphas.last(), Some(&1.0));
    }

    #[test]
    fn default_step_is_very_low_blur() {
        let mut config = Config::default();
        let step = config.opacity_step();
        config.set_opacity_step(step);
        assert_eq!(config.blur_strength, BlurStrength::VeryLow);
        assert_eq!(step, 11);
    }

    #[test]
    fn going_opaque_keeps_strength() {
        let mut config = Config::default();
        config.set_opacity_step(MAX_OPACITY_STEP);
        assert!(!config.translucent);
        assert_eq!(config.blur_strength, BlurStrength::VeryLow);
    }
}
