// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::collections::HashSet;
use std::convert::TryFrom;
use std::{collections::HashMap, fmt::Display, path::Path};

use anyhow::Context;

use beautytips::InputFilters;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize)]
#[serde(try_from = "String", expecting = "an action id")]
pub struct ActionId(String);

fn is_valid_id(id: &str) -> bool {
    !(id.chars()
        .any(|c| !c.is_ascii_lowercase() && c != '_' && !c.is_ascii_digit()))
        && !id.is_empty()
}

impl ActionId {
    /// Create a new `ActionId`
    ///
    /// # Errors
    ///
    /// Raise an invalid configuration error if the action id contains anything
    /// but lowercase ASCII letters or '_'.
    pub fn new(input: String) -> anyhow::Result<Self> {
        if is_valid_id(&input) {
            Ok(Self(input))
        } else {
            Err(anyhow::anyhow!("{input} is not a valid action id"))
        }
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
        Self::new(value.to_string())
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
        Self::new(s.to_string())
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
        if is_valid_id(&input) {
            Ok(Self(input))
        } else {
            Err(anyhow::anyhow!("{input} is not a valid action source"))
        }
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
        Self::new(value.to_string())
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
        Self::new(s.to_string())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize)]
#[serde(try_from = "String", expecting = "an action id")]
pub struct QualifiedActionId {
    source: Option<ActionSource>,
    id: ActionId,
}

impl QualifiedActionId {
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
            let source = input[..separator].try_into()?;
            let id = input[separator + 1..].try_into()?;
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

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputCondition {
    Never,
    Success,
    #[default]
    Failure,
    Always,
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
    pub run_sequentially: Option<bool>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub show_output: Option<OutputCondition>,
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

        Self::from_string(config_data.as_str(), source_name).context("Failed to parse toml string")
    }
}

fn hide_action(action: &TomlActionDefinition, action_map: &mut ActionMap) -> anyhow::Result<()> {
    let qid = QualifiedActionId::new(action.name.clone());
    if action.description.is_some()
        || action.show_output.is_some()
        || action.run_sequentially.is_some()
        || action.command.is_some()
        || action.exit_code.is_some()
        || action.inputs.is_some()
    {
        return Err(anyhow::anyhow!(format!(
            "{qid} is hiding an existing action, but has extra keys set"
        )));
    }
    if action_map.insert(qid.clone(), usize::MAX).is_none() {
        return Err(anyhow::anyhow!(format!(
            "{qid} is hiding an action that does not exist"
        )));
    }

    Ok(())
}

fn match_output_condition(output: &OutputCondition) -> beautytips::OutputCondition {
    match output {
        OutputCondition::Never => beautytips::OutputCondition::Never,
        OutputCondition::Success => beautytips::OutputCondition::Success,
        OutputCondition::Failure => beautytips::OutputCondition::Failure,
        OutputCondition::Always => beautytips::OutputCondition::Always,
    }
}

fn change_action(
    update: &mut TomlActionDefinition,
    source: &ActionSource,
    actions: &mut Vec<beautytips::ActionDefinition>,
    action_map: &mut ActionMap,
) -> anyhow::Result<()> {
    let qid = QualifiedActionId::new(update.name.clone());
    let s_qid = QualifiedActionId::new_from_source(update.name.clone(), source.clone());

    if update.description.is_none()
        && update.show_output.is_none()
        && update.run_sequentially.is_none()
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
    if let Some(show_output) = std::mem::take(&mut update.show_output) {
        ad.show_output = match_output_condition(&show_output);
    }
    if let Some(run_sequential) = std::mem::take(&mut update.run_sequentially) {
        ad.run_sequentially = run_sequential;
    }
    if let Some(command) = &update.command {
        ad.command = map_command(command)?;
    }
    if let Some(exit_code) = &update.exit_code {
        ad.expected_exit_code = *exit_code;
    }
    if let Some(inputs) = update.inputs.take() {
        ad.input_filters
            .update_from(inputs)
            .context(format!("While changing {qid}"))?;
    }

    let index = actions.len();
    actions.push(ad);
    action_map.insert(qid.clone(), index);
    action_map.insert(s_qid.clone(), index);

    Ok(())
}

