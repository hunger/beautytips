// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::{collections::HashMap, fmt::Display};

use anyhow::Context;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(try_from = "String", expecting = "an action id")]
pub struct ActionId(String);

impl ActionId {
    /// Create a new `ActionId`
    ///
    /// # Errors
    ///
    /// Raise an invaliv configuration error if the action id contains anything
    /// but lowercase ASCII letters or '_'.
    pub fn new(input: String) -> anyhow::Result<Self> {
        if input
            .chars()
            .any(|c| !c.is_ascii_lowercase() && c != '_' && !c.is_ascii_digit())
        {
            Err(anyhow::anyhow!("{input} is not a valid action id"))
        } else {
            Ok(ActionId(input))
        }
    }

    /// Create a new `ActionId`
    ///
    /// # Errors
    ///
    /// Raise an invaliv configuration error if the action id contains anything
    /// but lowercase ASCII letters or '_'.
    pub fn new_str(input: &str) -> crate::Result<Self> {
        Self::new(input.to_string())
    }
}

impl Display for ActionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for ActionId {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        ActionId::new_str(value)
    }
}

impl TryFrom<String> for ActionId {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ActionId::new(value)
    }
}

impl std::str::FromStr for ActionId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ActionId::new_str(s)
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ActionDefinition {
    pub name: ActionId,
    pub command: String,
    #[serde(default)]
    pub expected_exit_code: i32,
    #[serde(default)]
    pub inputs: HashMap<String, Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
struct ActionGroup {
    pub name: ActionId,
    pub actions: Vec<ActionId>,
}

#[derive(Debug, serde::Deserialize)]
struct TomlConfiguration {
    pub action_groups: Vec<ActionGroup>,
    pub actions: Vec<ActionDefinition>,
}

#[derive(Clone, Debug)]
pub struct Configuration {
    action_groups: HashMap<String, Vec<String>>,
    actions: HashMap<String, beautytips::ActionDefinition>,
}

pub fn load_config() -> anyhow::Result<Configuration> {
    let config_dir = dirs::config_dir()
        .map(|cd| cd.join("beautytips"))
        .ok_or(anyhow::anyhow!("Config directory not found"))?;
    let config_file = config_dir.join("config.toml");

    let config_data = std::fs::read_to_string(&config_file)
        .context(format!("Failed to read toml file {config_file:?}"))?;

    let toml_config: TomlConfiguration = toml::from_str(&config_data)
        .context(format!("Failed to parse toml file {config_file:?}"))?;
    let actions = toml_config
        .actions
        .into_iter()
        .try_fold(HashMap::new(), |mut acc, ad| {
            let id = ad.name.to_string();
            let command = shell_words::split(&ad.command).context(format!(
                "Failed to parse command '{}' of action '{id}'",
                ad.command
            ))?;
            let input_filters = ad
                .inputs
                .into_iter()
                .try_fold(HashMap::new(), |mut acc, (k, v)| {
                    let entry = acc.entry(k.clone());
                    if matches!(entry, std::collections::hash_map::Entry::Occupied(_)) {
                        return Err(anyhow::anyhow!(format!(
                            "Redefinition of input filters for '{k}'"
                        )));
                    }
                    let globs = v
                        .iter()
                        .map(|p| {
                            glob::Pattern::new(p)
                                .context(format!("Failed to parse glob pattern '{p}' for '{k}'"))
                        })
                        .collect::<Result<_, _>>()?;
                    entry.or_insert(globs);
                    Ok(acc)
                })
                .context("Parsing input filters for action '{id}'")?;
            let entry = acc.entry(id.clone());
            if matches!(entry, std::collections::hash_map::Entry::Occupied(_)) {
                return Err(anyhow::anyhow!(format!("Redefinition of action '{id}'")));
            }
            entry.or_insert(beautytips::ActionDefinition {
                id,
                command,
                expected_exit_code: ad.expected_exit_code,
                input_filters,
            });
            Ok(acc)
        })?;

    let action_groups = toml_config
        .action_groups
        .into_iter()
        .map(|ag| (ag.name, ag.actions))
        .try_fold(HashMap::new(), |mut acc, (v_id, v_val)| {
            let entry = acc.entry(v_id.to_string());
            if matches!(entry, std::collections::hash_map::Entry::Occupied(_)) {
                return Err(anyhow::anyhow!(format!(
                    "Redefinition of action group '{v_id}'"
                )));
            }
            entry.or_insert(
                v_val
                    .iter()
                    .map(|v| {
                        let v = v.to_string();
                        if !actions.contains_key(&v) {
                            Err(anyhow::anyhow!(format!(
                                "Action group '{v_id}' contains an unknown action '{v}'"
                            )))
                        } else {
                            Ok(v)
                        }
                    })
                    .collect::<Result<_, _>>()?,
            );

            Ok(acc)
        })?;

    Ok(Configuration {
        action_groups,
        actions,
    })
}
