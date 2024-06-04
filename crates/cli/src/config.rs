// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::collections::HashSet;
use std::convert::TryFrom;
use std::{collections::HashMap, fmt::Display, path::Path};

use anyhow::Context;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize)]
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

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize)]
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

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize)]
#[serde(try_from = "String", expecting = "an action id")]
pub struct QualifiedActionId {
    source: Option<ActionSource>,
    id: ActionId,
}

impl QualifiedActionId {
    pub fn from_def(action_definition: &beautytips::ActionDefinition) -> Self {
        Self::new_from_source(
            ActionId::new_str(&action_definition.id).expect("This is a valid action id"),
            ActionSource::new_str(&action_definition.source)
                .expect("This is a valid action source"),
        )
    }

    pub fn new_from_source(id: ActionId, source: ActionSource) -> Self {
        Self {
            id,
            source: Some(source),
        }
    }

    pub fn new(id: ActionId) -> Self {
        Self { id, source: None }
    }

    /// Create a new `QualifiedActionId`
    ///
    /// # Errors
    ///
    /// Raise an invalid configuration error if the action id contains anything
    /// but lowercase ASCII letters or '_'.
    pub fn parse(input: String) -> anyhow::Result<Self> {
        if let Some(separator) = input.find('/') {
            let source = ActionSource::new_str(&input[..separator])?;
            let id = ActionId::new_str(&input[separator + 1..])?;
            Ok(Self::new_from_source(id, source))
        } else {
            let id = ActionId::new(input).context("Failed to parse qualified action id")?;
            Ok(Self::new(id))
        }
    }

    /// Create a new `QualifiedActionId`
    ///
    /// # Errors
    ///
    /// Raise an invalid configuration error if the action id contains anything
    /// but lowercase ASCII letters or '_'.
    pub fn parse_str(input: &str) -> crate::Result<Self> {
        Self::parse(input.to_string())
    }

    pub fn unqualified_id(&self) -> &ActionId {
        &self.id
    }
}

