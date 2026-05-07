use std::{
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};

use bevy_math::{Quat, Vec3};
use vg_csg::{DomeCapZSpec, FloretArmSpec, LevelDsl, MaterialId, TriangleMesh};

const FLOOR: MaterialId = MaterialId(1);
const WALL: MaterialId = MaterialId(2);
const GLASS: MaterialId = MaterialId(3);
const SOLAR: MaterialId = MaterialId(4);
const RADIATOR: MaterialId = MaterialId(5);
const MACHINE: MaterialId = MaterialId(6);
const TOWER: MaterialId = MaterialId(7);
const PARK: MaterialId = MaterialId(8);
const LIGHT: MaterialId = MaterialId(9);

fn main() -> io::Result<()> {
    let mut level = LevelDsl::new();
    add_machine_nave(&mut level);
    add_anchor_city(&mut level);
    add_radial_petals(&mut level);
    add_service_arteries(&mut level);

    let output = level.assemble();
    let out_dir = output_dir();
    fs::create_dir_all(&out_dir)?;
    write_obj_bundle(&out_dir, "dream_machine_level", &output.mesh)?;
    fs::write(
        out_dir.join("dream_machine_level_preview.svg"),
        svg_preview(&output.mesh),
    )?;

    println!(
        "dream_machine_level brushes={} convex_fragments={} vertices={} triangles={} candidate_pairs={} rejected_pairs={} warnings={} obj={} preview={}",
        output.report.input_brushes,
        output.report.emitted_convex_fragments,
        output.mesh.vertex_count(),
        output.mesh.triangle_count(),
        output.report.candidate_pairs,
        output.report.rejected_pairs,
        output.report.warnings.len(),
        out_dir.join("dream_machine_level.obj").display(),
        out_dir.join("dream_machine_level_preview.svg").display(),
    );

    if !output.report.warnings.is_empty() {
        eprintln!("warnings: {:?}", output.report.warnings);
    }

    Ok(())
}

fn add_machine_nave(level: &mut LevelDsl) {
    level.solid_box(
        "machine nave foundation",
        Vec3::new(0.0, 0.0, -0.35),
        Vec3::new(70.0, 42.0, 0.7),
        FLOOR,
    );
    level.solid_box(
        "central reactor plinth",
        Vec3::new(0.0, 0.0, 0.35),
        Vec3::new(15.0, 15.0, 0.7),
        MACHINE,
    );
    level.cylinder_z(
        "reactor throat",
        Vec3::new(0.0, 0.0, 2.0),
        4.4,
        4.0,
        48,
        LIGHT,
    );
    level.dome_cap_z(
        "glass computation dome",
        DomeCapZSpec {
            center: Vec3::new(0.0, 0.0, 0.7),
            radius: 11.0,
            height: 7.0,
            rings: 12,
            segments: 64,
            material: GLASS,
        },
    );

    for side in [-1.0_f32, 1.0] {
        level.solid_box(
            format!("long retaining wall {side}"),
            Vec3::new(0.0, side * 21.0, 2.0),
            Vec3::new(70.0, 1.0, 4.0),
            WALL,
        );
        level.cut_box(
            format!("service arcade cut {side}"),
            Vec3::new(0.0, side * 21.0, 1.6),
            Vec3::new(54.0, 1.4, 2.6),
        );
    }

    for index in 0..18 {
        let x = -32.0 + index as f32 * 3.8;
        let angle = if index % 2 == 0 { 0.42 } else { -0.42 };
        level.cut_oriented_box(
            format!("diagonal vent throat {index}"),
            Vec3::new(x, 0.0, 0.25),
            Vec3::new(0.8, 35.0, 2.0),
            Quat::from_rotation_z(angle),
        );
    }

    for index in 0..9 {
        let x = -30.0 + index as f32 * 7.5;
        level.solid_box(
            format!("overhead rib {index}"),
            Vec3::new(x, 0.0, 5.2),
            Vec3::new(0.8, 43.5, 1.0),
            MACHINE,
        );
    }
}

