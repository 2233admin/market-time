//! Venue adapters: reading a venue's own publication into rules.
//!
//! An adapter is a parser plus a mapping. It carries no schedule: the times live in the
//! document the operator fetched, and what this repository contributes is the reading —
//! which published session name corresponds to which phase in the shared vocabulary, and
//! a refusal to invent whatever the document does not say.
//!
//! Adding a venue means adding an adapter here plus a source registration; it never means
//! touching the phase vocabulary or the resolver.

pub mod sse;
