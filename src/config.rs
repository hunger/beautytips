// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::collections::HashMap;

use anyhow::Context;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct TomlRunnerConfig {
    #[serde(default)]
    extra_env: HashMap<String, String>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct TomlConfig {
    #[serde(default)]
    runner: TomlRunnerConfig,
}

pub struct Configuration {
    config: TomlConfig,
}

impl Configuration {
    pub fn runner_config(&self) -> beautytips::RunnerConfig {
        beautytips::RunnerConfig {
            extra_env: self.config.runner.extra_env.clone(),
        }
    }
}

pub fn load_configuration() -> anyhow::Result<Configuration> {
    let config_dir = dirs::config_dir()
        .map(|cd| cd.join("beautytips"))
        .ok_or(anyhow::anyhow!("Config directory not found"))?;

    let config_file = config_dir.join("config.toml");

    let config_data = std::fs::read_to_string(&config_file).context(format!(
        "Failed to read toml file {}",
        config_file.display()
    ))?;

    let config: TomlConfig =
        toml::from_str(config_data.as_str()).context("Failed to parse toml string")?;

    let config = Configuration { config };

    Ok(config)
}
