// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use regex::Regex;
use std::{fs::{self, ReadDir}, path::{Path, PathBuf}};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("File-system access related error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Not a relative path: '{0}'")]
    RelativePath(PathBuf),
}

struct RecWalk {
    root: PathBuf,
    ignore_paths: Regex,
}

struct List(Vec<PathBuf>);

pub struct RecWalkIterator {
    listing: RecWalk,
    dirs_to_scan: Vec<PathBuf>,
    cur_dir: Option<ReadDir>,
}

impl RecWalkIterator {
    fn new(listing: RecWalk) -> Self {
        Self {
            listing,
            dirs_to_scan: vec![listing.root.to_path_buf()],
            cur_dir: None,
        }
    }
}

impl Iterator for RecWalkIterator {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(cur_dir) = self.dirs_to_scan.pop() {
            for entry_res in fs::read_dir(cur_dir)? {
                let entry = entry_res?;
                let path = entry.path();

                let rel_path = path
                    .strip_prefix(root)
                    .map_or_else(|_e| path.clone(), std::borrow::ToOwned::to_owned);
                if !rel_path.is_relative() {
                    return Err(Error::RelativePath(rel_path));
                }
                if self.listing.ignore_paths.is_match(rel_path.to_string_lossy().as_ref()) {
                    continue;
                }
                rel_listing.push(rel_path);
                if path.is_dir() {
                    self.dirs_to_scan.push(path.clone());
                }
            }
        }
        Ok(rel_listing)
    }
}

impl IntoIterator for RecWalk {
    type Item = PathBuf;

    type IntoIter = RecWalkIterator;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl IntoIterator for List {
    type Item = PathBuf;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}






// pub enum Listing {
//     RecWalk(PathBuf),
//     List(Vec<PathBuf>),
// }

// impl IntoIterator for Listing {
//     type Item = PathBuf;

//     type IntoIter = impl Iterator<Item=Self::Item>;

//     fn into_iter(self) -> Self::IntoIter {
//         match self {
//             Self::RecWalk(root) => {
//                 ListingIterator {
//                     listing: self,
//                     curr: 0,
//                 }
//             },
//             Self::List(paths) => {
//                 paths.into_iter()
//             },
//         }
//         // ListingIterator {
//         //     listing: self,
//         //     curr: 0,
//         // }
//     }
// }

// pub struct ListingIterator {
//     listing: Listing,
//     curr: usize,
// }

// impl Iterator for ListingIterator {
//     type Item = PathBuf;

//     fn next(&mut self) -> Option<Self::Item> {
//         match self.listing {
//             Self::RecWalk(root) => {

//             },
//             Self::List(paths) => {
//                 if
//             },
//         }
//     }

//     fn size_hint(&self) -> (usize, Option<usize>) {
//         (0, None)
//     }

//     fn count(self) -> usize
//     where
//         Self: Sized,
//     {
//         match self {
//             Self::RecWalk(root) => {
//                 Iterator::count(self)
//             },
//             Self::List(paths) => {
//                 paths.len()
//             },
//         }
//     }

//     fn last(self) -> Option<Self::Item>
//     where
//         Self: Sized,
//     {
//         match self {
//             Self::RecWalk(root) => {
//                 Iterator::last(self)
//             },
//             Self::List(paths) => {
//                 paths.last()
//             },
//         }
//     }

//     // fn advance_by(&mut self, n: usize) -> Result<(), usize> {
//     //     match self {
//     //         Self::RecWalk(root) => {
//     //             Iterator::advance_by(self)
//     //         },
//     //         Self::List(paths) => {
//     //             todo!();
//     //             Err(0)
//     //         },
//     //     }
//     // }

//     fn nth(&mut self, n: usize) -> Option<Self::Item> {
//         match self {
//             Self::RecWalk(root) => {
//                 Iterator::nth(self, n)
//             },
//             Self::List(paths) => {
//                 paths.get(n)
//             },
//         }
//     }

//     fn step_by(self, step: usize) -> std::iter::StepBy<Self>
//     where
//         Self: Sized,
//     {
//         match self {
//             Self::RecWalk(root) => {
//                 Iterator::step_by(self, step)
//             },
//             Self::List(paths) => {
//                             todo!()
//             },
//         }
//     }

// }

/// Given a `root`,
/// lists all the contained files and directories recursively.
///
/// # Errors
///
/// This function will return an error in the following situations,
/// but is not limited to just these cases:
///
/// * The provided path doesn't exist.
/// * The process lacks permissions to view the contents.
/// * The path points at a non-directory file.
/// * A path to a dir or file could not be converted to a relative path
pub fn dirs_and_files(root: &Path, ignore_paths: &Regex) -> Result<Vec<PathBuf>, Error> {
    let mut rel_listing = vec![];
    let mut dirs_to_scan = vec![root.to_path_buf()];
    while let Some(cur_dir) = dirs_to_scan.pop() {
        for entry_res in fs::read_dir(cur_dir)? {
            let entry = entry_res?;
            let path = entry.path();

            let rel_path = path
                .strip_prefix(root)
                .map_or_else(|_e| path.clone(), std::borrow::ToOwned::to_owned);
            if !rel_path.is_relative() {
                return Err(Error::RelativePath(rel_path));
            }
            if ignore_paths.is_match(rel_path.to_string_lossy().as_ref()) {
                continue;
            }
            rel_listing.push(rel_path);
            if path.is_dir() {
                dirs_to_scan.push(path.clone());
            }
        }
    }
    Ok(rel_listing)
}
