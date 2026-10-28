fn main() {
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
