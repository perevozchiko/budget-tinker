// A build reports its source commit, not a manually maintained release version.
#[test]
fn embedded_commit_is_short_hex_or_explicitly_unknown() {
    let revision = env!("BUDGETTINKER_GIT_SHA");
    assert!(
        revision == "unknown"
            || (revision.len() >= 7 && revision.bytes().all(|byte| byte.is_ascii_hexdigit())),
        "invalid embedded Git revision: {revision}"
    );
}

#[test]
fn embedded_commit_matches_checkout_head_when_available() {
    let revision = env!("BUDGETTINKER_GIT_SHA");
    if revision == "unknown" {
        // Source archives have no .git directory.
        return;
    }

    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        && output.status.success()
    {
        let checked_out = String::from_utf8(output.stdout).expect("Git prints UTF-8 commit SHA");
        assert_eq!(revision, checked_out.trim());
    }
}
