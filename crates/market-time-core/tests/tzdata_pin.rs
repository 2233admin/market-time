//! Guards the tzdata provenance claim.
//!
//! Two distinct failure modes are covered, because they fail differently:
//!
//! 1. The bundled database is absent or empty -- meaning `tzdb-bundle-always` is not
//!    actually active and the host's unpinned zoneinfo is answering instead. Answers
//!    would then vary by machine while provenance still claimed a version.
//! 2. The database is present but reports no release identifier, so an answer cannot
//!    name what produced it.

use market_time_core::tzdata;

#[test]
fn bundled_database_is_populated() {
    let db = jiff::tz::db();
    assert!(
        !db.is_definitively_empty(),
        "time zone database is empty: the build is not using jiff's          `tzdb-bundle-always` feature, so answers would depend on the host"
    );

    // A zone that has existed for decades and is in the launch set.
    assert!(
        db.get("Asia/Shanghai").is_ok(),
        "bundled database does not contain Asia/Shanghai"
    );
    assert!(
        db.get("America/New_York").is_ok(),
        "bundled database does not contain America/New_York"
    );
}

#[test]
fn release_identifier_is_reportable() {
    assert!(
        tzdata::is_verified(),
        "tzdata release is not reportable; provenance would have to be omitted from answers"
    );
    let v = tzdata::iana_tzdb_version().expect("checked by is_verified");
    println!("IANA tzdb release in this build: {v}");
}
