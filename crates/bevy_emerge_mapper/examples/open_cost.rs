//! Throwaway: what a project open actually spends its time on.

fn main() {
    let Some(root) = std::env::args().nth(1) else {
        eprintln!("usage: open_cost <project-root>");
        return;
    };
    let root = std::path::PathBuf::from(root);
    for round in 0..3 {
        let start = std::time::Instant::now();
        let project = match bevy_emerge_mapper::project::Project::open(&root, None) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };
        let open = start.elapsed();
        println!(
            "round {round}: open {:.1} ms ({} descriptors, {} kits)",
            open.as_secs_f64() * 1000.0,
            project.library.descriptors.len(),
            project.kits.len()
        );
        for kit in &project.kits {
            let start = std::time::Instant::now();
            let laid = bevy_emerge_core::policy::layered_library(&kit.dir).unwrap();
            println!(
                "  kit {} : layered {:.1} ms ({} rows)",
                kit.dir.file_name().unwrap().to_string_lossy(),
                start.elapsed().as_secs_f64() * 1000.0,
                laid.library.descriptors.len()
            );
        }
        let start = std::time::Instant::now();
        let mut p = project;
        p.remeasure_triangles();
        println!(
            "  triangles (whole library): {:.1} ms",
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
}
