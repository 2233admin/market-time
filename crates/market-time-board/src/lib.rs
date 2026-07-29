//! Mark Time — the global trading-hours board.
//!
//! A shell. It decides *how* to render; the core decides *what* is true.
//!
//! # Rules this surface is bound by
//!
//! - One `now` per render, shared across every venue tile, so the snapshot is coherent.
//! - Displaying "now" obliges surfacing the host's clock discipline bounds as uncertainty.
//!   When discipline data is unavailable the board MUST NOT fall back to presenting the
//!   clock as exact.
//! - An unknown venue renders visually distinct from a closed one. They are different
//!   claims, and the primary consumer is an agent that cannot infer the difference from
//!   styling.
//! - If a display cannot express an honest answer, **the display changes, not the answer**.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
