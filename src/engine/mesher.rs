pub struct Neighborhood<T> {
    pub center: T,
    pub top: T,
    pub bottom: T,
    pub left: T,
    pub right: T,
    pub front: T,
    pub back: T,
}

impl<'c> Neighborhood<&'c BlockChunk> {
    pub fn get(&self, pos: Point3<i32>) -> Option<Block> {
        const SIZE: i32 = CHUNK_SIZE as i32;
        let wrapped = ::util::get_chunk_pos(pos).1;
        if in_chunk_bounds(pos) {
            Some(self.center[pos])
        } else if pos.x >= SIZE {
            self.right.get(::util::to_point(wrapped)).cloned()
        } else if pos.x < 0 {
            self.left.get(::util::to_point(wrapped)).cloned()
        } else if pos.y >= SIZE {
            self.top.get(::util::to_point(wrapped)).cloned()
        } else if pos.y < 0 {
            self.bottom.get(::util::to_point(wrapped)).cloned()
        } else if pos.z >= SIZE {
            self.front.get(::util::to_point(wrapped)).cloned()
        } else if pos.z < 0 {
            self.back.get(::util::to_point(wrapped)).cloned()
        } else {
            unreachable!()
        }
    }
}

// NOTE: You probably should never debug print this, unless CHUNK_SIZE is pretty small.
// Otherwise, your terminal will be spitting out text for a solid 3 minutes straight.
pub struct CullMesher<'c> {
    pos: ChunkPos,
    index: u32,
    neighbors: Neighborhood<&'c BlockChunk>,
    vertices: Vec<BlockVertex>,
    indices: Vec<u32>,
}

vertex! {
    vertex BlockVertex {
        pos: Vector3<f32>,
        norm: Vector3<f32>,
        face: i32,
        tile: Vector2<f32>,
        uv: Vector2<f32>,
    }
}

fn calculate_vertex_data(block: Block, pre: Precomputed) -> BlockVertex {
    BlockVertex {
        pos: pre.pos,
        norm: pre.norm,
        face: pre.face,
        uv: pre.face_offset,
        tile: match block {
            Block::Air => Vector2::new(0.0, 0.0),
            Block::Stone => Vector2::new(1.0, 0.0),
            Block::Dirt => Vector2::new(2.0, 0.0),
            Block::Grass => match pre.side {
                Side::Top => Vector2::new(0.0, 0.0),
                Side::Bottom => Vector2::new(2.0, 0.0),
                _ => Vector2::new(0.0, 1.0),
            },
            Block::Water => Vector2::new(1.0, 0.0),
        }
    }
}

use engine::block::Block;
use engine::chunk::BlockChunk;
use cgmath::{Point2, Point3, Vector2, Vector3};
use collision::Aabb2;
use engine::chunk::{in_chunk_bounds, Chunk, CHUNK_SIZE, CHUNK_VOLUME};
use engine::mesh::{MeshIndex, Mesh};
use engine::{ChunkPos, Precomputed, Side, Voxel};
use gl_api::buffer::UsageType;
use gl_api::error::GlResult;
use gl_api::layout::GlLayout;

impl<'c> CullMesher<'c> {
    pub fn new(
        pos: ChunkPos,
        chunk: &'c BlockChunk,
        top: &'c BlockChunk,
        bottom: &'c BlockChunk,
        left: &'c BlockChunk,
        right: &'c BlockChunk,
        front: &'c BlockChunk,
        back: &'c BlockChunk,
    ) -> Self {
        CullMesher {
            pos,
            index: 0,
            vertices: Vec::new(),
            indices: Vec::new(),
            neighbors: Neighborhood {
                center: chunk,
                top,
                bottom,
                left,
                right,
                front,
                back,
            }
        }
    }

    // fn calculate_ao(&self) -> f32 {
    //     fn as_f32(val: bool) -> f32 { val as i32 as f32 }
    //     let side1 = false;
    //     let side2 = false;
    //     let corner = false;
    //     if side1 && side2 { return 0.0; }
    //     3.0 - (as_f32(side1) + as_f32(side2) + as_f32(corner))
    // }

