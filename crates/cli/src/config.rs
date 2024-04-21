// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use anyhow::Context;

#[derive(Debug, serde::Deserialize)]
pub struct Configuration {
    pub actions: Vec<Action>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Action {
    pub id: beautytips::ActionId,
}

pub fn load_config() -> anyhow::Result<Configuration> {
    let config_dir = dirs::config_dir()
        .map(|cd| cd.join("beautytips"))
        .ok_or(anyhow::anyhow!("Config directory not found"))?;
    let config_file = config_dir.join("config.toml");

    let config_data =
        std::fs::read_to_string(&config_file).context("Failed to read config file")?;

    toml::from_str(&config_data).context("Failed to parse toml")
}
