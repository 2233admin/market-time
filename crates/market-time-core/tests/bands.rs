//! Golden vectors for derived session bands and overlap windows (tasks 3.1-3.3).
//!
//! These exercise `market_time_core::{derive_band, derive_overlap}` against the two
//! synthetic band venues in `common`: `SYNTH-BAND-A` trades 08:00-12:00Z, `SYNTH-BAND-B`
//! trades 10:00-14:00Z with a wider published boundary uncertainty than A's, and
//! `SYNTH-BAND-B-GAP` is the same schedule as B with coverage cut off at 11:00Z. Neither
//! is any real venue's schedule — see `common::band_a_venue`'s doc comment.

mod common;

use common::{band_day, band_ruleset, instant};
use market_time_core::{
    BandDefinition, BandError, BandId, BandState, DerivationNote, OverlapState, Uncertainty,
    VenueId, derive_band, derive_overlap, resolve_timeline,
};

fn venue(id: &str) -> VenueId {
    VenueId::new(id).expect("valid identifier")
}

fn band_id(id: &str) -> BandId {
    BandId::new(id).expect("valid identifier")
}

fn derivation(reasoning: &str) -> DerivationNote {
    DerivationNote::new(reasoning).expect("reasoning is present")
}

/// The state and trading venues of the segment covering `at`, panicking (test-only) if no
/// segment covers it — which would itself mean `tiles_interval` is violated.
fn segment_covering<'a>(
    band: &'a market_time_core::SessionBand,
    at: &str,
) -> &'a market_time_core::BandSegment {
    let at = instant(at);
    band.segments()
        .iter()
        .find(|segment| segment.interval.contains(at))
        .unwrap_or_else(|| panic!("no segment covers {at}; tiles_interval should have caught this"))
}

// -------------------------------------------------------- band derivation (3.1, 3.2)

#[test]
fn a_band_from_two_venues_is_trading_across_the_union_and_names_who() {
    let ruleset = band_ruleset();
    let a = venue("SYNTH-BAND-A");
    let b = venue("SYNTH-BAND-B");
    let timeline_a = resolve_timeline(band_day(), &a, &ruleset);
    let timeline_b = resolve_timeline(band_day(), &b, &ruleset);

    let definition = BandDefinition::new(
        band_id("test-band-ab"),
        "Test Band AB",
        vec![a.clone(), b.clone()],
        derivation("union of two synthetic venues for the bands golden-vector suite"),
    )
    .expect("valid definition");

    let band = derive_band(&definition, &[timeline_a, timeline_b]).expect("band derives");

    assert!(
        band.tiles_interval(),
        "a session band must tile its queried interval, same as a Timeline"
    );

    // Before either venue opens: neither trading.
    let before = segment_covering(&band, "2026-01-01T02:00:00Z");
    assert_eq!(before.state, BandState::NotTrading);

    // 09:00Z: only A is trading (08:00-12:00); B is known but closed (it opens at 10:00).
    // Uncertainty still folds over every *known* member regardless of whether it is
    // trading, so B's wider published boundary (10 minutes) already applies here, not just
    // once B starts trading.
    let a_only = segment_covering(&band, "2026-01-01T09:00:00Z");
    assert_eq!(a_only.state, BandState::Trading);
    assert_eq!(a_only.venues_trading, vec![a.clone()]);
    assert!(a_only.venues_unknown.is_empty());
    assert_eq!(
        a_only.uncertainty,
        Some(Uncertainty::minutes(10)),
        "band uncertainty is never narrower than the widest constituent boundary uncertainty"
    );

    // 11:00Z: both open (A 08-12, B 10-14) — the union, not just a touch.
    let both = segment_covering(&band, "2026-01-01T11:00:00Z");
    assert_eq!(both.state, BandState::Trading);
    assert_eq!(both.venues_trading, vec![a.clone(), b.clone()]);
    assert!(
        both.uncertainty
            .as_ref()
            .expect("both members known")
            .is_at_least_as_wide_as(&Uncertainty::minutes(10)),
        "band uncertainty is never narrower than the widest constituent boundary uncertainty"
    );

    // 13:00Z: only B is open.
    let b_only = segment_covering(&band, "2026-01-01T13:00:00Z");
    assert_eq!(b_only.state, BandState::Trading);
    assert_eq!(b_only.venues_trading, vec![b.clone()]);
    assert_eq!(b_only.uncertainty, Some(Uncertainty::minutes(10)));

    // After both close: neither trading again.
    let after = segment_covering(&band, "2026-01-01T20:00:00Z");
    assert_eq!(after.state, BandState::NotTrading);
}

