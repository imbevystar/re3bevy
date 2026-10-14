//! Direct New Game development entry point.
use rustgta3_core::core::GameState::GtaBootRoute;

fn main() {
    rustgta3::App::build_app_with_route(GtaBootRoute::NewGame).run();
}
