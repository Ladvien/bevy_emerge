//! **A declared `extent.height` is checked against the glb** — issue #83.

use bevy_emerge_core::descriptor::{Align, Descriptor, Extent, Offers};
use bevy_emerge_core::library::{LIBRARY_VERSION, Library};

/// A one-mesh glb whose POSITION accessor spans `ymin..ymax` on Y.
fn glb_bytes(ymin: f32, ymax: f32) -> Vec<u8> {
    let mut json = format!(
        r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}}}}]}}],"accessors":[{{"componentType":5126,"count":2,"type":"VEC3","min":[-0.5,{ymin},-0.5],"max":[0.5,{ymax},0.5]}}]}}"#
    )
    .into_bytes();
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let total = 12 + 8 + json.len();
    let mut out = Vec::new();
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json);
    out
}

fn piece(id: &str, mesh: Option<&str>, height: Option<f32>, surface: bool) -> Descriptor {
    Descriptor {
        id: id.into(),
        mesh: mesh.map(Into::into),
        extent: Extent { footprint: Some((1.0, 1.0)), height },
        offers: Offers {
            surfaces: if surface { vec!["support".into()] } else { vec![] },
            ..Offers::default()
        },
        align: Align::default(),
        ..Descriptor::default()
    }
}

#[test]
fn only_the_short_surface_host_disagrees() {
    let dir = std::env::temp_dir().join(format!("emerge_height_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // Mesh from -0.052 to 0.893: 0.945 tall.
    std::fs::write(dir.join("honest.glb"), glb_bytes(-0.052, 0.893)).unwrap();
    std::fs::write(dir.join("short.glb"), glb_bytes(-0.052, 0.893)).unwrap();
    std::fs::write(dir.join("lamp.glb"), glb_bytes(0.0, 2.0)).unwrap();
    std::fs::write(dir.join("broken.glb"), b"not a glb").unwrap();

    let lib = Library {
        version: LIBRARY_VERSION,
        note: None,
        descriptors: vec![
            piece("honest", Some("honest.glb"), Some(0.945), true),
            piece("short", Some("short.glb"), Some(0.908), true),
            piece("lamp", Some("lamp.glb"), Some(9.0), false),
            piece("broken", Some("broken.glb"), Some(1.0), true),
        ],
    };
    let check = lib.height_disagreements(&dir, 0.005);
    std::fs::remove_dir_all(&dir).ok();

    assert_eq!(check.disagreements.len(), 1, "{:?}", check.disagreements);
    let d = &check.disagreements[0];
    assert_eq!(d.id, "short");
    assert!((d.error - (-0.037)).abs() < 1e-4, "error {}", d.error);
    assert_eq!(check.unmeasured.len(), 1);
    assert_eq!(check.unmeasured[0].0, "broken");
}
