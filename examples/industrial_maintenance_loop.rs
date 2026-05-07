use std::{
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};

use bevy_math::Vec3;
use vg_csg::{LevelDsl, MaterialId, TriangleMesh};

const DECK: MaterialId = MaterialId(21);
const SHELL: MaterialId = MaterialId(22);
const DARK: MaterialId = MaterialId(23);
const MACHINE: MaterialId = MaterialId(24);
const PIPE: MaterialId = MaterialId(25);
const AMBER: MaterialId = MaterialId(26);
const BLUE: MaterialId = MaterialId(27);
const GLASS: MaterialId = MaterialId(28);
const HAZARD: MaterialId = MaterialId(29);
const GEAR: MaterialId = MaterialId(30);

const INNER_LOOP_RADIUS: f32 = 6.6;
const OUTER_LOOP_RADIUS: f32 = 16.0;
const ROOM_HEIGHT: f32 = 7.2;

fn main() -> io::Result<()> {
    let mut level = LevelDsl::new();
    let mut shell_mesh = TriangleMesh::new();

    append_curved_room_shell(&mut shell_mesh);
    append_machinery_island_shroud(&mut shell_mesh);
    add_central_machinery_island(&mut level);
    add_inner_manifold_faces(&mut level);
    add_outer_staging_zone(&mut level);
    add_supervisor_gallery(&mut level);
    add_service_arteries_and_harness_gear(&mut level);
    add_ribs_and_panel_language(&mut level);

    let mut output = level.assemble();
    let index_offset = output.mesh.positions.len() as u32;
    output.mesh.positions.extend(shell_mesh.positions);
    output.mesh.normals.extend(shell_mesh.normals);
    output.mesh.uvs.extend(shell_mesh.uvs);
    output.mesh.indices.extend(
        shell_mesh
            .indices
            .into_iter()
            .map(|index| index + index_offset),
    );
    output
        .mesh
        .triangle_materials
        .extend(shell_mesh.triangle_materials);

    let out_dir = output_dir();
    fs::create_dir_all(&out_dir)?;
    write_obj_bundle(&out_dir, "industrial_maintenance_loop", &output.mesh)?;
    fs::write(
        out_dir.join("industrial_maintenance_loop_preview.svg"),
        svg_preview(&output.mesh),
    )?;

    println!(
        "industrial_maintenance_loop brushes={} convex_fragments={} vertices={} triangles={} candidate_pairs={} rejected_pairs={} warnings={} obj={} preview={}",
        output.report.input_brushes,
        output.report.emitted_convex_fragments,
        output.mesh.vertex_count(),
        output.mesh.triangle_count(),
        output.report.candidate_pairs,
        output.report.rejected_pairs,
        output.report.warnings.len(),
        out_dir.join("industrial_maintenance_loop.obj").display(),
        out_dir
            .join("industrial_maintenance_loop_preview.svg")
            .display(),
    );

    if !output.report.warnings.is_empty() {
        eprintln!("warnings: {:?}", output.report.warnings);
    }

    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct LoopFrame {
    angle: f32,
    radius: f32,
}

impl LoopFrame {
    fn at(angle: f32, radius: f32) -> Self {
        Self { angle, radius }
    }

    fn tangent(self) -> Vec3 {
        Vec3::new(-self.angle.sin(), self.angle.cos(), 0.0)
    }

    fn radial(self) -> Vec3 {
        Vec3::new(self.angle.cos(), self.angle.sin(), 0.0)
    }

    fn point(self, tangent: f32, radial: f32, z: f32) -> Vec3 {
        self.radial() * (self.radius + radial) + self.tangent() * tangent + Vec3::Z * z
    }

    fn yaw(self) -> bevy_math::Quat {
        bevy_math::Quat::from_rotation_z(self.angle + std::f32::consts::FRAC_PI_2)
    }
}

#[derive(Clone, Copy, Debug)]
struct LoopBox {
    tangent: f32,
    radial: f32,
    z: f32,
    size_tangent: f32,
    size_radial: f32,
    size_z: f32,
    material: MaterialId,
}

fn loop_box(level: &mut LevelDsl, name: impl Into<String>, frame: LoopFrame, spec: LoopBox) {
    level.solid_oriented_box(
        name,
        frame.point(spec.tangent, spec.radial, spec.z),
        Vec3::new(spec.size_tangent, spec.size_radial, spec.size_z),
        frame.yaw(),
        spec.material,
    );
}

fn append_curved_room_shell(mesh: &mut TriangleMesh) {
    append_annular_band(
        mesh,
        ArcBandSpec::new(INNER_LOOP_RADIUS, OUTER_LOOP_RADIUS, -2.85, 2.85, 0.0, DECK).segments(56),
    );
    append_annular_band(
        mesh,
        ArcBandSpec::new(
            INNER_LOOP_RADIUS - 0.35,
            OUTER_LOOP_RADIUS + 0.25,
            -2.85,
            2.85,
            ROOM_HEIGHT,
            SHELL,
        )
        .segments(56),
    );
    append_vertical_arc_wall(
        mesh,
        ArcWallSpec::new(
            OUTER_LOOP_RADIUS + 0.1,
            -2.85,
            2.85,
            0.0,
            ROOM_HEIGHT,
            SHELL,
        )
        .segments(56),
    );
    append_vertical_arc_wall(
        mesh,
        ArcWallSpec::new(
            INNER_LOOP_RADIUS - 0.25,
            -2.85,
            2.85,
            0.0,
            ROOM_HEIGHT,
            DARK,
        )
        .segments(56),
    );

    for index in 0..28 {
        let a0 = -2.85 + index as f32 * (5.70 / 28.0);
        let a1 = a0 + 5.70 / 28.0 * 0.82;
        append_annular_band(
            mesh,
            ArcBandSpec::new(
                INNER_LOOP_RADIUS + 0.4,
                OUTER_LOOP_RADIUS - 0.55,
                a0,
                a1,
                0.035,
                if index % 5 == 0 { HAZARD } else { DECK },
            ),
        );
    }
}

fn add_central_machinery_island(level: &mut LevelDsl) {
    for index in 0..24 {
        let angle = index as f32 * std::f32::consts::TAU / 24.0;
        let frame = LoopFrame::at(angle, 5.45);
        let height = 2.0 + (index % 4) as f32 * 0.75;
        loop_box(
            level,
            format!("machinery vertical casing {index}"),
            frame,
            LoopBox {
                tangent: 0.0,
                radial: 0.0,
                z: height * 0.5 + 0.35,
                size_tangent: 0.85,
                size_radial: 0.7,
                size_z: height,
                material: if index % 3 == 0 { PIPE } else { MACHINE },
            },
        );
        loop_box(
            level,
            format!("amber island screen {index}"),
            frame,
            LoopBox {
                tangent: 0.0,
                radial: 0.38,
                z: 1.55 + (index % 3) as f32 * 0.55,
                size_tangent: 0.48,
                size_radial: 0.08,
                size_z: 0.42,
                material: AMBER,
            },
        );
    }
}

fn append_machinery_island_shroud(mesh: &mut TriangleMesh) {
    append_vertical_arc_wall(
        mesh,
        ArcWallSpec::new(
            5.25,
            -std::f32::consts::PI,
            std::f32::consts::PI,
            0.15,
            ROOM_HEIGHT,
            DARK,
        )
        .segments(72),
    );
    append_vertical_arc_wall(
        mesh,
        ArcWallSpec::new(
            5.72,
            -std::f32::consts::PI,
            std::f32::consts::PI,
            0.0,
            0.45,
            MACHINE,
        )
        .segments(72),
    );
    append_vertical_arc_wall(
        mesh,
        ArcWallSpec::new(
            5.72,
            -std::f32::consts::PI,
            std::f32::consts::PI,
            ROOM_HEIGHT - 0.45,
            ROOM_HEIGHT,
            MACHINE,
        )
        .segments(72),
    );
}

fn add_inner_manifold_faces(level: &mut LevelDsl) {
    for bay in 0..18 {
        let angle = -2.55 + bay as f32 * (5.10 / 17.0);
        let frame = LoopFrame::at(angle, INNER_LOOP_RADIUS);
        loop_box(
            level,
            format!("gasket-rimmed manifold panel {bay}"),
            frame,
            LoopBox {
                tangent: 0.0,
                radial: -0.35,
                z: 1.45,
                size_tangent: 1.05,
                size_radial: 0.22,
                size_z: 1.9,
                material: MACHINE,
            },
        );
        loop_box(
            level,
            format!("round access hatch square proxy {bay}"),
            frame,
            LoopBox {
                tangent: -0.18,
                radial: -0.52,
                z: 1.25,
                size_tangent: 0.52,
                size_radial: 0.12,
                size_z: 0.52,
                material: DARK,
            },
        );
        loop_box(
            level,
            format!("diagnostic amber strip {bay}"),
            frame,
            LoopBox {
                tangent: 0.26,
                radial: -0.55,
                z: 1.95,
                size_tangent: 0.36,
                size_radial: 0.08,
                size_z: 0.58,
                material: AMBER,
            },
        );

        if bay % 2 == 0 {
            add_pipe_bundle(level, frame, bay);
        }
    }
}

fn add_pipe_bundle(level: &mut LevelDsl, frame: LoopFrame, index: usize) {
    for pipe in 0..3 {
        loop_box(
            level,
            format!("inner manifold cable bundle {index}-{pipe}"),
            frame,
            LoopBox {
                tangent: -0.45 + pipe as f32 * 0.45,
                radial: -0.78,
                z: 3.2 + pipe as f32 * 0.42,
                size_tangent: 0.16,
                size_radial: 0.28,
                size_z: 2.3,
                material: PIPE,
            },
        );
    }
}

fn add_outer_staging_zone(level: &mut LevelDsl) {
    for bay in 0..20 {
        let angle = -2.65 + bay as f32 * (5.30 / 19.0);
        let frame = LoopFrame::at(angle, OUTER_LOOP_RADIUS - 1.15);
        loop_box(
            level,
            format!("bolted outer anchor rail {bay}"),
            frame,
            LoopBox {
                tangent: 0.0,
                radial: 0.0,
                z: 0.55,
                size_tangent: 1.25,
                size_radial: 0.18,
                size_z: 0.18,
                material: HAZARD,
            },
        );
        loop_box(
            level,
            format!("sliding rail block {bay}"),
            frame,
            LoopBox {
                tangent: if bay % 2 == 0 { -0.32 } else { 0.34 },
                radial: -0.2,
                z: 0.78,
                size_tangent: 0.28,
                size_radial: 0.42,
                size_z: 0.32,
                material: MACHINE,
            },
        );

        if bay % 4 == 1 {
            add_tool_locker(level, frame, bay);
        }
        if bay % 5 == 2 {
            add_cart_and_rescue_line(level, frame, bay);
        }
    }
}

fn add_tool_locker(level: &mut LevelDsl, frame: LoopFrame, bay: usize) {
    loop_box(
        level,
        format!("outer tool locker {bay}"),
        frame,
        LoopBox {
            tangent: 0.0,
            radial: 0.78,
            z: 1.05,
            size_tangent: 0.72,
            size_radial: 0.55,
            size_z: 1.8,
            material: MACHINE,
        },
    );
    loop_box(
        level,
        format!("blue locker readout {bay}"),
        frame,
        LoopBox {
            tangent: 0.18,
            radial: 0.48,
            z: 1.45,
            size_tangent: 0.26,
            size_radial: 0.06,
            size_z: 0.36,
            material: BLUE,
        },
    );
}

fn add_cart_and_rescue_line(level: &mut LevelDsl, frame: LoopFrame, bay: usize) {
    loop_box(
        level,
        format!("low maintenance cart {bay}"),
        frame,
        LoopBox {
            tangent: -0.18,
            radial: -0.72,
            z: 0.38,
            size_tangent: 0.85,
            size_radial: 0.55,
            size_z: 0.55,
            material: GEAR,
        },
    );
    loop_box(
        level,
        format!("clipped rescue line {bay}"),
        frame,
        LoopBox {
            tangent: 0.36,
            radial: -0.62,
            z: 0.88,
            size_tangent: 0.1,
            size_radial: 1.15,
            size_z: 0.1,
            material: PIPE,
        },
    );
}

fn add_supervisor_gallery(level: &mut LevelDsl) {
    let frame = LoopFrame::at(-1.85, OUTER_LOOP_RADIUS - 1.0);
    loop_box(
        level,
        "single supervisor gallery deck",
        frame,
        LoopBox {
            tangent: 0.0,
            radial: 0.1,
            z: 3.65,
            size_tangent: 5.4,
            size_radial: 1.35,
            size_z: 0.28,
            material: MACHINE,
        },
    );
    loop_box(
        level,
        "supervisor gallery glass face",
        frame,
        LoopBox {
            tangent: 0.0,
            radial: -0.65,
            z: 4.55,
            size_tangent: 5.2,
            size_radial: 0.12,
            size_z: 1.35,
            material: GLASS,
        },
    );
    for step in 0..9 {
        loop_box(
            level,
            format!("visible gallery stair step {step}"),
            frame,
            LoopBox {
                tangent: -3.2 + step as f32 * 0.34,
                radial: -1.6 + step as f32 * 0.12,
                z: 0.42 + step as f32 * 0.33,
                size_tangent: 0.54,
                size_radial: 0.72,
                size_z: 0.12,
                material: HAZARD,
            },
        );
    }
    for panel in 0..4 {
        loop_box(
            level,
            format!("blue supervisor console {panel}"),
            frame,
            LoopBox {
                tangent: -1.8 + panel as f32 * 1.2,
                radial: 0.08,
                z: 4.15,
                size_tangent: 0.75,
                size_radial: 0.18,
                size_z: 0.42,
                material: BLUE,
            },
        );
    }
}

fn add_service_arteries_and_harness_gear(level: &mut LevelDsl) {
    for mouth in 0..7 {
        let angle = -0.55 + mouth as f32 * 0.18;
        let frame = LoopFrame::at(angle, INNER_LOOP_RADIUS);
        loop_box(
            level,
            format!("dark service artery mouth {mouth}"),
            frame,
            LoopBox {
                tangent: 0.0,
                radial: -0.72,
                z: 1.0 + (mouth % 3) as f32 * 0.55,
                size_tangent: 0.5,
                size_radial: 0.32,
                size_z: 0.42,
                material: DARK,
            },
        );
        loop_box(
            level,
            format!("pipe tunnel throat {mouth}"),
            frame,
            LoopBox {
                tangent: 0.0,
                radial: -1.35,
                z: 1.0 + (mouth % 3) as f32 * 0.55,
                size_tangent: 0.32,
                size_radial: 1.2,
                size_z: 0.28,
                material: PIPE,
            },
        );
    }

    let rack_frame = LoopFrame::at(0.78, OUTER_LOOP_RADIUS - 0.8);
    loop_box(
        level,
        "dry-operation mobility gear wall rack",
        rack_frame,
        LoopBox {
            tangent: 0.0,
            radial: 0.65,
            z: 1.65,
            size_tangent: 2.1,
            size_radial: 0.24,
            size_z: 2.2,
            material: MACHINE,
        },
    );
    for item in 0..8 {
        loop_box(
            level,
            format!("folded harness loop and cartridge {item}"),
            rack_frame,
            LoopBox {
                tangent: -0.85 + item as f32 * 0.24,
                radial: 0.38,
                z: 0.8 + (item % 4) as f32 * 0.42,
                size_tangent: 0.12,
                size_radial: 0.18,
                size_z: 0.62,
                material: if item % 3 == 0 { BLUE } else { GEAR },
            },
        );
    }
    for tube in 0..5 {
        loop_box(
            level,
            format!("coiled oxygenation tube proxy {tube}"),
            rack_frame,
            LoopBox {
                tangent: -0.75 + tube as f32 * 0.38,
                radial: -0.42,
                z: 0.46,
                size_tangent: 0.28,
                size_radial: 0.28,
                size_z: 0.24,
                material: PIPE,
            },
        );
    }
}

fn add_ribs_and_panel_language(level: &mut LevelDsl) {
    for rib in 0..18 {
        let angle = -2.75 + rib as f32 * (5.50 / 17.0);
        let frame = LoopFrame::at(angle, (INNER_LOOP_RADIUS + OUTER_LOOP_RADIUS) * 0.5);
        loop_box(
            level,
            format!("manufactured shell rib {rib}"),
            frame,
            LoopBox {
                tangent: 0.0,
                radial: 0.0,
                z: ROOM_HEIGHT - 0.28,
                size_tangent: 0.18,
                size_radial: OUTER_LOOP_RADIUS - INNER_LOOP_RADIUS + 0.8,
                size_z: 0.48,
                material: MACHINE,
            },
        );
        let outer = LoopFrame::at(angle, OUTER_LOOP_RADIUS - 0.15);
        loop_box(
            level,
            format!("outer shell pressure housing {rib}"),
            outer,
            LoopBox {
                tangent: 0.0,
                radial: 0.0,
                z: 3.15,
                size_tangent: 0.75,
                size_radial: 0.42,
                size_z: 1.7,
                material: if rib % 4 == 0 { AMBER } else { SHELL },
            },
        );
    }
}

#[derive(Clone, Copy, Debug)]
struct ArcBandSpec {
    inner: f32,
    outer: f32,
    start: f32,
    end: f32,
    segments: usize,
    z: f32,
    material: MaterialId,
}

impl ArcBandSpec {
    fn new(inner: f32, outer: f32, start: f32, end: f32, z: f32, material: MaterialId) -> Self {
        Self {
            inner,
            outer,
            start,
            end,
            segments: 1,
            z,
            material,
        }
    }

    fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

#[derive(Clone, Copy, Debug)]
struct ArcWallSpec {
    radius: f32,
    start: f32,
    end: f32,
    segments: usize,
    z_min: f32,
    z_max: f32,
    material: MaterialId,
}

impl ArcWallSpec {
    fn new(
        radius: f32,
        start: f32,
        end: f32,
        z_min: f32,
        z_max: f32,
        material: MaterialId,
    ) -> Self {
        Self {
            radius,
            start,
            end,
            segments: 1,
            z_min,
            z_max,
            material,
        }
    }

    fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

fn append_annular_band(mesh: &mut TriangleMesh, spec: ArcBandSpec) {
    let segments = spec.segments.max(1);
    for index in 0..segments {
        let a0 = spec.start + (spec.end - spec.start) * index as f32 / segments as f32;
        let a1 = spec.start + (spec.end - spec.start) * (index + 1) as f32 / segments as f32;
        let p0 = polar_point(spec.inner, a0, spec.z);
        let p1 = polar_point(spec.outer, a0, spec.z);
        let p2 = polar_point(spec.outer, a1, spec.z);
        let p3 = polar_point(spec.inner, a1, spec.z);
        let normal = if spec.z > ROOM_HEIGHT * 0.5 {
            Vec3::NEG_Z
        } else {
            Vec3::Z
        };
        mesh.append_quad([p0, p1, p2, p3], normal, spec.material);
    }
}

fn append_vertical_arc_wall(mesh: &mut TriangleMesh, spec: ArcWallSpec) {
    let segments = spec.segments.max(1);
    for index in 0..segments {
        let a0 = spec.start + (spec.end - spec.start) * index as f32 / segments as f32;
        let a1 = spec.start + (spec.end - spec.start) * (index + 1) as f32 / segments as f32;
        let p0 = polar_point(spec.radius, a0, spec.z_min);
        let p1 = polar_point(spec.radius, a1, spec.z_min);
        let p2 = polar_point(spec.radius, a1, spec.z_max);
        let p3 = polar_point(spec.radius, a0, spec.z_max);
        let normal = Vec3::new(((a0 + a1) * 0.5).cos(), ((a0 + a1) * 0.5).sin(), 0.0);
        mesh.append_quad([p0, p1, p2, p3], normal, spec.material);
    }
}

fn polar_point(radius: f32, angle: f32, z: f32) -> Vec3 {
    Vec3::new(angle.cos() * radius, angle.sin() * radius, z)
}

fn output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("experiments")
        .join("generated")
        .join("industrial_maintenance_loop")
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
        material_text(DECK, "0.21 0.20 0.18", "0.015 0.012 0.010"),
        material_text(SHELL, "0.28 0.26 0.22", "0.015 0.012 0.010"),
        material_text(DARK, "0.045 0.043 0.040", "0.0 0.0 0.0"),
        material_text(MACHINE, "0.18 0.17 0.15", "0.018 0.015 0.012"),
        material_text(PIPE, "0.09 0.085 0.075", "0.01 0.008 0.006"),
        material_text(AMBER, "0.95 0.48 0.12", "0.70 0.26 0.035"),
        material_text(BLUE, "0.08 0.55 0.95", "0.03 0.28 0.60"),
        material_text(GLASS, "0.30 0.65 0.90", "0.04 0.20 0.36"),
        material_text(HAZARD, "0.85 0.58 0.12", "0.20 0.09 0.02"),
        material_text(GEAR, "0.18 0.22 0.24", "0.02 0.025 0.03"),
    ]
    .join("\n")
}

fn material_text(id: MaterialId, kd: &str, ke: &str) -> String {
    format!("newmtl {}\nKd {kd}\nKe {ke}\nNs 65\n", material_name(id))
}

fn material_name(id: MaterialId) -> &'static str {
    match id {
        DECK => "mat_scuffed_deck",
        SHELL => "mat_curved_shell",
        DARK => "mat_dark_substrate",
        MACHINE => "mat_machinery",
        PIPE => "mat_pipe_trunks",
        AMBER => "mat_amber_screens",
        BLUE => "mat_blue_diagnostics",
        GLASS => "mat_gallery_glass",
        HAZARD => "mat_anchor_hazard",
        GEAR => "mat_mobility_gear",
        _ => "mat_default",
    }
}