fn add_action(
    update: &mut TomlActionDefinition,
    source: &ActionSource,
    actions: &mut Vec<beautytips::ActionDefinition>,
    action_map: &mut ActionMap,
) -> anyhow::Result<()> {
    let qid = QualifiedActionId::new(update.name.clone());
    let s_qid = QualifiedActionId::new_from_source(update.name.clone(), source.clone());

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
    let show_output =
        match_output_condition(&std::mem::take(&mut update.show_output).unwrap_or_default());
    let command = map_command(command).context("Processing command of {qid}")?;
    let run_sequentially = std::mem::take(&mut update.run_sequentially).unwrap_or(true);
    let expected_exit_code = update.exit_code.unwrap_or(0);
    let input_filters = if let Some(inputs) = update.inputs.take() {
        InputFilters::try_from(inputs)?
    } else {
        beautytips::InputFilters::default()
    };

    let ad = beautytips::ActionDefinition {
        id: update.name.to_string(),
        source: source.to_string(),
        show_output,
        run_sequentially,
        description,
        command,
        expected_exit_code,
        input_filters,
    };

    let index = actions.len();
    actions.push(ad);
    action_map.insert(qid, index);
    action_map.insert(s_qid, index);

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
        let s_qid = QualifiedActionId::new_from_source(ag.name.clone(), other.source.clone());
        let ids = ag
            .actions
            .iter()
            .map(|id| {
                if id.source == "this".try_into().ok() {
                    QualifiedActionId::new_from_source(id.id.clone(), other.source.clone())
                } else {
                    id.clone()
                }
            })
            .collect::<Vec<_>>();

        action_groups.insert(qid, ids.clone());
        action_groups.insert(s_qid, ids);
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
        } else if let Some(prefix) = action_name.to_string().strip_suffix("_all") {
            let prefix = format!("{prefix}_");
            for c in self
                .action_map
                .iter()
                .filter_map(|(qid, _)| qid.to_string().starts_with(&prefix).then_some(qid))
            {
                self.add_actions(c, result, visited)?;
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
            use std::convert::TryFrom;

            let config = Configuration::default();
            $(
                let config = config.merge(
                    ConfigurationSource::from_string(
                        include_str!(std::concat!($file, ".toml")),
                        ActionSource::try_from($file).expect(std::concat!($file, " is a valid action id"))
                    ).expect(std::concat!($file, " should parse fine"))
                )
                .expect(std::concat!($file, " merge ok"));
            )*
            config
        }
    }};
}

pub fn builtin() -> Configuration {
    import_rules!("builtin", "github", "rust", "spell", "toml")
}

