#![allow(non_upper_case_globals, non_snake_case, dead_code)]
use bevy::{prelude::*, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, math::{Vec3A, Mat3A}};
use rayon::prelude::*;
use hashbrown::HashMap;
use world::chunk_width;
// RwLock is about 10% or 20% slower than mutex
// However it would allow you to loop through the list of loaded chunks 8 times instead of 27
// Making 1 of both should be trivial and would just take a few changes so I should test it on
// 8x8x8, 16x16x16, and 32x32x32 number chunks
// the chunks can all be 8x8x8 nodes
//
// I also should see if making the world a hashmap would be faster for querying, I think it would
mod cam;
mod particle;
mod world;
mod morton;

use crate::world::World;


const dt: f32 = 0.4;
const sim_iterations: i32 = (1./dt) as i32;

const gravity: f32 = -0.3;

const rest_density: f32 = 4.0;
const dynamic_viscosity: f32 = 0.1;

const eos_stiffness: f32 = 10.0;
const eos_power: f32 = 4.;
// TODO: Fix cell_x accessing other chunks!
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // World Inspector Menu
        //.add_plugin(WorldInspectorPlugin::new())
         // Framerate logging
        .add_plugins((
                LogDiagnosticsPlugin::default(), 
                FrameTimeDiagnosticsPlugin::default(),
                cam::PlayerPlugin,
                ))
        .insert_resource(World::new())
        .insert_resource(ClearColor(Color::rgb(1., 1., 1.)))
        .run();
}


