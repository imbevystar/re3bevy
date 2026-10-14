//! re3 reference: references/re-GTA/src/extras/custompipes_gl.cpp
//!
//! GL custom-pipe backend marker. Bevy uses wgpu, so this stores source-backend intent only.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CustomPipesGlBackend {
    pub registered: bool,
}

impl CustomPipesGlBackend {
    pub fn register(&mut self) {
        self.registered = true;
    }
}
