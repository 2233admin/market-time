use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn ruleset_file() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock follows Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "market-time-cli-{}-{unique}.json",
        std::process::id()
    ));
    fs::write(
        &path,
        include_str!("../../../examples/synthetic-ruleset.json"),
    )
    .expect("write synthetic ruleset");
    path
}

#[test]
fn phase_command_runs_the_ruleset_to_answer_pipeline() {
    let ruleset = ruleset_file();
    let output = Command::new(env!("CARGO_BIN_EXE_market-time"))
        .args([
            "phase",
            "--ruleset",
            ruleset.to_str().expect("UTF-8 temp path"),
            "--venue",
            "X-MT-DEMO",
            "--at",
            "1970-01-01T00:00:00.000000010Z",
            "--format",
            "json",
        ])
        .output()
        .expect("run market-time CLI");
    fs::remove_file(ruleset).expect("remove synthetic ruleset");

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("CLI emits UTF-8");
    assert!(stdout.contains(r#""status":"known""#), "{stdout}");
    assert!(
        stdout.contains(r#""phase":"continuous_trading""#),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#""nanos_since_unix_epoch":"10""#),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            r#""source_url":"https://raw.githubusercontent.com/2233admin/market-time/main/examples/synthetic-ruleset.json""#
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#""dataset_revisions":["synthetic-r1"]"#),
        "{stdout}"
    );
    assert!(stdout.contains(r#""iana_tzdb_revision":"#), "{stdout}");
}

#[test]
fn phase_command_returns_unknown_at_the_exclusive_coverage_end() {
    let ruleset = ruleset_file();
    let output = Command::new(env!("CARGO_BIN_EXE_market-time"))
        .args([
            "phase",
            "--ruleset",
            ruleset.to_str().expect("UTF-8 temp path"),
            "--venue",
            "X-MT-DEMO",
            "--at",
            "1970-01-01T00:00:00.000000030Z",
            "--format",
            "json",
        ])
        .output()
        .expect("run market-time CLI");
    fs::remove_file(ruleset).expect("remove synthetic ruleset");

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("CLI emits UTF-8");
    assert!(stdout.contains(r#""status":"unknown""#), "{stdout}");
    assert!(stdout.contains(r#""valid_until_ns":"30""#), "{stdout}");
    assert!(
        stdout.contains(r#""dataset_revisions":["synthetic-r1"]"#),
        "{stdout}"
    );
}