impl Display for QualifiedActionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = &self.source {
            write!(f, "{s}/{}", self.id)
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

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeAction {
    Hide,
    Change,
    #[default]
    Add,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TomlActionDefinition {
    pub name: ActionId,
    #[serde(default)]
    pub merge: MergeAction,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub inputs: Option<HashMap<String, Vec<String>>>,
}

type ActionGroups = HashMap<QualifiedActionId, Vec<QualifiedActionId>>;
type ActionMap = HashMap<QualifiedActionId, usize>;

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TomlActionGroup {
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
    pub action_map: ActionMap,
}

#[derive(Debug)]
pub struct ConfigurationSource {
    pub source: ActionSource,
    pub action_groups: Vec<TomlActionGroup>,
    pub actions: Vec<TomlActionDefinition>,
}

impl ConfigurationSource {
    fn from_string(value: &str, source: ActionSource) -> anyhow::Result<Self> {
        let mut toml_config: TomlConfiguration =
            toml::from_str(value).context("Failed to parse toml")?;

        let actions = std::mem::take(&mut toml_config.actions);

        {
            let mut known_names = HashSet::new();

            for d in &actions {
                if !known_names.insert(d.name.clone()) {
                    return Err(anyhow::anyhow!(format!(
                        "Duplicate action \'{}\' found",
                        d.name
                    )));
                }
            }
        }

        let action_groups = std::mem::take(&mut toml_config.action_groups);

        {
            let mut known_names = HashSet::new();

            for d in &action_groups {
                if !known_names.insert(d.name.clone()) {
                    return Err(anyhow::anyhow!(format!(
                        "Duplicate action group \'{}\' found",
                        d.name
                    )));
                }
            }
        }

        Ok(Self {
            source,
            action_groups,
            actions,
        })
    }

    fn from_path(path: &Path, source_name: ActionSource) -> anyhow::Result<Self> {
        let config_data =
            std::fs::read_to_string(path).context(format!("Failed to read toml file {path:?}"))?;

        Self::from_string(config_data.as_str(), source_name)
            .context("Failed to parse toml file {value:?}")
    }
}

fn hide_action(action: &TomlActionDefinition, action_map: &mut ActionMap) -> anyhow::Result<()> {
    let qid = QualifiedActionId::new(action.name.clone());
    if action.description.is_some()
        || action.command.is_some()
        || action.exit_code.is_some()
        || action.inputs.is_some()
    {
        return Err(anyhow::anyhow!(format!(
            "{qid} is hidding an existing action, but has extra keys set"
        )));
    }
    if action_map.insert(qid.clone(), usize::MAX).is_none() {
        return Err(anyhow::anyhow!(format!(
            "{qid} is hidding an action that does not exist"
        )));
    }

    Ok(())
}

fn change_action(
    update: &mut TomlActionDefinition,
    source: &ActionSource,
    actions: &mut Vec<beautytips::ActionDefinition>,
    action_map: &mut ActionMap,
) -> anyhow::Result<()> {
    let qid = QualifiedActionId::new(update.name.clone());
    let sqid = QualifiedActionId::new_from_source(update.name.clone(), source.clone());

    if update.description.is_none()
        && update.command.is_none()
        && update.exit_code.is_none()
        && update.inputs.is_none()
    {
        return Err(anyhow::anyhow!(format!(
            "{qid} is changing an existing action, but has no extra keys set"
        )));
    }
    let Some(index) = action_map.get(&qid) else {
        return Err(anyhow::anyhow!(format!(
            "{qid} is changing an action that does not exist"
        )));
    };
    if *index == usize::MAX {
        return Err(anyhow::anyhow!(format!(
            "{qid} is changing an action that was hidden before"
        )));
    }

    let mut ad = actions
        .get(*index)
        .expect("must exist, we got an index")
        .clone();
    assert_eq!(ad.id, update.name.to_string());

    if let Some(description) = std::mem::take(&mut update.description) {
        ad.description = description;
    }
    if let Some(command) = &update.command {
        ad.command = map_command(command)?;
    }
    if let Some(exit_code) = &update.exit_code {
        ad.expected_exit_code = *exit_code;
    }
    if let Some(inputs) = &update.inputs {
        let mut inputs = map_input_filters(inputs)?;
        for (k, v) in inputs.drain() {
            if v.is_empty() {
                if ad.input_filters.remove(&k).is_none() {
                    return Err(anyhow::anyhow!(format!(
                        "{k} does not exist when trying to remove it from inputs"
                    )))
                    .context(format!("While changing {qid}"));
                }
            } else {
                ad.input_filters.insert(k, v);
            }
        }
    }

    let index = actions.len();
    actions.push(ad);
    action_map.insert(qid.clone(), index);
    action_map.insert(sqid.clone(), index);

    Ok(())
}

fn add_action(
    update: &mut TomlActionDefinition,
    source: &ActionSource,
    actions: &mut Vec<beautytips::ActionDefinition>,
    action_map: &mut ActionMap,
) -> anyhow::Result<()> {
    let qid = QualifiedActionId::new(update.name.clone());
    let sqid = QualifiedActionId::new_from_source(update.name.clone(), source.clone());

    let Some(command) = &update.command else {
        return Err(anyhow::anyhow!(format!(
            "Can not add {}: No command",
            update.name
        )));
    };

    if let Some(index) = action_map.get(&qid) {
        if *index != usize::MAX {
            return Err(anyhow::anyhow!(format!(
                "{} already exists, can not add",
                update.name
            )));
        }
    };

    let description = std::mem::take(&mut update.description).unwrap_or_default();
    let command = map_command(command).context("Processing command of {qid}")?;
    let expected_exit_code = update.exit_code.unwrap_or(0);
    let input_filters = if let Some(inputs) = &update.inputs {
        map_input_filters(inputs)?
    } else {
        beautytips::InputFilters::default()
    };

    let ad = beautytips::ActionDefinition {
        id: update.name.to_string(),
        source: source.to_string(),
        description,
        command,
        expected_exit_code,
        input_filters,
    };

    let index = actions.len();
    actions.push(ad);
    action_map.insert(qid, index);
    action_map.insert(sqid, index);

    Ok(())
}

fn merge_actions(
    mut actions: Vec<beautytips::ActionDefinition>,
    mut action_map: ActionMap,
    other: &mut ConfigurationSource,
) -> anyhow::Result<(Vec<beautytips::ActionDefinition>, ActionMap)> {
    for mut action in other.actions.drain(..) {
        match action.merge {
            MergeAction::Hide => hide_action(&action, &mut action_map)?,
            MergeAction::Change => {
                change_action(&mut action, &other.source, &mut actions, &mut action_map)?;
            }
            MergeAction::Add => {
                add_action(&mut action, &other.source, &mut actions, &mut action_map)?;
            }
        }
    }
    Ok((actions, action_map))
}

fn add_new_action_groups(
    mut action_groups: ActionGroups,
    other: &ConfigurationSource,
) -> ActionGroups {
    for ag in &other.action_groups {
        let qid = QualifiedActionId::new(ag.name.clone());
        let sqid = QualifiedActionId::new_from_source(ag.name.clone(), other.source.clone());
        let ids = ag
            .actions
            .iter()
            .map(|id| {
                if id.source == Some(ActionSource::new_str("this").unwrap()) {
                    QualifiedActionId::new_from_source(id.id.clone(), other.source.clone())
                } else {
                    id.clone()
                }
            })
            .collect::<Vec<_>>();

        action_groups.insert(qid, ids.clone());
        action_groups.insert(sqid, ids);
    }

    action_groups
}

fn map_command(toml_command: &str) -> anyhow::Result<Vec<String>> {
    let mut command = shell_words::split(toml_command.trim())
        .context(format!("Failed to parse command '{toml_command}'"))?;

    if let Some(executable) = command.first() {
        if executable == "{BEAUTY_TIPS}" {
            command[0] = std::env::current_exe()
                .context("Failed to get beauty_tips binary location")?
                .to_string_lossy()
                .to_string();
        }
    }

    Ok(command)
}

fn map_input_filters(
    toml_filters: &HashMap<String, Vec<String>>,
) -> anyhow::Result<beautytips::InputFilters> {
    toml_filters
        .iter()
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
        .context("Parsing input filters for action '{id}'")
}

fn validate_state(action_groups: &ActionGroups, action_map: &ActionMap) -> anyhow::Result<()> {
    for (k, v) in action_groups {
        for i in v {
            if action_map.get(i).is_none() && action_groups.get(i).is_none() {
                return Err(anyhow::anyhow!(
                    "Action Group {k} has unknown dependency {i}"
                ));
            }
        }
    }

    Ok(())
}

impl Configuration {
    /// Merge `other` onto the base of `self`
    pub fn merge(mut self, mut other: ConfigurationSource) -> anyhow::Result<Self> {
        let (actions, action_map) = merge_actions(
            std::mem::take(&mut self.actions),
            std::mem::take(&mut self.action_map),
            &mut other,
        )?;

        let action_groups = add_new_action_groups(std::mem::take(&mut self.action_groups), &other);

        validate_state(&action_groups, &action_map)?;

        Ok(Self {
            action_groups,
            actions,
            action_map,
        })
    }

    fn add_actions(
        &self,
        action_name: &QualifiedActionId,
        result: &mut HashSet<usize>,
        visited: &mut HashSet<QualifiedActionId>,
    ) -> anyhow::Result<()> {
        if !visited.insert(action_name.clone()) {
            return Ok(());
        }

        if let Some(index) = self.action_map.get(action_name) {
            result.insert(*index);
        } else if let Some(group_ids) = self.action_groups.get(action_name) {
            for g in group_ids {
                self.add_actions(g, result, visited)?;
            }
        } else if let Some(prefix) = action_name.id.to_string().strip_suffix("_all") {
            let prefix = format!("{prefix}_");
            eprintln!("Looking for actions starting with \"{prefix}\"");
            for c in self.actions.iter().filter_map(|ad| {
                let qid = QualifiedActionId::from_def(ad);
                eprintln!("Looking at {}...", qid.to_string());
                qid.unqualified_id()
                    .to_string()
                    .starts_with(&prefix)
                    .then_some(qid)
            }) {
                eprintln!("   To run: {c:?}");
                self.add_actions(&c, result, visited)?;
            }
        } else {
            return Err(anyhow::anyhow!(format!("Unknown action {action_name}")));
        }

        Ok(())
    }

    pub fn actions<'a>(
        &'a self,
        action_names: &[QualifiedActionId],
    ) -> anyhow::Result<beautytips::ActionDefinitionIterator<'a>> {
        let mut indices = HashSet::new();
        let mut visited = HashSet::new();

        for action_name in action_names {
            self.add_actions(action_name, &mut indices, &mut visited)?;
        }

        Ok(beautytips::ActionDefinitionIterator::new(
            &self.actions,
            indices,
        ))
    }
}

