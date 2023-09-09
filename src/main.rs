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
        .add_systems(Startup, initialize)
        .add_systems(Update, ((clear_grid, p2g1, p2g2, update_grid, g2p).chain(), draw))
        .run();
}

fn initialize (
    world: ResMut<World>
) {
    world.chunks.par_iter().for_each(|(_, c)| {
        let mut chunk = c.lock().unwrap();
        if chunk.update == false {
            return;
        }
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
                            
                            let node_dist = (rn_coord - p.x) + 0.5;

                            let Q = p.C * node_dist;

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

                let mut density: f32 = 0.;
                for gx in 0..3 {
                    for gy in 0..3 {
                        for gz in 0..3 {
                            let weight = weights[gx].x * weights[gy].y * weights[gz].z;

                            let rn_coord = ogn_coord + Vec3A::new(gx as f32 - 1., gy as f32 - 1., gz as f32 - 1.);  
                            let rc_coord = Chunk::in_bounds(rn_coord);

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
                                let outside_chunky = locked_chunks[c_index].lock().unwrap();
                                density += outside_chunky.nodes[n_index].m * weight;
                            }
                            // If the node is inside the chunk we don't do anything fancy
                            else {
                                let n_index = Chunk::get_index(Chunk::width ,rn_coord.x as i32, rn_coord.y as i32, rn_coord.z as i32);
                                density += locked_chunk.nodes[n_index].m * weight;
                            }
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
                            let rn_coord = ogn_coord + Vec3A::new(gx as f32 - 1., gy as f32 - 1., gz as f32 - 1.);  
                            let rc_coord = Chunk::in_bounds(rn_coord);

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
                                 
                                let cell_dist = (Vec3A::new(outer_chunk_x as f32, outer_chunk_y as f32, outer_chunk_z as f32) - p.x) + 0.5;
                                let momentum = eq_16_term_0 * weight * cell_dist;

                                let n_index = Chunk::get_index(Chunk::width, outer_chunk_x, outer_chunk_y, outer_chunk_z);
                                let mut outside_chunky = locked_chunks[c_index].lock().unwrap();
                                outside_chunky.nodes[n_index].v += Vec3A::from(momentum);
                            }
                            // If the node is inside the chunk we don't do anything fancy
                            else {
                                let cell_dist = (Vec3A::new(rn_coord.x as f32, rn_coord.y as f32, rn_coord.z as f32) - p.x) + 0.5;
                                let momentum = eq_16_term_0 * weight * cell_dist;
                                let n_index = Chunk::get_index(Chunk::width ,rn_coord.x as i32, rn_coord.y as i32, rn_coord.z as i32);
                                locked_chunk.nodes[n_index].v += Vec3A::from(momentum);
                            }
                        }
                    }
                }
            });
        });
    }
}

fn update_grid (
    world: ResMut<World>
) {
    for n in 0..Chunk::loopert_width*Chunk::loopert_width*Chunk::loopert_width {
        world.chunks.par_iter().for_each(|(&i, c)| {
            let mut chunk = c.lock().unwrap();

            // If chunk shouldn't be updated or isn't part of the current batch don't do them
            if !chunk.update {
                return;
            }
            if chunk.loopert != n {
                return;
            }

            let locked_chunks = world.get_surrounding_chunks(i);
            let mut update_list = vec![];
            for i in 0..locked_chunks.len() {
                if i == 13 {
                    update_list.push(true);
                    continue;
                }
                let temp_lock = locked_chunks[i].lock().unwrap();
                update_list.push(temp_lock.update);
                drop(temp_lock);
            }
            drop(locked_chunks);

            for (i, node) in chunk.nodes.iter_mut().enumerate() {
                node.v /= node.m;
                node.v.y += dt * gravity; 

                let pos = Chunk::pos_from_index(Chunk::width, i);
                if !update_list[Chunk::get_index(3, 0, 1, 1)] {
                    if pos.x < 2 {node.v.x = 0.}
                }
                else if !update_list[Chunk::get_index(3, 2, 1, 1)] {
                    if pos.x > Chunk::width as i32 - 3 {node.v.x = 0.}
                }
                if !update_list[Chunk::get_index(3, 1, 0, 1)] {
                    if pos.y < 2 {node.v.y = 0.}
                }
                else if !update_list[Chunk::get_index(3, 1, 2, 1)] {
                    if pos.y > Chunk::width as i32 - 3 {node.v.y = 0.}
                }
                if !update_list[Chunk::get_index(3, 1, 1, 0)] {
                    if pos.z < 2 {node.v.z = 0.}
                }
                else if !update_list[Chunk::get_index(3, 1, 1, 2)] {
                    if pos.z > Chunk::width as i32 - 3 {node.v.z = 0.}
                }
            }
        });
    }
}

