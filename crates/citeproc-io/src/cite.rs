// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2018 Corporation for Digital Scholarship

use super::output::OutputFormat;
use super::{DateOrRange, NumericValue};
use csl::terms::LocatorType;
use csl::Atom;

pub type CiteId = u32;
pub type ClusterId = u32;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Deserialize)]
pub enum Suppression {
    // For author-in-text, or whatever the style author wants to put inline.
    //
    // E.g. the author, or party names for a case.
    InText,
    // For the rest.
    //
    // E.g. the cite with the author suppressed, or a case without party names.
    Rest,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Deserialize)]
pub struct Locator(pub LocatorType, pub NumericValue);

impl Locator {
    pub fn type_of(&self) -> LocatorType {
        self.0
    }
    pub fn value(&self) -> &NumericValue {
        &self.1
    }
}

use serde::de::{Deserialize, Deserializer};

/// Techincally reference IDs are allowed to be numbers.
fn get_ref_id<'de, D>(d: D) -> Result<Atom, D::Error>
where
    D: Deserializer<'de>,
{
    use super::csl_json::IdOrNumber;
    let s = IdOrNumber::deserialize(d)?;
    Ok(Atom::from(s.0))
}

/// Represents one cite in someone's document, to exactly one reference.
///
/// Prefixes and suffixes
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", bound(deserialize = ""))]
pub struct Cite<O: OutputFormat> {
    #[serde(rename = "citeId")]
    pub id: CiteId,

    #[serde(rename = "id", deserialize_with = "get_ref_id")]
    pub ref_id: Atom,

    #[serde(default)]
    pub prefix: Option<O::Input>,

    #[serde(default)]
    pub suffix: Option<O::Input>,

    #[serde(default)]
    pub suppression: Option<Suppression>,

    // TODO: parse these out of the locator string
    // Enforce len() == 1 in CSL mode
    #[serde(default)]
    pub locators: Vec<Locator>,

    // CSL-M only
    #[serde(default)]
    pub locator_extra: Option<String>,
    // CSL-M only
    #[serde(default)]
    pub locator_date: Option<DateOrRange>,
}

impl<O: OutputFormat> Cite<O> {
    pub fn basic(id: CiteId, ref_id: impl Into<Atom>) -> Self {
        Cite {
            id,
            ref_id: ref_id.into(),
            prefix: Default::default(),
            suffix: Default::default(),
            suppression: None,
            locators: Vec::new(),
            locator_extra: None,
            locator_date: None,
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Ord)]
#[serde(untagged)]
pub enum IntraNote {
    Single(u32),
    Multi(u32, u32),
}

impl IntraNote {
    pub fn note_number(&self) -> u32 {
        match self {
            IntraNote::Single(x) | IntraNote::Multi(x, _) => *x,
        }
    }
}

impl PartialOrd for IntraNote {
    fn partial_cmp(&self, other: &IntraNote) -> Option<Ordering> {
        use IntraNote::*;
        match (self, other) {
            (Single(_), Multi(..)) => Some(Ordering::Less),
            (Multi(..), Single(_)) => Some(Ordering::Greater),
            (Single(a), Single(b)) => a.partial_cmp(b),
            (Multi(a, b), Multi(c, d)) => a.partial_cmp(c).and_then(|e| {
                if e == Ordering::Equal {
                    b.partial_cmp(d)
                } else {
                    Some(e)
                }
            }),
        }
    }
}

#[derive(Deserialize, Ord, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Clone, Copy, Debug)]
pub enum ClusterNumber {
    InText(u32),
    Note(IntraNote),
}

impl ClusterNumber {
    pub fn sub_note(self, note: IntraNote) -> Option<u32> {
        use ClusterNumber::*;
        use IntraNote::*;
        match self {
            Note(self_note) => match (self_note, note) {
                (Single(a), Single(b))
                | (Single(a), Multi(b, _))
                | (Multi(a, _), Single(b))
                | (Multi(a, _), Multi(b, _)) => Some(a - b),
            },
            _ => None,
        }
    }
}

use std::cmp::Ordering;
impl PartialOrd for ClusterNumber {
    fn partial_cmp(&self, other: &ClusterNumber) -> Option<Ordering> {
        use ClusterNumber::*;
        match (self, other) {
            (InText(_), Note(_)) => Some(Ordering::Less),
            (Note(_), InText(_)) => Some(Ordering::Greater),
            (InText(a), InText(b)) => a.partial_cmp(b),
            (Note(a), Note(b)) => a.partial_cmp(b),
        }
    }
}

/// The cluster number affects two things:
///
/// * The ordering of cites
/// * The `first-reference-note-number` variable
/// [`csl::variables::NumberVariable::FirstReferenceNoteNumber`]
///
/// Clusters can appear in footnotes, or in the body text of a document.
/// In JSON, that is `{ "note": 8, "id": ..., "cites": ... }` or `{ "in_text": 5, ...}`.
///
/// Because footnotes can sometimes contain more than one cite cluster, there is a facility for
/// providing one extra value to discriminate between these. The following would be the second
/// cluster in the 8th footnote.
///
/// ```json
/// { "note": [8, 2], ... }
/// ```
///
/// It is up to the library consumer to ensure multi-cluster notes are each updated to include the
/// discriminant, i.e. to swap what was originally `{ "note": 8 }` to `{ "note": [8, 1] }`. Note
/// `8` (without a discriminant) will come before `[8, 1]`, but will not reliably have any ordering
/// with respect to another `8`.
///
/// Similarly, it is up to a library consumer to make sure no clusters have the same number as any
/// other.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged, bound(deserialize = ""))]
pub enum Cluster2<O: OutputFormat> {
    Note {
        note: IntraNote,
        id: ClusterId,
        cites: Vec<Cite<O>>,
    },
    InText {
        in_text: u32,
        id: ClusterId,
        cites: Vec<Cite<O>>,
    },
}

impl<O: OutputFormat> Cluster2<O> {
    pub fn id(&self) -> ClusterId {
        match self {
            Cluster2::InText { id, .. } | Cluster2::Note { id, .. } => *id,
        }
    }
    pub fn cluster_number(&self) -> ClusterNumber {
        match self {
            Cluster2::InText { in_text, .. } => ClusterNumber::InText(*in_text),
            Cluster2::Note { note, .. } => ClusterNumber::Note(*note),
        }
    }
    pub fn split(self) -> (ClusterId, ClusterNumber, Vec<Cite<O>>) {
        match self {
            Cluster2::Note { id, note, cites } => (id, ClusterNumber::Note(note), cites),
            Cluster2::InText { id, in_text, cites } => (id, ClusterNumber::InText(in_text), cites),
        }
    }
}

#[test]
fn json_clusters() {
    use crate::output::html::Html;
    let c: Cluster2<Html> =
        serde_json::from_str(r#"{ "note": 32, "id": 5, "cites": [] }"#).unwrap();
    assert_eq!(
        c,
        Cluster2::Note {
            note: IntraNote::Single(32),
            id: 5,
            cites: vec![]
        }
    );
    let c2: Cluster2<Html> =
        serde_json::from_str(r#"{ "note": [8, 2], "id": 5, "cites": [] }"#).unwrap();
    assert_eq!(
        c2,
        Cluster2::Note {
            note: IntraNote::Multi(8, 2),
            id: 5,
            cites: vec![]
        }
    );
    let c3: Cluster2<Html> =
        serde_json::from_str(r#"{ "in_text": 32, "id": 5, "cites": [] }"#).unwrap();
    assert_eq!(
        c3,
        Cluster2::InText {
            in_text: 32,
            id: 5,
            cites: vec![]
        }
    );
}
