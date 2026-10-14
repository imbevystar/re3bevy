fn main() {
    // PL_MPEG, https://github.com/phoboslab/pl_mpeg
    // Pinned source: c871f2be022ece7ef4f64230b4fb8e1fb9eb6023 (MIT).
    println!("cargo:rerun-if-changed=native/mpeg.c");
    println!("cargo:rerun-if-changed=native/pl_mpeg.h");
    cc::Build::new()
        .file("native/mpeg.c")
        .opt_level(2)
        .warnings(false)
        .compile("rust_gta_3_mpeg");
}
