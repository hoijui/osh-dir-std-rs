<!--
SPDX-FileCopyrightText: 2022 - 2023 Robin Vobruba <hoijui.quaero@gmail.com>

SPDX-License-Identifier: CC0-1.0
-->

# OSH directory standard - Rust library

[![License: AGPL-3.0-or-later](
    https://img.shields.io/badge/License-AGPL%203.0+-blue.svg)](
    LICENSE.txt)
[![REUSE status](
    https://api.reuse.software/badge/github.com/hoijui/osh-dir-std-rs)](
    https://api.reuse.software/info/github.com/hoijui/osh-dir-std-rs)
[![Repo](
    https://img.shields.io/badge/Repo-GitHub-555555&logo=github.svg)](
    https://github.com/hoijui/osh-dir-std-rs)
[![Package Releases](
    https://img.shields.io/crates/v/osh_dir_std.svg)](
    https://crates.io/crates/osh_dir_std)
[![Documentation Releases](
    https://docs.rs/osh_dir_std/badge.svg)](
    https://docs.rs/osh_dir_std)
[![Dependency Status](
    https://deps.rs/repo/github/hoijui/osh-dir-std-rs/status.svg)](
    https://deps.rs/repo/github/hoijui/osh-dir-std-rs)
[![Build Status](
    https://github.com/hoijui/osh-dir-std-rs/workflows/build/badge.svg)](
    https://github.com/hoijui/osh-dir-std-rs/actions)

[![In cooperation with FabCity Hamburg](
    https://raw.githubusercontent.com/osegermany/tiny-files/master/res/media/img/badge-fchh.svg)](
    https://fabcity.hamburg)
[![In cooperation with Open Source Ecology Germany](
    https://raw.githubusercontent.com/osegermany/tiny-files/master/res/media/img/badge-oseg.svg)](
    https://opensourceecology.de)

* OSH: _**O**pen **S**ource **H**ardware_

Code that helps humans and machines deal with
the [OSH directory standard](
https://github.com/hoijui/osh-dir-std).

This consists of two parts:

1. a (Rust) library to parse the specification(s) into easily usable structures
2. a CLI tool (`osh-dir-std`) that helps checking a given projects file listing
    against one or multiple specifications.

**NOTE** \
This only checks the paths of files and directories
against a set of presets (aka "standards").
It does _not_ check the content of any files in any way.

## Example Usage

### CLI

The tool expects a new-line separated listing of files
(and optionally directories) of the project,
either on [`stdin`](
https://en.wikipedia.org/wiki/Standard_streams#Standard_input_(stdin)),
or in a file given as the first argument.
This list might come from git
(or any other [version control system (VCS)](
https://en.wikipedia.org/wiki/Version_control) used),
the file-system directly,
a ZIP file or even a web-site that lists the files.

A few examples of how to list files in different scenarios,
to rate the project against the known directory standards:

[git](https://git-scm.com/):

```shell
git ls-files --recurse-submodules | sed -e 's/^"\(.*\)"$/\1/' | osh-dir-std rate
```

[SVN](https://subversion.apache.org/):

```shell
svn ls | osh-dir-std rate
```

[Mercurial (`hg`)](https://www.mercurial-scm.org/):

```shell
hg status --all | osh-dir-std rate
```

[pijul](https://pijul.org/):

```shell
pijul list | osh-dir-std rate
```

file-system:

```shell
ls -r1 | osh-dir-std rate
```

sample output:

```json
[
  {
    "name": "unixish",
    "factor": 0.62724684
  },
]
```

A factor of `1.0` would mean that the projects file- and directory structure
adheres 100% to the respective standard.
`unixish` is the name of the default directory standard.

### Library

```rust
use osh_dir_std::{self, format::Rec};

fn find_rec(std: &str, record_path: &str) -> Result<&'static Rec<'static>, String> {
    for rec in &osh_dir_std::data::STDS.get(std).unwrap().records {
        if rec.path == record_path {
            return Ok(rec);
        }
    }
    Err(format!(
        "Failed to find record with path '{record_path}' in the '{std}' dir standard"
    ))
}

#[test]
fn unixish_res_fixed() -> Result<(), Error> {
    let rec = find_rec("unixish", "res/")?;
    assert!(rec.fixed);
    Ok(())
}
```

## Related Projects

* [`osh`-tool](https://github.com/hoijui/osh-tool) -
  Checks an OSH project against a set of predefined checks,
  to try to assess its overall machine-readability and openness. \
  (also uses this tool internally)

## Funding

This project was funded by the European Regional Development Fund (ERDF)
in the context of the [INTERFACER Project](https://www.interfacerproject.eu/),
from November 2022 (project start)
until March 2023.

![Logo of the EU ERDF program](
    https://cloud.fabcity.hamburg/s/TopenKEHkWJ8j5P/download/logo-eu-erdf.png)
