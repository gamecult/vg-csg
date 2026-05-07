use std::{
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};

use bevy_math::{Quat, Vec3};
use vg_csg::{DomeCapZSpec, LevelDsl, MaterialId, TriangleMesh};

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
    add_petal_hinges(&mut level);
    add_service_arteries(&mut level);

    let mut output = level.assemble();
    append_curved_ray_florets(&mut output.mesh);
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
    level.cylinder_z(
        "round machine garden foundation",
        Vec3::new(0.0, 0.0, -0.22),
        26.0,
        0.44,
        96,
        FLOOR,
    );
    level.cylinder_z(
        "inner raised city terrace",
        Vec3::new(0.0, 0.0, 0.08),
        19.0,
        0.34,
        96,
        WALL,
    );
    level.solid_box(
        "central reactor plinth",
        Vec3::new(0.0, 0.0, 0.45),
        Vec3::new(13.0, 13.0, 0.9),
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

    for index in 0..16 {
        let angle = index as f32 * std::f32::consts::TAU / 16.0;
        let radial = Vec3::new(angle.cos(), angle.sin(), 0.0);
        level.cut_oriented_box(
            format!("diagonal vent throat {index}"),
            radial * 17.5 + Vec3::new(0.0, 0.0, 0.24),
            Vec3::new(1.1, 15.0, 1.5),
            Quat::from_rotation_z(angle + 0.55),
        );
    }
}

fn add_anchor_city(level: &mut LevelDsl) {
    for ring in 0..4 {
        let radius = 11.8 + ring as f32 * 3.0;
        let blocks = 8 + ring * 8;
        let height_base = if ring == 0 {
            5.8
        } else {
            4.4 - ring as f32 * 0.65
        };
        let mat = if ring == 0 {
            TOWER
        } else if ring == 2 {
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
            let footprint = Vec3::new(1.05 + jitter.abs() * 0.65, 0.85 + ring as f32 * 0.12, 1.0);
            let height = (height_base + jitter * 1.5).max(0.55);

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
            radial * 14.5 + Vec3::new(0.0, 0.0, 0.34),
            Vec3::new(12.0, 0.34, 0.12),
            Quat::from_rotation_z(angle),
            LIGHT,
        );
    }
}

fn add_petal_hinges(level: &mut LevelDsl) {
    for index in 0..18 {
        let angle = index as f32 * std::f32::consts::TAU / 18.0;
        let direction = Vec3::new(angle.cos(), angle.sin(), 0.0);
        level.solid_oriented_box(
            format!("petal hinge spine {index}"),
            direction * 28.0 + Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(5.0, 0.55, 1.2),
            Quat::from_rotation_z(angle),
            MACHINE,
        );
    }
}

#[derive(Clone, Copy, Debug)]
struct CurvedPanelSpec {
    anchor: Vec3,
    direction: Vec3,
    length: f32,
    root_width: f32,
    tip_width: f32,
    thickness: f32,
    lift: f32,
    bend: f32,
    segments: usize,
    material: MaterialId,
}

fn append_curved_ray_florets(mesh: &mut TriangleMesh) {
    for index in 0..18 {
        let angle = index as f32 * std::f32::consts::TAU / 18.0;
        let direction = Vec3::new(angle.cos(), angle.sin(), 0.0);
        let material = if index % 2 == 0 { SOLAR } else { RADIATOR };
        append_curved_panel(
            mesh,
            CurvedPanelSpec {
                anchor: direction * 29.0 + Vec3::Z * 0.58,
                direction,
                length: if index % 3 == 0 { 74.0 } else { 58.0 },
                root_width: 5.2,
                tip_width: 16.5,
                thickness: 0.16,
                lift: if index % 2 == 0 { 7.0 } else { 4.4 },
                bend: if index % 2 == 0 { 7.5 } else { -5.5 },
                segments: 9,
                material,
            },
        );
    }
}

