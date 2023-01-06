// SPDX-FileCopyrightText: 2021-2023 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use clap::{command, value_parser, Arg, ArgAction, ArgGroup, Command, ValueHint};
use const_format::formatcp;
use osh_dir_std::{constants::PROJECT_ISSUES_URL, data::STD_NAMES};
use regex::Regex;
use std::env;

use crate::constants;

pub const SC_N_RATE: &str = "rate";

pub const A_P_OUTPUT: &str = "OUTPUT-FILE";

pub const A_L_VERSION: &str = "version";
pub const A_S_VERSION: char = 'V';

pub const A_L_QUIET: &str = "quiet";
pub const A_S_QUIET: char = 'q';

pub const A_L_INPUT_LISTING: &str = "listing";
pub const A_S_INPUT_LISTING: char = 'I';

pub const SC_N_MAP: &str = "map";

pub const A_L_STANDARD: &str = "standard";
pub const A_S_STANDARD: char = 's';

pub const A_L_ALL: &str = "all";
pub const A_S_ALL: char = 'a';

pub const A_L_IGNORE_PATHS: &str = "ignore-paths-regex";
pub const A_S_IGNORE_PATHS: char = 'i';

fn arg_output() -> Arg {
    Arg::new(A_P_OUTPUT)
        .help("The output file")
        .num_args(1)
        .value_name(A_P_OUTPUT)
        .value_hint(ValueHint::FilePath)
        .value_parser(value_parser!(std::path::PathBuf))
        .value_name("JSON-FILE")
        .action(ArgAction::Set)
        .global(true)
}

fn arg_version() -> Arg {
    #[allow(clippy::indexing_slicing)]
    Arg::new(A_L_VERSION)
        .help("Print version information and exit")
        .long_help(formatcp!(
            "Print version information and exit. \
May be combined with -{A_S_QUIET},--{A_L_QUIET}, to really only output the version string."
        ))
        .short(A_S_VERSION)
        .long(A_L_VERSION)
        .action(ArgAction::SetTrue)
        .global(true)
}

fn arg_quiet() -> Arg {
    Arg::new(A_L_QUIET)
        .help("Much less (or no) command-line output")
        .short(A_S_QUIET)
        .long(A_L_QUIET)
        .action(ArgAction::SetTrue)
        .global(true)
}

fn arg_input_listing() -> Arg {
    Arg::new(A_L_INPUT_LISTING)
        .help("Dirs and files listing to check")
        .long_help(
            "Dirs and files listing to check coverage for. \
Either the path to a file with new-line separated paths, \
or '-' or no argument, meaning the same format is expected on stdin.",
        )
        .short(A_S_INPUT_LISTING)
        .long(A_L_INPUT_LISTING)
        .alias("input-listing")
        .alias("input-lst")
        .alias("in-lst")
        .alias("input")
        .alias("in")
        .alias("lst")
        .num_args(1)
        .value_parser(value_parser!(std::path::PathBuf))
        .value_name("FILE")
        .value_hint(ValueHint::DirPath)
        .action(ArgAction::Set)
        .global(true)
}

fn subcom_rate() -> Command {
    Command::new(SC_N_RATE)
        .about("Rates a project repo directory with all known OSH dir standards, indicating for each standard how well it fits")
        .alias("r")
}

fn subcom_map() -> Command {
    Command::new(SC_N_MAP)
        .about("Maps project directories and files to parts of the standard")
        .alias("m")
}

fn arg_standard() -> Arg {
    Arg::new(A_L_STANDARD)
        .help("Which OSH directory standard to chekc coverage for")
        .num_args(1)
        .short(A_S_STANDARD)
        .long(A_L_STANDARD)
        .alias("std")
        .value_parser(STD_NAMES)
        .value_name("STD")
        .conflicts_with(A_L_ALL)
        .action(ArgAction::Set)
        .global(true)
}

fn arg_all() -> Arg {
    Arg::new(A_L_ALL)
        .help("Check coverage versus all OSH directory standards")
        .short(A_S_ALL)
        .long(A_L_ALL)
        .alias("all-standards")
        .alias("all-stds")
        .conflicts_with(A_L_STANDARD)
        .action(ArgAction::SetTrue)
        .global(true)
}

fn arg_ignore_paths() -> Arg {
    Arg::new(A_L_IGNORE_PATHS)
        .help(format!(
            "Paths to be ignored [default: '{}']",
            constants::DEFAULT_IGNORED_PATHS.as_str()
        ))
        .long_help(format!(
            "Regex capturing all paths to be ignored; \
relative to the project root, like all paths handled by this tool. \
[default: '{}']",
            constants::DEFAULT_IGNORED_PATHS.as_str()
        ))
        .num_args(1)
        .short(A_S_IGNORE_PATHS)
        .long(A_L_IGNORE_PATHS)
        .alias("ign")
        .alias("ignps")
        .alias("ip")
        .alias("ips")
        .value_parser(value_parser!(Regex))
        .value_name("REGEX")
        .action(ArgAction::Set)
        .global(true)
}

pub fn arg_matcher() -> Command {
    command!()
        .help_expected(true)
        .propagate_version(true)
        .subcommand_negates_reqs(true)
        .disable_version_flag(true)
        .disable_help_flag(false)
        .bin_name(clap::crate_name!())
        .before_help(format!(
            "Please leave feedback of any kind here (including bug reports):\n<{PROJECT_ISSUES_URL}>"
        ))
        .after_help("Please use --help for Examples.")
        .after_long_help(format!(
            r#"Examples:
  $ # 1. Lists git tracked files and directories,
  $ #    and rates them with all the known standards:
  $ ls -1 -d $(git ls-tree -rt HEAD --name-only) \
        | {} rate

  $ # 2. Lists git tracked files and directories,
  $ #    and maps them to the default standard:
  $ ls -1 -d $(git ls-tree -rt HEAD --name-only) \
        | {} map
"#,
            clap::crate_name!(),
            clap::crate_name!(),
        ))
        .arg(arg_output().index(1))
        .arg(arg_version())
        .arg(arg_quiet())
        .arg(arg_input_listing())
        .arg(arg_ignore_paths())
        .arg(arg_standard())
        .arg(arg_all())
        .group(
            ArgGroup::new("grp_standard")
                .args([A_L_STANDARD, A_L_ALL])
                .required(true),
        )
        .subcommand(subcom_rate())
        .subcommand(subcom_map())
}
