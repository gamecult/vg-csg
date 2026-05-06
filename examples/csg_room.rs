use bevy_math::Vec3;
use vg_csg::{DomeCapZSpec, FloretArmSpec, LevelDsl, MaterialId};

fn main() {
    let mut level = LevelDsl::new();
    level.solid_box(
        "floor",
        Vec3::new(0.0, 0.0, -0.1),
        Vec3::new(10.0, 10.0, 0.2),
        MaterialId(1),
    );
    level.solid_box(
        "wall with doorway",
        Vec3::new(0.0, 5.0, 1.5),
        Vec3::new(10.0, 0.3, 3.0),
        MaterialId(2),
    );
    level.cut_box(
        "doorway",
        Vec3::new(0.0, 5.0, 1.0),
        Vec3::new(1.4, 0.6, 2.0),
    );
    level.dome_cap_z(
        "flat-base city dome",
        DomeCapZSpec {
            center: Vec3::new(0.0, 0.0, 0.0),
            radius: 3.0,
            height: 1.8,
            rings: 8,
            segments: 32,
            material: MaterialId(3),
        },
    );
    level.floret_arm(
        "sunflower radiator arm",
        FloretArmSpec {
            anchor: Vec3::new(3.0, 0.0, 0.1),
            direction: Vec3::X,
            length: 16.0,
            root_width: 1.5,
            tip_width: 4.0,
            thickness: 0.08,
            tip_lift: 0.8,
            material: MaterialId(4),
        },
    );

    let output = level.assemble();
    println!(
        "brushes={} box_fragments={} vertices={} triangles={} warnings={}",
        output.report.input_brushes,
        output.report.emitted_box_fragments,
        output.mesh.vertex_count(),
        output.mesh.triangle_count(),
        output.report.warnings.len()
    );
}
