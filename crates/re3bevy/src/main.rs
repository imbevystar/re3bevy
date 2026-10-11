//! re3 reference: references/re-GTA/src/core/main.cpp
//! re3 reference: references/re-GTA/src/core/Game.cpp
//!
//! Binary entry point. GTA behavior is composed in `re3bevy::App`.

fn main() {
    eprintln!("re3bevy: building Bevy app");
    let mut app = re3bevy::App::build_app();
    eprintln!("re3bevy: entering Bevy app.run()");
    app.run();
}
