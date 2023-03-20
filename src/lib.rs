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
#![allow(clippy::cast_precision_loss)]

pub mod constants;
mod coverage;
pub mod data;
mod evaluation;
pub mod format;
pub mod stds;
pub mod tree;

pub use coverage::cover_listing;
pub use coverage::cover_listing_by_stds;
pub use coverage::cover_listing_with;
pub use coverage::Coverage;
pub use evaluation::best_fit;
pub use evaluation::rate_listing;
pub use evaluation::rate_listing_by_stds;
pub use evaluation::rate_listing_with;
pub use evaluation::Rating;
pub use evaluation::RatingCont;

use git_version::git_version;

pub use data::DEFAULT_STD_NAME;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
pub type BoxResult<T> = Result<T, BoxError>;

// This tests rust code in the README with doc-tests.
// Though, It will not appear in the generated documentaton.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

pub const VERSION: &str = git_version!();
