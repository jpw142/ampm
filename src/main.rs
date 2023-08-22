#![allow(non_upper_case_globals, non_snake_case, dead_code)]
use bevy::{prelude::*, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, math::Vec3A};
use rayon::prelude::*;
use bytemuck::{Pod, Zeroable};

mod cam;
mod particle;
mod world;

const dt: f32 = 0.4;
const sim_iterations: i32 = (1./dt) as i32;

const gravity: f32 = -0.3;

const rest_density: f32 = 4.0;
const dynamic_viscosity: f32 = 0.1;

const eos_stiffness: f32 = 10.0;
const eos_power: f32 = 4.;


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

