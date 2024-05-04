// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::convert::TryFrom;
use std::{collections::HashMap, fmt::Display, path::Path};

use anyhow::Context;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(try_from = "String", expecting = "an action id")]
pub struct ActionId(String);

const UNKNOWN_ACTION_OFFSET: usize = usize::MAX / 2;

fn map_id_to_index(
    id: &str,
    action_map: &ActionMap,
    unknown_actions: &mut Vec<String>,
) -> usize {
    eprintln!("map id {id} to index using {action_map:?} and {unknown_actions:?}.");
    let action_index = action_map.get(id);
    if let Some(ai) = action_index {
        eprintln!("    ==> Have action at id {ai}");
        *ai
    } else {
        let unknown_pos = unknown_actions.iter().position(|s| s == id);
        if let Some(up) = unknown_pos {
        eprintln!("    ==> Action is already unknown at index {}", up + UNKNOWN_ACTION_OFFSET);
            up + UNKNOWN_ACTION_OFFSET
        } else {
            let up = unknown_actions.len();
            unknown_actions.push(id.to_string());
        eprintln!("    ==> Action is newly unknown at index {}", up + UNKNOWN_ACTION_OFFSET);
            up + UNKNOWN_ACTION_OFFSET
        }
    }
}

fn map_index_to_id(
    index: usize,
    actions: &[beautytips::ActionDefinition],
    unknown_actions: &[String],
) -> String {
    if index < UNKNOWN_ACTION_OFFSET {
        actions
            .get(index)
            .expect("This index had to be valid")
            .id
            .clone()
    } else {
        unknown_actions
            .get(index - UNKNOWN_ACTION_OFFSET)
            .expect("This index had to be valid")
            .clone()
    }
}

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
    pub exit_code: i32,
    #[serde(default)]
    pub inputs: HashMap<String, Vec<String>>,
}

type ActionGroups = HashMap<String, Vec<usize>>;
type ActionMap = HashMap<String, usize>;

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
    action_groups: ActionGroups,
    actions: Vec<beautytips::ActionDefinition>,
    unknown_actions: Vec<String>,
    action_map: ActionMap,
}

fn merge_action_definition(
    _base: &beautytips::ActionDefinition,
    other: &beautytips::ActionDefinition,
) -> anyhow::Result<beautytips::ActionDefinition> {
    // TODO: Actually merge ;-)
    Ok(other.clone())
}

pub fn merge_actions(
    this: &Configuration,
    other: &Configuration,
) -> anyhow::Result<Vec<beautytips::ActionDefinition>> {
    #[derive(Debug)]
    enum ActionState<T> {
        Add(T),
        Change(T, T),
        Remove(T),
    }

    other
        .actions
        .iter()
        .map(|definition| {
            if let Some(sa) = this.action(&definition.id) {
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
        .chain(this.actions.iter().filter_map(|definition| {
            if other.action(&definition.id).is_some() {
                None
            } else {
                Some(ActionState::Add(definition.clone()))
            }
        }))
        .filter_map(|ad| match ad {
            ActionState::Add(d) => Some(Ok(d)),
            ActionState::Change(sd, od) => Some(merge_action_definition(&sd, &od)),
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
        .collect::<anyhow::Result<_>>()
}

pub fn merge_action_groups(
    this: &Configuration,
    other: &Configuration,
    action_map: &ActionMap,
) -> anyhow::Result<ActionGroups> {
    eprintln!("merging action groups: {action_map:?}");
    this.action_groups
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                v.iter()
                    .map(|index| map_index_to_id(*index, &this.actions, &this.unknown_actions))
                    .collect::<Vec<_>>(),
            )
        })
        .map(|(k, v)| {
            (
                k,
                Ok(v.into_iter()
                    .filter_map(|id| {
                        let index = map_id_to_index(&id, action_map, &mut vec![]);
                        // ignore unknwon actions here: They were removed by the merged config, which is fine
                        (index >= UNKNOWN_ACTION_OFFSET).then_some(index)
                    })
                    .collect::<Vec<_>>()),
            )
        })
        .chain(
            other
                .action_groups
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        v.iter()
                            .map(|index| {
                                map_index_to_id(*index, &other.actions, &other.unknown_actions)
                            })
                            .collect::<Vec<_>>(),
                    )
                })
                .map(|(k, v)| {
                    (
                        k.clone(),
                        v.into_iter()
                            .filter_map(|id| {
                                let index = map_id_to_index(&id, action_map, &mut vec![]);
                                // ignore unknwon actions here: They were removed by the merged config, which is fine
                                (index < UNKNOWN_ACTION_OFFSET)
                                    .then_some(Ok(index))
                                    .or(Some(Err(anyhow::anyhow!(format!(
                                        "Unknown action '{id}' in action group '{k}'"
                                    )))))
                            })
                            .collect::<anyhow::Result<Vec<_>>>(),
                    )
                }),
        )
        .try_fold(
            HashMap::new(),
            |mut acc, (k, v)| -> anyhow::Result<HashMap<_, _>> {
                let v = v?;
                if v.is_empty() {
                    if acc.remove_entry(&k).is_none() {
                        acc.insert(k, vec![]); // base used to define something and is empty now... Keep this for other to extend.
                    }
                } else {
                    let entry = acc.entry(k);
                    entry
                        .and_modify(|ov| {
                            ov.extend(v.iter());
                            ov.sort_unstable();
                            ov.dedup();
                        })
                        .or_insert(v);
                }
                Ok(acc)
            },
        )
}

