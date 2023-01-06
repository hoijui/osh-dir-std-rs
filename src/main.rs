// SPDX-FileCopyrightText: 2022-2023 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![warn(rust_2021_compatibility)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
#![warn(clippy::wildcard_enum_match_arm)]
#![warn(clippy::string_slice)]
#![warn(clippy::indexing_slicing)]
#![warn(clippy::clone_on_ref_ptr)]
#![warn(clippy::try_err)]
#![warn(clippy::shadow_reuse)]
#![warn(clippy::empty_structs_with_brackets)]
#![warn(clippy::else_if_without_else)]
#![warn(clippy::use_debug)]
#![warn(clippy::print_stdout)]
#![warn(clippy::print_stderr)]
#![allow(clippy::default_trait_access)]
// NOTE allowed because:
//      If the same regex is going to be applied to multiple inputs,
//      the precomputations done by Regex construction
//      can give significantly better performance
//      than any of the `str`-based methods.
#![allow(clippy::trivial_regex)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::fn_params_excessive_bools)]
#![allow(clippy::cast_precision_loss)]

mod cli;

use std::{
    collections::{HashMap, HashSet},
    env,
    io::{self, Write},
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
};

use clap::ArgMatches;
use cli::{A_L_INPUT_LISTING, A_L_QUIET, A_L_VERSION};
use once_cell::sync::Lazy;
use osh_dir_std::{
    constants, cover_listing, cover_listing_with, data::STDS, rate_listing, rate_listing_with,
    BoxResult,
};
use regex::Regex;
use tracing::error;

pub static EMPTY_PATH: Lazy<PathBuf> = Lazy::new(PathBuf::new);

fn ignored_paths(args: &ArgMatches) -> Regex {
    let ignored_paths = args
        .get_one::<Regex>(cli::A_L_IGNORE_PATHS)
        .cloned()
        .unwrap_or_else(|| constants::DEFAULT_IGNORED_PATHS.to_owned());
    // log::debug!("Using ignore paths regex: '{:#?}'", &ignored_paths);
    ignored_paths
}

fn out_stream(args: &ArgMatches) -> io::Result<Box<dyn Write>> {
    let out_stream_id = args.get_one::<PathBuf>(cli::A_P_OUTPUT);
    log::info!(
        "Writing output to {}",
        cli_utils::create_output_writer_description(&out_stream_id)
    );
    cli_utils::create_output_writer(&out_stream_id)
}

fn print_version_and_exit(quiet: bool) {
    #![allow(clippy::print_stdout)]

    if !quiet {
        print!("{} ", clap::crate_name!());
    }
    println!("{}", osh_dir_std::VERSION);
    std::process::exit(0);
}

fn line_to_path_res(res_line: io::Result<String>) -> BoxResult<PathBuf> {
    res_line.map_or_else(
        |err| Err(err.into()),
        |mut line| {
            // Removes "./" or ".\" (<- Windows) from the beginning of paths
            if line.starts_with("./") || line.starts_with(".\\") {
                line.pop();
                line.pop();
            }
            PathBuf::from_str(&line).map_err(std::convert::Into::into)
        },
    )
}

struct DirsAdder {
    visited_dirs_cache: HashSet<Rc<PathBuf>>,
}

impl DirsAdder {
    pub fn new() -> Self {
        Self {
            visited_dirs_cache: HashSet::new(),
        }
    }

    pub fn call_mut<P: AsRef<Path>>(
        &mut self,
        path_res: BoxResult<P>,
    ) -> Vec<BoxResult<Rc<PathBuf>>> {
        #[allow(clippy::option_if_let_else)]
        if let Ok(path) = path_res {
            path.as_ref()
                .ancestors()
                .filter(|ancestor| ancestor != &EMPTY_PATH.as_path())
                .map(Path::to_path_buf)
                .map(Rc::new) // We do this to not duplicate memory in cache and the iterator and the coverages
                .filter(|ancestor| self.visited_dirs_cache.insert(Rc::clone(ancestor)))
                .map(Ok)
                .collect::<Vec<BoxResult<_>>>()
        } else {
            vec![path_res
                .map(|path| Path::to_path_buf(path.as_ref()))
                .map(Rc::new)]
        }
    }
}

fn main() -> BoxResult<()> {
    tracing_subscriber::fmt::init();

    let arg_matcher = cli::arg_matcher();
    let args = &arg_matcher.get_matches();

    let quiet = args.get_flag(A_L_QUIET);
    let version = args.get_flag(A_L_VERSION);
    if version {
        print_version_and_exit(quiet);
    }

    let input_listing = args
        .get_one::<String>(A_L_INPUT_LISTING)
        .map(|path_str| path_str as &str);
    let ignored_paths = ignored_paths(args);
    let pretty = true; // TODO Make this a CLI arg

    if let Some((sub_com_name, sub_com_args)) = args.subcommand() {
        log::info!(
            "Reading input listing from {}.",
            cli_utils::create_input_reader_description(&input_listing)
        );
        let mut listing_strm = cli_utils::create_input_reader(&input_listing)?;
        let lines_iter = cli_utils::lines_iterator(&mut listing_strm, true);
        let dirs_and_files = lines_iter.map(line_to_path_res);

        // In case the input-listing only contains files,
        // we also want to iterate over their ancestor dirs,
        // while avoiding duplicate visiting of those.
        // As a side-effect, this also filters out duplicate input of any kind,
        // file or directory.
        // However, this also creates a cache in memory,
        // that in the end will usually be as big as the whole input-listing itsself.
        // TODO Thus we might want to add an option to skip this filtering, in case of large input listings.
        let mut dirs_adder = DirsAdder::new();
        let dirs_and_files = dirs_and_files.flat_map(|path_res| dirs_adder.call_mut(path_res));

        let dirs_and_files = dirs_and_files.collect::<BoxResult<Vec<_>>>()?; // TODO Instead of collecting here, lets get rid of Vecs completely and do everything with Iterators

        let mut out_stream = out_stream(args)?;

        match sub_com_name {
            cli::SC_N_RATE => {
                let rating = rate_listing(dirs_and_files, &ignored_paths);

                let json_rating = if pretty {
                    serde_json::to_string_pretty(&rating)
                } else {
                    serde_json::to_string(&rating)
                }?;
                out_stream.write_all(json_rating.as_bytes())?;
            }
            cli::SC_N_MAP => {
                let all = sub_com_args.get_flag(cli::A_L_ALL);

                let coverage: HashMap<String, _> = if all {
                    cover_listing(dirs_and_files, &ignored_paths)
                        .into_iter()
                        .map(|(k, v)| (k.to_owned(), v))
                        .collect()
                } else {
                    let standard_name = sub_com_args
                        .get_one::<String>(cli::A_L_STANDARD)
                        .cloned()
                        .expect("required argument");
                    let std = STDS
                        .get(&standard_name as &str)
                        .expect("Name was checked by clap, so can not fail");
                    vec![(
                        standard_name,
                        cover_listing_with(dirs_and_files, &ignored_paths, std),
                    )]
                    .into_iter()
                    .collect()
                };

                // TODO Output the coverage/mapping in JSON
                todo!();
            }
            _ => {
                error!("Sub-command not implemented: '{sub_com_name}'");
            }
        }
    } else {
        error!(
            "'{}' requires a subcommand, but none was provided",
            clap::crate_name!()
        );
        cli::arg_matcher().print_help()?;
        std::process::exit(1);
    }

    Ok(())
}
