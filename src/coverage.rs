// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use regex::Regex;
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf, rc::Rc};
use tracing::trace;

use crate::{
    best_fit,
    data::STDS,
    stds::Standards,
    tree::{self, RNode},
    BoxResult, Rating, DEFAULT_STD_NAME,
};

use super::format::DirStd;

/// Indicates which relative paths of all dirs and files in a project
/// are covered by what parts of a specific dir standard.
#[derive(Debug)]
pub struct Checker {
    /// the coverage in creation
    pub coverage: Coverage,
    ignored_paths: Regex,
    arbitrary_content_rgxs: Option<Vec<Regex>>,
    generated_content_rgxs: Option<Vec<Regex>>,
    records_tree: Option<(RNode<'static>, Vec<RNode<'static>>)>,
}

/// Indicates which relative paths of all dirs and files in a project
/// are covered by what parts of a specific dir standard.
#[derive(Debug, Serialize)]
pub struct Coverage {
    /// The standard that coverage was checked for
    pub std: &'static DirStd,
    /// Number of viable paths in the input-dir.
    /// These are all paths in the input dir,
    /// minus the ignored ones.
    pub num_paths: usize,
    /// The records in the checked standard
    /// that matched one or more paths in the input,
    /// together with all those matched paths.
    pub r#in: HashMap<&'static super::format::Rec<'static>, Vec<Rc<PathBuf>>>,
    /// The paths in the input dir that were ignored.
    pub ignored: Vec<Rc<PathBuf>>,
    /// The paths in the input dir that are below an arbitrary content root of the standard.
    /// This is similar to `ignored`, but defined in the standard itsself.
    pub arbitrary_content: Vec<Rc<PathBuf>>,
    /// The paths in the input dir that are below an generated content root of the standard,
    /// or fit a generated content regex otherwise.
    /// These paths makr files that may be tracked,
    /// even though they might be generated.
    /// This might make sense, because in practice,
    /// people *will* put generated files into git repositories in OSH projects,
    /// not the least because it is also hard to avoid in a practical manner.
    /// To allow the standard to dictate where this might happen,
    /// gives us a level of control,
    /// which allwos to concentrate these files under one or a few "generated content root dirs",
    /// which in could optionally be turned into git submodules
    /// to improve the overall clone size of a project,
    /// at the cost of the additional complexity of managing submodules.
    pub generated_content: Vec<Rc<PathBuf>>,
    /// The viable paths in the input dir that did not match any record
    /// of the checked standard.
    pub out: Vec<Rc<PathBuf>>,
}

fn create_arbitrary_content_rgxs(tree_recs: &[RNode]) -> Vec<Regex> {
    let mut cont_rgxs = vec![];
    for rec_node in tree_recs.iter() {
        let rec_brw = rec_node.borrow();
        if let Some(rec) = rec_brw.value {
            if let Some(arbitrary_content) = rec.arbitrary_content {
                if arbitrary_content {
                    if let Some(path_regex) = &rec_brw.path_regex {
                        let rgx = if rec.directory {
                            let mut rgx_str = path_regex.0.to_string();
                            // This squeezes in before the final "$"
                            rgx_str.insert_str(rgx_str.len() - 1, "/.*");
                            Regex::new(&rgx_str).unwrap_or_else(|_| {
                                panic!("Bad (assembled) arbitrary content dir regex '{rgx_str}'")
                            })
                        } else {
                            path_regex.0.clone()
                        };
                        cont_rgxs.push(rgx);
                    }
                }
            }
        }
    }
    cont_rgxs
}

