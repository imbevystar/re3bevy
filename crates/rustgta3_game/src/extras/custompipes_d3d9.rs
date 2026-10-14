//! re3 reference: references/re-GTA/src/extras/custompipes_d3d9.cpp
//!
//! D3D9 custom-pipe backend marker. Bevy uses wgpu, so this stores source-backend intent only.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CustomPipesD3d9Backend {
    pub registered: bool,
}

impl CustomPipesD3d9Backend {
    pub fn register(&mut self) {
        self.registered = true;
    }
}
