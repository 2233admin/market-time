//! The IANA time-zone database release this build answers from.
//!
//! # Why this exists
//!
//! Constitution Principle III requires every build to report the rule-data revisions it
//! runs against. For exchange calendars that is a dataset revision we author. For
//! time-zone data it is the IANA release, and this module is where that fact comes from.
//!
//! # Read at runtime, not hand-maintained
//!
//! Phase 0 research (D2) concluded that no public version constant existed and that the
//! release would have to be tracked by hand against jiff's changelog. **That conclusion
//! was wrong**, and the correction matters: `jiff-tzdb` does export one.
//!
//! ```text
//! // jiff-tzdb/lib.rs
//! pub static VERSION: Option<&str> = tzname::VERSION;   // Some("2026c") at 0.1.8
//! ```
//!
//! So the release is read from the compiled database itself rather than transcribed into
//! a constant that could silently drift out of step with the dependency. A hand-copied
//! provenance claim that goes stale is worse than none, because it is wrong with
//! confidence — precisely what Principle I forbids.
//!
//! # The build must actually be using the bundled database
//!
//! `jiff` is built with `tzdb-bundle-always` (see the workspace manifest). Without it,
//! jiff's default behaviour on Unix reads the host's `/usr/share/zoneinfo`, which is
//! unpinned — answers would then vary by machine while this module still reported a
//! version, which is the exact failure mode Principle III exists to prevent.
//! `tests/tzdata_pin.rs` asserts the bundled database is present and populated.

/// The IANA tzdb release compiled into this build, e.g. `"2026c"`.
///
/// `None` would mean the bundled database carries no version marker. That is treated as
/// unknown provenance rather than as an absent problem — see [`is_verified`].
#[must_use]
pub fn iana_tzdb_version() -> Option<&'static str> {
    jiff_tzdb::VERSION
}

/// Whether tzdata provenance can be published with an answer.
///
/// Callers attaching provenance MUST check this rather than unwrapping
/// [`iana_tzdb_version`], so an unknown release surfaces as a missing claim instead of a
/// fabricated one.
#[must_use]
pub fn is_verified() -> bool {
    iana_tzdb_version().is_some_and(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_present_and_well_formed() {
        let v = iana_tzdb_version().expect(
            "bundled tzdb reports no version; the build is not using jiff's \
             `tzdb-bundle-always` feature and answers would vary by host",
        );

        // IANA releases are a four-digit year followed by a lowercase letter, e.g. 2026c.
        let mut chars = v.chars();
        let year: String = chars.by_ref().take(4).collect();
        let suffix: String = chars.collect();
        assert!(
            year.len() == 4 && year.chars().all(|c| c.is_ascii_digit()),
            "tzdb version {v:?} does not start with a four-digit year"
        );
        assert!(
            suffix.len() == 1 && suffix.chars().all(|c| c.is_ascii_lowercase()),
            "tzdb version {v:?} does not end with a single lowercase release letter"
        );

        assert!(is_verified());
    }
}
