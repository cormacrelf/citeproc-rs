// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2019 Corporation for Digital Scholarship

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use super::processor::Interner;
use citeproc_db::ClusterId as ClusterIdInternal;
use citeproc_io::output::{markup::Markup, OutputFormat};
use citeproc_io::{Cite, ClusterMode, SmartString};
use csl::Atom;
use fnv::FnvHashMap;
use std::str::FromStr;
use std::sync::Arc;

/// A symbol that identifies a cluster; a newtyped u32. This corresponds to an interned string
/// identifier, but `citeproc_db` is not responsible for interning those ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClusterId(ClusterIdInternal);

impl ClusterId {
    pub(crate) fn new(internal: ClusterIdInternal) -> Self {
        ClusterId(internal)
    }
    pub(crate) fn raw(&self) -> ClusterIdInternal {
        self.0
    }
}

/// See [Special Citation Forms](https://citeproc-js.readthedocs.io/en/latest/running.html#special-citation-forms)
///
///
/// ```
/// use serde::Deserialize;
/// use citeproc_io::{Cite, ClusterMode, output::markup::Markup};
/// use citeproc::Cluster;
/// let json = r#"
/// [ { "id": 1, "cites": [{ "id": "smith" }] }
/// , { "id": 2, "cites": [{ "id": "smith" }], "mode": "AuthorOnly" }
/// , { "id": 2, "cites": [{ "id": "smith" }], "mode": "SuppressAuthor" }
/// , { "id": 3, "cites": [{ "id": "smith" }, { "id": "jones" }],
///     "mode": "SuppressAuthor", "suppressFirst": 2 }
/// , { "id": 4, "cites": [{ "id": "smith" }], "mode": "Composite" }
/// , { "id": 5, "cites": [{ "id": "smith" }, { "id": "jones" }],
///     "mode": "Composite", "suppressFirst": 2 }
/// ]"#;
/// let clusters: Vec<Cluster<Markup, i32>> = serde_json::from_str(json).unwrap();
/// use pretty_assertions::assert_eq;
/// assert_eq!(clusters, vec![
///     Cluster { id: 1, cites: vec![Cite::basic("smith")], mode: None, },
///     Cluster { id: 2, cites: vec![Cite::basic("smith")], mode: Some(ClusterMode::AuthorOnly), },
///     Cluster { id: 2, cites: vec![Cite::basic("smith")], mode: Some(ClusterMode::SuppressAuthor
///     { suppress_first: 1 }), },
///     Cluster { id: 3, cites: vec![Cite::basic("smith"), Cite::basic("jones")],
///               mode: Some(ClusterMode::SuppressAuthor { suppress_first: 2 }), },
///     Cluster { id: 4, cites: vec![Cite::basic("smith")], mode: Some(ClusterMode::Composite
///     { infix: None, suppress_first: 1 }), },
///     Cluster { id: 5, cites: vec![Cite::basic("smith"), Cite::basic("jones")],
///               mode: Some(ClusterMode::Composite { infix: None, suppress_first: 2 }), },
/// ])
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(bound(
    serialize = "Id: serde::Serialize",
    deserialize = "Id: serde::Deserialize<'de>"
))]
pub struct Cluster<O: OutputFormat = Markup, Id = ClusterId> {
    pub id: Id,
    pub cites: Vec<Cite<O>>,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<ClusterMode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterPosition {
    pub id: ClusterId,
    /// If this is None, the piece is an in-text cluster. If it is Some, it is a note cluster.
    pub note: Option<u32>,
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum ReorderingError {
    #[error(
        "set_cluster_order called with a note number {0} that was out of order (e.g. [1, 2, 3, 1])"
    )]
    NonMonotonicNoteNumber(u32),
    #[error("call to preview_citation_cluster must provide exactly one preview position")]
    DidNotSupplyZeroPosition,
    #[error("non-existent cluster {0:?}")]
    NonExistentCluster(ClusterId),
}

impl ReorderingError {
    pub(crate) fn to_external(self, interner: &Interner) -> string_id::ReorderingError {
        match self {
            ReorderingError::NonExistentCluster(id) => {
                if let Some(string) = interner.resolve(id.raw()) {
                    string_id::ReorderingError::NonExistentCluster(SmartString::from(string))
                } else {
                    string_id::ReorderingError::Internal(self)
                }
            }
            _ => string_id::ReorderingError::Internal(self),
        }
    }
}

pub mod string_id {
    //! This is the API using string IDs only, useful for exposing citeproc-rs to non-Rust
    //! consumers.
    use super::{BibEntry, BibliographyUpdate};
    use serde::{Deserialize, Serialize};
    use citeproc_io::{
        output::{markup::Markup, OutputFormat},
        SmartString,
    };
    use fnv::FnvHashMap;
    use std::sync::Arc;

