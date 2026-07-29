//! Mark Time — time-scale ingest boundary.
//!
//! Converts an instant tagged with a non-UTC time scale (TAI, GNSS system time, a host
//! monotonic scale) into UTC, leap-second-aware, using hifitime's IERS table.
//!
//! # The seam
//!
//! This is the only place in the tree where that conversion happens, deliberately.
//! Downstream of here everything operates on UTC nanoseconds via `jiff`, which does not
//! model leap seconds — so an instant that crosses this boundary becomes leap-second-naive
//! and cannot round-trip back through a real historical leap second.
//!
//! That is an architectural seam, not a defect, and it is documented rather than papered
//! over (research D1a). No venue phase boundary has ever fallen inside a leap second, so
//! the limitation is acceptable for this domain — but it is asserted by a golden vector
//! rather than assumed.
//!
//! Converting by adding a hardcoded constant is forbidden (Principle II): the UTC-to-GNSS
//! offset is itself a dated fact that changes.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
