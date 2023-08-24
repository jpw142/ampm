#![allow(non_upper_case_globals, non_snake_case, dead_code)]
use bevy::{prelude::*, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, math::{Vec3A, Mat3A}};
use rayon::prelude::*;
use world::chunk_width;

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
        .add_systems(Update, (
                clear_grid, 
                p2g1,
                p2g2,
                update_grid
            ).chain())
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
    world.chunks.par_iter_mut().for_each(|c| {
        let mut chunk = c.lock().unwrap();
        // This clone must be slow TODO fix this clone
        chunk.particles.clone().iter().for_each(|&p| {
            let cell_idx = p.x.floor();
            let cell_diff = (p.x - cell_idx) - 0.5;

            let weights = [
                0.5 * (0.5 - cell_diff).powf(2.),
                0.75 - (cell_diff).powf(2.),
                0.5 * (0.5 + cell_diff).powf(2.),
            ]; 
            for gx in 0..3 {
                for gy in 0..3 {
                    for gz in 0..3 {
                        let weight = weights[gx].x * weights[gy].y * weights[gz].z;

                        let cell_x = Vec3A::from([
                                                 (cell_idx.x + gx as f32 - 1.).floor(), 
                                                 (cell_idx.y + gy as f32 - 1.).floor(),
                                                 (cell_idx.z + gz as f32 - 1.).floor(),
                        ]);
                        let cell_dist = (cell_x - p.x) + 0.5;
                        // TODO Make sure it's component multiplication, aka x*x y*y z*z
                        let Q = p.C * cell_dist;

                        let mass_contrib = weight * p.m;
                        let cell_index = (cell_x.z as usize * chunk_width as usize * chunk_width as usize) + (cell_x.x as usize * chunk_width as usize) + cell_x.y as usize;
                        // TODO This is the only section stopping full parrelelization, can't get lockfree access to grid, maybe atomics?
                        // AtomicPtr
                        chunk.nodes[cell_index as usize].m += mass_contrib;
                        chunk.nodes[cell_index as usize].v += mass_contrib * (p.v + Vec3A::from(Q));
                    }
                }
            }

        })
    })
}

fn p2g2 (
    mut world: ResMut<World>
) { 
    world.chunks.par_iter_mut().for_each(|c| {
        let mut chunk = c.lock().unwrap();
        // This clone must be slow TODO fix this clone
        chunk.particles.clone().iter().for_each(|&p| {
            let cell_idx = p.x.floor();
            let cell_diff = (p.x - cell_idx) - 0.5;

            let weights = [
                0.5 * (0.5 - cell_diff).powf(2.),
                0.75 - (cell_diff).powf(2.),
                0.5 * (0.5 + cell_diff).powf(2.),
            ];
            // estimating particle volume by summing up neighbourhood's weighted mass contribution
            // MPM course, equation 152 
            let mut density: f32 = 0.;
            for gx in 0..3 {
                for gy in 0..3 {
                    for gz in 0..3 {
                        let weight = weights[gx].x * weights[gy].y * weights[gz].z;
                        let cell_x = Vec3A::from([
                                                 (cell_idx.x + gx as f32 - 1.).floor(), 
                                                 (cell_idx.y + gy as f32 - 1.).floor(),
                                                 (cell_idx.z + gz as f32 - 1.).floor(),
                        ]);
                        let cell_index = (cell_x.z as usize * chunk_width as usize * chunk_width as usize) + (cell_x.x as usize * chunk_width as usize) + cell_x.y as usize;
                        density += chunk.nodes[cell_index as usize].m * weight;
                    }
                }
            }
            let volume = p.m / density;
            let pressure = (-0.1_f32).max(eos_stiffness * (density / rest_density).powf(eos_power) - 1.);
            // ! THIS IS 100% WRONG FOR 3D PLEASE HELP
            let mut stress = Mat3A::from_cols_array(&[
                                                    -pressure, 0., 0., 
                                                    0., -pressure, 0.,
                                                    0., 0., -pressure,
            ]);
            let mut strain = p.C;

            let trace = strain.z_axis.x + strain.y_axis.y + strain.x_axis.x;
            strain.x_axis.z = trace;
            strain.y_axis.y = trace;
            strain.z_axis.x = trace;

            let viscosity_term = dynamic_viscosity * strain;
            stress += viscosity_term;

            let eq_16_term_0 = -volume * 4. * stress * dt;

            for gx in 0..3 {
                for gy in 0..3 {
                    for gz in 0..3 {
                        let weight = weights[gx].x * weights[gy].y * weights[gz].z;

                        let cell_x = Vec3A::from([
                                                 (cell_idx.x + gx as f32 - 1.).floor(), 
                                                 (cell_idx.y + gy as f32 - 1.).floor(),
                                                 (cell_idx.z + gz as f32 - 1.).floor(),
                        ]);
                        let cell_index = (cell_x.z as usize * chunk_width as usize * chunk_width as usize) + (cell_x.x as usize * chunk_width as usize) + cell_x.y as usize;
                        let cell_dist = (cell_x - p.x) + 0.5;

                        let momentum = eq_16_term_0 * weight * cell_dist;
                        chunk.nodes[cell_index as usize].v += Vec3A::from(momentum);
                    }
                }
            }
        })
    })
}

fn update_grid (
    mut world: ResMut<World>,
    ) {
    let grid_res = chunk_width;
    world.chunks.par_iter_mut().for_each(|c| {
        let mut locked = c.lock().unwrap();
            for n in &mut locked.nodes.iter_mut().enumerate() {
                let (i, cell) = n;
                if cell.m > 0. {
                    cell.v /= cell.m;
                    cell.v.y += dt * gravity;

                    let z = i /(grid_res as usize * grid_res as usize);
                    let x = (i % (grid_res * grid_res) as usize) / grid_res as usize;
                    let y = i % grid_res as usize;
                    if (x < 2) || (x > (grid_res - 3) as usize) { cell.v.x = 0.}
                    if (y < 2) || (y > (grid_res - 3) as usize) { cell.v.y = 0.}
                    if (z < 2) || (z > (grid_res - 3) as usize) { cell.v.z = 0.}
                }
        }
    })
}
