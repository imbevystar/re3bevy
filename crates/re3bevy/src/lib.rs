#![allow(non_snake_case)]

pub mod App;

#[cfg(target_os = "android")]
#[bevy::prelude::bevy_main]
fn main() {
    App::build_app().run();
}
