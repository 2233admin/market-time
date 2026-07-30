//! Mark Time — dataset revisions and loading. All I/O lives here.
//!
//! # Why this crate carries the whole ingestion story
//!
//! Every launch venue forbids commercial redistribution of its published schedule, and
//! none carves out factual or calendar data (research D6a):
//!
//! | Venue   | Governing text            | Position                                       |
//! |---------|---------------------------|------------------------------------------------|
//! | SSE     | Trading Rules Art. 5.1.3  | use or publication requires Exchange permission |
//! | NYSE    | ICE Terms of Use          | personal, non-commercial only                   |
//! | Binance | ADGM Terms cl. 27         | non-commercial personal or internal use only    |
//!
//! So **no venue dataset ships in this repository**. Fetch-at-run-time is not one option
//! among several; it is the only compliant shape. Mark Time is a client, not a
//! redistributor — the operator fetches under their own relationship with each venue and
//! owns their own compliance.
//!
//! A source's terms are recorded at registration alongside its evidence, so "under what
//! terms did we obtain this record" is answerable per record, with the same discipline as
//! `source_url` and `fetched_at`.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

pub mod adapters;
pub mod fetch;
pub mod format;
pub mod load;
pub mod revision;

pub use fetch::{FetchError, FetchedDocument, FileFetcher, SourceFetcher, SourceRegistration};
pub use load::{
    LoadError, LoadedDataset, load_dataset, load_ruleset, parse_dataset, parse_ruleset,
};
pub use revision::{AssemblyError, RevisionAssembly, evidence_from};
