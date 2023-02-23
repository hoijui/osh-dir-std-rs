// SPDX-FileCopyrightText: 2022 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;
use serde::{Deserialize, Serialize, Serializer};

use codify::Codify;

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("Failed to read data to be parsed (e.g. from a file)")]
    IO(#[from] std::io::Error),

    #[error("Failed to extract directory name from CSV path: '{0}'")]
    DirNameExtraction(PathBuf),

    #[error("Failed to parse CSV: {0}")]
    Csv(#[from] csv::Error),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum OptBool {
    False,
    True,
    #[serde(rename = "-")]
    None,
}

impl From<Option<bool>> for OptBool {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(false) => Self::False,
            Some(true) => Self::True,
            None => Self::None,
        }
    }
}

impl OptBool {
    #[must_use]
    pub const fn init_code(&self) -> &str {
        match self {
            Self::False => "Some(false)",
            Self::True => "Some(true)",
            Self::None => "None",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RegexEq(#[serde(with = "serde_regex")] pub Regex);

impl PartialEq for RegexEq {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Eq for RegexEq {}

impl core::hash::Hash for RegexEq {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}

impl std::ops::Deref for RegexEq {
    type Target = Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct Rec<'a> {
    pub path: &'a str,
    pub fixed: bool,
    pub source: bool,
    pub module: bool,
    pub arbitrary_content: Option<bool>,
    pub indicativeness: f32,
    pub regex: RegexEq,
    pub description: &'a str,
    pub sample_content: &'a str,
}

impl PartialEq for Rec<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for Rec<'_> {}

impl core::hash::Hash for Rec<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl Rec<'_> {
    #[must_use]
    #[allow(dead_code)]
    pub fn to_record(&self) -> Record {
        Record {
            path: self.path.to_owned(),
            fixed: self.fixed,
            source: self.source,
            module: self.module,
            arbitrary_content: self.arbitrary_content.into(),
            indicativeness: self.indicativeness,
            regex: self.regex.0.clone(),
            description: self.description.to_owned(),
            sample_content: self.sample_content.to_owned(),
        }
    }
}

/// We serialize this to only its `path`
/// as a HACK that allows us to implement serializing a [`crate::coverage::Coverage`]
/// to JSON with a shortcut, using serde,
/// without creating and filling an additional struct
/// just for JSON serialization.
impl Serialize for Rec<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.path)
    }
}

// NOTE The field names in this struct are NOT in the same order as
// the fields in the CSV data!
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Record {
    pub path: String,
    pub fixed: bool,
    pub source: bool,
    pub module: bool,
    pub arbitrary_content: OptBool,
    pub indicativeness: f32,
    #[serde(with = "serde_regex")]
    pub regex: Regex,
    pub description: String,
    #[serde(rename(serialize = "Sample Content", deserialize = "Sample Content"))]
    pub sample_content: String,
}

impl Codify for Record {
    fn init_code(&self) -> Cow<'static, str> {
        Cow::Owned(format!(
            r##"format::Rec {{
            path: r#"{}"#,
            fixed: {},
            source: {},
            module: {},
            arbitrary_content: {},
            #[allow(clippy::unreadable_literal)]
            indicativeness: {:#?}_f32,
            regex: format::RegexEq(Regex::new(r#"{}"#).unwrap()),
            description: r#"{}"#,
            sample_content: r#"{}"#,
        }}"##,
            self.path,
            self.fixed,
            self.source,
            self.module,
            self.arbitrary_content.init_code(),
            self.indicativeness,
            self.regex,
            self.description,
            self.sample_content,
        ))
    }
}

#[derive(Debug)]
pub struct DirStd {
    pub name: &'static str,
    pub records: Vec<Rec<'static>>,
}

/// We serialize this to only its `name`
/// as a HACK that allows us to implement serializing a [`crate::coverage::Coverage`]
/// to JSON with a shortcut, using serde,
/// without creating and filling an additional struct
/// just for JSON serialization.
impl Serialize for DirStd {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.name)
    }
}

pub struct DirStandard {
    pub name: String,
    pub records: Vec<Record>,
}

impl Codify for DirStandard {
    fn init_code(&self) -> Cow<'static, str> {
        Cow::Owned(format!(
            r##"format::DirStd {{
            name: "{}",
            records: {},
        }}"##,
            self.name,
            self.records.init_code(),
        ))
    }
}

impl DirStandard {
    /// Reads a directory standard from a CSV source,
    /// as it is used in the hoijui/osh-dir-std repo.
    ///
    /// # Errors
    ///
    /// If There was a problem reading the file,
    /// or parsing it failed.
    /// The most likely reason for the later would be,
    /// that this code is not adjusted to the version of the standards CSV format.
    pub fn from_csv_reader<R: std::io::Read>(
        name: String,
        rdr: &mut csv::Reader<R>,
    ) -> Result<Self, ParseError> {
        let mut records_raw = vec![];
        // with this we ensure, that all the records `indicativeness` values
        // add up to ~= 1.0
        let mut indicativeness_sum = 0.0_f32;
        for result in rdr.deserialize() {
            let record: Record = result?;
            indicativeness_sum += record.indicativeness;
            records_raw.push(record);
            // trace!("{:?}", record);
            // Try this if you don't like each record smushed on one line:
            // trace!("{:#?}", record);
        }
        let mut records = vec![];
        for mut record in records_raw {
            record.indicativeness /= indicativeness_sum;
            // NOTE We do this to force a case insensitive matching, and for the whole string!
            //      see <https://github.com/rust-lang/regex/discussions/737#discussioncomment-264790>
            record.regex = Regex::new(&format!("(?i)^(?:{})$", record.regex)).expect(
                "This should always be a valid regex, if the original was valid, \
                which it has to be, due to being successfully parsed already",
            );
            records.push(record);
        }

        Ok(Self { name, records })
    }

    /// Reads a directory standard from a CSV file,
    /// as it is used in the hoijui/osh-dir-std repo.
    ///
    /// # Errors
    ///
    /// If There was a problem reading the file,
    /// or parsing it failed.
    /// The most likely reason for the later would be,
    /// that this code is not adjusted to the version of the standards CSV format.
    pub fn from_csv_file(csv_file: &Path) -> Result<Self, ParseError> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(fs::File::open(csv_file)?);
        let name = csv_file
            .parent()
            .ok_or_else(|| ParseError::DirNameExtraction(csv_file.to_path_buf()))?
            .file_name()
            .ok_or_else(|| ParseError::DirNameExtraction(csv_file.to_path_buf()))?
            .to_string_lossy()
            .to_string();
        Self::from_csv_reader(name, &mut rdr)
    }
}
