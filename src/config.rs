// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::collections::hash_set::Iter;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::{convert::TryFrom, fmt::Display, path::Path};

use anyhow::Context;

use beautytips::InputFilters;

fn is_valid_id(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }

    let mut first_part_valid = false;
    let mut had_separator = false;
    let mut second_part_valid = false;

    for c in id.chars() {
        match c {
            '/' => {
                if had_separator || !first_part_valid {
                    return false;
                }
                had_separator = true;
            }
            '_' => { /* do nothing */ }
            c if c.is_ascii_digit() => { /* do nothing */ }
            c if c.is_ascii_lowercase() => {
                if had_separator {
                    second_part_valid = true;
                } else {
                    first_part_valid = true;
                }
            }
            _ => return false,
        }
    }

    second_part_valid
}

fn find_selectors(action_groups: &ActionGroups, selectors: &ActionSelectors) -> ActionSelectors {
    let mut result = ActionSelectors::default();
    let mut next_result = selectors.clone();

    while next_result.len() != result.len() {
        result.0.extend(next_result.0.iter().cloned());
        for (n, group_selectors) in action_groups {
            for s in &result.0 {
                if s.matches(n) {
                    next_result.extend(group_selectors.clone());
                }
            }
        }
    }

    next_result
}

fn find_actions<'a>(
    actions: &'a ActionMap,
    selectors: &ActionSelectors,
) -> Vec<&'a beautytips::ActionDefinition> {
    actions
        .iter()
        .filter_map(move |(_, ad)| {
            selectors
                .0
                .iter()
                .find_map(|s| s.matches(&ad.id).then_some(ad))
        })
        .collect()
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize)]
#[serde(try_from = "String", expecting = "an action id")]
pub struct ActionId(String);

impl ActionId {
    /// Create a new `ActionId`
    ///
    /// # Errors
    ///
    /// Complain when the `input` is not a valid Action Id
    pub fn new(input: String) -> anyhow::Result<Self> {
        if is_valid_id(&input) {
            Ok(Self(input))
        } else {
            Err(anyhow::anyhow!("{input} is not a valid action id"))
        }
    }
}

impl std::ops::Deref for ActionId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
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
#[serde(try_from = "String", expecting = "an action id selector")]
pub struct ActionSelector(glob::Pattern);

impl ActionSelector {
    /// Create a new `ActionId`
    ///
    /// # Errors
    ///
    /// Complain if the selector is not valid
    pub fn new(input: &str) -> anyhow::Result<Self> {
        let pattern = if input.contains('/') {
            glob::Pattern::new(input).context("Failed to parse action selector")?
        } else {
            glob::Pattern::new(&format!("*/{input}")).context("Failed to parse action selector")?
        };

        Ok(Self(pattern))
    }

    pub fn matches(&self, input: &str) -> bool {
        self.0.matches_with(
            input,
            glob::MatchOptions {
                case_sensitive: true,
                require_literal_separator: true,
                require_literal_leading_dot: false,
            },
        )
    }
}

impl Display for ActionSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for ActionSelector {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl TryFrom<&str> for ActionSelector {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl std::str::FromStr for ActionSelector {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ActionSelectors(HashSet<ActionSelector>);

impl ActionSelectors {
    /// Create a new `ActionId`
    ///
    /// # Errors
    ///
    /// Complain if the selector is not valid
    pub fn new<const N: usize>(input: [&str; N]) -> anyhow::Result<Self> {
        Ok(Self(
            input
                .iter()
                .map(|s| ActionSelector::new(s))
                .collect::<anyhow::Result<_>>()?,
        ))
    }

    pub fn matches(&self, input: &str) -> bool {
        self.0.iter().any(|s| s.matches(input))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn contains(&self, needle: &ActionSelector) -> bool {
        self.0.contains(needle)
    }
}

impl From<Vec<ActionSelector>> for ActionSelectors {
    fn from(input: Vec<ActionSelector>) -> ActionSelectors {
        ActionSelectors(input.into_iter().collect())
    }
}

impl std::iter::Extend<ActionSelector> for ActionSelectors {
    #[inline]
    fn extend<I: IntoIterator<Item = ActionSelector>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<'a> IntoIterator for &'a ActionSelectors {
    type Item = &'a ActionSelector;
    type IntoIter = Iter<'a, ActionSelector>;

    #[inline]
    fn into_iter(self) -> Iter<'a, ActionSelector> {
        self.0.iter()
    }
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeAction {
    Remove,
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
    pub environment: Option<Vec<String>>,
    #[serde(default)]
    pub run_sequentially: Option<bool>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub show_output: Option<OutputCondition>,
    #[serde(default)]
    pub inputs: Option<HashMap<String, Vec<String>>>,
}

type ActionGroups = HashMap<ActionId, Vec<ActionSelector>>;
type ActionMap = BTreeMap<ActionId, beautytips::ActionDefinition>;

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TomlActionGroup {
    pub name: ActionId,
    pub actions: Vec<ActionSelector>,
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
    pub action_map: ActionMap,
}

#[derive(Debug)]
pub struct ConfigurationSource {
    pub action_groups: Vec<TomlActionGroup>,
    pub actions: Vec<TomlActionDefinition>,
}

impl ConfigurationSource {
    fn from_string(value: &str) -> anyhow::Result<Self> {
        let mut toml_config: TomlConfiguration =
            toml::from_str(value).context("Failed to parse toml")?;

        let actions = std::mem::take(&mut toml_config.actions);
        let action_groups = std::mem::take(&mut toml_config.action_groups);

        Ok(Self {
            action_groups,
            actions,
        })
    }

