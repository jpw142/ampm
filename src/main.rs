#![allow(non_upper_case_globals)]
use std::sync::RwLock;
use bevy::prelude::*;
use rayon::prelude::*;

mod cam;

const node_length: usize = 10;
const num_nodes: usize = node_length * node_length;

const chunk_length: usize = 10;
const num_chunks: usize = chunk_length * chunk_length;

const num_particles: usize = 10;

const unit_delta_t: f32 = 1e-6;
const inv_unit_delta_t: f32 = 1./unit_delta_t;

const max_units: i32 = 8192;
const cfl_dt_mul: f32 = 1.0;
const strength_dt_mul: f32 = 1.0;

#[derive(Clone, Copy)]
struct Particle {
    x: Vec2,
    v: Vec2,    // velocity
    C: Mat2,     // affine momentum matrix
    m: f32,     // mass
}

#[derive(Clone, Copy)]
struct Node {
    // https://github.com/rust-lang/rust/issues/72353 
    v: Vec2,    // velocity Z 
    m: f32,     // mass
}

#[derive(Clone, Copy)]
struct Chunk {
    nodes: [Node; num_nodes],
    particles: [Particle; num_particles],
    sdl: i32, // strength dt limit
    cdl: i32, // cfl dt limit
    limit: i32, // continuous dt limit
    local_min_dt_limit: i32,
}

impl Particle {
    fn new() -> Self {
        Particle {
            x: Vec2::ZERO,
            v: Vec2::ZERO,
            C: Mat2::ZERO,
            m: 1.,
        }
    }
}

impl Node {
    fn new() -> Self {
        Node {
            v: Vec2::ZERO,
            m: 0.,
        }
    }
}

impl Chunk {
    fn new() -> RwLock<Self> {
        RwLock::new(Chunk {
           nodes: [Node::new(); num_nodes],
           particles: [Particle::new(); num_particles],
           sdl: 1,
           cdl: 1,
           limit: 1,
           local_min_dt_limit: 1,
        })
    }
}

#[derive(Resource)]
struct World {
    chunks: Vec<RwLock<Chunk>>,
    t_int: usize,
}

fn draw_chunks (
    world: Res<World>,
    mut gizmos: Gizmos,
) {
    for i in 0..chunk_length {
        for j in 0..chunk_length {
            // Draw Chunk
            gizmos.rect_2d(Vec2::new(i as f32 + 0.5, j as f32 + 0.5), 0., Vec2::ONE, Color::GREEN);
        }
    }
}

fn draw_particles (
    world: Res<World>,
    mut gizmos: Gizmos,
) {
    for c in world.chunks.iter() {
        for p in c.read().unwrap().particles {
            gizmos.circle(Vec3::new(p.x.x, p.x.y, 0.1), Vec3::Z, 0.1, Color::BLUE);
        }
    }
}

fn inv_sqrt (f: f32) -> f32 {
    1./(f.sqrt())
}

fn update_dt_limits (
    mut world: ResMut<World>
) {
    world.chunks.par_iter()
        .for_each(|c| {
            // This is essentially a faster time % limit == 0 && particles isn't empty
            if (world.t_int as i32 & c.read().unwrap().limit - 1) == 0 && !c.read().unwrap().particles.is_empty() {
            }

            c.write().unwrap().cdl = 1;
        })
}

fn main() {
    // Initialize all the chunks
    let mut chunk_vec: Vec<RwLock<Chunk>> = vec![];
    for _ in 0..num_chunks {
        chunk_vec.push(Chunk::new());
    }
    
    App::new()
        .add_plugins((DefaultPlugins,
                     cam::PlayerPlugin
                    ))
        .insert_resource(World{chunks: chunk_vec, t_int: 0})
        .add_systems(Update, (
                draw_chunks, 
                draw_particles,
                ))
        .add_systems(Update, (update_dt_limits))
        .run()
}