macro_rules! import_rules {
    ( $( $file: tt ),* ) => {{
        {
            let config = Configuration::default();
            $(
                let config = config.merge(
                    ConfigurationSource::from_string(
                        include_str!(std::concat!($file, ".rules")),
                        ActionSource::new_str($file).expect(std::concat!($file, " is a valid action id"))
                    ).expect(std::concat!($file, " should parse fine"))
                )
                .expect(std::concat!($file, " merge ok"));
            )*
            config
        }
    }};
}

pub fn builtin() -> Configuration {
    import_rules!("builtin", "rust")
}

pub fn load_user_configuration() -> anyhow::Result<Configuration> {
    let base = builtin();

    let config_dir = dirs::config_dir()
        .map(|cd| cd.join("beautytips"))
        .ok_or(anyhow::anyhow!("Config directory not found"))?;
    let config_file = config_dir.join("config.toml");

    let user = ConfigurationSource::from_path(
        config_file.as_path(),
        ActionSource::new_str("user").unwrap(),
    )
    .context("Failed to parse configuration file {config_file:?}")?;
    base.merge(user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_from_str_ok() {
        let base = r#"[[actions]]
name = "test_1"
description = "foo"
command = "foobar x y z"

[[actions]]
name = "test_2"
description = "foo"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[action_groups]]
name = "test"
actions = [ "test_1", "test_2" ]
"#;

        let base =
            ConfigurationSource::from_string(base, ActionSource::new_str("test").unwrap()).unwrap();
        let base = Configuration::default().merge(base).unwrap();

        assert_eq!(base.actions.len(), 2);
        assert_eq!(
            base.actions(&[QualifiedActionId::new(ActionId::new_str("test_1").unwrap())])
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            base.actions(&[QualifiedActionId::new(ActionId::new_str("test_2").unwrap())])
                .unwrap()
                .count(),
            1
        );
        assert!(base
            .actions(&[QualifiedActionId::new(ActionId::new_str("test_3").unwrap())])
            .is_err());
        assert_eq!(base.action_groups.len(), 2);
        assert_eq!(
            base.actions(&[QualifiedActionId::new(ActionId::new_str("test").unwrap())])
                .unwrap()
                .count(),
            2
        );
        assert_eq!(
            base.actions(&[QualifiedActionId::new(
                ActionId::new_str("test_all").unwrap()
            )])
            .unwrap()
            .count(),
            2
        );
    }

    #[test]
    fn test_configuration_from_str_empty_ok() {
        let base = "";

        let base =
            ConfigurationSource::from_string(base, ActionSource::new_str("test").unwrap()).unwrap();
        let base = Configuration::default().merge(base).unwrap();

        assert_eq!(base.actions.len(), 0);
        assert_eq!(base.action_groups.len(), 0);
    }

    #[test]
    fn test_configuration_from_str_invalid_top_level_key() {
        let base = r#"[[action]]
name = "test_1"
command = "foobar x y z"
"#;

        assert!(
            ConfigurationSource::from_string(base, ActionSource::new_str("test").unwrap()).is_err()
        );
    }

    #[test]
    fn test_configuration_from_str_invalid_action_key() {
        let base = r#"[[actions]]
name = "test_1"
command = "foobar x y z"

[[actions]]
name = "test_2"
description = "foo"
id = "foobar"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[action_groups]]
name = "test"
actions = [ "test1", "test2" ]
"#;

        assert!(
            ConfigurationSource::from_string(base, ActionSource::new_str("test").unwrap()).is_err()
        );
    }

    #[test]
    fn test_configuration_from_str_invalid_action_group_key() {
        let base = r#"[[actions]]
name = "test_1"
command = "foobar x y z"

[[actions]]
name = "test_2"
description = "foo"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[action_groups]]
name = "test"
id = "foobar"
actions = [ "test1", "test2" ]
"#;

        assert!(
            ConfigurationSource::from_string(base, ActionSource::new_str("test").unwrap()).is_err()
        );
    }

    #[test]
    fn test_configuration_from_str_invalid_action_id() {
        let base = r#"[[actions]]
name = "test-1"
command = "foobar x y z"

[[actions]]
name = "test_2"
description = "foo"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[action_groups]]
name = "test"
actions = [ "test1", "test2" ]
"#;

        assert!(
            ConfigurationSource::from_string(base, ActionSource::new_str("test").unwrap()).is_err()
        );
    }

    #[test]
    fn test_configuration_from_str_invalid_glob() {
        let base = r#"[[actions]]
name = "test_1"
command = "foobar x y z"

[[actions]]
name = "test_2"
description = "foo"
command = "foobar \"a b c\""
inputs.files = [ "**a", "**/Cargo.toml" ]

[[action_groups]]
name = "test"
actions = [ "test1", "test2" ]
"#;

        let base =
            ConfigurationSource::from_string(base, ActionSource::new_str("test").unwrap()).unwrap();
        assert!(Configuration::default().merge(base).is_err());
    }

    #[test]
    fn test_configuration_merge_empty() {
        let base = r#"[[actions]]
description = "foo"
name = "test_1"
command = "foobar x y z"

[[actions]]
name = "test_2"
description = "foo"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[actions]]
name = "test_3b"
description = "foo"
command = "do something"
inputs.files = [ "**/*.slint", "**/*.rs" ]

