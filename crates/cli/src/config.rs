// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::collections::HashSet;
use std::convert::TryFrom;
use std::str::FromStr;
use std::{collections::HashMap, fmt::Display, path::Path};

use anyhow::Context;

const UNKNOWN_ACTION_OFFSET: usize = usize::MAX / 2;

fn map_id_to_index(id: &str, action_map: &ActionMap, unknown_actions: &mut Vec<String>) -> usize {
    let action_index = action_map.get(id);
    if let Some(ai) = action_index {
        *ai
    } else {
        let unknown_pos = unknown_actions.iter().position(|s| s == id);
        if let Some(up) = unknown_pos {
            up + UNKNOWN_ACTION_OFFSET
        } else {
            let up = unknown_actions.len();
            unknown_actions.push(id.to_string());
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

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(try_from = "String", expecting = "an action id")]
pub struct ActionId(String);

impl ActionId {
    /// Create a new `ActionId`
    ///
    /// # Errors
    ///
    /// Raise an invalid configuration error if the action id contains anything
    /// but lowercase ASCII letters or '_'.
    pub fn new(input: String) -> anyhow::Result<Self> {
        if input
            .chars()
            .any(|c| !c.is_ascii_lowercase() && c != '_' && !c.is_ascii_digit())
            && !input.is_empty()
        {
            Err(anyhow::anyhow!("{input} is not a valid action id"))
        } else {
            Ok(Self(input))
        }
    }

    /// Create a new `ActionId`
    ///
    /// # Errors
    ///
    /// Raise an invalid configuration error if the action id contains anything
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
        Self::new_str(value)
    }
}

impl TryFrom<String> for ActionId {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl std::str::FromStr for ActionId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new_str(s)
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(try_from = "String", expecting = "an action id")]
pub struct ActionSource(String);

impl ActionSource {
    /// Create a new `ActionSource`
    ///
    /// # Errors
    ///
    /// Raise an invalid configuration error if the action id contains anything
    /// but lowercase ASCII letters or '_'.
    pub fn new(input: String) -> anyhow::Result<Self> {
        if input
            .chars()
            .any(|c| !c.is_ascii_lowercase() && !c.is_ascii_digit())
            && !input.is_empty()
        {
            Err(anyhow::anyhow!("{input} is not a valid action source"))
        } else {
            Ok(Self(input))
        }
    }

    /// Create a new `ActionSource`
    ///
    /// # Errors
    ///
    /// Raise an invalid configuration error if the action id contains anything
    /// but lowercase ASCII letters or '_'.
    pub fn new_str(input: &str) -> crate::Result<Self> {
        Self::new(input.to_string())
    }
}

impl Display for ActionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for ActionSource {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new_str(value)
    }
}

impl TryFrom<String> for ActionSource {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl std::str::FromStr for ActionSource {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new_str(s)
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(try_from = "String", expecting = "an action id")]
pub struct QualifiedActionId {
    id: ActionId,
    source: Option<ActionSource>,
    priority: Option<u32>,
}

impl QualifiedActionId {
    pub fn new_prioritized(id: ActionId, priority: u32) -> Self {
        Self {
            id,
            source: None,
            priority: Some(priority),
        }
    }

    pub fn new_from_source(id: ActionId, source: ActionSource) -> Self {
        Self {
            id,
            source: Some(source),
            priority: None,
        }
    }

    pub fn new(id: ActionId) -> Self {
        Self {
            id,
            source: None,
            priority: None,
        }
    }

    /// Create a new `QualifiedActionId`
    ///
    /// # Errors
    ///
    /// Raise an invalid configuration error if the action id contains anything
    /// but lowercase ASCII letters or '_'.
    pub fn parse(input: String) -> anyhow::Result<Self> {
        if let Some(separator) = input.find(&['/', '@']) {
            if input.as_bytes()[separator] == b'/' {
                let source = ActionSource::new_str(&input[..separator])?;
                let id = ActionId::new_str(&input[separator + 1..])?;
                Ok(Self::new_from_source(id, source))
            } else {
                let id = ActionId::new_str(&input[..separator])?;
                let priority = u32::from_str(&input[separator + 1..])
                    .map_err(|_| anyhow::anyhow!("Could not parse priority"))
                    .context("Failed to parse qualified action id {input}")?;
                Ok(Self::new_prioritized(id, priority))
            }
        } else {
            let id = ActionId::new(input).context("Failed to parse qualified action id")?;
            Ok(Self::new(id))
        }
    }

    /// Create a new `ActionId`
    ///
    /// # Errors
    ///
    /// Raise an invalid configuration error if the action id contains anything
    /// but lowercase ASCII letters or '_'.
    pub fn parse_str(input: &str) -> crate::Result<Self> {
        Self::parse(input.to_string())
    }
}

impl Display for QualifiedActionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = &self.source {
            write!(f, "{s}/{}", self.id)
        } else if let Some(p) = self.priority {
            write!(f, "{}@{p}", self.id)
        } else {
            write!(f, "{}", self.id)
        }
    }
}

impl TryFrom<&str> for QualifiedActionId {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse_str(value)
    }
}

