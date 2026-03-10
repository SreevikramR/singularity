// SPDX-License-Identifier: GPL-3.0-only

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[version = 2]
pub struct Config {
    /// Whether dark mode is enabled.
    pub dark_mode: bool,
    /// Whether night light (warm color shift) is enabled.
    pub night_light: bool,
    /// Whether do-not-disturb mode is enabled.
    pub do_not_disturb: bool,
    /// Screenshot command to execute.
    pub screenshot_command: String,
    /// Maximum volume percentage limit.
    #[serde(default = "default_max_volume")]
    pub max_volume: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dark_mode: false,
            night_light: false,
            do_not_disturb: false,
            screenshot_command: String::new(),
            max_volume: 150,
        }
    }
}

fn default_max_volume() -> u32 {
    150
}