fn add_anchor_city(level: &mut LevelDsl) {
    for ring in 0..5 {
        let radius = 13.5 + ring as f32 * 3.2;
        let blocks = 10 + ring * 6;
        let height_base = 6.5 - ring as f32 * 0.95;
        let mat = if ring < 2 {
            TOWER
        } else if ring == 3 {
            PARK
        } else {
            WALL
        };

        for index in 0..blocks {
            let t = index as f32 / blocks as f32;
            let angle = t * std::f32::consts::TAU + ring as f32 * 0.21;
            let jitter = seeded_wave(ring as f32 * 17.0 + index as f32 * 3.1);
            let radial = Vec3::new(angle.cos(), angle.sin(), 0.0);
            let tangent_angle = angle + std::f32::consts::FRAC_PI_2;
            let center = radial * (radius + jitter * 0.55);
            let footprint = Vec3::new(1.4 + jitter.abs() * 0.8, 1.0 + ring as f32 * 0.15, 1.0);
            let height = (height_base + jitter * 2.1).max(1.2);

            level.solid_oriented_box(
                format!("anchor city block r{ring} b{index}"),
                Vec3::new(center.x, center.y, height * 0.5),
                Vec3::new(footprint.x, footprint.y, height),
                Quat::from_rotation_z(tangent_angle),
                mat,
            );
        }
    }

    for index in 0..12 {
        let angle = index as f32 * std::f32::consts::TAU / 12.0;
        let radial = Vec3::new(angle.cos(), angle.sin(), 0.0);
        level.solid_oriented_box(
            format!("anchor plaza spoke {index}"),
            radial * 17.0 + Vec3::new(0.0, 0.0, 0.08),
            Vec3::new(18.0, 0.45, 0.16),
            Quat::from_rotation_z(angle),
            LIGHT,
        );
    }
}

fn add_radial_petals(level: &mut LevelDsl) {
    for index in 0..24 {
        let angle = index as f32 * std::f32::consts::TAU / 24.0;
        let direction = Vec3::new(angle.cos(), angle.sin(), 0.0);
        let material = if index % 2 == 0 { SOLAR } else { RADIATOR };
        let length = if index % 3 == 0 { 58.0 } else { 44.0 };
        let lift = if index % 2 == 0 { 2.8 } else { 1.4 };
        level.floret_arm(
            format!("citadel-scale ray floret {index}"),
            FloretArmSpec {
                anchor: direction * 22.0 + Vec3::Z * 0.4,
                direction,
                length,
                root_width: 3.0,
                tip_width: 9.0,
                thickness: 0.16,
                tip_lift: lift,
                material,
            },
        );
    }
}

fn add_service_arteries(level: &mut LevelDsl) {
    for index in 0..16 {
        let angle = index as f32 * std::f32::consts::TAU / 16.0;
        let radius = 24.0 + (index % 4) as f32 * 4.0;
        let center_angle = angle + 0.38;
        let center = Vec3::new(
            center_angle.cos() * radius,
            center_angle.sin() * radius,
            1.1,
        );
        let length = 21.0 + (index % 3) as f32 * 5.0;
        let width = if index % 2 == 0 { 0.7 } else { 1.05 };
        let material = if index % 2 == 0 { MACHINE } else { LIGHT };

        level.solid_oriented_box(
            format!("looping cargo artery {index}"),
            center,
            Vec3::new(length, width, 0.55),
            Quat::from_rotation_z(angle + 0.95),
            material,
        );
    }

    for index in 0..10 {
        let angle = index as f32 * std::f32::consts::TAU / 10.0 + 0.18;
        let radial = Vec3::new(angle.cos(), angle.sin(), 0.0);
        level.solid_box(
            format!("outer machine pier {index}"),
            radial * 38.0 + Vec3::new(0.0, 0.0, 1.4),
            Vec3::new(1.1, 1.1, 2.8),
            MACHINE,
        );
    }
}

fn seeded_wave(value: f32) -> f32 {
    (value.sin() * 0.63 + (value * 1.73).cos() * 0.37).clamp(-1.0, 1.0)
}

fn output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("experiments")
        .join("generated")
        .join("dream_machine_level")
}

fn write_obj_bundle(dir: &Path, name: &str, mesh: &TriangleMesh) -> io::Result<()> {
    fs::write(dir.join(format!("{name}.mtl")), material_library())?;
    fs::write(dir.join(format!("{name}.obj")), obj_text(name, mesh))?;
    Ok(())
}

fn obj_text(name: &str, mesh: &TriangleMesh) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "mtllib {name}.mtl");
    let _ = writeln!(out, "o {name}");

    for position in &mesh.positions {
        let _ = writeln!(out, "v {} {} {}", position.x, position.y, position.z);
    }
    for uv in &mesh.uvs {
        let _ = writeln!(out, "vt {} {}", uv.x, uv.y);
    }
    for normal in &mesh.normals {
        let _ = writeln!(out, "vn {} {} {}", normal.x, normal.y, normal.z);
    }

    let mut active_material = None;
    for (tri_index, face) in mesh.indices.chunks_exact(3).enumerate() {
        let material = mesh
            .triangle_materials
            .get(tri_index)
            .copied()
            .unwrap_or_default();
        if active_material != Some(material) {
            let _ = writeln!(out, "usemtl {}", material_name(material));
            active_material = Some(material);
        }
        let a = face[0] + 1;
        let b = face[1] + 1;
        let c = face[2] + 1;
        let _ = writeln!(out, "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}");
    }

    out
}

