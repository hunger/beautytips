// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

async fn get_target_from_cargo_toml(path: &Path) -> Option<String> {
    let contents = tokio::fs::read(path).await.ok()?;
    let cargo_toml = cargo_toml::Manifest::from_slice(&contents).ok()?;

    cargo_toml.package.map(|p| p.name.clone())
}

async fn find_cargo_toml(top_directory: &Path, dir: &Path) -> Option<String> {
    let mut dir = dir;
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if !cargo_toml.starts_with(top_directory) {
            return None;
        }
        let target = get_target_from_cargo_toml(&cargo_toml).await;
        if target.is_some() {
            return target;
        }

        dir = dir.parent()?;
    }
}

async fn find_cargo_target(top_directory: &Path, path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_string_lossy();
    if file_name == "Cargo.toml" {
        get_target_from_cargo_toml(path).await
    } else if file_name.ends_with(".rs") {
        find_cargo_toml(top_directory, path.parent()?).await
    } else {
        None
    }
}

pub(crate) async fn find_cargo_targets(top_directory: PathBuf, files: &[PathBuf]) -> Vec<PathBuf> {
    let mut targets = HashSet::new();

    for f in files {
        if let Some(target) = find_cargo_target(&top_directory, f).await {
            targets.insert(target);
        }
    }

    let mut targets: Vec<_> = targets.iter().map(PathBuf::from).collect();
    targets.sort();

    targets
}
