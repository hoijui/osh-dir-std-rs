// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use regex::Regex;
use relative_path::RelativePath;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::trace;

use crate::{constants, file_listing, BoxResult, Coverage};

#[derive(Serialize, Deserialize)]
pub struct Rating {
    name: &'static str,
    factor: f32,
}

impl Rating {
    /// Calculates how much the input listing adheres to the input dir standard.
    /// 0.0 means not at all, 1.0 means totally/fully.
    #[must_use]
    pub fn rate_coverage(name: &'static str, coverage: &Coverage) -> Self {
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
pub fn rate_listing<'a, T, S>(dirs_and_files: T, ignored_paths: &Regex) -> Vec<Rating>
where
    T: IntoIterator<Item = &'a S> + Copy,
    S: AsRef<RelativePath> + 'a,
{
    let mut ratings = vec![];
    for (key, cov) in Coverage::all(dirs_and_files, ignored_paths) {
        ratings.push(Rating {
            name: key,
            factor: cov.rate(),
        });
    }
    // let mut ratings = HashMap::new();
    // for (std_name, std_records) in super::data::STDS.iter() {
    //     trace!("");
    //     trace!("std: {}", std_name);
    //     let std_coverage = Coverage::check(dirs_and_files, std_records, ignored_paths);
    //     let rating = std_coverage.rate();
    //     ratings.insert(*std_name, rating);
    // }
    ratings
}

/// Rates the current directory,
/// using the default ignored paths regex.
///
/// # Errors
///
/// The only possible errors that may happen,
/// happen during the file-listing phase.
/// See [`file_listing::dirs_and_files`] for details about these errors.
pub fn rate_dir<P: AsRef<Path>>(
    proj_repo: P,
    ignore_paths: Option<&Regex>,
) -> BoxResult<Vec<Rating>> {
    let ignored_paths = ignore_paths.unwrap_or(&constants::DEFAULT_IGNORED_PATHS);
    let dirs_and_files = file_listing::dirs_and_files(proj_repo.as_ref(), ignored_paths);
    // NOTE: Problem!!! RelativePath only supports UTF8 (?) -> very bad! .. we can't use it?
    let rating = dirs_and_files.map(|ref lst| rate_listing(lst, ignored_paths))?;
    Ok(rating)
}
