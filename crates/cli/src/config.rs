// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::{collections::HashMap, fmt::Display, path::Path};
use std::convert::TryFrom;

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
            && !input.is_empty()
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
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct TomlActionDefinition {
    pub name: ActionId,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub expected_exit_code: i32,
    #[serde(default)]
    pub inputs: HashMap<String, Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlActionGroup {
    pub name: ActionId,
    pub actions: Vec<ActionId>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlConfiguration {
    #[serde(default)]
    pub action_groups: Vec<TomlActionGroup>,
    #[serde(default)]
    pub actions: Vec<TomlActionDefinition>,
}

#[derive(Clone, Debug, Default)]
pub struct Configuration {
    action_groups: HashMap<String, Vec<String>>,
    actions: Vec<beautytips::ActionDefinition>,
    action_map: HashMap<String, usize>,
}

fn merge_action_definition(
    _base: &beautytips::ActionDefinition,
    other: &beautytips::ActionDefinition,
) -> anyhow::Result<beautytips::ActionDefinition> {
    // TODO: Actually merge ;-)
    Ok(other.clone())
}

impl Configuration {
    /// Merge `other` onto the base of `self`
    pub fn merge(mut self: Self, mut other: Self) -> anyhow::Result<Self> {
        #[derive(Debug)]
        enum ActionState<T> {
            Add(T),
            Change(T, T),
            Remove(T),
        }

        let mut actions: Vec<beautytips::ActionDefinition> = other
            .actions
            .iter()
            .map(|definition| {
                if let Some(sa) = self.action(&definition.id).clone() {
                    if definition.command.len() == 1
                        && definition.command.first() == Some(&"/dev/null".to_string())
                    {
                        ActionState::Remove(definition.clone())
                    } else {
                        ActionState::Change(sa.clone(), definition.clone())
                    }
                } else {
                    ActionState::Add(definition.clone())
                }
            })
            .chain(self.actions.iter().filter_map(|definition| {
                if other.action(&definition.id).is_some() {
                    None
                } else {
                    Some(ActionState::Add(definition.clone()))
                }
            }))
            .filter_map(|ad| match ad {
                ActionState::Add(d) => Some(Ok(d)),
                ActionState::Change(sd, od) => {
                    Some(merge_action_definition(&sd, &od))
                }
                ActionState::Remove(d) => {
                    if d.expected_exit_code != 0 || !d.input_filters.is_empty() {
                        Some(Err(anyhow::anyhow!(format!(
                            "Removal of '{}' failed: Too many fields",
                            d.id
                        ))))
                    } else {
                        None
                    }
                }
            })
            .collect::<anyhow::Result<_>>()?;

        actions.sort();
        let action_map: HashMap<_, _> = actions.iter().enumerate().map(|(index, d)| (d.id.clone(), index)).collect();

        let action_groups = self
            .action_groups
            .drain()
            .map(|(k, v)| {
                (
                    k,
                    v.into_iter()
                        .filter(|v| !action_map.contains_key(v.as_str()))
                        .collect::<Vec<_>>(),
                )
            })
            .chain(other.action_groups.drain())
            .fold(HashMap::new(), |mut acc, (k, v)| {
                if v.is_empty() {
                    if acc.remove_entry(&k).is_none() {
                        acc.insert(k, vec![]); // base used to define something and is empty now... Keep this for other to extend.
                    }
                } else {
                    let entry = acc.entry(k);
                    entry
                        .and_modify(|ov| {
                            ov.extend(v.iter().cloned());
                            ov.sort();
                            ov.dedup();
                        })
                        .or_insert(v);
                }
                acc
            });

        Ok(Self {
            action_groups,
            actions,
            action_map,
        })
    }

    pub fn action<'a, 'b>(&'a self, name: &'b str) -> Option<&'a beautytips::ActionDefinition> {
        self.action_map.get(name).and_then(|index| self.actions.get(*index))
    }

    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    pub fn action_group<'a, 'b>(&'a self, name: &'b str) -> Option<&'a Vec<String>> {
        // TODO: Return an iterator over the `ActionDefinition`s
        self.action_groups.get(name)
    }

    pub fn action_group_count(&self) -> usize {
        self.action_groups.len()
    }
}