    pub type Cluster<O = Markup> = super::Cluster<O, SmartString>;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct ClusterPosition {
        pub id: Option<SmartString>,
        /// If this is None, the piece is an in-text cluster. If it is Some, it is a note cluster.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub note: Option<u32>,
    }

    #[derive(Default, Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateSummary<O: OutputFormat = Markup> {
        /// A list of clusters that were updated, paired with the formatted output for each
        pub clusters: Vec<(SmartString, Arc<O::Output>)>,
        pub bibliography: Option<BibliographyUpdate>,
    }

    #[derive(Serialize, Default, Debug, Clone, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct FullRender {
        pub all_clusters: FnvHashMap<SmartString, Arc<SmartString>>,
        pub bib_entries: Vec<BibEntry<Markup>>,
    }

    #[derive(Debug, thiserror::Error)]
    pub enum ReorderingError {
        #[error("{0}")]
        Internal(#[from] super::ReorderingError),
        #[error("non-existent cluster id {0:?}")]
        NonExistentCluster(SmartString),
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SecondFieldAlign {
    Flush,
    Margin,
}

/// Mostly imitates the citeproc-js API.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BibliographyMeta<O: OutputFormat = Markup> {
    pub max_offset: u32,
    /// Represents line spacing between entries
    pub entry_spacing: u32,
    /// Represents line spacing within entries
    pub line_spacing: u32,
    /// Whether hanging-indent should be applied
    pub hanging_indent: bool,

    // XXX: the CSL spec does a bad job explaining how to implement this.
    /// When the second-field-align CSL option is set, this returns either “flush” or “margin”.
    /// The calling application should align text in bibliography output as described in the CSL specification.
    /// Where second-field-align is not set, this is undefined.
    pub second_field_align: Option<SecondFieldAlign>,

    /// Contains information along the lines of citeproc-js' `bibstart` and `bibend` strings for
    /// open and close tags
    pub format_meta: O::BibMeta,
}

#[derive(Clone, Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BibliographyUpdate<O: OutputFormat = Markup> {
    /// Contains Reference Ids mapped to their bibliography outputs
    pub updated_entries: FnvHashMap<Atom, Arc<O::Output>>,
    /// None if the sort is the same, otherwise contains all entries in order
    /// Entries that cease to be present in the list between updates are considered to have been removed.
    pub entry_ids: Option<Vec<Atom>>,
}

impl BibliographyUpdate {
    pub fn new() -> Self {
        BibliographyUpdate::default()
    }
}

#[derive(Default, Debug, Clone)]
pub struct UpdateSummary<O: OutputFormat = Markup> {
    /// A list of clusters that were updated, paired with the formatted output for each
    pub clusters: Vec<(ClusterId, Arc<O::Output>)>,
    pub bibliography: Option<BibliographyUpdate>,
}

#[derive(Serialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BibEntry<O: OutputFormat = Markup> {
    pub id: Atom,
    pub value: Arc<O::Output>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct FullRender {
    pub all_clusters: FnvHashMap<ClusterId, Arc<SmartString>>,
    pub bib_entries: Vec<BibEntry<Markup>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, Ord, PartialOrd, PartialEq)]
pub enum IncludeUncited {
    /// The default
    None,
    /// All references, cited or not, are included in the bibliography.
    All,
    /// Specifically these references are included in the bibliography whether cited or not.
    Specific(Vec<String>),
}

impl Default for IncludeUncited {
    fn default() -> Self {
        IncludeUncited::None
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SupportedFormat {
    Html,
    Rtf,
    Plain,
    TestHtml,
}

impl SupportedFormat {
    pub fn make_markup(&self) -> Markup {
        match self {
            SupportedFormat::Html => Markup::html(),
            SupportedFormat::Rtf => Markup::rtf(),
            SupportedFormat::Plain => Markup::plain(),
            SupportedFormat::TestHtml => Markup::test_html(),
        }
    }
}

impl FromStr for SupportedFormat {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "html" => Ok(SupportedFormat::Html),
            "rtf" => Ok(SupportedFormat::Rtf),
            "plain" => Ok(SupportedFormat::Plain),
            _ => Err(()),
        }
    }
}

impl<'de> serde::de::Deserialize<'de> for SupportedFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        use serde::de::Error as DeError;
        SupportedFormat::from_str(s.as_str())
            .map_err(|()| DeError::custom(format!("unknown format {}", s.as_str())))
    }
}

pub enum PreviewPosition<'a> {
    /// Convenience, if your user is merely editing a cluster.
    ReplaceCluster(ClusterId),
    /// Full power method, temporarily renumbers the entire document. You specify where the preview
    /// goes by setting the id to 0 in one of the `ClusterPosition`s. Thus, you can replace a
    /// cluster (by omitting the original), but also insert one at any position, including in a new
    /// note or in-text position, or even between existing clusters in an existing note.
    MarkWithZero(&'a [ClusterPosition]),
    MarkWithZeroStr(&'a [string_id::ClusterPosition]),
}
