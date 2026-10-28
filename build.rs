// Keep the build commit identifiable without changing the package version.
fn git_output(root: &std::path::Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let output = String::from_utf8(output.stdout).ok()?;
    let output = output.trim();
    (!output.is_empty()).then(|| output.to_owned())
}

fn watch_git_path(root: &std::path::Path, name: &str) {
    if let Some(path) = git_output(root, &["rev-parse", "--git-path", name]) {
        let path = std::path::PathBuf::from(path);
        let path = if path.is_absolute() { path } else { root.join(path) };
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn embed_git_revision() {
    let root = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR"),
    );
    // Do not accidentally report the parent repository's SHA for an unpacked source ZIP.
    let in_repo = git_output(&root, &["rev-parse", "--show-toplevel"])
        .and_then(|path| std::path::PathBuf::from(path).canonicalize().ok())
        == root.canonicalize().ok();

    let sha = if in_repo {
        // HEAD itself does not change when a branch advances, so also track its ref.
        watch_git_path(&root, "HEAD");
        if let Some(branch) = git_output(&root, &["symbolic-ref", "-q", "HEAD"]) {
            watch_git_path(&root, &branch);
        }
        // Handles packed references; detached HEAD is covered by HEAD above.
        watch_git_path(&root, "packed-refs");
        git_output(&root, &["rev-parse", "--short=7", "HEAD"])
    } else {
        None
    };
    let sha = sha
        .filter(|value| value.len() >= 7 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=BUDGETTINKER_GIT_SHA={sha}");
}

fn main() {
    embed_git_revision();
    println!("cargo:rerun-if-changed=scripts/report-pdf.swift");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("budgettinker-report-pdf");
        let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
            "aarch64" => "arm64",
            "x86_64" => "x86_64",
            other => panic!("Unsupported macOS architecture: {other}"),
        };
        let status = std::process::Command::new("xcrun")
            .args([
                "swiftc",
                "-O",
                "-target",
                &format!("{arch}-apple-macosx12.0"),
                "scripts/report-pdf.swift",
                "-o",
            ])
            .arg(output)
            .status()
            .expect("Swift compiler is required for the macOS PDF renderer");
        assert!(status.success(), "failed to build the macOS PDF renderer");
    }
}
