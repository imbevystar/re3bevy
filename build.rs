// Cargo has no post-build hook. The linker publishes successfully linked game
// binaries; configuration changes also refresh adjacent runtime files. Stable
// markers detect deleted folders without watching executable timestamps, which
// would make every link invalidate the next build.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    for input in [
        "scripts/dist-linker.cmd",
        "scripts/update-dist.ps1",
        "config",
        "dist/.cargo-dist",
    ] {
        println!("cargo::rerun-if-changed={input}");
    }
    for binary in ["rust-gta-3", "rust-gta-3-newgame"] {
        let marker = format!("dist/{binary}/.cargo-dist");
        if std::path::Path::new(&marker).is_file() {
            println!("cargo::rerun-if-changed={marker}");
        }
    }
}