    fn add_face(&mut self, side: Side, pos: Point3<i32>, voxel: Block) {
        let index_len = self.index;
        let cx = CHUNK_SIZE as f32 * self.pos.x as f32;
        let cy = CHUNK_SIZE as f32 * self.pos.y as f32;
        let cz = CHUNK_SIZE as f32 * self.pos.z as f32;
        let x = pos.x as f32;
        let y = pos.y as f32;
        let z = pos.z as f32;

        macro_rules! face { 
            (side $side:ident,
             ind [$($index:expr),*],
             vert [$($vx:expr, $vy:expr, $vz:expr);*],
             off [$($ou:expr, $ov:expr);*],
             norm $nx:expr,$ny:expr,$nz:expr;,
             face $face:expr) => {{
                $(self.indices.push(index_len + $index);)*
                $(self.vertices.push(calculate_vertex_data(voxel, Precomputed {
                    side: Side::$side,
                    face_offset: Vector2::new($ou as f32, $ov as f32),
                    pos: Vector3::new(cx+x+$vx as f32, cy+y+$vy as f32, cz+z+$vz as f32),
                    norm: Vector3::new($nx as f32, $ny as f32, $nz as f32),
                    face: $face
                }));)*
                self.index += 6;
            }}
        }

        match side {
            Side::Top    => face! { side Top,    ind [0,1,2,3,2,1], vert [0,1,0; 1,1,0; 0,1,1; 1,1,1], off [0,1; 1,1; 0,0; 1,0], norm 0, 1,0;, face 1 },
            Side::Bottom => face! { side Bottom, ind [0,2,1,3,1,2], vert [0,0,0; 1,0,0; 0,0,1; 1,0,1], off [0,1; 1,1; 0,0; 1,0], norm 0,-1,0;, face 1 },
            Side::Front  => face! { side Front,  ind [0,1,2,3,2,1], vert [0,1,1; 1,1,1; 0,0,1; 1,0,1], off [0,0; 1,0; 0,1; 1,1], norm 0,0, 1;, face 0 },
            Side::Back   => face! { side Back,   ind [0,2,1,3,1,2], vert [0,1,0; 1,1,0; 0,0,0; 1,0,0], off [0,0; 1,0; 0,1; 1,1], norm 0,0,-1;, face 0 },
            Side::Left   => face! { side Left,   ind [0,1,2,3,2,1], vert [0,1,0; 0,1,1; 0,0,0; 0,0,1], off [0,0; 1,0; 0,1; 1,1], norm -1,0,0;, face 2 },
            Side::Right  => face! { side Right,  ind [0,2,1,3,1,2], vert [1,1,0; 1,1,1; 1,0,0; 1,0,1], off [0,0; 1,0; 0,1; 1,1], norm 1, 0,0;, face 2 },
        }
    }

    fn needs_face(&self, pos: Point3<i32>) -> bool {
        self.neighbors.get(pos).map(|voxel| !voxel.properties().opaque).unwrap_or(false)
    }

    fn mesh(&mut self) {
        for i in 0..(CHUNK_SIZE as i32) {
            for j in 0..(CHUNK_SIZE as i32) {
                for k in 0..(CHUNK_SIZE as i32) {
                    let pos = Point3::new(i, j, k);
                    let block = self.neighbors.get(pos).unwrap();
                    if !block.properties().opaque {
                        continue;
                    }

                    if self.needs_face(pos + Vector3::unit_z()) {
                        self.add_face(Side::Front, pos, block)
                    }
                    if self.needs_face(pos - Vector3::unit_z()) {
                        self.add_face(Side::Back, pos, block)
                    }
                    if self.needs_face(pos + Vector3::unit_y()) {
                        self.add_face(Side::Top, pos, block)
                    }
                    if self.needs_face(pos - Vector3::unit_y()) {
                        self.add_face(Side::Bottom, pos, block)
                    }
                    if self.needs_face(pos + Vector3::unit_x()) {
                        self.add_face(Side::Right, pos, block)
                    }
                    if self.needs_face(pos - Vector3::unit_x()) {
                        self.add_face(Side::Left, pos, block)
                    }
                }
            }
        }
    }
}

type Quad = Aabb2<i32>;

pub struct GreedyMesher<'c> {
    pos: Point3<i32>,
    index: u32,
    neighbors: Neighborhood<&'c BlockChunk>,
    mask: Box<[bool]>,
    dimension: Side,
    vertices: Vec<BlockVertex>,
    indices: Vec<u32>,
}

const SIZE: i32 = CHUNK_SIZE as i32;

