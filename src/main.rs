#![allow(non_upper_case_globals, non_snake_case, dead_code)]
use bevy::{prelude::*, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, math::{Vec3A, Mat3A}};
use rayon::prelude::*;
use world::{Chunk, Node};
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
        .add_systems(Update, (clear_grid,p2g1).chain())
        .run();
}

fn initialize (
    world: ResMut<World>
) {
    world.chunks.par_iter().for_each(|(_, c)| {
        let mut chunk = c.lock().unwrap();
        for x in 0..Chunk::width {
            for y in 0..Chunk::width {
                for z in 0..Chunk::width {
                    chunk.particles.push(
                        particle::Particle { 
                            x: Vec3A::new(x as f32, y as f32, z as f32), 
                            v: Vec3A::ZERO, 
                            C: Mat3A::ZERO, 
                            m: 1.,
                        }  
                    );
                }
            }
        }
    });
}

fn clear_grid(
    world: ResMut<World>
) {
    world.chunks.par_iter().for_each(|(_, c)| {
        let chunk = c.lock().unwrap();
        // If the chunk shouldn't be updated it shouldn't be cleared
        if !chunk.update {
            return;
        }
        for mut node in chunk.nodes {
            node.zero(); 
        }
    });
}

fn p2g1 (
    world: ResMut<World>,
) {
    for n in 0..Chunk::loopert_width*Chunk::loopert_width*Chunk::loopert_width {
        world.chunks.par_iter().for_each(|(&i, c)| {
            let ch = c.lock().unwrap();

            // If chunk shouldn't be updated or isn't part of the current batch don't do them
            if !ch.update {
                return;
            }
            if ch.loopert != n {
                return;
            }
            drop(ch);

            let locked_chunks = world.get_surrounding_chunks(i);
            // Loop through the particles
            let mut locked_chunk = locked_chunks[Chunk::get_index(3, 1, 1, 1)].lock().unwrap();
            locked_chunk.particles.clone().iter_mut().for_each(|p| {
                // Original node coord
                // All particles must be between 0 and 
                let ogn_coord = p.x.floor(); 
                let ogn_diff = (p.x - ogn_coord) - 0.5;

                let weights = [
                    0.5 * (0.5 - ogn_diff).powf(2.),
                    0.75 - (ogn_diff).powf(2.),
                    0.5 * (0.5 + ogn_diff).powf(2.),
                ];
                

                for gx in 0..3 {
                    for gy in 0..3 {
                        for gz in 0..3 {
                            let weight = weights[gx].x * weights[gy].y * weights[gz].z;

                            let rn_coord = ogn_coord + Vec3A::new(gx as f32 - 1., gy as f32 - 1., gz as f32 - 1.);  
                            let rc_coord = Chunk::in_bounds(rn_coord);

                            let Q = p.C * ogn_diff;

                            let m_contrib = weight * p.m;

                            if rc_coord != IVec3::ZERO {
                                let c_index = Chunk::get_index(3, rc_coord.x + 1, rc_coord.y + 1, rc_coord.z + 1);
                                // -1 + 1 is 0 so if its in a chunk to the right it will be 0
                                // -1 + -1 is -2 and -2.rem(9) is 7 which would be the end
                                let mut outer_chunk_x = rn_coord.x as i32;
                                let mut outer_chunk_y = rn_coord.y as i32;
                                let mut outer_chunk_z = rn_coord.z as i32;

                                if rc_coord.x != 0 {outer_chunk_x = (-1 + rc_coord.x).rem_euclid(Chunk::width as i32 + 1);}
                                if rc_coord.y != 0 {outer_chunk_y = (-1 + rc_coord.y).rem_euclid(Chunk::width as i32 + 1);}
                                if rc_coord.z != 0 {outer_chunk_z = (-1 + rc_coord.z).rem_euclid(Chunk::width as i32 + 1);}

                                let n_index = Chunk::get_index(Chunk::width, outer_chunk_x, outer_chunk_y, outer_chunk_z);
                                let mut outside_chunky = locked_chunks[c_index].lock().unwrap();
                                outside_chunky.nodes[n_index].m += m_contrib;
                                outside_chunky.nodes[n_index].v += m_contrib * (p.v + Vec3A::from(Q));
                            }
                            // If the node is inside the chunk we don't do anything fancy
                            else {
                                let n_index = Chunk::get_index(Chunk::width ,rn_coord.x as i32, rn_coord.y as i32, rn_coord.z as i32);
                                locked_chunk.nodes[n_index].m += m_contrib;
                                locked_chunk.nodes[n_index].v += m_contrib * (p.v + Vec3A::from(Q));
                            }
                        }
                    }
                }
            });
            drop(locked_chunks);
        });
    }
}

fn p2g2 (
    world: ResMut<World>,
) {
}



// // Reads all the surrounding chunks and puts them in a list
// for j in 0..surrounding_chunk_offsets.len(){
//     let coord = i + surrounding_chunk_offsets[j]; 
//     let outside_chunk = world.chunks.get(&coord).unwrap().read().unwrap();
//     locked_chunks.push(outside_chunk);
// }
