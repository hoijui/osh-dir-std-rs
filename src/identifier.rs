// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use regex::Regex;
use relative_path::RelativePath;
use std::collections::HashMap;
use tracing::trace;

/// Indicates which relative paths of all dirs and files in a project
/// are covered by what parts of a specific dir standard.
#[derive(Debug)]
struct Coverage<'a> {
    std: &'static super::format::DirStd,
    num_paths: usize,
    r#in: HashMap<&'static super::format::Rec<'static>, Vec<&'a RelativePath>>,
    out: Vec<&'a RelativePath>,
}

impl<'a> Coverage<'a> {
    /// Given a set of the relative paths of all dirs and files in a project,
    /// figures out which of them are covered by what parts
    /// of a given dir standard.
    pub fn check<'b, T, S>(
        dirs_and_files: T,
        std: &'static super::format::DirStd,
        ignored_paths: &Regex,
    ) -> Coverage<'b>
    where
        T: IntoIterator<Item = &'b S> + Copy,
        S: AsRef<RelativePath> + 'b,
    {
        let mut rec_ratings = Coverage {
            std,
            num_paths: 0,
            r#in: HashMap::new(),
            out: Vec::new(),
        };
        for dir_or_file in dirs_and_files {
            if ignored_paths.is_match(dir_or_file.as_ref().as_str()) {
                continue;
            }
            rec_ratings.num_paths += 1;
            let mut matched = false;
            for record in &std.records {
                if record.regex.is_match(dir_or_file.as_ref().as_str()) {
                    rec_ratings
                        .r#in
                        .entry(record)
                        .or_insert_with(Vec::new)
                        .push(dir_or_file.as_ref());
                    matched = true;
                }
            }
            if !matched {
                rec_ratings.out.push(dir_or_file.as_ref());
            }
        }
        rec_ratings
    }

    /// Calculates how much the input listing adheres to the input dir standard.
    /// 0.0 means not at all, 1.0 means totally/fully.
    pub fn rate(&self) -> f32 {
        let mut pos_rating = 0.0;
        let mut matches_records = false;
        for (record, paths) in &self.r#in {
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
            return 0.0;
        }

        let mut ind_sum = 0.0;
        for rec in &self.std.records {
            ind_sum += rec.indicativeness;
        }
        let av_ind = ind_sum / self.std.records.len() as f32;

        let neg_rating = self.out.len() as f32 * av_ind;
        // trace!("{:#?}", self);
        trace!("ai: {}", av_ind);
        trace!("nr: {}", neg_rating);
        trace!("pr: {}", pos_rating);
        trace!("out: {:#?}", self.out);

        pos_rating / (pos_rating + neg_rating)
    }

    /// Returns a list of the identified module(/parts) directories.
    /// In addition to these,
    /// we should also consider all dirs that contain an okh.toml file.
    pub fn module_dirs(&self) -> Vec<&RelativePath> {
        let mut dirs = vec![];
        for (record, paths) in &self.r#in {
            if record.module {
                for &path in paths {
                    dirs.push(path);
                }
            }
        }
        dirs
    }
}

/// Given a set of the relative paths of all dirs and files in a project,
/// for each of the known dir standards from
/// <https://github.com/hoijui/osh-dir-std/>,
/// calculate how likely it seems
/// that the project is following this standard.
pub fn rate_listing<'a, T, S>(
    dirs_and_files: T,
    ignored_paths: &Regex,
) -> HashMap<&'static str, f32>
where
    T: IntoIterator<Item = &'a S> + Copy,
    S: AsRef<RelativePath> + 'a,
{
    let mut ratings = HashMap::new();
    for (std_name, std_records) in super::data::STDS.iter() {
        trace!("");
        trace!("std: {}", std_name);
        let std_coverage = Coverage::check(dirs_and_files, std_records, ignored_paths);
        let rating = std_coverage.rate();
        ratings.insert(*std_name, rating);
    }
    ratings
}
