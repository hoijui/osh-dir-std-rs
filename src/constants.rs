// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use once_cell::sync::Lazy;
use regex::Regex;

pub static DEFAULT_IGNORED_PATHS: Lazy<Regex> =
    Lazy::new(|| Regex::new("^(.git|.gitignore|.gitmodule)$").unwrap());
