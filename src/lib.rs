// SPDX-FileCopyrightText: 2021-2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![warn(rust_2021_compatibility)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
#![warn(clippy::wildcard_enum_match_arm)]
#![warn(clippy::string_slice)]
#![warn(clippy::indexing_slicing)]
#![warn(clippy::clone_on_ref_ptr)]
#![warn(clippy::try_err)]
#![warn(clippy::shadow_reuse)]
#![warn(clippy::empty_structs_with_brackets)]
#![warn(clippy::else_if_without_else)]
#![warn(clippy::use_debug)]
#![warn(clippy::print_stdout)]
#![warn(clippy::print_stderr)]
#![allow(clippy::default_trait_access)]
// NOTE allowed because:
//      If the same regex is going to be applied to multiple inputs,
//      the precomputations done by Regex construction
//      can give significantly better performance
//      than any of the `str`-based methods.
#![allow(clippy::trivial_regex)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::fn_params_excessive_bools)]

pub mod constants;
mod coverage;
pub mod data;
pub mod file_listing;
pub mod format;

pub use coverage::Coverage;

use std::path::Path;

pub type BoxResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Rates the current directory,
/// using the default ignored paths regex.
///
/// # Errors
///
/// The only possible errors that may happen,
/// happen during the file-listing phase.
/// See [`file_listing::dirs_and_files`] for details about these errors.
pub fn rate<P: AsRef<Path>>(proj_repo: P) -> BoxResult<Vec<(&'static str, f32)>> {
    let ignored_paths = &constants::DEFAULT_IGNORED_PATHS;
    let dirs_and_files = file_listing::dirs_and_files(proj_repo.as_ref(), ignored_paths);
    // NOTE: Problem!!! RelativePath only supports UTF8 (?) -> very bad! .. we can't use it?
    let rating = dirs_and_files.map(|ref lst| coverage::rate_listing(lst, ignored_paths))?;
    Ok(rating)
}
