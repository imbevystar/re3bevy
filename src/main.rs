//! re3 reference: references/re-GTA/src/core/main.cpp
//! re3 reference: references/re-GTA/src/core/Game.cpp
//!
//! Binary entry point. GTA behavior is composed in `rust_gta_3::App`.

fn main() {
    eprintln!("rust_gta_3: building Bevy app");
    let mut app = rust_gta_3::App::build_app();
    eprintln!("rust_gta_3: entering Bevy app.run()");
    app.run();
}
