//! Mark Time — pure phase-resolution core.
//!
//! Given an instant and a venue's ruleset, answer what trading phase that venue is in,
//! with the evidence behind the answer and an honest statement of how precisely it is
//! known.
//!
//! # What this crate will not do
//!
//! Constitution Principle IV: **no I/O, no network, and no clock reads in the decision
//! path.** Instants are passed in; rule data is passed in. That is what makes golden
//! vectors replay deterministically, and it is enforced by the dependency graph rather
//! than by review — see `tests/contract/no_io_no_clock.rs`.
//!
//! Loading rule data is [`market-time-data`]'s job. Converting a non-UTC instant is
//! [`market-time-scales`]' job, and that conversion is leap-second-aware exactly once,
//! at that boundary (research D1a).
//!
//! [`market-time-data`]: https://github.com/2233admin/market-time
//! [`market-time-scales`]: https://github.com/2233admin/market-time

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

pub mod tzdata;
