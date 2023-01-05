// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::{cover_listing, coverage::cover_listing_with, data::STDS, Coverage};

#[derive(Serialize, Deserialize)]
pub struct Rating {
    name: String,
    factor: f32,
}

impl Rating {
    /// Calculates how much the input listing adheres to the input dir standard.
    /// 0.0 means not at all, 1.0 means totally/fully.
    #[must_use]
    pub fn rate_coverage<P: AsRef<Path>>(name: String, coverage: &Coverage) -> Self {
        let mut pos_rating = 0.0;
        let mut matches_records = false;
        for (record, paths) in &coverage.r#in {
            if !paths.is_empty() {
                pos_rating += record.indicativeness;
                trace!("rp: {}", record.path);
                trace!("rr: {:#?}", record.regex);
                trace!("ri: {}", record.indicativeness);
                trace!("mps: {:#?}", paths);
                matches_records = true;
            }
        }
        if !matches_records {
            return Self { name, factor: 0.0 };
        }

        let mut ind_sum = 0.0;
        for rec in &coverage.std.records {
            ind_sum += rec.indicativeness;
        }
        let av_ind = ind_sum / coverage.std.records.len() as f32;

        let neg_rating = coverage.out.len() as f32 * av_ind;
        // trace!("{:#?}", self);
        trace!("ai: {}", av_ind);
        trace!("nr: {}", neg_rating);
        trace!("pr: {}", pos_rating);
        trace!("out: {:#?}", coverage.out);

        let factor = pos_rating / (pos_rating + neg_rating);
        Self { name, factor }
    }
}

/// Given a set of the relative paths of all dirs and files in a project,
/// for each of the known dir standards from
/// <https://github.com/hoijui/osh-dir-std/>,
/// calculate how likely it seems
/// that the project is following this standard.
pub fn rate_listing<T>(dirs_and_files: T, ignored_paths: &Regex) -> Vec<Rating>
where
    T: IntoIterator<Item = Rc<PathBuf>> + Clone,
{
    let coverages = cover_listing(dirs_and_files, ignored_paths);
    let mut ratings = vec![];
    for (std, coverage) in coverages {
        ratings.push(Rating {
            name: std.to_owned(),
            factor: coverage.rate(),
        });
    }
    ratings
}

/// Given a set of the relative paths of all dirs and files in a project,
/// for the given directory standard,
/// calculate how likely it seems
/// that the project is following this standard.
///
/// # Panics
///
/// If `std_name` does not equal any known directory standards name.
pub fn rate_listing_with<T>(dirs_and_files: T, ignored_paths: &Regex, std_name: String) -> Rating
where
    T: IntoIterator<Item = Rc<PathBuf>> + Clone,
{
    let std = STDS
        .get(&std_name)
        .unwrap_or_else(|| panic!("Unknown directory standard: '{std_name}'"));
    let coverage = cover_listing_with(dirs_and_files, ignored_paths, std);
    Rating {
        name: std_name,
        factor: coverage.rate(),
    }
}