#[test]
fn a_member_out_of_coverage_makes_the_band_unknown_for_exactly_that_stretch() {
    // Task 3.3's vector: SYNTH-BAND-B-GAP's coverage ends at 11:00Z, mid-way through both
    // its own trading window (10:00-14:00) and A's (08:00-12:00).
    let ruleset = band_ruleset();
    let a = venue("SYNTH-BAND-A");
    let b_gap = venue("SYNTH-BAND-B-GAP");
    let timeline_a = resolve_timeline(band_day(), &a, &ruleset);
    let timeline_b_gap = resolve_timeline(band_day(), &b_gap, &ruleset);

    let definition = BandDefinition::new(
        band_id("test-band-gap"),
        "Test Band With Gap",
        vec![a.clone(), b_gap.clone()],
        derivation("A alongside a venue whose coverage ends mid-window, for task 3.3"),
    )
    .expect("valid definition");

    let band = derive_band(&definition, &[timeline_a, timeline_b_gap]).expect("band derives");
    assert!(band.tiles_interval());

    let cutoff = instant("2026-01-01T11:00:00Z");

    // Before the cutoff, the band answers normally: A alone trades 08:00-10:00, both trade
    // 10:00-11:00.
    let before_cutoff = segment_covering(&band, "2026-01-01T09:00:00Z");
    assert_ne!(
        before_cutoff.state,
        BandState::Unknown,
        "coverage holds before the gap opens"
    );
    let just_before_cutoff = segment_covering(&band, "2026-01-01T10:59:00Z");
    assert_eq!(just_before_cutoff.state, BandState::Trading);

    // From 11:00Z on, B is out of coverage. A is still trading until 12:00 and then closed
    // until the next day — but the band must read Unknown throughout, never Trading (drawn
    // from A while A trades) and never NotTrading (drawn from A once A closes).
    for probe in [
        "2026-01-01T11:00:00Z", // A trading, B unknown
        "2026-01-01T11:59:00Z", // A trading, B unknown
        "2026-01-01T13:00:00Z", // A closed, B unknown
        "2026-01-01T23:00:00Z", // A closed, B unknown
    ] {
        let segment = segment_covering(&band, probe);
        assert_eq!(
            segment.state,
            BandState::Unknown,
            "at {probe} the band must be unknown, not narrowed to A alone"
        );
        assert_eq!(segment.venues_unknown, vec![b_gap.clone()]);
    }

    // The stretch strictly before the cutoff is never marked unknown.
    assert!(
        band.segments()
            .iter()
            .filter(|segment| segment.interval.end <= cutoff)
            .all(|segment| segment.state != BandState::Unknown),
        "unknown is scoped to exactly the out-of-coverage stretch"
    );
}

#[test]
fn a_slice_with_every_member_unknown_carries_no_uncertainty_at_all() {
    // SYNTH-BAND-A-GAP's coverage ends at 09:00Z, SYNTH-BAND-B-GAP's at 11:00Z. Between
    // them, 09:00-11:00 has exactly one member unknown (uncertainty still folds from the
    // other, known, member) and 11:00 onward has *both* unknown — the only stretch in the
    // whole suite where `BandSegment::uncertainty` should read `None`, per its doc comment.
    let ruleset = band_ruleset();
    let a_gap = venue("SYNTH-BAND-A-GAP");
    let b_gap = venue("SYNTH-BAND-B-GAP");
    let timeline_a_gap = resolve_timeline(band_day(), &a_gap, &ruleset);
    let timeline_b_gap = resolve_timeline(band_day(), &b_gap, &ruleset);

    let definition = BandDefinition::new(
        band_id("test-band-all-unknown"),
        "Test Band All Unknown",
        vec![a_gap.clone(), b_gap.clone()],
        derivation("both members lose coverage before the day ends, at different points"),
    )
    .expect("valid definition");

    let band = derive_band(&definition, &[timeline_a_gap, timeline_b_gap]).expect("band derives");
    assert!(band.tiles_interval());

    // 09:00-11:00Z: only A-gap is unknown; B-gap is still known (and trading from 10:00).
    let one_unknown = segment_covering(&band, "2026-01-01T09:30:00Z");
    assert_eq!(one_unknown.state, BandState::Unknown);
    assert_eq!(one_unknown.venues_unknown, vec![a_gap.clone()]);
    assert_eq!(
        one_unknown.uncertainty,
        Some(Uncertainty::minutes(10)),
        "one known member (B-gap) still contributes its own uncertainty"
    );

    // From 11:00Z on, both members are unknown: nothing is known about this stretch at
    // all, so uncertainty must be `None`, never a number and never a stale carried-over
    // value from an earlier slice.
    for probe in ["2026-01-01T11:00:00Z", "2026-01-01T23:00:00Z"] {
        let segment = segment_covering(&band, probe);
        assert_eq!(segment.state, BandState::Unknown);
        assert_eq!(segment.venues_unknown, vec![a_gap.clone(), b_gap.clone()]);
        assert!(
            segment.uncertainty.is_none(),
            "at {probe} every member is unknown, so precision is not a meaningful question"
        );
    }
}

// -------------------------------------------------------------------- overlaps (3.2)

