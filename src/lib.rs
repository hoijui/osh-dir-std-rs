// SPDX-FileCopyrightText: 2021 - 2023 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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

// This tests rust code in the README with doc-tests.
// Though, It will not appear in the generated documentaton.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

pub const VERSION: &str = git_version!();