fn svg_preview(mesh: &TriangleMesh) -> String {
    let width = 1600.0_f32;
    let height = 1050.0_f32;
    let margin = 34.0_f32;
    let mut projected = Vec::with_capacity(mesh.positions.len());

    for point in &mesh.positions {
        let x = (point.x - point.y) * 0.92;
        let y = (point.x + point.y) * 0.38 - point.z * 1.35;
        let depth = point.x + point.y + point.z * 0.75;
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
        "<rect width=\"100%\" height=\"100%\" fill=\"#090908\"/>"
    );
    let _ = writeln!(
        out,
        "<g stroke=\"#050504\" stroke-width=\"0.42\" stroke-linejoin=\"round\">"
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
        DECK => "#35322d",
        SHELL => "#474239",
        DARK => "#10100f",
        MACHINE => "#2c2924",
        PIPE => "#181612",
        AMBER => "#e88b23",
        BLUE => "#1796dd",
        GLASS => "#68b9df",
        HAZARD => "#d79827",
        GEAR => "#29363b",
        _ => "#808080",
    }
}

fn svg_opacity(id: MaterialId) -> &'static str {
    match id {
        SHELL => "0.28",
        DECK => "0.84",
        DARK => "0.72",
        GLASS => "0.36",
        AMBER | BLUE => "0.92",
        _ => "1.0",
    }
}
