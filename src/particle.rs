use bevy::prelude::*;

#[derive(Component, Clone, Copy, Debug)]
pub struct Particle {
    x: Vec3,
    v: Vec3,    // velocity
    C: Mat3,     // affine momentum matrix
    m: f32,     // mass
}
