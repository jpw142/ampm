use bevy::{prelude::*, math::{Vec3A, Mat3A}};
use std::sync::Mutex;
use crate::particle::Particle;
use hashbrown::HashMap;

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
pub struct World{pub chunks: HashMap<IVec3, Mutex<Chunk>>}

pub struct Chunk{
    // lowest bottom left back corner
    pub pos: IVec3,
    pub loopert: usize,
    pub update: bool,
    pub nodes: [Node; Chunk::num_nodes],
    pub particles: Vec<Particle>,
}

impl Chunk {
    pub const loopert_width: usize = 3;
    pub const width: usize = 8;
    const num_nodes: usize = Chunk::width * Chunk::width * Chunk::width;

    // If it's in the chunk it will be at 0,0,0 else it gives -1 or +1 depending on surrounding
    // chunks
    pub fn in_bounds(n_pos: Vec3A) -> IVec3 {
        // Relative chunk coord that will tell us if the target node is in the
        // chunk
        let mut rc_coord = IVec3::ZERO;

        // Checks if the coordinates is outside the chunk boundaries
        if n_pos.x > Chunk::width as f32 - 1. {rc_coord.x += 1;}
        else if n_pos.x < 0. {rc_coord.x -= 1;}

        if n_pos.y > Chunk::width as f32 - 1. {rc_coord.y += 1;}
        else if n_pos.y < 0. {rc_coord.y -= 1}

        if n_pos.z > Chunk::width as f32 - 1. {rc_coord.z += 1;}
        else if n_pos.z < 0. {rc_coord.z -= 1}
        rc_coord
    }

    pub fn get_index(width: usize, x: i32, y: i32, z:i32) -> usize {
        let index = (x as usize * width * width) + (y as usize * width) + z as usize;
        return index
    }
}

impl World {
    const width: usize = 4;
    const map_chunks: usize = World::width * World::width * World::width;
    const surrounding_chunk_offsets: [IVec3; 27] = [
        IVec3::new(-1, -1, -1),
        IVec3::new(-1, -1, 0),
        IVec3::new(-1, -1, 1),
        IVec3::new(-1, 0, -1),
        IVec3::new(-1, 0, 0),
        IVec3::new(-1, 0, 1),
        IVec3::new(-1, 1, -1),
        IVec3::new(-1, 1, 0),
        IVec3::new(-1, 1, 1),
        IVec3::new(0, -1, -1),
        IVec3::new(0, -1, 0),
        IVec3::new(0, -1, 1),
        IVec3::new(0, 0, -1),
        IVec3::new(0, 0, 0),
        IVec3::new(0, 0, 1),
        IVec3::new(0, 1, -1),
        IVec3::new(0, 1, 0),
        IVec3::new(0, 1, 1),
        IVec3::new(1, -1, -1),
        IVec3::new(1, -1, 0),
        IVec3::new(1, -1, 1),
        IVec3::new(1, 0, -1),
        IVec3::new(1, 0, 0),
        IVec3::new(1, 0, 1),
        IVec3::new(1, 1, -1),
        IVec3::new(1, 1, 0),
        IVec3::new(1, 1, 1),
    ];

    pub fn new() -> Self {
        let mut world = World{chunks: HashMap::new()};
        for x in 0..World::width {
            for y in 0..World::width {
                for z in 0..World::width {
                    // real modulo, -1.rem_euclid(3) = 2
                    // these are just the x y z mod 3 i decided to name them like this cause i'm
                    // stupid
                    let looxer = x.rem_euclid(Chunk::loopert_width);
                    let looyer = y.rem_euclid(Chunk::loopert_width);
                    let loozer = z.rem_euclid(Chunk::loopert_width);
                    let mut edge = false;
                    if (x % (World::width - 1) == 0) || (y % (World::width - 1) == 0) || (z % (World::width - 1) == 0) {
                        edge = true;
                    }
                    let vec: Vec<Node> = (0..Chunk::num_nodes).into_iter().fold(vec![], |mut acc, _| {
                        acc.push(Node{v: Vec3A::ZERO, m: 0.});
                        acc
                    });
                    let chunk: [Node; Chunk::num_nodes] = vec.try_into().unwrap();
                    world.chunks.insert(IVec3::new(x as i32, y as i32, z as i32), Mutex::new(
                        Chunk{
                        pos: IVec3::new((x * Chunk::width) as i32, (y * Chunk::width) as i32, (z * Chunk::width) as i32),
                        nodes: chunk,
                        update: !edge,
                        loopert: (looxer * Chunk::loopert_width * Chunk::loopert_width) + (looyer * Chunk::loopert_width) + loozer,
                        particles: vec![]
                        }
                    ));
                }
            }
        }
        world
    }
    pub fn get_surrounding_chunks (&self, pos: IVec3) -> Vec<&Mutex<Chunk>> { 
            // Gather surrounding chunks
            let mut surrounding_chunks = vec![];
            // Reads all the surrounding chunks and puts them in a list
            for j in 0..World::surrounding_chunk_offsets.len(){
                let coord = pos + World::surrounding_chunk_offsets[j]; 
                let outside_chunk = self.chunks.get(&coord).unwrap();
                surrounding_chunks.push(outside_chunk);
            }
            surrounding_chunks
    }
}
#[cfg(test)]
mod tests {
    use bevy::math::Vec3A;
    use crate::world::Chunk;
    use super::World;

    #[test]
    fn chunks_update_properly() {
        let world = World::new();
        world.chunks.iter().for_each(|(i, c)| {
            let chunk = c.lock().unwrap();
            let mut edge = false;
            if i.x == 0 || i.x == World::width as i32 - 1{
                edge = true;
            }
            if i.y == 0 || i.y == World::width as i32 - 1 {
                edge = true;
            }
            if i.z == 0 || i.z == World::width as i32 - 1 {
                edge = true;
            }
            if edge == true {
                assert!(chunk.update == false)
            }
            else {
                assert!(chunk.update == true)
            }
        });
    }
    #[test]
    fn in_bounds_works() {
        let mut numbers = vec![];
        for num in 0..Chunk::width {
            numbers.push(num);
        }
        for x in -1..=Chunk::width as i32 {
            for y in -1..=Chunk::width as i32 {
                for z in -1..=Chunk::width as i32 {
                    let chunk = Chunk::in_bounds(Vec3A::new(x as f32, y as f32, z as f32));
                    if x == -1 {
                        assert!(chunk.x == -1);
                    }
                    else if x == Chunk::width as i32 {
                        assert!(chunk.x == 1);
                    }
                    else {
                        assert!(chunk.x == 0);
                    }
                    if y == -1 {
                        assert!(chunk.y == -1);
                    }
                    else if y == Chunk::width as i32 {
                        assert!(chunk.y == 1);
                    }
                    else {
                        assert!(chunk.y == 0);
                    }
                    if z == -1 {
                        assert!(chunk.z == -1);
                    }
                    else if z == Chunk::width as i32 {
                        assert!(chunk.z == 1);
                    }
                    else {
                        assert!(chunk.z == 0);
                    }
                }
            }
        }
    }
    // Write two more tests, 1 for get_surrounding chunks and 1 for making sure loopert works
}
