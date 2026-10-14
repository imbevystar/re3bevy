//! re3 reference: references/re-GTA/src/core/main.cpp
//! re3 reference: references/re-GTA/src/core/Game.cpp
//!
//! Binary entry point. GTA behavior is composed in `rustgta3::App`.

fn main() {
    eprintln!("rustgta3: building Bevy app");
    let mut app = rustgta3::App::build_app();
    eprintln!("rustgta3: entering Bevy app.run()");
    app.run();
}
