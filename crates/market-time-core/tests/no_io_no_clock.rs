//! The guard behind Principle IV.
//!
//! The claim "the core reads no clock and performs no I/O" is worth exactly as much as
//! the thing that checks it. Two checks, both cheap:
//!
//! 1. the crate's declared dependencies stay on an allow-list of crates that cannot read
//!    a clock or open a socket, and
//! 2. no source file in the crate names a clock-reading or filesystem API.
//!
//! A test binary may of course read files; the crate under test may not.

use std::fs;
use std::path::{Path, PathBuf};

/// Crates `market-time-core` may depend on.
///
/// `jiff` performs zone lookups against a database compiled into the binary
/// (`tzdb-bundle-always`), not against the host filesystem, and `jiff-tzdb` is that
/// database. Adding anything to this list is a constitutional decision, not a
/// convenience: it is what keeps a Principle IV violation a compile error.
const ALLOWED_DEPENDENCIES: [&str; 2] = ["jiff", "jiff-tzdb"];

/// APIs that would let the core read a clock or touch the outside world.
const FORBIDDEN_APIS: [&str; 8] = [
    "SystemTime",
    "Instant::now",
    "Zoned::now",
    "Timestamp::now",
    "Epoch::now",
    "std::fs",
    "std::net",
    "std::process",
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn dependencies_stay_on_the_allow_list() {
    let manifest =
        fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("manifest is readable");
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .expect("the crate declares dependencies");

    for line in dependencies.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            if line.starts_with('[') {
                break;
            }
            continue;
        }
        let name = line
            .split(['=', '.'])
            .next()
            .expect("a dependency line names a crate")
            .trim();
        assert!(
            ALLOWED_DEPENDENCIES.contains(&name),
            "market-time-core depends on `{name}`, which is not on the Principle IV \
             allow-list. A dependency capable of I/O or a clock read makes the promise \
             unenforceable; if this is deliberate, amend the constitution first."
        );
    }
}

#[test]
fn no_source_file_names_a_clock_or_the_filesystem() {
    let src = manifest_dir().join("src");
    let mut checked = 0_usize;

    for path in rust_files(&src) {
        let text = fs::read_to_string(&path).expect("source file is readable");
        checked += 1;
        for api in FORBIDDEN_APIS {
            assert!(
                !text.contains(api),
                "{} names `{api}`. The core's decision path takes the instant and the \
                 rule data as arguments; reading either itself is what Principle IV \
                 forbids.",
                path.display()
            );
        }
    }

    assert!(checked > 0, "the guard found no source files to check");
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let entries = fs::read_dir(dir).expect("source directory is readable");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(rust_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
    found
}
