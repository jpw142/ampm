use bevy::{prelude::*, math::{Vec3A, Mat3A}};
use std::sync::RwLock;
use crate::particle::Particle;
use hashbrown::HashMap;

const map_width: usize = 4;
const map_chunks: usize = map_width * map_width * map_width;

pub const chunk_width: usize = 8;
const chunk_nodes: usize = chunk_width * chunk_width * chunk_width;

// The length of the buffer 3 for mutex 2 for rwlock
pub const loopert_width: usize = 2;


#[derive(Component, Debug, Clone, Copy,)]
pub struct Node {
    // https://github.com/rust-lang/rust/issues/72353 
    pub v: Vec3A,    // velocity Z 
    pub m: f32,     // mass
}

impl Node {
    pub fn new() -> Self {
        return Node { v:Vec3A::ZERO, m: 0. }
    }

    pub fn zero(&mut self) {
        self.m = 0.;
        self.v = Vec3A::ZERO;
    }
}

#[derive(Resource)]
pub struct World{pub chunks: HashMap<IVec3, RwLock<Chunk>>}


pub struct Chunk{
    // lowest bottom left back corner
    pub pos: IVec3,
    pub loopert: usize,
    pub update: bool,
    pub nodes: [Node; chunk_nodes],
    pub particles: Vec<Particle>,
}

impl World {
    pub fn new() -> Self {
        let mut world = World{chunks: HashMap::new()};
        for x in 0..map_width {
            for y in 0..map_width {
                for z in 0..map_width {
                    // real modulo, -1.rem_euclid(3) = 2
                    // these are just the x y z mod 3 i decided to name them like this cause i'm
                    // stupid
                    let looxer = x.rem_euclid(loopert_width);
                    let looyer = y.rem_euclid(loopert_width);
                    let loozer = z.rem_euclid(loopert_width); 
                    let vec: Vec<Node> = (0..chunk_nodes).into_iter().fold(vec![], |mut acc, _| {
                        acc.push(Node{v: Vec3A::ZERO, m: 0.});
                        acc
                    });
                    let chunk: [Node; chunk_nodes] = vec.try_into().unwrap();
                    world.chunks.insert(IVec3::new(x as i32, y as i32, z as i32), RwLock::new(
                        Chunk{
                        pos: IVec3::new((x * chunk_width) as i32, (y * chunk_width) as i32, (z * chunk_width) as i32),
                        nodes: chunk,
                        update: true,
                        loopert: (looxer * loopert_width * loopert_width) + (looyer * loopert_width) + loozer,
                        particles: vec![Particle{x: Vec3A::ZERO, C: Mat3A::ZERO, m: 0., v: Vec3A::ZERO}; chunk_nodes]
                        }
                    ));
                }
            }
        }
        world
    }
}
