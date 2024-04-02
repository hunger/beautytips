// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

pub mod errors;
pub(crate) mod util;
pub mod vcs;

use std::path::{Path, PathBuf};

use errors::{Error, Result};

pub struct Context {
    sh: xshell::Shell,
}

/// A `Context` is an opaque object used to interact with various parts of
/// `beautytips`
impl Context {
    /// Create a new `Context`
    ///
    /// # Errors
    ///
    /// This might fail when the current working directory is not valid
    pub fn new() -> Result<Self> {
        let sh = xshell::Shell::new().map_err(Error::new_current_directory)?;

        Ok(Self { sh })
    }

    /// Create a new `Context` in a given `directory`
    ///
    /// # Errors
    ///
    /// This might fail when the current working directory is not valid or the
    /// provided path is not a directory
    pub fn new_in(directory: &Path) -> Result<Self> {
        let sh = xshell::Shell::new().map_err(Error::new_current_directory)?;
        sh.change_dir(directory);

        let sh_path = sh.current_dir();
        if !sh_path.is_dir() {
            return Err(Error::new_not_a_directory(sh_path));
        }

        Ok(Self { sh })
    }
}

pub fn find_files_changed_in_vcs(ctx: &Context) -> Result<Vec<PathBuf>> {
    let known_vcs = vcs::known_vcses();
    let Some((vcs, root_dir)) = known_vcs
        .iter()
        .find_map(|vcs| vcs.repository_root(ctx).map(|r| (vcs, r)))
    else {
        return Ok(vec![]);
    };

    Ok(vec![])
}
