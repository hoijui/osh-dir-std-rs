// SPDX-FileCopyrightText: 2022-2025 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use regex::Regex;
use std::sync::LazyLock;

pub static DEFAULT_IGNORED_PATHS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(^|.*/)(\\..+)$").unwrap());

pub const PROJECT_ISSUES_URL: &str = "https://github.com/hoijui/osh-dir-std-rs/issues";
