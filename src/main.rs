#![allow(non_upper_case_globals, non_snake_case, dead_code)]
use bevy::{prelude::*, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, math::Vec3A};
use rayon::prelude::*;
use bytemuck::{Pod, Zeroable};

mod cam;
mod particle;
mod world;

use crate::world::World;

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
        .add_systems(Update, clear_grid)
        .run();
}

// This is fast enough
fn clear_grid(
    mut world: ResMut<World>
) {
    world.chunks.par_iter_mut().for_each(|chunk| {
        for mut node in chunk.lock().unwrap().nodes {
            node.zero();
        }
    })
}

fn p2g1 (
    mut world: ResMut<World>
) {
    for x_selected in 0..3 {
        for y_selected in 0..3 {
            for z_selected in 0..3 {
                world.chunks.par_iter_mut().for_each(|chunk| {
                    let chunk = chunk.lock().unwrap();
                    let pos_x = chunk.pos.x % 3;
                    let pos_y = chunk.pos.y % 3;
                    let pos_z = chunk.pos.z % 3;
                    // Buffer 2 blocks between active chunks so they don't access the same chunks
                    if (pos_x != x_selected) || (pos_y != y_selected) || (pos_z != z_selected) {
                        return;
                    }
                })















            }
        }
    }

}