fn g2p (
    world: ResMut<World>
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

            let mut update_list = vec![];
            for i in 0..locked_chunks.len() {
                if i == 13 {
                    update_list.push(true);
                    continue;
                }
                let temp_lock = locked_chunks[i].lock().unwrap();
                update_list.push(temp_lock.update);
                drop(temp_lock);
            }

            // Loop through the particles
            let locked_chunk = locked_chunks[Chunk::get_index(3, 1, 1, 1)].lock().unwrap();
            locked_chunk.particles.clone().iter_mut().for_each(|p| {
                // Original node coord
                // All particles must be between 0 and 
                p.v = Vec3A::ZERO;

                let ogn_coord = p.x.floor(); 
                let ogn_diff = (p.x - ogn_coord) - 0.5;

                let weights = [
                    0.5 * (0.5 - ogn_diff).powf(2.),
                    0.75 - (ogn_diff).powf(2.),
                    0.5 * (0.5 + ogn_diff).powf(2.),
                ];
                
                let mut b: Mat3A = Mat3A::ZERO;
                for gx in 0..3 {
                    for gy in 0..3 {
                        for gz in 0..3 {
                            let weight = weights[gx].x * weights[gy].y * weights[gz].z;

                            let rn_coord = ogn_coord + Vec3A::new(gx as f32 - 1., gy as f32 - 1., gz as f32 - 1.);  
                            let rc_coord = Chunk::in_bounds(rn_coord);

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

                                let outside_chunky = locked_chunks[c_index].lock().unwrap();

                                let cell_dist = (Vec3A::new(outer_chunk_x as f32, outer_chunk_y as f32, outer_chunk_z as f32) - p.x) + 0.5;
                                let n_index = Chunk::get_index(Chunk::width, outer_chunk_x, outer_chunk_y, outer_chunk_z);
                                let w_v = outside_chunky.nodes[n_index].v * weight;
                                let term = Mat3A::from_cols(w_v * cell_dist.x, w_v * cell_dist.y, w_v * cell_dist.z);
                                b += term;
                                p.v += w_v;
                            }
                            // If the node is inside the chunk we don't do anything fancy
                            else {
                                let cell_dist = (Vec3A::new(rn_coord.x as f32, rn_coord.y as f32, rn_coord.z as f32) - p.x) + 0.5;
                                let n_index = Chunk::get_index(Chunk::width ,rn_coord.x as i32, rn_coord.y as i32, rn_coord.z as i32);
                                let w_v = locked_chunk.nodes[n_index].v * weight;
                                let term = Mat3A::from_cols(w_v * cell_dist.x, w_v * cell_dist.y, w_v * cell_dist.z);
                                b += term;
                                p.v += w_v;
                            }
                        }
                    }
                }
                p.C = b.mul_scalar(4.);
                p.x += Vec3A::from(p.v) * dt;
                let x_n = p.x + p.v;

                if !update_list[Chunk::get_index(3, 0, 1, 1)] {
                    if p.x.x < 2. {p.v.x += 3. - x_n.x}
                }
                else if !update_list[Chunk::get_index(3, 2, 1, 1)] {
                    if p.x.x > Chunk::width as f32 - 3. {p.v.x += 3. - x_n.x}
                }
                if !update_list[Chunk::get_index(3, 1, 0, 1)] {
                    if p.x.y < 2. {p.v.y += 3. - x_n.y}
                }
                else if !update_list[Chunk::get_index(3, 1, 2, 1)] {
                    if p.x.y > Chunk::width as f32 - 3. {p.v.y += 3. - x_n.y}
                }
                if !update_list[Chunk::get_index(3, 1, 1, 0)] {
                    if p.x.z < 2. {p.v.z += 3. - x_n.z}
                }
                else if !update_list[Chunk::get_index(3, 1, 1, 2)] {
                    if p.x.z > Chunk::width as f32 - 3. {p.v.z += 3. - x_n.z}
                }
            });
            drop(locked_chunks);
        });
    }

}

fn draw(
    mut gizmos: Gizmos,
    world: ResMut<World>,
) {
    world.chunks.iter().for_each(|(&i, c)| {
        let chunk = c.lock().unwrap();
        let mut color: Color;
        match chunk.loopert {
            0 => color = Color::BLUE,
            1 => color = Color::RED,
            2 => color = Color::CYAN,
            3 => color = Color::GOLD,
            4 => color = Color::MAROON,
            5 => color = Color::NAVY,
            6 => color = Color::VIOLET,
            7 => color = Color::GREEN,
            8 => color = Color::PINK,
            9 => color = Color::FUCHSIA,
            10 => color = Color::SEA_GREEN,
            11 => color = Color::DARK_GRAY,
            12 => color = Color::DARK_GREEN,
            13 => color = Color::ANTIQUE_WHITE,
            14 => color = Color::ORANGE,
            15 => color = Color::MIDNIGHT_BLUE,
            16 => color = Color::ORANGE_RED,
            17 => color = Color::ALICE_BLUE,
            18 => color = Color::LIME_GREEN,
            19 => color = Color::YELLOW_GREEN,
            20 => color = Color::ALICE_BLUE,
            21 => color = Color::CRIMSON,
            22 => color = Color::YELLOW,
            23 => color = Color::TOMATO,
            24 => color = Color::SALMON,
            25 => color = Color::OLIVE,
            26 => color = Color::TURQUOISE,
            _ => color = Color::BLACK,
        }
       // if chunk.update == false {
       //     return;
       // }
        for particle in &chunk.particles {
            if particle.x.distance(Vec3A::ZERO) > 10. {
                color = Color::BLUE;
            }
            else {
                color = Color::BLACK;
            }
            print!("{:?}", particle.x);
            gizmos.sphere(Vec3::new((i.x * Chunk::width as i32) as f32 + particle.x.x,(i.y * Chunk::width as i32) as f32 + particle.x.y,(i.z * Chunk::width as i32) as f32+ particle.x.z), Quat::IDENTITY, 0.25, color);
        }
    });
}
