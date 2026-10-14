//! Persistent settings, stored with cosmic-config.

use cosmic::cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::cosmic_theme::BlurStrength;
use serde::{Deserialize, Serialize};

pub const CONFIG_VERSION: u64 = 1;

#[derive(Clone, CosmicConfigEntry, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Config {
    /// Whether the content area lets the blurred desktop show through.
    pub translucent: bool,
    /// How translucent the content area is, when `translucent` is set. The value is
    /// turned into an opacity by the theme's alpha map, as COSMIC does for its own
    /// frosted surfaces.
    pub blur_strength: BlurStrength,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            translucent: true,
            blur_strength: BlurStrength::VeryLow,
        }
    }
}

impl Config {
    /// Loads the settings, falling back to defaults for anything missing or invalid.
    pub fn load(app_id: &str) -> (Option<cosmic_config::Config>, Self) {
        match cosmic_config::Config::new(app_id, CONFIG_VERSION) {
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

/// Number of blur strength levels, for the settings slider.
pub const BLUR_LEVELS: u8 = 14;

/// The strength at slider position `level`; out of range values are clamped.
pub fn blur_strength(level: u8) -> BlurStrength {
    BlurStrength::try_from(level.min(BLUR_LEVELS - 1)).unwrap_or_default()
}

/// The slider position of `strength`.
pub fn blur_level(strength: BlurStrength) -> u8 {
    strength as u8
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn levels_round_trip() {
        for level in 0..BLUR_LEVELS {
            assert_eq!(blur_level(blur_strength(level)), level);
        }
        assert_eq!(blur_strength(200), BlurStrength::ExtremelyHigh2);
    }
}
