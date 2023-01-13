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
2. a CLI tool (`osh-dir-std`) that helps checking a given projects file listing
    against one or multiple specifications.

**NOTE** \
This only checks the paths of files and directories
against a set of presets (aka "standards").
It does *not* check the content of any files in any way.

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

```shell
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

``` rust
use osh_dir_std::{self, BoxResult, format::Rec};

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

## Related Projects

* [`osh`-tool](https://github.com/hoijui/osh-tool) -
  Checks an OSH project against a set of predefined checks,
  to try to assess its overall machine-readability and openness. \
  (also uses this tool internally)
