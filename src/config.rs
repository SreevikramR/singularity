// SPDX-License-Identifier: GPL-3.0-only

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};

#[derive(Debug, Default, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    /// Whether dark mode is enabled.
    pub dark_mode: bool,
    /// Whether night light (warm color shift) is enabled.
    pub night_light: bool,
    /// Whether do-not-disturb mode is enabled.
    pub do_not_disturb: bool,
}
