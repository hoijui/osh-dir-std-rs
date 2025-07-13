// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use regex::Regex;
use serde::Serialize;
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    io,
    path::PathBuf,
    rc::Rc,
};
use thiserror::Error;
use tracing::trace;

use crate::{
    best_fit,
    data::STDS,
    evaluation::{BestFitError, RatingCont},
    stds::Standards,
    tree::{self, RNode},
    Rating, DEFAULT_STD_NAME,
};

use super::format::DirStd;

/// Indicates which relative paths of all dirs and files in a project
/// are covered by what parts of a specific dir standard.
#[derive(Debug)]
pub struct Checker {
    /// the coverage in creation
    coverage: Coverage,
    ignored_paths: Regex,
    arbitrary_content_rgxs: Option<Vec<Regex>>,
    generated_content_rgxs: Option<Vec<Regex>>,
    module_rgxs: Option<Vec<Regex>>,
    modules: HashMap<PathBuf, Checker>,
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
    /// minus the ignored ones,
    /// and excluding paths covered by modules.
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
    /// The coverages for the modules directly included in the root listing;
    /// sub-modules (modules of modules) are contained in the sub coverage.
    /// The path used as key here, is the path of the moduel directory -
    /// modules always are assumed to be rooted in one directory each.
    /// We also assume, that the name of that directory
    /// is the (machine-readable version of) the modules name.
    pub modules: HashMap<PathBuf, Coverage>,
}

fn create_arbitrary_content_rgxs(tree_recs: &[RNode]) -> Vec<Regex> {
    let mut rgxs = vec![];
    for rec_node in tree_recs {
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
                        rgxs.push(rgx);
                    }
                }
            }
        }
    }
    rgxs
}

fn create_generated_content_rgxs(tree_recs: &[RNode]) -> Vec<Regex> {
    let mut rgxs = vec![];
    for rec_node in tree_recs {
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
                    rgxs.push(rgx);
                }
            }
        }
    }
    rgxs
}

fn create_module_rgxs(tree_recs: &[RNode]) -> Vec<Regex> {
    let mut rgxs = HashMap::new();
    log::warn!("module rgxs:");
    for rec_node in tree_recs {
        let rec_brw = rec_node.borrow();
        if let Some(rec) = rec_brw.value {
            if rec.module {
                if let Some(path_regex) = &rec_brw.path_regex {
                    let rgx = if rec.directory {
                        let mut rgx_str = path_regex.0.to_string();
                        // This removes the final "$"
                        rgx_str.remove(rgx_str.len() - 1);
                        rgx_str.insert(rgx_str.len(), '/');
                        log::warn!("{rgx_str}");
                        Regex::new(&rgx_str).unwrap_or_else(|_| {
                            panic!("Bad (assembled) module dir regex '{rgx_str}'")
                        })
                    } else {
                        path_regex.0.clone()
                    };
                    let mut hasher = DefaultHasher::new();
                    rgx.as_str().hash(&mut hasher);
                    rgxs.insert(hasher.finish(), rgx);
                }
            }
        }
    }
    log::warn!("");
    rgxs.into_iter().map(|rgxeq| rgxeq.1).collect()
}

