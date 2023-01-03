<!--
SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>

SPDX-License-Identifier: CC0-1.0
-->

# OSH directory standard - Rust library

[![GitHub license](
    https://img.shields.io/github/license/hoijui/osh-dir-std-rs.svg?style=flat)](
    ./LICENSE)
[![REUSE status](
    https://api.reuse.software/badge/github.com/hoijui/osh-dir-std-rs)](
    https://api.reuse.software/info/github.com/hoijui/osh-dir-std-rs)
[![In cooperation with FabCity Hamburg](
    https://custom-icon-badges.demolab.com/badge/-FCHH-dddddd.svg?logo=fc_logo)](
    https://fabcity.hamburg)

* OSH: _**O**pen **S**ource **H**ardware_

Code that helps humans and machines deal with
the [OSH directory standard](
https://github.com/hoijui/osh-dir-std).

This consists of two parts:

1. a (Rust) library to parse the specification(s) into easily usable structures
2. (**TODO**:) a CLI tool (`osh-dir`) that helps checking a given project-root directory
    against one or multiple specifications.

## Example Usage

### CLI

```shell
ls -1 -d $(git ls-tree -rt HEAD --name-only) | osh-dir-std rate
```

### Library

``` rust
use osh_dir_std::{self, format::Rec};

fn find_rec(std: &str, record_path: &str) -> BoxResult<&'static Rec<'static>> {
    for rec in &osh_dir_std::data::STDS.get(std).unwrap().records {
        if rec.path == record_path {
            return Ok(rec);
        }
    }
    Err(format!(
        "Failed to find record with path '{}' in the '{}' dir standard",
        record_path, std
    )
    .into())
}

#[test]
fn unixish_res_fixed() -> BoxResult<()> {
    let rec = find_rec("unixish", "res/")?;
    assert!(rec.fixed);
    Ok(())
}
```
