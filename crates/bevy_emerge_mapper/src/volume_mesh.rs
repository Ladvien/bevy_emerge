//! **Meshing a carved volume in chunks** (#101, F5), through `isomesh`.
//!
//! The edit list ([`Volume`]) is the data; this samples it, over a ground, one chunk at a time. A
//! chunk samples `cells + 1` planes so two neighbours share their plane exactly; the seam test in
//! `tests/volume_meshes_without_cracks.rs` holds that.

use bevy_emerge_core::volume::Volume;
use isomesh::chunk::{ChunkId, ChunkLayout};
use isomesh::marching_cubes::MarchingCubes;
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
}

/// **Snow on a carved volume** (#101), through `bevy_elements`' own `SnowyField`: the volume over a
/// heightfield ground (`tops`, world heights on `grid`), with `snow` metres (depths on `snow_grid`) laid
/// over each column's top. `SnowyField`'s contract keeps a tunnel under a roof thicker than one voxel
/// free of snow, so a carve is never filled by it. Needs `--features volume_snow`.
#[cfg(feature = "volume_snow")]
pub fn snowy_volume(
    volume: Volume,
    cell_size: f32,
    tops: std::sync::Arc<[f32]>,
    grid: bevy_elements::TerrainGrid,
    snow: std::sync::Arc<[f32]>,
    snow_grid: bevy_elements::TerrainGrid,
) -> bevy_elements::SnowyField {
    let owned = OwnedVolume { volume, tops: tops.clone(), grid };
    bevy_elements::SnowyField::new(std::sync::Arc::new(owned), cell_size, tops, grid, snow, snow_grid)
}

/// [`VolumeField`] that owns its data, so a field can be handed to a plugin as `Arc<dyn VolumeField>`.
#[cfg(feature = "volume_snow")]
struct OwnedVolume {
    volume: Volume,
    tops: std::sync::Arc<[f32]>,
    grid: bevy_elements::TerrainGrid,
}

#[cfg(feature = "volume_snow")]
impl Sdf for OwnedVolume {
    type Scalar = f32;
    fn sample(&self, p: [f32; 3]) -> f32 {
        let ground = self.grid.sample(&self.tops, bevy::math::Vec2::new(p[0], p[2]));
        self.volume.sdf(p, p[1] - ground)
    }
}