pub fn load_user_configuration() -> anyhow::Result<Configuration> {
    let base = builtin();

    let config_dir = dirs::config_dir()
        .map(|cd| cd.join("beautytips"))
        .ok_or(anyhow::anyhow!("Config directory not found"))?;
    let config_file = config_dir.join("config.toml");

    if !config_file.exists() {
        return Ok(base);
    }

    let user = ConfigurationSource::from_path(config_file.as_path(), "user".try_into().unwrap())
        .context(format!(
            "Failed to parse configuration file {config_file:?}"
        ))?;
    base.merge(user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_id() {
        assert!(is_valid_id("foo"));
        assert!(is_valid_id("_foo_"));
        assert!(is_valid_id("bar_123_foo"));
        assert!(!is_valid_id("Bar_123_foo"));
        assert!(!is_valid_id("123_Bar"));
        assert!(!is_valid_id(""));
    }

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

        let base = ConfigurationSource::from_string(base, ActionSource::try_from("test").unwrap())
            .unwrap();
        let base = Configuration::default().merge(base).unwrap();

        assert_eq!(base.actions.len(), 2);
        assert_eq!(
            base.actions(&[QualifiedActionId::new(
                ActionId::try_from("test_1").unwrap()
            )])
            .unwrap()
            .count(),
            1
        );
        assert_eq!(
            base.actions(&[QualifiedActionId::new(
                ActionId::try_from("test_2").unwrap()
            )])
            .unwrap()
            .count(),
            1
        );
        assert!(base
            .actions(&[QualifiedActionId::new(
                ActionId::try_from("test_3").unwrap()
            )])
            .is_err());
        assert_eq!(base.action_groups.len(), 2);
        assert_eq!(
            base.actions(&[QualifiedActionId::new(ActionId::try_from("test").unwrap())])
                .unwrap()
                .count(),
            2
        );
        assert_eq!(
            base.actions(&[QualifiedActionId::new(
                ActionId::try_from("test_all").unwrap()
            )])
            .unwrap()
            .count(),
            2
        );
    }

    #[test]
    fn test_configuration_from_str_empty_ok() {
        let base = "";

        let base = ConfigurationSource::from_string(base, ActionSource::try_from("test").unwrap())
            .unwrap();
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
            ConfigurationSource::from_string(base, ActionSource::try_from("test").unwrap())
                .is_err()
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
            ConfigurationSource::from_string(base, ActionSource::try_from("test").unwrap())
                .is_err()
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
            ConfigurationSource::from_string(base, ActionSource::try_from("test").unwrap())
                .is_err()
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
            ConfigurationSource::from_string(base, ActionSource::try_from("test").unwrap())
                .is_err()
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

        let base = ConfigurationSource::from_string(base, ActionSource::try_from("test").unwrap())
            .unwrap();
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
inputs.files = [ "**/*.slt", "**/*.rs" ]

[[action_groups]]
name = "test"
actions = [ "test_1", "test_2" ]
"#;

        let base = ConfigurationSource::from_string(base, ActionSource::try_from("test").unwrap())
            .unwrap();
        let merge = Configuration::default().merge(base).unwrap();

        assert_eq!(merge.actions.len(), 3);
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(
                    ActionId::try_from("test_1").unwrap()
                )])
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(
                    ActionId::try_from("test_3b").unwrap()
                )])
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(
                    ActionId::try_from("test_2").unwrap()
                )])
                .unwrap()
                .count(),
            1
        );
        assert_eq!(merge.action_groups.len(), 2);
        // let it = merge
        //     .named_actions(&[QualifiedActionId::new(ActionId::try_from("test").unwrap())])
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
inputs.files = [ "**/*.slt", "**/*.rs" ]

[[action_groups]]
name = "test"
actions = [ "test_1", "test_2" ]
"#;

        let base = ConfigurationSource::from_string(base, ActionSource::try_from("test").unwrap())
            .unwrap();
        let base = Configuration::default().merge(base).unwrap();

        let other = r#"[[actions]]
name = "test_3o"
description = "foo"
command = "bar foo x y z"

[[actions]]
name = "test_2"
merge = "change"
command = "/dev/null"

[[actions]]
name = "test_1"
merge = "change"
command = "bar foo x y z"

[[action_groups]]
name = "test"
actions = [ "test_1", "test_3o", "test_3b" ]

[[action_groups]]
name = "test_group"
actions = [ "test_3b" ]
"#;
        let other =
            ConfigurationSource::from_string(other, ActionSource::try_from("test2").unwrap())
                .unwrap();

        let merge = base.merge(other).unwrap();

        assert_eq!(merge.actions.len(), 6);
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(
                    ActionId::try_from("test_1").unwrap()
                )])
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(
                    ActionId::try_from("test_3b").unwrap()
                )])
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(
                    ActionId::try_from("test_3o").unwrap()
                )])
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&[QualifiedActionId::new(
                    ActionId::try_from("test_group").unwrap()
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