impl TryFrom<&str> for Configuration {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let toml_config: TomlConfiguration =
            toml::from_str(value).context(format!("Failed to parse toml"))?;

        let mut actions: Vec<_> = toml_config
            .actions
            .into_iter()
            .map(|ad| {
                let id = ad.name.to_string();

                let command = shell_words::split(ad.command.trim()).context(format!(
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
                                glob::Pattern::new(p).context(format!(
                                    "Failed to parse glob pattern '{p}' for '{k}'"
                                ))
                            })
                            .collect::<Result<_, _>>()?;
                        entry.or_insert(globs);
                        Ok(acc)
                    })
                    .context("Parsing input filters for action '{id}'")?;

                Ok(
                    beautytips::ActionDefinition {
                        id: id.clone(),
                        command,
                        expected_exit_code: ad.expected_exit_code,
                        input_filters,
                    },
                )
            }).collect::<anyhow::Result<_>>()?;

        actions.sort();

        {
            let mut current = None;

            for d in &actions {
                if Some(&d.id) == current {
                    return Err(anyhow::anyhow!(format!("Duplicate action \'{}\' found", d.id)));
                }
                current = Some(&d.id);
            }
        }
        let action_map: HashMap<_, _> = actions.iter().enumerate().map(|(index, d)| (d.id.clone(), index)).collect();

        let action_groups = toml_config
            .action_groups
            .into_iter()
            .map(|ag| (ag.name, ag.actions))
            .try_fold(HashMap::new(), |mut acc, (v_id, v_val)| {
                let mut v = v_val.iter().map(|v| v.to_string()).collect::<Vec<_>>();
                v.sort();
                v.dedup();

                if v.len() != v_val.len() {
                    return Err(anyhow::anyhow!(
                        "Action group '{v_id}' has duplicate actions"
                    ));
                }
                let old = acc.insert(v_id.to_string(), v);

                if old.is_some() {
                    return Err(anyhow::anyhow!(format!(
                        "Action group '{v_id}' defined twice in one config location"
                    )));
                }

                Ok(acc)
            })?;

        Ok(Configuration {
            action_groups,
            actions,
            action_map,
        })
    }
}

impl TryFrom<&Path> for Configuration {
    type Error = anyhow::Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        let config_data = std::fs::read_to_string(&value)
            .context(format!("Failed to read toml file {value:?}"))?;

        Configuration::try_from(config_data.as_str()).context("Failed to parse toml file {value:?}")
    }
}

struct ActionDefinitionIterator<'a> {
    actions: &'a[beautytips::ActionDefinition],
    filter : &'a dyn Fn(&'a beautytips::ActionDefinition) -> bool,
    current_item: usize,
}

impl<'a> Iterator for ActionDefinitionIterator<'a> {
    type Item = &'a beautytips::ActionDefinition;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_item < self.actions.len() {
            let cur = self.current_item;
            self.current_item += 1;

            let cur_item =  unsafe { self.actions.get_unchecked(cur) };
            if (self.filter)(cur_item) {
                return Some(cur_item);
            }
        }
        None
    }
}

pub fn builtin() -> Configuration {
    let toml = include_str!("rules.toml");
    let config = Configuration::try_from(toml).expect("builtins should parse fine");

    let base = Configuration::default();
    base.merge(config).expect("builtins should merge just fine")
}

