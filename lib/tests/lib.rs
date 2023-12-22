// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use osh_dir_std::{self, format::Rec};

pub type BoxResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn find_rec(std: &str, record_path: &str) -> BoxResult<&'static Rec<'static>> {
    for rec in &osh_dir_std::data::STDS.get(std).unwrap().records {
        if rec.path == record_path {
            return Ok(rec);
        }
    }
    Err(
        format!("Failed to find record with path '{record_path}' in the '{std}' dir standard")
            .into(),
    )
}

#[test]
fn unixish_res_normative() -> BoxResult<()> {
    let rec = find_rec("unixish", "res/")?;
    assert!(rec.normative);
    Ok(())
}

#[test]
fn prusaish_print_bom() -> BoxResult<()> {
    let rec = find_rec("prusaish", "bom/")?;
    println!("{rec:?}");
    Ok(())
}
