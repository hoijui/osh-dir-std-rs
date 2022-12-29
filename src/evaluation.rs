// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::Coverage;

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
    S: AsRef<Path> + 'a,
{
    let mut ratings = vec![];
    for (key, cov) in Coverage::all(dirs_and_files, ignored_paths) {
        ratings.push(Rating {
            name: key,
            factor: cov.rate(),
        });
    }
    ratings
}