impl<'c> GreedyMesher<'c> {
    pub fn new(
        pos: Point3<i32>,
        chunk: &'c BlockChunk,
        top: &'c BlockChunk,
        bottom: &'c BlockChunk,
        left: &'c BlockChunk,
        right: &'c BlockChunk,
        front: &'c BlockChunk,
        back: &'c BlockChunk,
    ) -> Self {
        GreedyMesher {
            neighbors: Neighborhood {
                center: chunk,
                top,
                bottom,
                left,
                right,
                front,
                back,
            },
            pos,
            index: 0,
            vertices: Vec::new(),
            indices: Vec::new(),
            mask: vec![false; CHUNK_SIZE * CHUNK_SIZE].into_boxed_slice(),
            dimension: Side::Top,
        }
    }

    fn set_mask(&mut self, u: i32, v: i32, value: bool) {
        self.mask[(SIZE * u + v) as usize] = value;
    }

    fn get_mask(&self, u: i32, v: i32) -> bool {
        if u >= SIZE || u < 0 || v >= SIZE || v < 0 { false }
        else { self.mask[(SIZE * u + v) as usize] }
    }

    fn to_world_space(&self, u: i32, v: i32, layer: i32) -> Point3<i32> {
        match self.dimension {
            Side::Top | Side::Bottom => Point3::new(u, layer, v),
            Side::Right | Side::Left => Point3::new(layer, u, v),
            Side::Front | Side::Back => Point3::new(u, v, layer),
        }
    }

    fn get_offset_vec(&self) -> Vector3<i32> {
        match self.dimension {
            Side::Top => Vector3::new(0, 1, 0),
            Side::Right => Vector3::new(1, 0, 0),
            Side::Front => Vector3::new(0, 0, 1),
            Side::Bottom => Vector3::new(0, -1, 0),
            Side::Left => Vector3::new(-1, 0, 0),
            Side::Back => Vector3::new(0, 0, -1),
        }
    }

    fn get_center(&self, u: i32, v: i32, layer: i32) -> Option<Block> {
        let pos = self.to_world_space(u, v, layer);
        self.neighbors.center.get(pos).cloned()
    }

    fn expand_right(&self, u: i32, v: i32, layer: i32) -> Quad {
        let start = self.get_center(u, v, layer).unwrap();
        for un in u..SIZE {
            let cur = self.get_center(un + 1, v, layer);
            if Some(start) != cur || !self.get_mask(un + 1, v) {
                return Aabb2::new(
                    Point2::new(u, v),
                    Point2::new(un, v)
                );
            }
        }
        unreachable!()
    }

    fn expand_down(&self, quad: Quad, layer: i32) -> Quad {
        let minu = quad.min.x;
        let minv = quad.min.y;
        let maxu = quad.max.x;
        let start = self.get_center(minu, minv, layer).unwrap();
        for vn in minv..SIZE {
            for un in minu..=maxu {
                // let cur_point = Point3::new(un, layer, vn + 1);
                let cur = self.get_center(un, vn + 1, layer);
                if Some(start) != cur || !self.get_mask(un, vn + 1) {
                    return Aabb2::new(
                        Point2::new(minu, minv),
                        Point2::new(maxu, vn),
                    );
                }
            }
        }
        unreachable!()
    }

    fn fill_mask(&mut self, layer: i32) {
        for u in 0..SIZE {
            for v in 0..SIZE {
                let pos = self.to_world_space(u, v, layer);
                let current = &self.neighbors.center[pos];
                // UNWRAP: unwrap is ok because there will always be a block one
                // outside of the center chunk
                let above = self.neighbors.get(pos + self.get_offset_vec()).unwrap();
                // We need to set the mask for any visible face. A face is
                // visible if the voxel above it is transparent, and the current
                // voxel is not transparent.
                let val = current.properties().opaque && !above.properties().opaque;
                self.set_mask(u, v, val);
            }
        }
    }

    fn pick_pos(&self) -> Option<Point2<i32>> {
        // TODO: could this be made faster?
        for u in 0..SIZE {
            for v in 0..SIZE {
                if self.get_mask(u, v) {
                    return Some(Point2::new(u, v));
                }
            }
        }
        None
    }