fn append_curved_panel(mesh: &mut TriangleMesh, spec: CurvedPanelSpec) {
    let forward = spec.direction.normalize_or_zero();
    let forward = if forward.length_squared() == 0.0 {
        Vec3::X
    } else {
        forward
    };
    let mut side = forward.cross(Vec3::Z).normalize_or_zero();
    if side.length_squared() == 0.0 {
        side = Vec3::Y;
    }

    let segments = spec.segments.max(2);
    let mut left_top = Vec::with_capacity(segments + 1);
    let mut right_top = Vec::with_capacity(segments + 1);
    let mut left_bottom = Vec::with_capacity(segments + 1);
    let mut right_bottom = Vec::with_capacity(segments + 1);
    let half_thickness = Vec3::Z * (spec.thickness * 0.5);

    for index in 0..=segments {
        let t = index as f32 / segments as f32;
        let smooth_t = t * t * (3.0 - 2.0 * t);
        let width = spec.root_width + (spec.tip_width - spec.root_width) * smooth_t;
        let center = spec.anchor
            + forward * (spec.length * t)
            + side * (spec.bend * t * (1.0 - t))
            + Vec3::Z * (spec.lift * smooth_t);
        let half_width = side * (width * 0.5);
        left_top.push(center - half_width + half_thickness);
        right_top.push(center + half_width + half_thickness);
        left_bottom.push(center - half_width - half_thickness);
        right_bottom.push(center + half_width - half_thickness);
    }

    for index in 0..segments {
        mesh.append_quad(
            [
                left_top[index],
                right_top[index],
                right_top[index + 1],
                left_top[index + 1],
            ],
            Vec3::Z,
            spec.material,
        );
        mesh.append_quad(
            [
                left_bottom[index],
                left_bottom[index + 1],
                right_bottom[index + 1],
                right_bottom[index],
            ],
            Vec3::NEG_Z,
            spec.material,
        );
        mesh.append_quad(
            [
                left_bottom[index],
                left_top[index],
                left_top[index + 1],
                left_bottom[index + 1],
            ],
            -side,
            spec.material,
        );
        mesh.append_quad(
            [
                right_bottom[index],
                right_bottom[index + 1],
                right_top[index + 1],
                right_top[index],
            ],
            side,
            spec.material,
        );
    }

    mesh.append_quad(
        [left_bottom[0], right_bottom[0], right_top[0], left_top[0]],
        -forward,
        spec.material,
    );
    mesh.append_quad(
        [
            left_bottom[segments],
            left_top[segments],
            right_top[segments],
            right_bottom[segments],
        ],
        forward,
        spec.material,
    );
}

fn add_service_arteries(level: &mut LevelDsl) {
    for ring in 0..3 {
        let segments = 18 + ring * 6;
        let radius = 23.0 + ring as f32 * 4.8;
        for index in 0..segments {
            if (index + ring) % 3 == 0 {
                continue;
            }
            let angle = index as f32 * std::f32::consts::TAU / segments as f32;
            let tangent = angle + std::f32::consts::FRAC_PI_2;
            let center = Vec3::new(angle.cos() * radius, angle.sin() * radius, 0.92);
            let length = std::f32::consts::TAU * radius / segments as f32 * 0.82;
            let material = if ring == 1 { LIGHT } else { MACHINE };

            level.solid_oriented_box(
                format!("orbiting cargo artery r{ring} s{index}"),
                center,
                Vec3::new(length, 0.52, 0.36),
                Quat::from_rotation_z(tangent),
                material,
            );
        }
    }

    for index in 0..18 {
        let angle = index as f32 * std::f32::consts::TAU / 18.0 + 0.18;
        let radial = Vec3::new(angle.cos(), angle.sin(), 0.0);
        level.solid_oriented_box(
            format!("outer machine pier {index}"),
            radial * 26.0 + Vec3::new(0.0, 0.0, 1.05),
            Vec3::new(2.0, 0.65, 2.1),
            Quat::from_rotation_z(angle),
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
