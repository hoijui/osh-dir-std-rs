// SPDX-FileCopyrightText: 2022 - 2023 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod cli;

use std::{
    collections::HashSet,
    env,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
};

use clap::ArgMatches;
use cli::{A_L_INPUT_LISTING, A_L_QUIET, A_L_VERSION};
use once_cell::sync::Lazy;
use osh_dir_std::{
    constants, cover_listing_by_stds,
    format::{Rec, Record},
    rate_listing_by_stds,
    stds::Standards,
    BoxResult, Coverage, RatingCont,
};
use regex::Regex;
use serde::Serialize;
use tracing::{error, metadata::LevelFilter};
use tracing_subscriber::{
    fmt,
    prelude::*,
    reload::{self, Handle},
    Registry,
};

pub static EMPTY_PATH: Lazy<PathBuf> = Lazy::new(PathBuf::new);

fn ignored_paths(args: &ArgMatches) -> Regex {
    let ignored_paths = args
        .get_one::<Regex>(cli::A_L_IGNORE_PATHS)
        .cloned()
        .unwrap_or_else(|| constants::DEFAULT_IGNORED_PATHS.to_owned());
    // log::debug!("Using ignore paths regex: '{:#?}'", &ignored_paths);
    ignored_paths
}

fn input_stream(args: &ArgMatches) -> io::Result<Box<dyn BufRead>> {
    let input_listing = args.get_one::<PathBuf>(A_L_INPUT_LISTING);
    log::info!(
        "Reading input listing from {}.",
        cli_utils::create_input_reader_description(input_listing)
    );
    cli_utils::create_input_reader(input_listing)
}

fn dirs_and_files(
    listing_strm: &mut Box<dyn BufRead>,
) -> impl Iterator<Item = BoxResult<Rc<PathBuf>>> + '_ {
    let lines_iter = cli_utils::lines_iterator(listing_strm, true);
    let no_comments_lines = lines_iter.filter(|line_res| {
        line_res
            .as_ref()
            .map_or(true, |line| !(line.starts_with('#') || line.is_empty()))
    });
    let files = no_comments_lines.fuse().map(line_to_path_res);

    // In case the input-listing only contains files,
    // we also want to iterate over their ancestor dirs,
    // while avoiding duplicate visiting of those.
    // As a side-effect, this also filters out duplicate input of any kind,
    // file or directory.
    // However, this also creates a cache in memory,
    // that in the end will usually be as big as the whole input-listing itsself.
    // TODO Thus we might want to add an option to skip this filtering, in case of large input listings.
    let mut dirs_adder = DirsAdder::new();
    files.flat_map(move |path_res| dirs_adder.call_mut(path_res))
}

fn standards(args: &ArgMatches) -> Standards {
    let all = args.get_flag(cli::A_L_ALL);
    let best_fit = args.get_flag(cli::A_L_BEST_FIT);
    let std = args.get_one::<String>(cli::A_L_STANDARD);
    Standards::from_opts(all, best_fit, std)
}

fn out_stream(args: &ArgMatches) -> io::Result<Box<dyn Write>> {
    let out_stream_id = args.get_one::<PathBuf>(cli::A_P_OUTPUT);
    log::info!(
        "Writing output to {}",
        cli_utils::create_output_writer_description(out_stream_id)
    );
    cli_utils::create_output_writer(out_stream_id)
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

/// Sets up logging, with a way to change the log level later on,
/// and with all output going to stderr,
/// as suggested by <https://clig.dev/>.
///
/// # Errors
///
/// If initializing the registry (logger) failed.
fn setup_logging() -> BoxResult<Handle<LevelFilter, Registry>> {
    let level_filter = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };
    let (l_filter, reload_handle) = reload::Layer::new(level_filter);

    let l_stderr = fmt::layer().map_writer(move |_| io::stderr);

    tracing_subscriber::registry()
        .with(l_filter)
        .with(l_stderr)
        .try_init()?;
    Ok(reload_handle)
}

#[derive(Serialize)]
struct CovEntry {
    name: String,
    coverage: Coverage,
    records: Vec<Record>,
}

impl From<Coverage> for CovEntry {
    fn from(coverage: Coverage) -> Self {
        let records = coverage
            .r#in
            .keys()
            .map(ToOwned::to_owned)
            .map(Rec::to_record)
            .collect::<Vec<_>>();
        Self {
            name: coverage.std.name.to_owned(),
            coverage,
            records,
        }
    }
}

fn main() -> BoxResult<()> {
    let log_reload_handle = setup_logging()?;

    let arg_matcher = cli::arg_matcher();
    let args = &arg_matcher.get_matches();

    let quiet = args.get_flag(A_L_QUIET);
    let version = args.get_flag(A_L_VERSION);
    if version {
        print_version_and_exit(quiet);
    }
    if quiet {
        log_reload_handle.modify(|filter| *filter = LevelFilter::WARN)?;
    }

    let ignored_paths = ignored_paths(args);
    let pretty = true; // TODO Make this a CLI arg

    if let Some((sub_com_name, sub_com_args)) = args.subcommand() {
        let mut listing_strm = input_stream(args)?;
        let dirs_and_files = dirs_and_files(&mut listing_strm);

        let stds = standards(args);

        let mut out_stream = out_stream(args)?;

        match sub_com_name {
            cli::SC_N_RATE => {
                log::info!("Rating listing according to standard(s) ...");
                let mut rating = rate_listing_by_stds(dirs_and_files, &ignored_paths, &stds)?;
                let include_coverage = sub_com_args.get_flag(cli::A_L_INCLUDE_COVERAGE);
                if !include_coverage {
                    rating = rating
                        .into_iter()
                        .map(RatingCont::remove_coverage)
                        .collect();
                }

                log::info!("Converting results to JSON ...");
                let json_rating = if pretty {
                    serde_json::to_string_pretty(&rating)
                } else {
                    serde_json::to_string(&rating)
                }?;
                out_stream.write_all(json_rating.as_bytes())?;
            }
            cli::SC_N_MAP => {
                log::info!("Mapping listing to standard(s) ...");
                let coverage = cover_listing_by_stds(dirs_and_files, &ignored_paths, &stds)?;

                let decorated_cov = coverage.into_iter().map(CovEntry::from).collect::<Vec<_>>();

                log::info!("Converting results to JSON ...");
                let json_coverage = if pretty {
                    serde_json::to_string_pretty(&decorated_cov)
                } else {
                    serde_json::to_string(&decorated_cov)
                }?;
                out_stream.write_all(json_coverage.as_bytes())?;
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

    log::info!("done.");
    Ok(())
}