pub fn load_user_config() -> anyhow::Result<Configuration> {
    let base = builtin();

    let config_dir = dirs::config_dir()
        .map(|cd| cd.join("beautytips"))
        .ok_or(anyhow::anyhow!("Config directory not found"))?;
    let config_file = config_dir.join("config.toml");

    let user =
        Configuration::try_from(config_file.as_path()).context("Failed to parse configuration file {config_file:?}")?;
    base.merge(user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_from_str_ok() {
        let base = r#"[[actions]]
name = "test1"
command = "foobar x y z"

[[actions]]
name = "test2"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[action_groups]]
name = "test"
actions = [ "test1", "test2" ]
"#;

        let base: Configuration = base.try_into().unwrap();
        eprintln!("{base:?}");

        assert_eq!(base.action_count(), 2);
        assert!(base.action("test1").is_some());
        assert!(base.action("test2").is_some());
        assert_eq!(base.action_group_count(), 1);
    }

    #[test]
    fn test_configuration_from_str_empty_ok() {
        let base = "";

        let base: Configuration = base.try_into().unwrap();
        eprintln!("{base:?}");

        assert_eq!(base.action_count(), 0);
        assert_eq!(base.action_group_count(), 0);
    }

    #[test]
    fn test_configuration_from_str_invalid_top_level_key() {
        let base = r#"[[action]]
name = "test1"
command = "foobar x y z"
"#;

        assert!(TryInto::<Configuration>::try_into(base).is_err());
    }

    #[test]
    fn test_configuration_from_str_invalid_action_key() {
        let base = r#"[[actions]]
name = "test1"
command = "foobar x y z"

[[actions]]
name = "test2"
id = "foobar"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[action_groups]]
name = "test"
actions = [ "test1", "test2" ]
"#;

        assert!(TryInto::<Configuration>::try_into(base).is_err());
    }

    #[test]
    fn test_configuration_from_str_invalid_action_group_key() {
        let base = r#"[[actions]]
name = "test1"
command = "foobar x y z"

[[actions]]
name = "test2"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[action_groups]]
name = "test"
id = "foobar"
actions = [ "test1", "test2" ]
"#;

        assert!(TryInto::<Configuration>::try_into(base).is_err());
    }

    #[test]
    fn test_configuration_from_str_invalid_action_id() {
        let base = r#"[[actions]]
name = "test-1"
command = "foobar x y z"

[[actions]]
name = "test2"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[action_groups]]
name = "test"
actions = [ "test1", "test2" ]
"#;

        assert!(TryInto::<Configuration>::try_into(base).is_err());
    }

    #[test]
    fn test_configuration_from_str_invalid_glob() {
        let base = r#"[[actions]]
name = "test1"
command = "foobar x y z"

[[actions]]
name = "test2"
command = "foobar \"a b c\""
inputs.files = [ "**a", "**/Cargo.toml" ]

[[action_groups]]
name = "test"
actions = [ "test1", "test2" ]
"#;

        assert!(TryInto::<Configuration>::try_into(base).is_err());
    }

    #[test]
    fn test_configuration_merge() {
        let base = r#"[[actions]]
name = "test1"
command = "foobar x y z"

[[actions]]
name = "test2"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[actions]]
name = "test3_b"
command = "do something"
inputs.files = [ "**/*.slint", "**/*.rs" ]

[[action_groups]]
name = "test"
actions = [ "test1", "test2" ]
"#;

        let base: Configuration = base.try_into().unwrap();
        eprintln!("Base: {base:?}");

        let other = r#"[[actions]]
name = "test3_o"
command = "barfoo x y z"

[[actions]]
name = "test2"
command = "/dev/null"

[[actions]]
name = "test1"
command = "barfoo x y z"

[[action_groups]]
name = "test"
actions = [ "test1", "test3" ]
"#;
        let other: Configuration = other.try_into().unwrap();
        eprintln!("Other: {other:?}");

        let merge = base.merge(other).unwrap();

        assert_eq!(merge.actions.len(), 3);
        assert_eq!(merge.action_groups.len(), 1);
    }
}