    fn add_quad(&mut self, mut quad: Quad, voxel: Block, layer: i32) {
        quad.max.x += 1;
        quad.max.y += 1;
        let index_len = self.index;
        let cx = CHUNK_SIZE as f32 * self.pos.x as f32;
        let cy = CHUNK_SIZE as f32 * self.pos.y as f32;
        let cz = CHUNK_SIZE as f32 * self.pos.z as f32;
        let fq: Aabb2<f32> = Aabb2::new(quad.min.cast().unwrap(), quad.max.cast().unwrap());

        macro_rules! face { 
            (side $side:ident,
             ind [$($index:expr),*],
             norm $nx:expr,$ny:expr,$nz:expr;,
             face $face:expr,
             vert [$($vx:expr, $vy:expr, $vz:expr);*],
             off [$($ou:expr, $ov:expr);*]
             ) => {{
                $(self.indices.push(index_len + $index);)*
                $(self.vertices.push(calculate_vertex_data(voxel, Precomputed {
                    side: Side::$side,
                    face_offset: Vector2::new($ou as f32, $ov as f32),
                    pos: Vector3::new(cx+$vx as f32, cy+$vy as f32, cz+$vz as f32),
                    norm: Vector3::new($nx as f32, $ny as f32, $nz as f32),
                    face: $face
                }));)*
                self.index += 6;
            }}
        }

        let (top, bot) = (layer as f32 + 1.0, layer as f32);
        let (minu, minv, maxu, maxv) = (fq.min.x, fq.min.y, fq.max.x, fq.max.y);
        let (lenu, lenv) = (maxu - minu, maxv - minv);

        match self.dimension {
            Side::Top => face! { side Top, ind [0,1,2,3,2,1], norm 0,1,0;, face 1,
                vert [minu, top, minv; maxu, top, minv; minu, top, maxv; maxu, top, maxv],
                off  [0,lenv; lenu,lenv; 0,0; lenu,0] },
            Side::Bottom => face! { side Bottom, ind [0,2,1,3,1,2], norm 0,1,0;, face 1,
                vert [minu, bot, minv; maxu, bot, minv; minu, bot, maxv; maxu, bot, maxv],
                off  [0,lenv; lenu,lenv; 0,0; lenu,0] },

            Side::Front => face! { side Front, ind [0,1,2,3,2,1], norm 0,0,1;, face 0,
                vert [minu,maxv,top; maxu,maxv,top; minu,minv,top; maxu,minv,top],
                off  [0,0; lenu,0; 0,lenv; lenu,lenv] },
            Side::Back => face! { side Back, ind [0,2,1,3,1,2], norm 0,0,-1;, face 0,
                vert [minu,maxv,bot; maxu,maxv,bot; minu,minv,bot; maxu,minv,bot],
                off  [0,0; lenu,0; 0,lenv; lenu,lenv] },

            Side::Left => face! { side Left, ind [0,1,2,3,2,1], norm -1,0,0;, face 2,
                vert [bot,maxu,minv; bot,maxu,maxv; bot,minu,minv; bot,minu,maxv],
                off  [0,0; lenv,0; 0,lenu; lenv,lenu] },
            Side::Right => face! { side Right, ind [0,2,1,3,1,2], norm 1,0,0;, face 2,
                vert [top,maxu,minv; top,maxu,maxv; top,minu,minv; top,minu,maxv],
                off  [0,0; lenv,0; 0,lenu; lenv,lenu] },
        }
    }

    fn mark_visited(&mut self, quad: Quad) {
        for x in quad.min.x..=quad.max.x {
            for z in quad.min.y..=quad.max.y {
                self.set_mask(x, z, false);
            }
        }
    }

    pub fn create_mesh(&self) -> GlResult<Mesh<BlockVertex, u32>> {
        let mut mesh = Mesh::new()?;
        mesh.upload(&self.vertices, &self.indices, UsageType::StaticDraw)?;
        Ok(mesh)
    }

    pub fn mesh(&mut self) {
        for &dim in &[
            Side::Top, Side::Right, Side::Front,
            Side::Bottom, Side::Left, Side::Back
        ] {
            self.dimension = dim;
            for layer in 0..SIZE {
                self.fill_mask(layer);
                // While unvisited faces remain, pick a position from the remaining
                while let Some(pos) = self.pick_pos() {
                    let (u, v) = (pos.x, pos.y);
                    let voxel = self.get_center(u, v, layer).unwrap();
                    // Construct a quad that reaches as far right as possible
                    let quad = self.expand_right(u, v, layer);
                    // Expand that quad as far down as possible
                    let quad = self.expand_down(quad, layer);
                    self.mark_visited(quad);
                    self.add_quad(quad, voxel, layer);
                }
            }
        }
    }
}