fn material_library() -> String {
    [
        material_text(FLOOR, "0.36 0.35 0.31", "0.05 0.05 0.05"),
        material_text(WALL, "0.42 0.43 0.42", "0.04 0.04 0.04"),
        material_text(GLASS, "0.30 0.80 1.00", "0.20 0.55 0.70"),
        material_text(SOLAR, "0.05 0.11 0.20", "0.00 0.18 0.28"),
        material_text(RADIATOR, "0.70 0.58 0.44", "0.28 0.12 0.04"),
        material_text(MACHINE, "0.16 0.17 0.18", "0.06 0.07 0.08"),
        material_text(TOWER, "0.72 0.76 0.78", "0.12 0.16 0.18"),
        material_text(PARK, "0.22 0.46 0.25", "0.02 0.08 0.02"),
        material_text(LIGHT, "0.08 0.75 0.90", "0.00 0.65 0.85"),
    ]
    .join("\n")
}

fn material_text(id: MaterialId, kd: &str, ke: &str) -> String {
    format!("newmtl {}\nKd {kd}\nKe {ke}\nNs 80\n", material_name(id))
}

fn material_name(id: MaterialId) -> &'static str {
    match id {
        FLOOR => "mat_floor",
        WALL => "mat_wall",
        GLASS => "mat_glass",
        SOLAR => "mat_solar",
        RADIATOR => "mat_radiator",
        MACHINE => "mat_machine",
        TOWER => "mat_tower",
        PARK => "mat_park",
        LIGHT => "mat_light",
        _ => "mat_default",
    }
}

fn svg_preview(mesh: &TriangleMesh) -> String {
    let width = 1600.0_f32;
    let height = 1050.0_f32;
    let margin = 42.0_f32;
    let mut projected = Vec::with_capacity(mesh.positions.len());

    for point in &mesh.positions {
        let x = (point.x - point.y) * 0.82;
        let y = (point.x + point.y) * 0.34 - point.z * 1.15;
        let depth = point.x + point.y + point.z * 0.7;
        projected.push((x, y, depth));
    }

    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for (x, y, _) in &projected {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }

    let span_x = (max_x - min_x).max(1.0);
    let span_y = (max_y - min_y).max(1.0);
    let scale = ((width - margin * 2.0) / span_x).min((height - margin * 2.0) / span_y);
    let ox = (width - span_x * scale) * 0.5 - min_x * scale;
    let oy = (height - span_y * scale) * 0.5 - min_y * scale;

    let mut tris = Vec::with_capacity(mesh.triangle_count());
    for (tri_index, face) in mesh.indices.chunks_exact(3).enumerate() {
        let a = projected[face[0] as usize];
        let b = projected[face[1] as usize];
        let c = projected[face[2] as usize];
        let depth = (a.2 + b.2 + c.2) / 3.0;
        let points = [
            (a.0 * scale + ox, a.1 * scale + oy),
            (b.0 * scale + ox, b.1 * scale + oy),
            (c.0 * scale + ox, c.1 * scale + oy),
        ];
        let material = mesh
            .triangle_materials
            .get(tri_index)
            .copied()
            .unwrap_or_default();
        tris.push((depth, material, points));
    }
    tris.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut out = String::new();
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\">"
    );
    let _ = writeln!(
        out,
        "<rect width=\"100%\" height=\"100%\" fill=\"#111315\"/>"
    );
    let _ = writeln!(
        out,
        "<g stroke=\"#07090a\" stroke-width=\"0.45\" stroke-linejoin=\"round\">"
    );
    for (_, material, points) in tris {
        let _ = writeln!(
            out,
            "<polygon points=\"{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}\" fill=\"{}\" opacity=\"{}\"/>",
            points[0].0,
            points[0].1,
            points[1].0,
            points[1].1,
            points[2].0,
            points[2].1,
            svg_fill(material),
            svg_opacity(material),
        );
    }
    let _ = writeln!(out, "</g>");
    let _ = writeln!(out, "</svg>");
    out
}

fn svg_fill(id: MaterialId) -> &'static str {
    match id {
        FLOOR => "#5c5a51",
        WALL => "#6b6d6a",
        GLASS => "#54cff5",
        SOLAR => "#112134",
        RADIATOR => "#b59670",
        MACHINE => "#292c2f",
        TOWER => "#b7c0c4",
        PARK => "#387541",
        LIGHT => "#20d6f0",
        _ => "#808080",
    }
}

fn svg_opacity(id: MaterialId) -> &'static str {
    match id {
        GLASS => "0.42",
        LIGHT => "0.88",
        SOLAR | RADIATOR => "0.94",
        _ => "1.0",
    }
}
