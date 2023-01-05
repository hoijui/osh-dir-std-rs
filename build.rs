// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::{self, File},
    io::Write,
    path::Path,
    process,
};

#[path = "src/format.rs"]
mod format;

use codify::Codify;

const DIR_STD_DIRS_ROOT: &str = "resources/osh-dir-std/mod/";

fn read_dir_stds() -> Result<HashMap<String, format::DirStandard>, Box<dyn Error>> {
    let mut dir_stds = HashMap::new();
    for f in fs::read_dir(DIR_STD_DIRS_ROOT)? {
        let f = f?;

        if !f.file_type()?.is_dir() {
            continue;
        }

        let def_file = fs::canonicalize(f.path().join("definition.csv"))?;
        println!("cargo:rerun-if-changed={}", def_file.display());
        let dir_standard = format::DirStandard::from_csv_file(&def_file)?;
        dir_stds.insert(f.file_name().to_string_lossy().to_string(), dir_standard);
    }

    Ok(dir_stds)
}

fn transcribe_dir_stds() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("data_gen.rs");

    let mut dir_stds_out = File::create(dest_path)?;

    writeln!(
        &mut dir_stds_out,
        r##"
use std::collections::HashMap;
use once_cell::sync::Lazy;
use regex::Regex;
use crate::format;
    
    "##,
    )?;
    let stds = read_dir_stds()?;
    let mut std_names_sorted: Vec<&String> = stds.keys().collect();
    std_names_sorted.sort();
    writeln!(
        &mut dir_stds_out,
        r##"pub const STD_NAMES: [&str; {}] = {:?};
"##,
        std_names_sorted.len(),
        std_names_sorted,
    )?;
    writeln!(
        &mut dir_stds_out,
        r##"pub static STDS: Lazy<HashMap<String, format::DirStd>> = Lazy::new(|| {});
"##,
        stds.init_code()
    )?;

    Ok(())
}

fn main() {
    if let Err(err) = transcribe_dir_stds() {
        println!("error running transcribe_dir_stds(): {err}");
        process::exit(2);
    }
}