fn create_generated_content_rgxs(tree_recs: &[RNode]) -> Vec<Regex> {
    let mut cont_rgxs = vec![];
    for rec_node in tree_recs.iter() {
        let rec_brw = rec_node.borrow();
        if let Some(rec) = rec_brw.value {
            if rec.generated {
                if let Some(path_regex) = &rec_brw.path_regex {
                    let rgx = if rec.directory {
                        let mut rgx_str = path_regex.0.to_string();
                        // This squeezes in before the final "$"
                        rgx_str.insert_str(rgx_str.len() - 1, "/.*");
                        Regex::new(&rgx_str).unwrap_or_else(|_| {
                            panic!("Bad (assembled) generated content dir regex '{rgx_str}'")
                        })
                    } else {
                        path_regex.0.clone()
                    };
                    cont_rgxs.push(rgx);
                }
            }
        }
    }
    cont_rgxs
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
                ignored: Vec::new(),
                arbitrary_content: Vec::new(),
                generated_content: Vec::new(),
                out: Vec::new(),
            },
            ignored_paths: ignored_paths.clone(),
            arbitrary_content_rgxs: None,
            generated_content_rgxs: None,
            records_tree: None,
        }
    }

    /// Creates a map of checkers with one entry for each standard.
    pub fn new_all(ignored_paths: &Regex) -> Vec<Self> {
        let mut checkers = Vec::new();
        for (_std_name, std_records) in super::data::STDS.iter() {
            checkers.push(Self::new(std_records, ignored_paths));
        }
        checkers
    }

    pub fn cover(&mut self, dir_or_file: &Rc<PathBuf>) {
        let dir_or_file_str_lossy = dir_or_file.as_ref().to_string_lossy();
        if self.ignored_paths.is_match(&dir_or_file_str_lossy) {
            self.coverage.ignored.push(Rc::clone(dir_or_file));
            return;
        }
        self.coverage.num_paths += 1;
        let (_recs_tree_root, tree_recs) = self
            .records_tree
            .get_or_insert_with(|| tree::create(self.coverage.std));

        // lazy-init arbitrary_content_rgxs
        if self.arbitrary_content_rgxs.is_none() {
            self.arbitrary_content_rgxs = Some(create_arbitrary_content_rgxs(tree_recs));
        }

        // lazy-init generated_content_rgxs
        if self.generated_content_rgxs.is_none() {
            self.generated_content_rgxs = Some(create_generated_content_rgxs(tree_recs));
        }

        // NOTE This is the version using full(-relative)-path regexes
        //      -> much simpler and so far has more features
        let mut matching = false;
        for rec_node in tree_recs {
            let rec_node_brwd = rec_node.borrow();
            if let Some(path_regex) = &rec_node_brwd.path_regex {
                if path_regex.is_match(dir_or_file_str_lossy.as_ref()) {
                    matching = true;
                    let rec = rec_node_brwd
                        .value
                        .expect("A tree node with path_regex set should never have a None value");
                    self.coverage
                        .r#in
                        .entry(rec)
                        .or_insert_with(Vec::new)
                        .push(Rc::clone(dir_or_file));
                }
            }
        }

        if !matching {
            'cont_types: for (rgx, cont) in vec![
                (
                    self.generated_content_rgxs.as_ref(),
                    &mut self.coverage.generated_content,
                ),
                (
                    self.arbitrary_content_rgxs.as_ref(),
                    &mut self.coverage.arbitrary_content,
                ),
            ] {
                for gen_cont_rgx in rgx.expect("Was initialized further up in this function") {
                    if gen_cont_rgx.is_match(&dir_or_file_str_lossy) {
                        matching = true;
                        cont.push(Rc::clone(dir_or_file));
                        break 'cont_types;
                    }
                }
            }
        }

        if !matching {
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

        let num_out_files = self
            .out
            .iter()
            .filter(|path_buf| path_buf.as_path().is_file())
            .count();
        let neg_rating = num_out_files as f32 * av_ind;
        // trace!("{:#?}", self);
        trace!("ai: {av_ind}");
        trace!("of: {num_out_files}");
        trace!("nr: {neg_rating}");
        trace!("pr: {pos_rating}");
        trace!("out: {:#?}", self.out);

        let total_rating = pos_rating + neg_rating;
        if total_rating > 0.0 {
            pos_rating / total_rating
        } else {
            pos_rating
        }
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
///
/// # Errors
///
/// If any of the input listing entires is an error,
/// usually caused by an I/O issue.
pub fn cover_listing<T, E>(dirs_and_files: T, ignored_paths: &Regex) -> Result<Vec<Coverage>, E>
where
    T: Iterator<Item = Result<Rc<PathBuf>, E>>,
{
    let mut checkers = Checker::new_all(ignored_paths);
    for dir_or_file_res in dirs_and_files {
        let dir_or_file = dir_or_file_res?;
        for checker in &mut checkers {
            checker.cover(&dir_or_file);
        }
    }
    let mut coverages = vec![];
    for checker in checkers {
        coverages.push(checker.coverage);
    }
    Ok(coverages)
}

/// Given a set of the relative paths of all dirs and files in a project,
/// for the given directory standard,
/// calculate what record of the standard each dir or file might be covered under.
///
/// # Errors
///
/// If any of the input listing entires is an error,
/// usually caused by an I/O issue.
pub fn cover_listing_with<T, E>(
    dirs_and_files: T,
    ignored_paths: &Regex,
    std: &'static DirStd,
) -> Result<Coverage, E>
where
    T: Iterator<Item = Result<Rc<PathBuf>, E>>,
{
    let mut checker = Checker::new(std, ignored_paths);
    for dir_or_file_res in dirs_and_files {
        let dir_or_file = dir_or_file_res?;
        checker.cover(&dir_or_file);
    }
    Ok(checker.coverage)
}

/// Given a set of the relative paths of all dirs and files in a project,
/// for each of the known dir standards from
/// <https://github.com/hoijui/osh-dir-std/>,
/// calculate how likely it seems
/// that the project is following this standard,
/// and then only return the coverage for the best fit.
///
/// # Errors
///
/// If any of the input listing entires is an error,
/// usually caused by an I/O issue.
pub fn cover_listing_by_stds<T>(
    dirs_and_files: T,
    ignored_paths: &Regex,
    stds: &Standards,
) -> BoxResult<Vec<Coverage>>
where
    T: Iterator<Item = BoxResult<Rc<PathBuf>>>,
{
    Ok(match stds {
        Standards::Default => {
            let std = STDS
                .get(DEFAULT_STD_NAME)
                .expect("Clap already checked the name!");
            vec![cover_listing_with(dirs_and_files, ignored_paths, std)?]
        }
        Standards::All => cover_listing(dirs_and_files, ignored_paths)?,
        Standards::BestFit => {
            let coverages = cover_listing(dirs_and_files, ignored_paths)?;
            let ratings = coverages.iter().map(Rating::rate_coverage).collect();
            let max_rating = best_fit(&ratings)?;
            coverages
                .into_iter()
                .filter(|cvrg| cvrg.std.name == max_rating.name)
                .collect()
        }
        Standards::Specific(std_name) => {
            let std = STDS.get(std_name).expect("Clap already checked the name!");
            vec![cover_listing_with(dirs_and_files, ignored_paths, std)?]
        }
    })
}
