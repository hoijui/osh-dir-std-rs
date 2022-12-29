// SPDX-FileCopyrightText: 2021-2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use clap::{command, value_parser, Arg, ArgAction, ArgGroup, Command, ValueHint};
use const_format::formatcp;
use osh_dir_std::data::STD_NAMES;
use regex::Regex;
// use const_format::formatcp;
use std::env;

pub const SC_N_RATE: &str = "rate";

// pub const A_P_INPUT: &str = "INPUT";
pub const A_P_OUTPUT: &str = "OUTPUT-FILE";
pub const A_P_D_OUTPUT: &str = ".osh-dir-std";

pub const A_L_VERSION: &str = "version";
pub const A_S_VERSION: char = 'V';

pub const A_L_QUIET: &str = "quiet";
pub const A_S_QUIET: char = 'q';

pub const A_L_PROJECT_DIR: &str = "proj-dir";
pub const A_S_PROJECT_DIR: char = 'C';

pub const SC_N_MAP: &str = "map";

pub const A_L_STANDARD: &str = "standard";
pub const A_S_STANDARD: char = 's';

pub const A_L_ALL: &str = "all";
pub const A_S_ALL: char = 'a';

pub const A_L_IGNORE_PATHS: &str = "ignore-paths-regex";
pub const A_S_IGNORE_PATHS: char = 'i';

// pub const A_L_CONTINUE_ON_ERROR: &str = "continue";
// pub const A_S_CONTINUE_ON_ERROR: char = 'c';

// pub const A_L_OVERWRITE: &str = "overwrite";
// pub const A_S_OVERWRITE: char = 'o';

// pub const SC_N_GENERATE: &str = "gen";

// fn arg_input() -> Arg {
//     Arg::new(A_P_INPUT)
//         .help("The input file or dir path")
//         .num_args(1)
//         .value_name("INPUT")
// .value_hint(ValueHint::DirPath)
// .value_parser(value_parser!(std::path::PathBuf))
//         .required(true)
// }

fn arg_output() -> Arg {
    Arg::new(A_P_OUTPUT)
        .help("The output file or dir path")
        .num_args(1)
        .value_name(A_P_OUTPUT)
        .value_hint(ValueHint::FilePath)
        .value_parser(value_parser!(std::path::PathBuf))
        .action(ArgAction::Set)
        .required(false)
}

// fn arg_recursive() -> Arg {
//     Arg::new(A_L_RECURSIVE)
//         .help("If the input path is a directory, search for input files recursively")
//         .short(A_S_RECURSIVE)
//         .long(A_L_RECURSIVE)
//         .required(false)
// }

fn arg_version() -> Arg {
    Arg::new(A_L_VERSION)
        .help("Print version information and exit. May be combined with --quiet, to really only output the version string.")
        .short(A_S_VERSION)
        .long(A_L_VERSION)
        .action(ArgAction::SetTrue)
}

fn arg_quiet() -> Arg {
    Arg::new(A_L_QUIET)
        .help("Much less (or no) command-line output")
        .short(A_S_QUIET)
        .long(A_L_QUIET)
        .action(ArgAction::SetTrue)
}

fn arg_project_dir() -> Arg {
    Arg::new(A_L_PROJECT_DIR)
        .help("Path of the project repo to check")
        .short(A_S_PROJECT_DIR)
        .long(A_L_PROJECT_DIR)
        .num_args(1)
        .value_parser(value_parser!(std::path::PathBuf))
        .value_name("DIR")
        .value_hint(ValueHint::DirPath)
        .action(ArgAction::Set)
        .default_value(".")
}

fn subcom_rate() -> Command {
    Command::new(SC_N_RATE)
        .about("Rates a project repo directory with all known OSH dir standards, indicating for each standard how well it fits")
}

fn subcom_map() -> Command {
    Command::new(SC_N_MAP)
        .about("Maps project directories and files to parts of the standard")
        .arg(arg_standard())
        .arg(arg_all())
        .group(
            ArgGroup::new("standard")
                .args([A_L_STANDARD, A_L_ALL])
                .required(true),
        )
}

fn arg_standard() -> Arg {
    Arg::new(A_L_STANDARD)
        .help("Which OSH directory standard to chekc coverage for")
        .num_args(1)
        .short(A_S_STANDARD)
        .long(A_L_STANDARD)
        .value_parser(STD_NAMES)
        .conflicts_with(A_L_ALL)
        .action(ArgAction::Set)
}

fn arg_all() -> Arg {
    Arg::new(A_L_ALL)
        .help("Check coverage versus all OSH directory standards")
        .short(A_S_ALL)
        .long(A_L_ALL)
        .conflicts_with(A_L_STANDARD)
        .action(ArgAction::SetTrue)
}

fn arg_ignore_paths() -> Arg {
    Arg::new(A_L_IGNORE_PATHS)
        .help(formatcp!("Regex capturing all paths to be ignored, relative to -{A_S_PROJECT_DIR},--{A_L_PROJECT_DIR}"))
        .num_args(1)
        .short(A_S_IGNORE_PATHS)
        .long(A_L_IGNORE_PATHS)
        .value_parser(value_parser!(Regex))
        .action(ArgAction::Set)
        .global(true)
}

// fn arg_continue_on_error() -> Arg {
//     Arg::new(A_L_CONTINUE_ON_ERROR)
//         .help("If the input path is a directory, continue processing further files, even after an error")
//         .short(A_S_CONTINUE_ON_ERROR)
//         .long(A_L_CONTINUE_ON_ERROR)
//         .required(false)
// }

// fn arg_overwrite() -> Arg {
//     Arg::new(A_L_OVERWRITE)
//         .help("If the outout file alreayd exists, overwrite it, instead of skipping the conversion")
//         .short(A_S_OVERWRITE)
//         .long(A_L_OVERWRITE)
//         .required(false)
// }

// fn subcom_validate() -> App {
//     App::new(SC_N_VALIDATE)
//     .about("Validates manifest files for validity using JSON Schema (currently supports OKH-v1 and OKH-LOSH)")
//     .arg(arg_input().index(1))
//     .arg(arg_okhv())
//     .arg(arg_recursive())
//     .arg(arg_continue_on_error())
// }

pub fn arg_matcher() -> Command {
    command!()
        .help_expected(true)
        .propagate_version(true)
        .subcommand_negates_reqs(true)
        .disable_version_flag(true)
        .disable_help_flag(false)
        .bin_name(clap::crate_name!())
        .arg(arg_output().index(1))
        .arg(arg_version())
        .arg(arg_quiet())
        .arg(arg_project_dir())
        .arg(arg_ignore_paths())
        .subcommand(subcom_rate())
        .subcommand(subcom_map())
}
