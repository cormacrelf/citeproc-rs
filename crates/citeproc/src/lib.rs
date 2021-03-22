// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2018 Corporation for Digital Scholarship

#[macro_use]
extern crate serde_derive;
// #[macro_use]
// extern crate log;

pub(crate) mod api;
pub(crate) mod processor;

#[cfg(test)]
mod test;

pub use self::api::*;

pub use self::processor::{Processor, InitOptions};

pub mod prelude {
    pub use crate::api::*;
    pub use crate::processor::{Processor, InitOptions};
    pub use citeproc_db::{
        CiteDatabase, CiteId, LocaleDatabase, LocaleFetchError, LocaleFetcher, StyleDatabase,
        IntraNote, ClusterNumber
    };
    pub use citeproc_io::output::{markup::Markup, OutputFormat};
    pub use citeproc_io::{ Cite, Reference, SmartString, };
    pub use citeproc_proc::db::{ImplementationDetails, IrDatabase};
    pub use citeproc_db::PredefinedLocales;
    pub use csl::Atom;
}

pub fn random_cluster_id() -> citeproc_io::SmartString {
    use rand::{thread_rng, Rng};
    use rand::distributions::Alphanumeric;
    let prefix = "cluster-";
    let mut string = citeproc_io::SmartString::from(prefix);
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(23 - prefix.len())
        .for_each(|ch| string.push(ch));
    string
}
