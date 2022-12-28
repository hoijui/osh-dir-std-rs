// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
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

mod cli;
mod file_listing;

use std::{collections::HashMap, env, path::PathBuf, str::FromStr};

use clap::ArgMatches;
use cli::{A_L_QUIET, A_L_VERSION};
use osh_dir_std::{constants, data::STDS, rate, BoxResult, Coverage};
use tracing::error;

fn proj_dir(args: &ArgMatches) -> PathBuf {
    let proj_dir = args
        .get_one::<PathBuf>(cli::A_L_PROJECT_DIR)
        .cloned()
        .unwrap_or_else(PathBuf::new);
    // log::debug!("Using repo path '{:#?}'.", &proj_dir);
    proj_dir
}

fn out_file(args: &ArgMatches, out_type: &str) -> PathBuf {
    let out_file = args
        .get_one::<PathBuf>(cli::A_P_OUTPUT)
        .cloned()
        .unwrap_or_else(|| {
            PathBuf::from_str(&format!("{}-{out_type}.json", cli::A_P_D_OUTPUT))
                .expect("How on earth ...")
        });
    // log::debug!("Using output file '{:#?}'.", &out_file);
    out_file
}

fn print_version_and_exit(quiet: bool) {
    #![allow(clippy::print_stdout)]

    if !quiet {
        print!("{} ", clap::crate_name!());
    }
    println!("{}", osh_dir_std::VERSION);
    std::process::exit(0);
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

    if let Some((sub_com_name, sub_com_args)) = args.subcommand() {
        match sub_com_name {
            cli::SC_N_RATE => {
                // let input_path = sub_com
                //     .value_of(cli::A_P_INPUT)
                //     .map(Path::new)
                //     .map(Path::to_path_buf)
                //     .unwrap();
                let out_file = out_file(sub_com_args, cli::SC_N_RATE);
                // let recursive = sub_com_args.is_present(cli::A_L_RECURSIVE);
                // let cont = sub_com_args.is_present(cli::A_L_CONTINUE_ON_ERROR);
                // let overwrite = sub_com_args.is_present(cli::A_L_OVERWRITE);
                let proj_dir = proj_dir(args);
                let rating = rate(proj_dir)?;
                println!("{rating:#?}");
            }
            cli::SC_N_MAP => {
                let out_file = out_file(sub_com_args, cli::SC_N_MAP);
                let proj_dir = proj_dir(args);
                let ignored_paths = &constants::DEFAULT_IGNORED_PATHS;
                let all = sub_com_args.get_flag(cli::A_L_ALL);
                let dirs_and_files = file_listing::dirs_and_files(&proj_dir, ignored_paths)?;
                let coverage: HashMap<String, _> = if all {
                    Coverage::all(&dirs_and_files, ignored_paths)
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
                        Coverage::new(&dirs_and_files, std, ignored_paths),
                    )]
                    .into_iter()
                    .collect()
                };
                todo!();
            }
            _ => {
                error!("Sub-command not implemented: '{sub_com_name}'");
            }
        }
    } else {
        error!(
            "'{}' requires a subcommand but none was provided",
            clap::crate_name!()
        );
        cli::arg_matcher().print_help()?;
        std::process::exit(1);
    }

    Ok(())
}
