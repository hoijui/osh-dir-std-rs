// SPDX-FileCopyrightText: 2023 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::data::DEFAULT_STD_NAME;
use std::fmt::Display;

#[derive(Default, Clone)]
pub enum Standards {
    #[default]
    Default,
    All,
    BestFit,
    Specific(String),
}

impl Display for Standards {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => write!(f, "<default>({DEFAULT_STD_NAME})"),
            Self::All => write!(f, "<all>"),
            Self::BestFit => write!(f, "<best-fit>(...)"),
            Self::Specific(std_name) => write!(f, "{std_name}"),
        }
    }
}

impl Standards {
    pub fn from_opts(all: bool, best_fit: bool, specific: Option<&String>) -> Self {
        let stds = if all {
            Self::All
        } else if best_fit {
            Self::BestFit
        } else {
            specific
                .cloned()
                .map_or_else(|| Self::Default, Self::Specific)
        };
        log::info!("Using standard(s): {stds}");
        stds
    }
}
