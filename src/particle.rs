use bevy::{prelude::*, math::{Vec3A, Mat3A}};

#[derive(Component, Clone, Copy, Debug)]
pub struct Particle {
    pub x: Vec3A,
    pub v: Vec3A,    // velocity
    pub C: Mat3A,     // affine momentum matrix
    pub m: f32,     // mass
}
