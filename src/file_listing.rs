// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use regex::Regex;
use relative_path::RelativePathBuf;
use std::{fs, path::Path};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("File-system access related error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Failed to convert to a relative path: {0}")]
    RelativePath(#[from] relative_path::FromPathError),
}

/// Given a `proj_root`,
/// lists all the contained files and directories recursively.
///
/// # Errors
///
/// This function will return an error in the following situations,
/// but is not limited to just these cases:
///
/// * The provided path doesn't exist.
/// * The process lacks permissions to view the contents.
/// * The path points at a non-directory file.
/// * A path to a dir or file could not be converted to a relative path
pub fn dirs_and_files(
    proj_root: &Path,
    ignore_paths: &Regex,
) -> Result<Vec<RelativePathBuf>, Error> {
    let mut rel_listing = vec![];
    let mut dirs_to_scan = vec![proj_root.to_path_buf()];
    while let Some(cur_dir) = dirs_to_scan.pop() {
        for entry_res in fs::read_dir(cur_dir)? {
            let entry = entry_res?;
            let path = entry.path();

            let rel_path =
                RelativePathBuf::from_path(if let Ok(new_path) = path.strip_prefix(proj_root) {
                    new_path
                } else {
                    proj_root
                })?;
            if ignore_paths.is_match(rel_path.as_str()) {
                continue;
            }
            rel_listing.push(rel_path);
            if path.is_dir() {
                dirs_to_scan.push(path.clone());
            }
        }
    }
    Ok(rel_listing)
}
