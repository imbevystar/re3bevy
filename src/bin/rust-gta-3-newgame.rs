//! Direct New Game development entry point.
use rust_gta_3_core::core::GameState::GtaBootRoute;

fn main() {
    rust_gta_3::App::build_app_with_route(GtaBootRoute::NewGame).run();
}
