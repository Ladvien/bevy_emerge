<<<<<<< HEAD
//! Meshes a map's [`Volume`] per chunk with `isomesh` and welds the chunks into one surface (#101, F5).
//!
//! The volume stays its edit list; this samples [`Volume::sdf`] over a region and extracts the
//! zero surface with Marching Cubes, the extractor `isomesh` measures to tile across chunk seams.
//! Needs the `isomesh` feature.
=======
//! **Meshing a carved volume in chunks** (#101, F5), through `isomesh`.
//!
//! The edit list ([`Volume`]) is the data; this samples it, over a ground, one chunk at a time. A
//! chunk samples `cells + 1` planes so two neighbours share their plane exactly; the seam test in
//! `tests/volume_meshes_without_cracks.rs` holds that.
>>>>>>> 1cab1eb71b6586e50293675c7729e2e98e2c691d

use bevy_emerge_core::volume::Volume;
use isomesh::chunk::{ChunkId, ChunkLayout};
use isomesh::marching_cubes::MarchingCubes;
<<<<<<< HEAD
use isomesh::validate::{validate, MeshReport, ValidateConfig};
use isomesh::weld::{epsilon_for, Welder};
use isomesh::{MeshBuffer, Sdf};

/// The region to mesh: `chunks` chunks per axis of `chunk_cells` cells, starting at `min` (metres).
#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub min: [f32; 3],
    pub chunks: [u32; 3],
    pub chunk_cells: u32,
    /// Use a power of two (0.5, 1, 2): `isomesh` measures seams as exact only for those.
    pub cell_m: f32,
}

struct Field<'a, F: Fn([f32; 3]) -> f32> {
    volume: &'a Volume,
    base: F,
}

impl<F: Fn([f32; 3]) -> f32> Sdf for Field<'_, F> {
    type Scalar = f32;
    fn sample(&self, p: [f32; 3]) -> f32 {
        self.volume.sdf(p, (self.base)(p))
    }
}

/// Mesh `volume` over `region`, one chunk at a time, then weld across the seams. `base` is the
/// distance to the ground beneath the volume (`p.y - height`), or `f32::INFINITY` for empty space.
pub fn mesh_volume(
    volume: &Volume,
    base: impl Fn([f32; 3]) -> f32,
    region: Region,
) -> Result<MeshBuffer<f32>, String> {
    let layout = ChunkLayout::new(region.chunk_cells, region.cell_m, region.min).map_err(|e| e.to_string())?;
    let shape = layout.sample_shape().map_err(|e| e.to_string())?;
    let field = Field { volume, base };
    let mut mc = MarchingCubes::<f32>::new();
    let mut all = MeshBuffer::<f32>::new();
    let mut one = MeshBuffer::<f32>::new();
    for x in 0..region.chunks[0] as i32 {
        for y in 0..region.chunks[1] as i32 {
            for z in 0..region.chunks[2] as i32 {
                one.reset();
                let origin = layout.sample_origin(ChunkId::new([x, y, z]));
                mc.extract(&field, &shape, origin, region.cell_m, &mut one).map_err(|e| e.to_string())?;
                all.append(&one).map_err(|e| e.to_string())?;
            }
        }
    }
    Welder::<f32>::new().weld(&mut all, epsilon_for(region.cell_m)).map_err(|e| e.to_string())?;
    Ok(all)
}

/// `isomesh`'s own census of a welded mesh: holes (`boundary_edges`), non-manifold edges, genus.
pub fn report(mesh: &MeshBuffer<f32>, cell_m: f32) -> Result<MeshReport, String> {
    let cfg = ValidateConfig::from_cell_size(f64::from(cell_m)).map_err(|e| e.to_string())?;
    Ok(validate(mesh, &cfg))
=======
use isomesh::{MeshBuffer, Sdf};

/// The volume over a heightfield ground: `p.y - ground(x, z)`, negative in rock, then every edit.
pub struct VolumeField<'a, G: Fn(f32, f32) -> f32> {
    pub volume: &'a Volume,
    pub ground: G,
}

impl<G: Fn(f32, f32) -> f32> Sdf for VolumeField<'_, G> {
    type Scalar = f32;
    fn sample(&self, p: [f32; 3]) -> f32 {
        self.volume.sdf(p, p[1] - (self.ground)(p[0], p[2]))
    }
}

/// Mesh one chunk of `layout`.
pub fn mesh_chunk<G: Fn(f32, f32) -> f32>(
    field: &VolumeField<'_, G>,
    layout: &ChunkLayout<f32>,
    id: ChunkId,
) -> Result<MeshBuffer<f32>, isomesh::Error> {
    let shape = layout.sample_shape()?;
    let mut out = MeshBuffer::<f32>::new();
    MarchingCubes::<f32>::new().extract(field, &shape, layout.sample_origin(id), layout.cell_size(), &mut out)?;
    Ok(out)
>>>>>>> 1cab1eb71b6586e50293675c7729e2e98e2c691d
}