#[test]
fn two_bands_overlapping_produce_exactly_the_shared_stretch() {
    let ruleset = band_ruleset();
    let a = venue("SYNTH-BAND-A");
    let b = venue("SYNTH-BAND-B");
    let timeline_a = resolve_timeline(band_day(), &a, &ruleset);
    let timeline_b = resolve_timeline(band_day(), &b, &ruleset);

    let band_a = derive_band(
        &BandDefinition::new(
            band_id("band-x"),
            "Band X",
            vec![a],
            derivation("single-venue band X for the overlap golden vector"),
        )
        .expect("valid definition"),
        &[timeline_a],
    )
    .expect("band derives");

    let band_b = derive_band(
        &BandDefinition::new(
            band_id("band-y"),
            "Band Y",
            vec![b],
            derivation("single-venue band Y for the overlap golden vector"),
        )
        .expect("valid definition"),
        &[timeline_b],
    )
    .expect("band derives");

    let overlap = derive_overlap(&band_a, &band_b).expect("overlap derives");
    assert!(overlap.tiles_interval());
    assert!(!overlap.derivation().reasoning().is_empty());

    let windows: Vec<_> = overlap.windows().collect();
    assert_eq!(
        windows.len(),
        1,
        "exactly one shared stretch is overlapping"
    );

    let window = windows[0];
    assert_eq!(window.interval.start, instant("2026-01-01T10:00:00Z"));
    assert_eq!(window.interval.end, instant("2026-01-01T12:00:00Z"));
    assert!(
        window
            .uncertainty
            .as_ref()
            .expect("both bands known here")
            .is_at_least_as_wide_as(&Uncertainty::minutes(10)),
        "overlap uncertainty is at least as wide as the least precise band boundary involved"
    );
}

#[test]
fn an_unknown_band_makes_the_overlap_unknown_never_not_overlapping() {
    let ruleset = band_ruleset();
    let a = venue("SYNTH-BAND-A");
    let b_gap = venue("SYNTH-BAND-B-GAP");
    let timeline_a = resolve_timeline(band_day(), &a, &ruleset);
    let timeline_b_gap = resolve_timeline(band_day(), &b_gap, &ruleset);

    let band_a = derive_band(
        &BandDefinition::new(
            band_id("band-full"),
            "Band Full Coverage",
            vec![a],
            derivation("single-venue band with full coverage"),
        )
        .expect("valid definition"),
        &[timeline_a],
    )
    .expect("band derives");

    let band_b_gap = derive_band(
        &BandDefinition::new(
            band_id("band-gapped"),
            "Band With Gap",
            vec![b_gap],
            derivation("single-venue band whose coverage ends at 11:00Z"),
        )
        .expect("valid definition"),
        &[timeline_b_gap],
    )
    .expect("band derives");

    let overlap = derive_overlap(&band_a, &band_b_gap).expect("overlap derives");
    assert!(overlap.tiles_interval());

    // After 11:00Z, band_b_gap is unknown. Band A is trading until 12:00 and then closed —
    // both cases must read Unknown, never NotOverlapping just because A itself is not
    // trading in the second half.
    for probe in ["2026-01-01T11:30:00Z", "2026-01-01T13:00:00Z"] {
        let at = instant(probe);
        let segment = overlap
            .segments()
            .iter()
            .find(|segment| segment.interval.contains(at))
            .expect("tiles_interval holds");
        assert_eq!(
            segment.state,
            OverlapState::Unknown,
            "at {probe} an unknown band must win, not read as NotOverlapping"
        );
    }
}

#[test]
fn deriving_an_overlap_of_a_band_with_itself_is_rejected() {
    let ruleset = band_ruleset();
    let a = venue("SYNTH-BAND-A");
    let timeline_a = resolve_timeline(band_day(), &a, &ruleset);

    let band_a = derive_band(
        &BandDefinition::new(
            band_id("same-band"),
            "Same Band",
            vec![a],
            derivation("single-venue band reused on both sides of derive_overlap"),
        )
        .expect("valid definition"),
        &[timeline_a],
    )
    .expect("band derives");

    assert_eq!(
        derive_overlap(&band_a, &band_a),
        Err(BandError::SameBand(band_id("same-band")))
    );
}

// ------------------------------------------------------------- BandDefinition validation

#[test]
fn a_band_definition_rejects_empty_and_duplicate_members() {
    assert_eq!(
        BandDefinition::new(
            band_id("empty"),
            "Empty Band",
            Vec::new(),
            derivation("no members"),
        ),
        Err(BandError::NoMembers)
    );

    let a = venue("SYNTH-BAND-A");
    assert_eq!(
        BandDefinition::new(
            band_id("dup"),
            "Duplicate Band",
            vec![a.clone(), a.clone()],
            derivation("the same venue listed twice"),
        ),
        Err(BandError::DuplicateMember(a))
    );
}

#[test]
fn a_band_definition_carries_its_derivation_note() {
    // A `BandDefinition` cannot be constructed without a `DerivationNote` — the constructor
    // requires one as a parameter, so "derived, unexplained" is not representable, the same
    // guarantee `DerivationNote::new` itself gives (evidence.rs).
    let definition = BandDefinition::new(
        band_id("labelled"),
        "Labelled Band",
        vec![venue("SYNTH-BAND-A")],
        derivation("exactly one member, for inspecting the definition's accessors"),
    )
    .expect("valid definition");

    assert_eq!(definition.id(), &band_id("labelled"));
    assert_eq!(definition.display_name(), "Labelled Band");
    assert_eq!(
        definition.derivation().reasoning(),
        "exactly one member, for inspecting the definition's accessors"
    );
}