[[action_groups]]
name = "test"
actions = [ "test_1", "test_2" ]
"#;

        let base =
            ConfigurationSource::from_string(base, ActionSource::new_str("test").unwrap()).unwrap();
        let merge = Configuration::default().merge(base).unwrap();

        assert_eq!(merge.actions.len(), 3);
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(ActionId::new_str("test_1").unwrap())])
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(
                    ActionId::new_str("test_3b").unwrap()
                )])
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(ActionId::new_str("test_2").unwrap())])
                .unwrap()
                .count(),
            1
        );
        assert_eq!(merge.action_groups.len(), 2);
        // let it = merge
        //     .named_actions(&[QualifiedActionId::new(ActionId::new_str("test").unwrap())])
        //     .unwrap();
        // assert_eq!(it.count(), 2);
    }

    #[test]
    fn test_configuration_merge() {
        let base = r#"[[actions]]
name = "test_1"
description = "foo"
command = "foobar x y z"

[[actions]]
name = "test_2"
description = "foo"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[actions]]
name = "test_3b"
description = "foo"
command = "do something"
inputs.files = [ "**/*.slint", "**/*.rs" ]

[[action_groups]]
name = "test"
actions = [ "test_1", "test_2" ]
"#;

        let base =
            ConfigurationSource::from_string(base, ActionSource::new_str("test").unwrap()).unwrap();
        let base = Configuration::default().merge(base).unwrap();

        let other = r#"[[actions]]
name = "test_3o"
description = "foo"
command = "barfoo x y z"

[[actions]]
name = "test_2"
merge = "change"
command = "/dev/null"

[[actions]]
name = "test_1"
merge = "change"
command = "barfoo x y z"

[[action_groups]]
name = "test"
actions = [ "test_1", "test_3o", "test_3b" ]

[[action_groups]]
name = "test_group"
actions = [ "test_3b" ]
"#;
        let other =
            ConfigurationSource::from_string(other, ActionSource::new_str("test2").unwrap())
                .unwrap();

        let merge = base.merge(other).unwrap();

        assert_eq!(merge.actions.len(), 6);
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(ActionId::new_str("test_1").unwrap())])
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(
                    ActionId::new_str("test_3b").unwrap()
                )])
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(
                    ActionId::new_str("test_3o").unwrap()
                )])
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(
                    ActionId::new_str("test_group").unwrap()
                )])
                .unwrap()
                .count(),
            1
        );
    }

    #[test]
    fn test_builtins() {
        let builtin = builtin();

        assert!(!builtin.actions.is_empty());
        assert!(builtin.action_groups.is_empty());
    }
}
