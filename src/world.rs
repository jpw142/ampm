use bevy::prelude::*;
use std::sync::Mutex;
use crate::particle::Particle;

const map_width: usize = 8;
const map_chunks: usize = map_width * map_width * map_width;

const chunk_width: usize = 8;
const chunk_nodes: usize = chunk_width * chunk_width * chunk_width;

#[derive(Component, Debug, Clone, Copy,)]
struct Node {
    // https://github.com/rust-lang/rust/issues/72353 
    v: Vec3,    // velocity Z 
    m: f32,     // mass
}

impl Node {
    pub fn new() -> Self {
        return Node { v:Vec3::ZERO, m: 0. }
    }

    pub fn zero(&mut self) {
        self.m = 0.;
        self.v = Vec3::ZERO;
    }
}

#[derive(Resource)]
pub struct World{pub chunks: Vec<Mutex<Chunk>>}

#[derive(Debug)]
pub struct Chunk{
    // lowest bottom left back corner
    pub pos: IVec3,
    pub nodes: [Node; chunk_nodes],
    pub particles: Vec<Particle>,
}

impl World {
    fn new() -> Self {
        let mut world = World{chunks: vec![]};
        for x in 0..map_width {
            for y in 0..map_width {
                for z in 0..map_width {
                    let vec: Vec<Node> = (0..chunk_nodes).into_iter().fold(vec![], |mut acc, _| {
                        acc.push(Node{v: Vec3::ZERO, m: 0.});
                        acc
                    });
                    let chunk: [Node; chunk_nodes] = vec.try_into().unwrap();
                    world.chunks.push(Mutex::new(
                        Chunk{
                        pos: IVec3::new((x * chunk_width) as i32, (y * chunk_width) as i32, (z * chunk_width) as i32),
                        nodes: chunk,
                        particles: vec![]
                        }
                    ));
                }
            }
        }
        world
    }
}
