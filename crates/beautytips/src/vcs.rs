// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::path::PathBuf;

mod git;
mod jj;

pub type BoxedVcs = Box<dyn Vcs>;

/// Trait used to supposrt different version control systems
pub trait Vcs {
    /// The name of the version control system
    fn name(&self) -> &str;
    
    /// Is this VCS supported on this platform?
    fn is_supported(&self, ctx: &crate::Context) -> bool;
    
    /// Find changed files in the `root_directory`
    fn changed_files(&self, ctx: &crate::Context) -> Vec<PathBuf>;

    /// Find the directory root
    fn repository_root(&self, ctx: &crate::Context) -> Option<PathBuf>;
}

#[must_use]
pub fn known_vcses() -> Vec<BoxedVcs> {
    vec![
        Box::new(git::Git::new()),
        Box::new(jj::Jj::new()),
    ]
}
