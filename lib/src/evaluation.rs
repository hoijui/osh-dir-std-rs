// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io;
use std::{path::PathBuf, rc::Rc};

use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::trace;

use crate::{cover_listing, coverage::cover_listing_with, data::STDS, stds::Standards, Coverage};

#[derive(Serialize, Deserialize, Clone)]
pub struct Rating {
    pub name: String,
    pub factor: f32,
}

#[derive(Serialize)]
pub struct RatingCont {
    pub rating: Rating,
    pub coverage: Option<Coverage>,
}

impl RatingCont {
    #[must_use]
    pub fn remove_coverage(self) -> Self {
        Self {
            rating: self.rating,
            coverage: None,
        }
    }
}

impl Rating {
    /// Calculates how much the input listing adheres to the input dir standard.
    /// 0.0 means not at all, 1.0 means totally/fully.
    #[must_use]
    pub fn rate_coverage(coverage: &Coverage) -> Self {
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
        let name = coverage.std.name.to_owned();
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
///
/// # Errors
///
/// If any of the input listing entries is an error,
/// usually caused by an I/O issue.
pub fn rate_listing<T, E>(dirs_and_files: T, ignored_paths: &Regex) -> Result<Vec<RatingCont>, E>
where
    T: Iterator<Item = Result<Rc<PathBuf>, E>>,
{
    let coverages = cover_listing(dirs_and_files, ignored_paths)?;
    let mut ratings = vec![];
    for coverage in coverages {
        ratings.push(RatingCont {
            rating: Rating {
                name: coverage.std.name.to_owned(),
                factor: coverage.rate(),
            },
            coverage: Some(coverage),
        });
    }
    Ok(ratings)
}

/// Given a set of the relative paths of all dirs and files in a project,
/// for the given directory standard,
/// calculate how likely it seems
/// that the project is following this standard.
///
/// # Errors
///
/// If any of the input listing entires is an error,
/// usually caused by an I/O issue.
///
/// # Panics
///
/// If `std_name` does not equal any known directory standards name.
pub fn rate_listing_with<T, E>(
    dirs_and_files: T,
    ignored_paths: &Regex,
    std_name: &str,
) -> Result<RatingCont, E>
where
    T: Iterator<Item = Result<Rc<PathBuf>, E>>,
{
    let std = STDS
        .get(std_name)
        .unwrap_or_else(|| panic!("Unknown directory standard: '{std_name}'"));
    let coverage = cover_listing_with(dirs_and_files, ignored_paths, std)?;
    Ok(RatingCont {
        rating: Rating {
            name: std_name.to_string(),
            factor: coverage.rate(),
        },
        coverage: Some(coverage),
    })
}

#[derive(Error, Debug)]
pub enum BestFitError {
    #[error("None of the supplied ratings has a factor higher then 0.0")]
    NoneViable,
}

/// Given a set of ratings, filters out the one with the higest factor.
/// If multiple have the same, highest factor, the first one is returned.
///
/// # Errors
///
/// If none of the supplied ratings has a factor higher then 0.0.
pub fn best_fit(ratings: Vec<RatingCont>) -> Result<RatingCont, BestFitError> {
    let mut max_rating: Option<RatingCont> = None;
    for rating_cont in ratings {
        if let Some(ref max_rating_val) = max_rating {
            if rating_cont.rating.factor > max_rating_val.rating.factor {
                max_rating = Some(rating_cont);
            }
        } else {
            max_rating = Some(rating_cont);
        }
    }
    max_rating.ok_or(BestFitError::NoneViable)
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to evaluate the best fit, because: {0:?}")]
    BestFitError(#[from] BestFitError),

    /// Represents all other cases of `std::io::Error`.
    #[error(transparent)]
    IO(#[from] std::io::Error),
}

/// Given a set of the relative paths of all dirs and files in a project,
/// for each of the known dir standards from
/// <https://github.com/hoijui/osh-dir-std/>,
/// calculate how likely it seems
/// that the project is following this standard,
/// and then only return the rating for the best fit.
///
/// # Errors
///
/// If any of the input listing entries is an error,
/// usually caused by an I/O issue.
pub fn rate_listing_by_stds<T>(
    dirs_and_files: T,
    ignored_paths: &Regex,
    stds: &Standards,
) -> Result<Vec<RatingCont>, Error>
where
    T: Iterator<Item = Result<Rc<PathBuf>, io::Error>>,
{
    Ok(match stds {
        Standards::Default => vec![rate_listing_with(
            dirs_and_files,
            ignored_paths,
            crate::DEFAULT_STD_NAME,
        )?],
        Standards::All => rate_listing(dirs_and_files, ignored_paths)?,
        Standards::BestFit => {
            let ratings: Vec<RatingCont> =
                rate_listing(dirs_and_files, ignored_paths)?;
            let max_rating: RatingCont = best_fit(ratings)?;
            vec![max_rating]
        }
        Standards::Specific(std_name) => {
            vec![rate_listing_with(dirs_and_files, ignored_paths, std_name)?]
        }
    })
}
