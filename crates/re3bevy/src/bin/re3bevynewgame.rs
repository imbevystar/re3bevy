//! Direct New Game development entry point.
use re3bevy_core::core::GameState::GtaBootRoute;

fn main() {
    re3bevy::App::build_app_with_route(GtaBootRoute::NewGame).run();
}
