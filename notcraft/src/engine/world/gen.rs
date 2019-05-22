use crate::engine::prelude::*;
use noise::{MultiFractal, NoiseFn, RidgedMulti, SuperSimplex};
use specs::world::EntitiesRes;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct NoiseGenerator {
    noise: RidgedMulti,
    biome_noise: SuperSimplex,
}

impl NoiseGenerator {
    pub fn new_default() -> Self {
        let noise = RidgedMulti::default()
            .set_frequency(0.001)
            .set_lacunarity(4.0)
            // .set_attenuation(0.01)
            .set_persistence(0.7);
        let biome_noise = SuperSimplex::new();
        NoiseGenerator { noise, biome_noise }
    }

    fn block_at(&self, x: f64, y: f64, z: f64, registry: &block::BlockRegistry) -> BlockId {
        let noise = 20.0 * self.noise.get([x, z]);

        if noise - 2.0 > y {
            block::STONE
        } else if noise - 1.0 > y {
            if y < -15.0 {
                block::SAND
            } else {
                block::DIRT
            }
        } else if noise > y {
            if y < -15.0 {
                block::SAND
            } else {
                block::GRASS
            }
        } else {
            if y < -18.0 {
                block::WATER
            } else {
                block::AIR
            }
        }
    }

    pub fn make_chunk(&self, pos: ChunkPos, registry: &block::BlockRegistry) -> Chunk {
        use rand::Rng;
        let size = chunk::SIZE as i32;
        let mut vec = Vec::with_capacity(chunk::VOLUME);
        let base = pos.base().0;
        // let gold_id = registry.get_id("gold_block");
        let mut rng = rand::thread_rng();
        let mut rand_id = || BlockId(rng.gen_range(1, registry.num_entries() - 1));
        for x in 0..size {
            for y in 0..size {
                for z in 0..size {
                    let pos = base + Vector3::new(x, y, z);

                    vec.push({
                        let noise: f64 = 20.0 * self.noise.get([x as f64, z as f64]);

                        if noise > pos.y as f64 {
                            rand_id()
                        } else {
                            block::AIR
                        }

                        // if noise - 2.0 > y {
                        //     block::STONE
                        // } else if noise - 1.0 > y {
                        //     if y < -15.0 {
                        //         block::SAND
                        //     } else {
                        //         block::DIRT
                        //     }
                        // } else if noise > y {
                        //     if y < -15.0 {
                        //         block::SAND
                        //     } else {
                        //         block::GRASS
                        //     }
                        // } else {
                        //     if y < -18.0 {
                        //         block::WATER
                        //     } else {
                        //         block::AIR
                        //     }
                        // }
                    });
                }
            }
        }
        Chunk::new(vec)
    }
}

// impl job::Worker for NoiseGenerator {
//     type Input = ChunkPos;
//     type Output = Chunk;

//     fn compute(&mut self, pos: &Self::Input) -> Self::Output {}
// }

// pub struct ChunkUnloader {
//     keep_loaded: HashSet<ChunkPos>,
// }

// impl Default for ChunkUnloader {
//     fn default() -> Self {
//         ChunkUnloader {
//             keep_loaded: HashSet::new(),
//         }
//     }
// }

// impl<'a> System<'a> for ChunkUnloader {
//     type SystemData = (
//         WriteExpect<'a, VoxelWorld>,
//         WriteStorage<'a, comp::MarkedForDeletion>,
//         ReadStorage<'a, comp::ChunkId>,
//         ReadStorage<'a, comp::Player>,
//         ReadStorage<'a, comp::Transform>,
//         Read<'a, res::ViewDistance>,
//         Entities<'a>,
//         // ReadExpect<'a, DebugAccumulator>,
//     );

//     fn run(
//         &mut self,
//         (mut world, mut marked, chunks, players, transforms, distance,
// entities): Self::SystemData,     ) {
//         let distance = distance.0;

//         for (entity, chunk, _) in (&entities, &chunks, &marked).join() {
//             world.unload_chunk(chunk.0);
//             let _ = entities.delete(entity);
//         }

//         self.keep_loaded.clear();
//         for &comp::ChunkId(chunk) in chunks.join() {
//             for (transform, _) in (&transforms, &players).join() {
//                 let center: ChunkPos = WorldPos(transform.position).into();
//                 if crate::util::in_range(chunk.0, center.0, distance) {
//                     self.keep_loaded.insert(chunk);
//                 }
//             }
//         }

//         for (entity, &comp::ChunkId(chunk)) in (&entities, &chunks).join() {
//             if !self.keep_loaded.contains(&chunk) {
//                 let _ = marked.insert(entity, comp::MarkedForDeletion);
//             }
//         }
//     }
// }

// #[derive(Debug)]
// pub struct TerrainGenerator {
//     service: job::Service<NoiseGenerator>,
//     queue: HashSet<ChunkPos>,
// }

// impl TerrainGenerator {
//     pub fn new() -> Self {
//         let service = job::Service::new("Chunk Generator", 4,
// NoiseGenerator::new_default());

//         TerrainGenerator {
//             service,
//             queue: HashSet::default(),
//         }
//     }

//     pub fn enqueue_radius(&mut self, world: &VoxelWorld, center: ChunkPos,
// radius: usize) {         let radius = radius as i32;
//         for xo in -radius..=radius {
//             for yo in -radius..=radius {
//                 for zo in -radius..=radius {
//                     let pos = center.offset((xo, yo, zo));
//                     if !world.chunk_exists(pos) && !self.queue.contains(&pos)
// {                         self.queue.insert(pos);
//                         self.service.request(pos);
//                     }
//                 }
//             }
//         }
//     }

//     pub fn drain_finished_chunks(&mut self) -> impl Iterator<Item =
// (ChunkPos, Chunk)> + '_ {         self.service.gather()
//     }
// }

// impl<'a> System<'a> for TerrainGenerator {
//     type SystemData = (
//         WriteExpect<'a, VoxelWorld>,
//         ReadStorage<'a, comp::Player>,
//         ReadStorage<'a, comp::Transform>,
//         Read<'a, res::ViewDistance>,
//         Read<'a, LazyUpdate>,
//         Read<'a, EntitiesRes>,
//         // ReadExpect<'a, DebugAccumulator>,
//     );

//     fn run(
//         &mut self,
//         (mut voxel_world, players, transforms, view_distance, lazy,
// entity_res): Self::SystemData,     ) {
//         let dist = view_distance.0;
//         for (_, transform) in (&players, &transforms).join() {
//             self.enqueue_radius(
//                 &voxel_world,
//                 WorldPos(transform.position).into(),
//                 dist.x as usize,
//             );
//         }

//         // let mut section = debug.section("terrain generation");
//         // for item in self.queue.iter() {
//         //     section.chunk(*item, 2.0, Vector4::new(1.0, 0.0, 0.0, 1.0));
//         // }

//         for (pos, chunk) in self.service.gather() {
//             // section.chunk(pos, 2.0, Vector4::new(0.0, 1.0, 0.0, 1.0));
//             voxel_world.set_chunk(pos, chunk);
//             self.queue.remove(&pos);
//             lazy.create_entity(&entity_res)
//                 .with(comp::ChunkId(pos))
//
// .with(comp::Transform::default().with_position(pos.base().base().0))
//                 .build();
//         }
//     }
// }