impl TryFrom<String> for QualifiedActionId {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl std::str::FromStr for QualifiedActionId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct TomlActionDefinition {
    pub name: ActionId,
    pub description: String,
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
    pub actions: Vec<QualifiedActionId>,
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
    pub action_groups: ActionGroups,
    pub actions: Vec<beautytips::ActionDefinition>,
    unknown_actions: Vec<String>,
    pub action_map: ActionMap,
}

fn merge_action_definition(
    base: &beautytips::ActionDefinition,
    other: &beautytips::ActionDefinition,
) -> anyhow::Result<beautytips::ActionDefinition> {
    // TODO: Actually merge ;-)
    Ok(other.clone())
}

fn merge_actions(
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
        .map(|ad| match ad {
            ActionState::Add(d) => Ok(d),
            ActionState::Change(sd, od) => merge_action_definition(&sd, &od),
            ActionState::Remove(d) => {
                if d.expected_exit_code != 0 || !d.input_filters.is_empty() {
                    Err(anyhow::anyhow!(format!(
                        "Removal of '{}' failed: Too many fields",
                        d.id
                    )))
                } else {
                    Ok(d)
                }
            }
        })
        .collect::<anyhow::Result<_>>()
}

fn merge_action_groups(
    this: &Configuration,
    other: &Configuration,
    action_map: &ActionMap,
) -> anyhow::Result<ActionGroups> {
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
                        (index < UNKNOWN_ACTION_OFFSET).then_some(index)
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
            ActionGroups::new(),
            |mut acc, (k, v)| -> anyhow::Result<ActionGroups> {
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

fn map_toml_action(
    ad: TomlActionDefinition,
    source_name: &str,
    priority: u8,
) -> anyhow::Result<beautytips::ActionDefinition> {
    let id = ad.name.to_string();

    let command = {
        let mut command = shell_words::split(ad.command.trim()).context(format!(
            "Failed to parse command '{}' of action '{id}'",
            ad.command
        ))?;

        if let Some(executable) = command.first() {
            if executable == "{BEAUTY_TIPS}" {
                command[0] = std::env::current_exe()
                    .context("Failed to get beauty_tips binary location")?
                    .to_string_lossy()
                    .to_string();
            }
        }

        command
    };

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

    Ok(beautytips::ActionDefinition {
        id: id.clone(),
        source: source_name.to_string(),
        priority,
        description: ad.description.clone(),
        command,
        expected_exit_code: ad.exit_code,
        input_filters,
    })
}

fn populate_action_map(actions: &[beautytips::ActionDefinition]) -> ActionMap {
    let mut map: ActionMap = actions
        .iter()
        .enumerate()
        .filter_map(|(index, d)| {
            (d.command.len() != 1
                || d.command.first().map(std::string::String::as_str) != Some("/dev/null"))
            .then_some((d.id.clone(), index))
        })
        .collect();
    map.extend(
        actions
            .iter()
            .enumerate()
            .map(|(index, d)| (format!("{}/{}", d.source, d.id), index)),
    );
    map.extend(
        actions
            .iter()
            .enumerate()
            .map(|(index, d)| (format!("{}@{}", d.id, d.priority), index)),
    );

    eprintln!("*** Action Map:");
    for k in map.keys() {
        eprintln!("     {k}");
    }
    map
}

fn group_action_id(id: &str) -> Option<String> {
    let mut prefix = String::new();
    let mut main_part = String::new();
    let mut main_start = String::new();
    let mut priority = String::new();

    let mut current = String::new();
    let mut candidate = String::new();

    for c in id.chars() {
        match c {
            '/' => {
                assert!(!current.is_empty());
                assert!(prefix.is_empty());

                prefix = current;
                current = String::new();
                candidate = String::new();
            }
            '_' => {
                assert!(!current.is_empty());

                if candidate.is_empty() {
                    candidate.clone_from(&current);
                }
            }
            '@' => {
                assert!(!current.is_empty());
                assert!(main_part.is_empty());
                assert!(main_start.is_empty());

                main_part = current;
                main_start = candidate;
                current = String::new();
                candidate = String::new();
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        if main_part.is_empty() {
            main_part = current;
            main_start = candidate;
        } else {
            priority = current;
        }
    }

    assert!(!main_part.is_empty());

    if main_start == main_part {
        return None;
    }

    match (prefix.is_empty(), priority.is_empty()) {
        (false, false) => Some(format!("{prefix}/{main_start}_all@{priority}")), // should not happen...
        (true, false) => Some(format!("{main_start}_all@{priority}")),
        (false, true) => Some(format!("{prefix}/{main_start}_all")),
        (true, true) => Some(format!("{main_start}_all")),
    }
}

fn add_auto_groups(action_groups: &mut ActionGroups, action_map: &ActionMap) {
    for (k, v) in action_map
        .iter()
        .filter_map(|(k, v)| group_action_id(k).map(|id| (id, *v)))
    {
        action_groups.entry(k).or_default().push(v);
    }
}

impl Configuration {
    /// Merge `other` onto the base of `self`
    pub fn merge(self, other: Self) -> anyhow::Result<Self> {
        assert!(self.unknown_actions.is_empty());

        let mut actions: Vec<beautytips::ActionDefinition> = merge_actions(&self, &other)?;

        actions.sort();
        let action_map: ActionMap = populate_action_map(&actions);
        let action_groups = {
            let mut ags = merge_action_groups(&self, &other, &action_map)?;

            add_auto_groups(&mut ags, &action_map);

            ags
        };

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

    pub fn named_actions<'a>(
        &'a self,
        action_names: &[String],
    ) -> anyhow::Result<beautytips::ActionDefinitionIterator<'a>> {
        let mut indices = HashSet::new();
        for action_name in action_names {
            if let Some(index) = self.action_map.get(action_name) {
                indices.insert(*index);
            } else if let Some(group_indices) = self.action_groups.get(action_name) {
                indices.extend(group_indices.iter());
            } else {
                return Err(anyhow::anyhow!(format!("Unknown action {action_name}")));
            }
        }

        Ok(beautytips::ActionDefinitionIterator::new(
            &self.actions,
            indices,
        ))
    }

    fn from_string(value: &str, source_name: &str) -> anyhow::Result<Configuration> {
        assert!(!source_name.is_empty());
        assert!(source_name.chars().all(|c| c.is_ascii_alphabetic()));

        static PRIORITY: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

        let priority = PRIORITY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let toml_config: TomlConfiguration =
            toml::from_str(value).context("Failed to parse toml")?;

        let mut actions: Vec<_> = toml_config
            .actions
            .into_iter()
            .map(|ad| map_toml_action(ad, source_name, priority))
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
        let action_map = populate_action_map(&actions);
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

                let v: Vec<usize> = v
                    .iter()
                    .map(|id| map_id_to_index(id, &action_map, &mut unknown_actions))
                    .collect();

                let old = acc.insert(v_id.to_string(), v.clone());
                if old.is_some() {
                    return Err(anyhow::anyhow!(format!(
                        "Action group '{v_id}' defined twice in one config location"
                    )));
                }
                let old = acc.insert(format!("{source_name}/{v_id}"), v.clone());
                if old.is_some() {
                    return Err(anyhow::anyhow!(format!(
                        "Action group '{source_name}/{v_id}' defined twice in one config location"
                    )));
                }
                let old = acc.insert(format!("{v_id}@{priority}"), v);
                if old.is_some() {
                    return Err(anyhow::anyhow!(format!(
                        "Action group '{v_id}@{priority}' defined twice in one config location"
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

    fn from_path(path: &Path, source_name: &str) -> anyhow::Result<Self> {
        let config_data =
            std::fs::read_to_string(path).context(format!("Failed to read toml file {path:?}"))?;

        Configuration::from_string(config_data.as_str(), source_name)
            .context("Failed to parse toml file {value:?}")
    }
}

pub fn builtin() -> Configuration {
    let toml = include_str!("rules.toml");
    let config = Configuration::from_string(toml, "builtin").expect("builtins should parse fine");

    let base = Configuration::default();
    base.merge(config).expect("builtins should merge just fine")
}

pub fn load_user_configuration() -> anyhow::Result<Configuration> {
    let base = builtin();

    let config_dir = dirs::config_dir()
        .map(|cd| cd.join("beautytips"))
        .ok_or(anyhow::anyhow!("Config directory not found"))?;
    let config_file = config_dir.join("config.toml");

    let user = Configuration::from_path(config_file.as_path(), "user")
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

        assert_eq!(base.action_count(), 2);
        assert!(base.action("test1").is_some());
        assert!(base.action("test2").is_some());
        assert_eq!(base.action_group_count(), 1);
    }

    #[test]
    fn test_configuration_from_str_empty_ok() {
        let base = "";

        let base: Configuration = base.try_into().unwrap();

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
    fn test_configuration_merge_empty() {
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

        let other: Configuration = Configuration::default();

        let merge = base.merge(other).unwrap();

        assert_eq!(merge.action_count(), 3);
        assert!(merge.action("test1").is_some());
        assert!(merge.action("test3_b").is_some());
        assert!(merge.action("test2").is_some());
        assert_eq!(merge.action_group_count(), 1);
        let it = merge.action_group("test").unwrap();
        assert_eq!(it.count(), 2);
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

[[action_groups]]
name = "test_group"
actions = [ "test3_b" ]
"#;
        let other: Configuration = other.try_into().unwrap();

        let merge = base.merge(other).unwrap();

        assert_eq!(merge.action_count(), 3);
        assert!(merge.action("test1").is_some());
        assert!(merge.action("test3_b").is_some());
        assert!(merge.action("test3_o").is_some());
        assert_eq!(merge.action_group_count(), 2);
        let mut it = merge.action_group("test_group").unwrap();
        assert_eq!(it.next().unwrap().id.as_str(), "test3_b");
        assert!(it.next().is_none());
    }

    #[test]
    fn test_builtins() {
        let builtin = builtin();

        assert!(builtin.action_count() > 0);
        assert!(builtin.action_group_count() > 0);
        let it = builtin.action_group("test_me").unwrap();
        assert!(it.count() > 1);
    }
}