    fn from_path(path: &Path) -> anyhow::Result<Self> {
        let config_data =
            std::fs::read_to_string(path).context(format!("Failed to read toml file {path:?}"))?;

        Self::from_string(config_data.as_str()).context("Failed to parse toml string")
    }
}

fn remove_action(action: &TomlActionDefinition, action_map: &mut ActionMap) -> anyhow::Result<()> {
    let id = action.name.clone();
    if action.description.is_some()
        || action.show_output.is_some()
        || action.run_sequentially.is_some()
        || action.command.is_some()
        || action.exit_code.is_some()
        || action.inputs.is_some()
    {
        return Err(anyhow::anyhow!(format!(
            "{id} is removing an action, but has extra keys set"
        )));
    }

    if action_map.remove(&id).is_none() {
        return Err(anyhow::anyhow!(format!(
            "{id} is removing an action that did not exist"
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

fn map_environment(environment: &[String]) -> Vec<(String, String)> {
    environment
        .iter()
        .map(|k| {
            k.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .unwrap_or_else(|| (k.to_string(), String::new()))
        })
        .collect()
}

fn change_action(
    update: &mut TomlActionDefinition,
    action_map: &mut ActionMap,
) -> anyhow::Result<()> {
    let id = update.name.clone();

    if update.description.is_none()
        && update.show_output.is_none()
        && update.run_sequentially.is_none()
        && update.command.is_none()
        && update.environment.is_none()
        && update.exit_code.is_none()
        && update.inputs.is_none()
    {
        return Err(anyhow::anyhow!(format!(
            "{id} is changing an existing action, but has no extra keys set"
        )));
    }
    let Some(ad) = action_map.get_mut(&id) else {
        return Err(anyhow::anyhow!(format!(
            "{id} is changing an action that does not exist"
        )));
    };

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
    if let Some(env) = update.environment.take() {
        ad.environment = map_environment(&env);
    }
    if let Some(exit_code) = &update.exit_code {
        ad.expected_exit_code = *exit_code;
    }
    if let Some(inputs) = update.inputs.take() {
        ad.input_filters
            .update_from(inputs)
            .context(format!("While changing {id}"))?;
    }

    Ok(())
}

fn add_action(update: &mut TomlActionDefinition, action_map: &mut ActionMap) -> anyhow::Result<()> {
    let id = update.name.clone();

    let Some(command) = &update.command else {
        return Err(anyhow::anyhow!(format!(
            "Can not add {}: No command",
            update.name
        )));
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
    let environment = if let Some(env) = &update.environment {
        map_environment(&env)
    } else {
        vec![]
    };

    let ad = beautytips::ActionDefinition {
        id: update.name.to_string(),
        show_output,
        run_sequentially,
        description,
        command,
        environment,
        expected_exit_code,
        input_filters,
    };

    let entry = action_map.entry(id);
    if matches!(entry, std::collections::btree_map::Entry::Occupied(_)) {
        return Err(anyhow::anyhow!(format!(
            "{} already exists, can not add",
            entry.key()
        )));
    };

    entry.or_insert(ad);

    Ok(())
}

fn merge_actions(
    mut action_map: ActionMap,
    other: &mut ConfigurationSource,
) -> anyhow::Result<ActionMap> {
    for mut action in other.actions.drain(..) {
        match action.merge {
            MergeAction::Remove => remove_action(&action, &mut action_map)?,
            MergeAction::Change => {
                change_action(&mut action, &mut action_map)?;
            }
            MergeAction::Add => {
                add_action(&mut action, &mut action_map)?;
            }
        }
    }
    Ok(action_map)
}

fn add_new_action_groups(
    mut action_groups: ActionGroups,
    other: &mut ConfigurationSource,
) -> ActionGroups {
    for ag in other.action_groups.drain(..) {
        action_groups.insert(ag.name, ag.actions);
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

impl Configuration {
    /// Merge `other` onto the base of `self`
    pub fn merge(mut self, mut other: ConfigurationSource) -> anyhow::Result<Self> {
        let action_map = merge_actions(std::mem::take(&mut self.action_map), &mut other)?;

        let action_groups =
            add_new_action_groups(std::mem::take(&mut self.action_groups), &mut other);

        Ok(Self {
            action_groups,
            action_map,
        })
    }

    pub fn actions<'a>(
        &'a self,
        selectors: &ActionSelectors,
    ) -> beautytips::ActionDefinitionIterator<'a> {
        let selectors = find_selectors(&self.action_groups, selectors);
        beautytips::ActionDefinitionIterator::new(find_actions(&self.action_map, &selectors))
    }
}

macro_rules! import_rules {
    ( $( $file: tt ),* ) => {{
        {
            let config = Configuration::default();
            $(
                let config = config.merge(
                    ConfigurationSource::from_string(
                        include_str!(std::concat!($file, ".toml")),
                    ).expect(std::concat!($file, " should parse fine"))
                )
                .expect(std::concat!($file, " merge ok"));
            )*
            config
        }
    }};
}

pub fn builtin() -> Configuration {
    import_rules!(
        "actionlint",
        "biome",
        "builtin",
        "cargo",
        "cspell",
        "mypy",
        "ruff",
        "taplo"
    )
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

    let user = ConfigurationSource::from_path(config_file.as_path()).context(format!(
        "Failed to parse configuration file {config_file:?}"
    ))?;
    base.merge(user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_id() {
        assert!(is_valid_id("foo/bar"));
        assert!(is_valid_id("_a/_b"));
        assert!(is_valid_id("1a/1b"));
        assert!(is_valid_id("1___a_dsd_/b___342144_zdfj"));
        assert!(!is_valid_id(""));
        assert!(!is_valid_id("__1/bar"));
        assert!(!is_valid_id("foo/1__"));
        assert!(!is_valid_id("foo"));
        assert!(!is_valid_id("_foo_"));
        assert!(!is_valid_id("a/Bar"));
    }

    #[test]
    fn test_find_selectors_recursive() {
        let selectors = find_selectors(
            &HashMap::from([
                (
                    ActionId::new("test/g1".to_string()).unwrap(),
                    vec![
                        ActionSelector::new("test/g2").unwrap(),
                        ActionSelector::new("test/test1").unwrap(),
                    ],
                ),
                (
                    ActionId::new("test/g2".to_string()).unwrap(),
                    vec![ActionSelector::new("test/group2").unwrap()],
                ),
            ]),
            &ActionSelectors::new(["test/g1"]).unwrap(),
        );

        assert_eq!(selectors.len(), 4);
        assert!(selectors.contains(&ActionSelector::new("test/g1").unwrap()));
        assert!(selectors.contains(&ActionSelector::new("test/g2").unwrap()));
        assert!(selectors.contains(&ActionSelector::new("test/group2").unwrap()));
    }

    #[test]
    fn test_configuration_from_str_ok() {
        let base = r#"[[actions]]
name = "test/t1"
description = "foo"
command = "foobar x y z"

[[actions]]
name = "test/t2"
description = "foo"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[action_groups]]
name = "test/g1"
actions = [ "test/t1", "test/t2" ]
"#;

        let base = ConfigurationSource::from_string(base).unwrap();
        let base = Configuration::default().merge(base).unwrap();

        assert_eq!(base.action_map.len(), 2);
        assert_eq!(base.action_groups.len(), 1);

        assert_eq!(
            base.actions(&ActionSelectors::new(["test/t1"]).unwrap())
                .count(),
            1
        );
        assert_eq!(
            base.actions(&ActionSelectors::new(["test/t2"]).unwrap())
                .count(),
            1
        );
        assert_eq!(
            base.actions(&ActionSelectors::new(["test/t3"]).unwrap())
                .count(),
            0
        );
        assert_eq!(
            base.actions(&ActionSelectors::new(["test/g1"]).unwrap())
                .count(),
            2
        );
        assert_eq!(
            base.actions(&ActionSelectors::new(["test/*"]).unwrap())
                .count(),
            2
        );
    }

    #[test]
    fn test_configuration_from_str_empty_ok() {
        let base = "";

        let base = ConfigurationSource::from_string(base).unwrap();
        let base = Configuration::default().merge(base).unwrap();

        assert_eq!(base.action_map.len(), 0);
        assert_eq!(base.action_groups.len(), 0);
    }

    #[test]
    fn test_configuration_from_str_invalid_top_level_key() {
        let base = r#"[[action]]
name = "test/t1"
command = "foobar x y z"
"#;

        assert!(ConfigurationSource::from_string(base).is_err());
    }

    #[test]
    fn test_configuration_from_str_invalid_action_key() {
        let base = r#"[[actions]]
name = "test/t1"
command = "foobar x y z"

[[actions]]
name = "test/t2"
description = "foo"
id = "foobar"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[action_groups]]
name = "test/g1"
actions = [ "test/t1", "test/t2" ]
"#;

        assert!(ConfigurationSource::from_string(base).is_err());
    }

    #[test]
    fn test_configuration_from_str_invalid_action_group_key() {
        let base = r#"[[actions]]
name = "test/t1"
command = "foobar x y z"

[[actions]]
name = "test/t2"
description = "foo"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[action_groups]]
name = "test/g1"
id = "foobar"
actions = [ "test/t1", "test/t2" ]
"#;

        assert!(ConfigurationSource::from_string(base).is_err());
    }

    #[test]
    fn test_configuration_from_str_invalid_action_selector() {
        let base = r#"[[actions]]
name = "test/t1"
command = "foobar x y z"

[[action_groups]]
name = "test/g1"
actions = [ "/**/foo**" ]
"#;

        assert!(ConfigurationSource::from_string(base).is_err());
    }

    #[test]
    fn test_configuration_from_str_invalid_action_id() {
        let base = r#"[[actions]]
name = "INVALID"
command = "foobar x y z"
"#;

        assert!(ConfigurationSource::from_string(base).is_err());
    }

    #[test]
    fn test_configuration_from_str_invalid_glob() {
        let base = r#"[[actions]]
name = "test/t1"
command = "foobar x y z"

[[actions]]
name = "test/t2"
description = "foo"
command = "foobar \"a b c\""
inputs.files = [ "**a", "**/Cargo.toml" ]

[[action_groups]]
name = "test/g1"
actions = [ "test/t1", "test/t2" ]
"#;

        let base = ConfigurationSource::from_string(base).unwrap();
        assert!(Configuration::default().merge(base).is_err());
    }

    #[test]
    fn test_configuration_merge_empty() {
        let base = r#"[[actions]]
description = "foo"
name = "test/t1"
command = "foobar x y z"

[[actions]]
name = "test/t2"
description = "foo"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[actions]]
name = "test/t3b"
description = "foo"
command = "do something"
inputs.files = [ "**/*.slt", "**/*.rs" ]

[[action_groups]]
name = "test/g1"
actions = [ "test/t1", "test/t2" ]
"#;

        let base = ConfigurationSource::from_string(base).unwrap();
        let merge = Configuration::default().merge(base).unwrap();

        assert_eq!(merge.action_map.len(), 3);
        assert_eq!(
            merge
                .actions(&ActionSelectors::new(["test/t1"]).unwrap())
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&ActionSelectors::new(["test/t3b"]).unwrap())
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&ActionSelectors::new(["test/t2"]).unwrap())
                .count(),
            1
        );
        assert_eq!(merge.action_groups.len(), 1);
        // let it = merge
        //     .named_actions(&[QualifiedActionId::new(ActionId::try_from("test").unwrap())])
        //     .unwrap();
        // assert_eq!(it.count(), 2);
    }

    #[test]
    fn test_configuration_merge() {
        let base = r#"[[actions]]
name = "test/t1"
description = "foo"
command = "foobar x y z"

[[actions]]
name = "test/t2"
description = "foo"
command = "foobar \"a b c\""
inputs.files = [ "**/*.rs", "**/Cargo.toml" ]

[[actions]]
name = "test/t3b"
description = "foo"
command = "do something"
inputs.files = [ "**/*.slt", "**/*.rs" ]

[[action_groups]]
name = "test/g1"
actions = [ "test/t1", "test/t2" ]
"#;

        let base = ConfigurationSource::from_string(base).unwrap();
        let base = Configuration::default().merge(base).unwrap();

        let other = r#"[[actions]]
name = "test/t3o"
description = "foo"
command = "bar foo x y z"

[[actions]]
name = "test/t2"
merge = "change"
command = "/dev/null"

[[actions]]
name = "test/t1"
merge = "change"
command = "bar foo x y z"

[[action_groups]]
name = "test/g1"
actions = [ "test/t1", "test/t3o", "test/t3b" ]

[[action_groups]]
name = "test/g2"
actions = [ "test/t3b" ]
"#;
        let other = ConfigurationSource::from_string(other).unwrap();

        let merge = base.merge(other).unwrap();

        assert_eq!(merge.action_map.len(), 4);
        assert_eq!(
            merge
                .actions(&ActionSelectors::new(["test/t1"]).unwrap())
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&ActionSelectors::new(["test/t3b"]).unwrap())
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&ActionSelectors::new(["test/t3o"]).unwrap())
                .count(),
            1
        );
        assert_eq!(
            merge
                .actions(&ActionSelectors::new(["test/g1"]).unwrap())
                .count(),
            3
        );
    }

    #[test]
    fn test_builtins() {
        let builtin = builtin();

        assert!(!builtin.action_map.is_empty());
        assert!(builtin.action_groups.is_empty());
    }
}