impl Configuration {
    /// Merge `other` onto the base of `self`
    pub fn merge(self, other: Self) -> anyhow::Result<Self> {
        eprintln!("MERGING: {self:?}\n <== \n{other:?}\n\n");
        assert!(self.unknown_actions.is_empty());

        let mut actions: Vec<beautytips::ActionDefinition> = merge_actions(&self, &other)?;

        actions.sort();
        let action_map: HashMap<_, _> = actions
            .iter()
            .enumerate()
            .map(|(index, d)| (d.id.clone(), index))
            .collect();

        let action_groups = merge_action_groups(&self, &other, &action_map)?;

        drop(other); // consume other!

        Ok(Self {
            action_groups,
            unknown_actions: Vec::new(),
            actions,
            action_map,
        })
    }

    pub fn action<'a>(&'a self, name: &str) -> Option<&'a beautytips::ActionDefinition> {
        self.action_map
            .get(name)
            .and_then(|index| self.actions.get(*index))
    }

    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    pub fn action_group_count(&self) -> usize {
        self.action_groups.len()
    }
}

impl TryFrom<&str> for Configuration {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let toml_config: TomlConfiguration =
            toml::from_str(value).context("Failed to parse toml")?;

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

                Ok(beautytips::ActionDefinition {
                    id: id.clone(),
                    command,
                    expected_exit_code: ad.exit_code,
                    input_filters,
                })
            })
            .collect::<anyhow::Result<_>>()?;

        actions.sort();

        {
            let mut current = None;

            for d in &actions {
                if Some(&d.id) == current {
                    return Err(anyhow::anyhow!(format!(
                        "Duplicate action \'{}\' found",
                        d.id
                    )));
                }
                current = Some(&d.id);
            }
        }
        let action_map: HashMap<_, _> = actions
            .iter()
            .enumerate()
            .map(|(index, d)| (d.id.clone(), index))
            .collect();

        let mut unknown_actions = Vec::new();

        let action_groups = toml_config
            .action_groups
            .into_iter()
            .map(|ag| (ag.name, ag.actions))
            .try_fold(HashMap::new(), |mut acc, (v_id, v_val)| {
                let mut v = v_val
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>();
                v.sort();
                v.dedup();

                if v.len() != v_val.len() {
                    return Err(anyhow::anyhow!(
                        "Action group '{v_id}' has duplicate actions"
                    ));
                }

                let v = v
                    .iter()
                    .map(|id| map_id_to_index(id, &action_map, &mut unknown_actions))
                    .collect();

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
            unknown_actions,
            action_map,
        })
    }
}

impl TryFrom<&Path> for Configuration {
    type Error = anyhow::Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        let config_data = std::fs::read_to_string(value)
            .context(format!("Failed to read toml file {value:?}"))?;

        Configuration::try_from(config_data.as_str()).context("Failed to parse toml file {value:?}")
    }
}

struct ActionDefinitionIterator<'a> {
    actions: &'a [beautytips::ActionDefinition],
    filter: &'a dyn Fn(&'a beautytips::ActionDefinition) -> bool,
    current_item: usize,
}

impl<'a> Iterator for ActionDefinitionIterator<'a> {
    type Item = &'a beautytips::ActionDefinition;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_item < self.actions.len() {
            let cur = self.current_item;
            self.current_item += 1;

            let cur_item = unsafe { self.actions.get_unchecked(cur) };
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

    let user = Configuration::try_from(config_file.as_path())
        .context("Failed to parse configuration file {config_file:?}")?;
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
actions = [ "test1", "test3_o", "test3_b" ]
"#;
        let other: Configuration = other.try_into().unwrap();
        eprintln!("Other: {other:?}");

        let merge = base.merge(other).unwrap();

        assert_eq!(merge.action_count(), 3);
        assert!(merge.action("test1").is_some());
        assert!(merge.action("test3_b").is_some());
        assert!(merge.action("test3_o").is_some());
        assert_eq!(merge.action_group_count(), 1);
    }
}
