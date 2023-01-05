// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use regex::Regex;
use std::{collections::HashMap, path::PathBuf, rc::Rc};
use tracing::trace;

use super::format::DirStd;

/// Indicates which relative paths of all dirs and files in a project
/// are covered by what parts of a specific dir standard.
#[derive(Debug)]
pub struct Checker {
    /// the coverage in creation
    pub coverage: Coverage,
    ignored_paths: Regex,
}

/// Indicates which relative paths of all dirs and files in a project
/// are covered by what parts of a specific dir standard.
#[derive(Debug)]
pub struct Coverage {
    /// Name of the standard that coverage was checked for
    pub std: &'static super::format::DirStd,
    /// Number of viable paths in the input-dir.
    /// These are all paths in the input dir,
    /// minus the ignored ones.
    pub num_paths: usize,
    /// The records in the checked standard
    /// that matched one or more paths in the input,
    /// together with all those matched paths.
    pub r#in: HashMap<&'static super::format::Rec<'static>, Vec<Rc<PathBuf>>>,
    /// The viable paths in the input dir that did not match any record
    /// of the checked standard.
    pub out: Vec<Rc<PathBuf>>,
}

impl Checker {
    /// Given a set of the relative paths of all dirs and files in a project,
    /// figures out which of them are covered by what parts
    /// of a given dir standard.
    pub fn new(std: &'static super::format::DirStd, ignored_paths: &Regex) -> Self {
        Self {
            coverage: Coverage {
                std,
                num_paths: 0,
                r#in: HashMap::new(),
                out: Vec::new(),
            },
            ignored_paths: ignored_paths.clone(),
        }
    }

    /// Creates a map of checkers with one entry for each standard.
    pub fn new_all(ignored_paths: &Regex) -> HashMap<&'static str, Self> {
        let mut checkers = HashMap::new();
        for (std_name, std_records) in super::data::STDS.iter() {
            let std_checker = Self::new(std_records, ignored_paths);
            checkers.insert(std_name as &str, std_checker);
        }
        checkers
    }

    pub fn cover(&mut self, dir_or_file: &Rc<PathBuf>) {
        let dir_or_file_str_lossy = dir_or_file.as_ref().to_string_lossy();
        if self.ignored_paths.is_match(&dir_or_file_str_lossy) {
            return;
        }
        self.coverage.num_paths += 1;
        let mut matched = false;
        for record in &self.coverage.std.records {
            if record.regex.is_match(&dir_or_file_str_lossy) {
                self.coverage
                    .r#in
                    .entry(record)
                    .or_insert_with(Vec::new)
                    .push(Rc::clone(dir_or_file));
                matched = true;
            }
        }
        if !matched {
            self.coverage.out.push(Rc::clone(dir_or_file));
        }
    }
}

impl Coverage {
    /// Calculates how much the input listing adheres to the input dir standard.
    /// 0.0 means not at all, 1.0 means totally/fully.
    #[must_use]
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
    #[must_use]
    pub fn module_dirs(&self) -> Vec<Rc<PathBuf>> {
        let mut dirs = vec![];
        for (record, paths) in &self.r#in {
            if record.module {
                for path in paths {
                    dirs.push(Rc::clone(path));
                }
            }
        }
        dirs
    }
}

/// Given a set of the relative paths of all dirs and files in a project,
/// for each of the known dir standards from
/// <https://github.com/hoijui/osh-dir-std/>,
/// calculate what record of the standard each dir or file might be covered under.
pub fn cover_listing<T>(dirs_and_files: T, ignored_paths: &Regex) -> Vec<(&'static str, Coverage)>
where
    T: IntoIterator<Item = Rc<PathBuf>> + Clone,
{
    let mut checkers = Checker::new_all(ignored_paths);
    for dir_or_file in dirs_and_files {
        for checker in checkers.values_mut() {
            checker.cover(&dir_or_file);
        }
    }
    let mut coverages = vec![];
    for (std, checker) in checkers {
        coverages.push((std, checker.coverage));
    }
    coverages
}

/// Given a set of the relative paths of all dirs and files in a project,
/// for the given directory standard,
/// calculate what record of the standard each dir or file might be covered under.
pub fn cover_listing_with<T>(
    dirs_and_files: T,
    ignored_paths: &Regex,
    std: &'static DirStd,
) -> Coverage
where
    T: IntoIterator<Item = Rc<PathBuf>> + Clone,
{
    let mut checker = Checker::new(std, ignored_paths);
    for dir_or_file in dirs_and_files {
        checker.cover(&dir_or_file);
    }
    checker.coverage
}