impl Checker {
    /// Given a set of the relative paths of all dirs and files in a project,
    /// figures out which of them are covered by what parts
    /// of a given dir standard.
    pub fn new(std: &'static super::format::DirStd, ignored_paths: &Regex) -> Self {
        Self {
            coverage: Coverage::new(std),
            ignored_paths: ignored_paths.clone(),
            arbitrary_content_rgxs: None,
            generated_content_rgxs: None,
            module_rgxs: None,
            modules: HashMap::new(),
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

        let (_recs_tree_root, tree_recs) = self
            .records_tree
            .get_or_insert_with(|| tree::create(self.coverage.std));

        // lazy-init module_rgxs
        if self.module_rgxs.is_none() {
            self.module_rgxs = Some(create_module_rgxs(tree_recs));
        }

        for mod_rgx in self
            .module_rgxs
            .as_ref()
            .expect("Was initialized further up in this function")
        {
            if let Some(mtch) =
                mod_rgx
                    .captures_iter(&dir_or_file_str_lossy)
                    .next()
                    .map(|capture| {
                        capture
                            .get(0)
                            .expect("If a Module path matches, it should always have a first match")
                    })
            {
                log::warn!("\nmodule related path: {dir_or_file_str_lossy}");
                let mod_dir: PathBuf = mtch.as_str().into();
                let sub_dir_or_file = Rc::new(PathBuf::from(
                    mod_rgx.replace(&dir_or_file_str_lossy, "").as_ref(),
                ));
                log::warn!("      mod_dir: {}", mod_dir.display());
                log::warn!("      mod_dir stripped away: {sub_dir_or_file:?}");
                self.modules
                    .entry(mod_dir)
                    .or_insert_with(|| Self::new(self.coverage.std, &self.ignored_paths))
                    .cover(&sub_dir_or_file);
                return;
            }
        }

        if self.ignored_paths.is_match(&dir_or_file_str_lossy) {
            self.coverage.ignored.push(Rc::clone(dir_or_file));
            return;
        }
        self.coverage.num_paths += 1;

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
                        .or_default()
                        .push(Rc::clone(dir_or_file));
                }
            }
        }

        if !matching {
            let rgxs = self.arbitrary_content_rgxs.as_ref();
            let cont = &mut self.coverage.arbitrary_content;
            for rgx in rgxs.expect("Was initialized further up in this function") {
                if rgx.is_match(&dir_or_file_str_lossy) {
                    matching = true;
                    cont.push(Rc::clone(dir_or_file));
                    break;
                }
            }
        }

        {
            let rgxs = self.generated_content_rgxs.as_ref();
            let cont = &mut self.coverage.generated_content;
            for rgx in rgxs.expect("Was initialized further up in this function") {
                if rgx.is_match(&dir_or_file_str_lossy) {
                    matching = true;
                    cont.push(Rc::clone(dir_or_file));
                    break;
                }
            }
        }

        if !matching {
            self.coverage.out.push(Rc::clone(dir_or_file));
        }
    }

    pub fn coverage(mut self) -> Coverage {
        self.coverage.modules.clear();
        for (mod_path, mod_checker) in self.modules {
            self.coverage
                .modules
                .insert(mod_path, mod_checker.coverage());
        }
        self.coverage
    }
}

impl Coverage {
    #[must_use]
    pub fn new(std: &'static super::format::DirStd) -> Self {
        Self {
            std,
            num_paths: 0,
            r#in: HashMap::new(),
            ignored: Vec::new(),
            arbitrary_content: Vec::new(),
            generated_content: Vec::new(),
            out: Vec::new(),
            modules: HashMap::new(),
        }
    }

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
        // the main rating is the whole rating, excluding the modules
        let main_rating = if total_rating > 0.0 {
            pos_rating / total_rating
        } else {
            pos_rating
        };

        let mut rating_parts = vec![(self.num_paths, main_rating)];
        for mod_coverage in self.modules.values() {
            rating_parts.push((mod_coverage.num_paths, mod_coverage.rate()));
        }
        let num_combined_paths = rating_parts
            .iter()
            .fold(0, |sum, (num_paths, _part_rating)| sum + num_paths)
            as f32;
        let combined_rating = rating_parts
            .iter()
            .fold(0.0, |sum, (num_paths, part_rating)| {
                sum + (part_rating * (*num_paths as f32 / num_combined_paths))
            });
        combined_rating
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
        coverages.push(checker.coverage());
    }
    Ok(coverages)
}

/// Given a set of the relative paths of all dirs and files in a project,
/// for the given directory standard,
/// calculate what record of the standard each dir or file might be covered under.
///
/// # Errors
///
/// If any of the input listing entries is an error,
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
    Ok(checker.coverage())
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
/// and then only return the coverage for the best fit.
///
/// # Errors
///
/// If any of the input listing entries is an error,
/// usually caused by an I/O issue.
///
/// # Panics
///
/// Expecting `Option`s that logically have to be `Some`,
/// thus this should never panic in practice.
pub fn cover_listing_by_stds<T>(
    dirs_and_files: T,
    ignored_paths: &Regex,
    stds: &Standards,
) -> Result<Vec<Coverage>, Error>
where
    T: Iterator<Item = Result<Rc<PathBuf>, io::Error>>,
{
    Ok(match stds {
        Standards::Default => {
            let std = STDS.get(DEFAULT_STD_NAME).expect(
                "This name was chosen from the data itsself, so it should alwyas be available",
            );
            vec![cover_listing_with(dirs_and_files, ignored_paths, std)?]
        }
        Standards::All => cover_listing(dirs_and_files, ignored_paths)?,
        Standards::BestFit => {
            let coverages = cover_listing(dirs_and_files, ignored_paths)?;
            let ratings = coverages
                .into_iter()
                .map(|coverage| RatingCont {
                    rating: Rating::rate_coverage(&coverage),
                    coverage: Some(coverage),
                })
                .collect();
            let max_rating = best_fit(ratings)?;
            vec![max_rating
                .coverage
                .expect("At this point, all coverages have to be present")]
        }
        Standards::Specific(std_name) => {
            let std = STDS.get(std_name).expect("Clap already checked the name!");
            vec![cover_listing_with(dirs_and_files, ignored_paths, std)?]
        }
    })
}
