//! # `hartex_conftoml` - The HarTex Configuration TOML Library.
//!
//! The `hartex_conftoml` provides an interface for serializing and deserializing TOML
//! configuration for HarTex Discord bot.

#![allow(non_snake_case)]
#![deny(clippy::pedantic, warnings, unsafe_code)]
#![feature(format_args_capture)]

use hartex_core::{
    error::{
        HarTexError,
        HarTexResult
    },
    logging::tracing
};
use serde::Deserialize;

pub mod dashacc;
pub mod guildconf;
pub mod nightly;

/// # Struct `TomlConfig`
///
/// Represents the top-level configuration, all other configuration branches from here.
#[derive(Deserialize)]
pub struct TomlConfig {
    pub DashboardAccess: Vec<dashacc::DashboardAccess>,
    pub GuildConfiguration: guildconf::GuildConfiguration,
    #[serde(default)]
    pub NightlyFeatures: nightly::NightlyFeatures
}

pub fn from_string(input: String) -> HarTexResult<TomlConfig> {
    Ok(match toml::from_str(input.as_str()) {
        Ok(config) => config,
        Err(error) => {
            tracing::error!("failed to deserialize config: {error}");

            return Err(HarTexError::from(error));
        }
    })
}
