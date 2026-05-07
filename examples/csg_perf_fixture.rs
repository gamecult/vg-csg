use std::{
    hint::black_box,
    time::{Duration, Instant},
};

use bevy_math::{Quat, Vec3};
use vg_csg::{Aabb, Assembler, BrushOp, MaterialId, Primitive};

const WARMUP_ITERS: usize = 8;
const MEASURE_ITERS: usize = 64;

fn main() {
    let cases = [
        PerfCase {
            name: "single_center_cut",
            brushes: 2,
            build: single_center_cut,
        },
        PerfCase {
            name: "room_grid_8x8_doors",
            brushes: 192,
            build: room_grid_8x8_doors,
        },
        PerfCase {
            name: "rotated_cut_stack_64",
            brushes: 65,
            build: rotated_cut_stack_64,
        },
        PerfCase {
            name: "common_box_chain_64",
            brushes: 64,
            build: common_box_chain_64,
        },
        PerfCase {
            name: "distant_cutters_512",
            brushes: 513,
            build: distant_cutters_512,
        },
        PerfCase {
            name: "distant_oriented_cutters_128",
            brushes: 129,
            build: distant_oriented_cutters_128,
        },
    ];

    for case in cases {
        run_case(case, PerfMode::Stable);
        run_case(case, PerfMode::Dirty);
        run_case(case, PerfMode::IncrementalDirty);
        run_case(case, PerfMode::Routed);
    }
}

#[derive(Clone, Copy)]
struct PerfCase {
    name: &'static str,
    brushes: usize,
    build: fn() -> Assembler,
}

#[derive(Clone, Copy)]
enum PerfMode {
    Stable,
    Dirty,
    IncrementalDirty,
    Routed,
}

impl PerfMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Dirty => "dirty",
            Self::IncrementalDirty => "incremental_dirty",
            Self::Routed => "routed",
        }
    }

    fn build(self, assembler: &mut Assembler, iteration: usize) -> vg_csg::BuildOutput {
        match self {
            Self::Stable => assembler.build(),
            Self::Dirty => {
                nudge_tail_brush(assembler, iteration);
                assembler.rebuild()
            }
            Self::IncrementalDirty => {
                nudge_tail_brush(assembler, iteration);
                assembler.build_incremental()
            }
            Self::Routed => assembler.rebuild_routed_surfaces(),
        }
    }
}

fn run_case(case: PerfCase, mode: PerfMode) {
    let mut assembler = (case.build)();
    if matches!(mode, PerfMode::Routed) && !assembler.supports_routed_surfaces() {
        return;
    }
    let warmup = if matches!(mode, PerfMode::IncrementalDirty) {
        assembler.build_incremental()
    } else {
        assembler.build()
    };
    black_box(warmup.mesh.triangle_count());

    for _ in 0..WARMUP_ITERS {
        let output = mode.build(&mut assembler, 0);
        black_box(output.mesh.triangle_count());
    }

    let mut timings = Vec::with_capacity(MEASURE_ITERS);
    let mut triangles = 0;
    let mut fragments = 0;
    let mut warnings = 0;
    let mut candidate_pairs = 0;
    let mut rejected_pairs = 0;
    let mut reused_mesh = false;

    for iteration in 0..MEASURE_ITERS {
        let start = Instant::now();
        let output = mode.build(&mut assembler, iteration);
        let elapsed = start.elapsed();
        triangles = output.mesh.triangle_count();
        fragments = output.report.emitted_convex_fragments;
        warnings = output.report.warnings.len();
        candidate_pairs = output.report.candidate_pairs;
        rejected_pairs = output.report.rejected_pairs;
        reused_mesh = output.report.reused_mesh;
        black_box((
            triangles,
            fragments,
            warnings,
            candidate_pairs,
            rejected_pairs,
            reused_mesh,
        ));
        timings.push(elapsed);
    }

    timings.sort_unstable();
    let total = timings
        .iter()
        .fold(Duration::ZERO, |sum, value| sum + *value);
    let mean_ns = total.as_nanos() / timings.len() as u128;
    let min_ns = timings.first().map_or(0, Duration::as_nanos);
    let max_ns = timings.last().map_or(0, Duration::as_nanos);
    let p50_ns = percentile_ns(&timings, 50);
    let p95_ns = percentile_ns(&timings, 95);

    println!(
        "{{\"kernel\":\"vg_csg\",\"mode\":\"{}\",\"scenario\":\"{}\",\"brushes\":{},\"iterations\":{},\"warmup_iterations\":{},\"mean_ns\":{},\"min_ns\":{},\"p50_ns\":{},\"p95_ns\":{},\"max_ns\":{},\"triangles\":{},\"fragments\":{},\"warnings\":{},\"candidate_pairs\":{},\"rejected_pairs\":{},\"reused_mesh\":{}}}",
        mode.as_str(),
        case.name,
        case.brushes,
        MEASURE_ITERS,
        WARMUP_ITERS,
        mean_ns,
        min_ns,
        p50_ns,
        p95_ns,
        max_ns,
        triangles,
        fragments,
        warnings,
        candidate_pairs,
        rejected_pairs,
        reused_mesh
    );
}

fn nudge_tail_brush(assembler: &mut Assembler, iteration: usize) {
    let Some(brush) = assembler.brushes().last().cloned() else {
        return;
    };
    let phase = if iteration.is_multiple_of(2) {
        0.0
    } else {
        0.01
    };
    match brush.primitive {
        Primitive::Box { bounds } => {
            let center = bounds.center();
            let size = bounds.size() + Vec3::splat(phase);
            assembler.set_brush_primitive(
                brush.id,
                Primitive::Box {
                    bounds: Aabb::from_center_size(center, size),
                },
            );
        }
        Primitive::OrientedBox {
            center,
            size,
            rotation,
        } => {
            assembler.set_brush_primitive(
                brush.id,
                Primitive::OrientedBox {
                    center,
                    size: size + Vec3::splat(phase),
                    rotation,
                },
            );
        }
        _ => {}
    }
}

fn percentile_ns(values: &[Duration], percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() - 1) * percentile) / 100;
    values[index].as_nanos()
}

fn single_center_cut() -> Assembler {
    let mut asm = Assembler::new();
    asm.solid_box(
        "source",
        Aabb::from_center_size(Vec3::ZERO, Vec3::splat(4.0)),
        MaterialId(1),
    );
    asm.cut_box("void", Aabb::from_center_size(Vec3::ZERO, Vec3::splat(2.0)));
    asm
}

fn room_grid_8x8_doors() -> Assembler {
    let mut asm = Assembler::new();
    let wall = MaterialId(1);
    let floor = MaterialId(2);
    let cell = 6.0;

    for y in 0..8 {
        for x in 0..8 {
            let center = Vec3::new((x as f32 - 3.5) * cell, (y as f32 - 3.5) * cell, 0.0);
            asm.solid_box(
                format!("floor_{x}_{y}"),
                Aabb::from_center_size(
                    center + Vec3::new(0.0, 0.0, -0.1),
                    Vec3::new(5.6, 5.6, 0.2),
                ),
                floor,
            );
            asm.solid_box(
                format!("north_wall_{x}_{y}"),
                Aabb::from_center_size(
                    center + Vec3::new(0.0, 2.8, 1.5),
                    Vec3::new(5.6, 0.25, 3.0),
                ),
                wall,
            );
            asm.cut_box(
                format!("door_{x}_{y}"),
                Aabb::from_center_size(center + Vec3::new(0.0, 2.8, 1.0), Vec3::new(1.2, 0.5, 2.0)),
            );
        }
    }

    asm
}

fn rotated_cut_stack_64() -> Assembler {
    let mut asm = Assembler::new();
    asm.solid_box(
        "slab",
        Aabb::from_center_size(Vec3::ZERO, Vec3::new(32.0, 32.0, 4.0)),
        MaterialId(1),
    );

    for index in 0..64 {
        let angle = index as f32 * 0.173;
        let radius = 11.0 + (index % 7) as f32 * 0.35;
        let center = Vec3::new(angle.cos() * radius, angle.sin() * radius, 0.0);
        asm.cut_oriented_box(
            format!("rotated_void_{index}"),
            center,
            Vec3::new(1.0 + (index % 3) as f32 * 0.3, 8.0, 5.0),
            Quat::from_rotation_z(angle),
        );
    }

    asm
}

fn common_box_chain_64() -> Assembler {
    let mut asm = Assembler::new();
    asm.solid_box(
        "source",
        Aabb::from_center_size(Vec3::ZERO, Vec3::new(32.0, 16.0, 8.0)),
        MaterialId(1),
    );

    for index in 1..64 {
        let t = index as f32 / 63.0;
        let center = Vec3::new(
            (t - 0.5) * 6.0,
            (t * std::f32::consts::TAU).sin() * 1.5,
            0.0,
        );
        asm.add_brush(
            format!("common_{index}"),
            BrushOp::Intersect,
            Primitive::OrientedBox {
                center,
                size: Vec3::new(30.0 - t * 8.0, 14.0, 7.0),
                rotation: Quat::from_rotation_z(t * 0.2),
            },
            MaterialId(1),
        );
    }

    asm
}

fn distant_cutters_512() -> Assembler {
    let mut asm = Assembler::new();
    asm.solid_box(
        "source",
        Aabb::from_center_size(Vec3::ZERO, Vec3::splat(8.0)),
        MaterialId(1),
    );

    for index in 0..512 {
        let row = index / 32;
        let col = index % 32;
        asm.cut_box(
            format!("far_void_{index}"),
            Aabb::from_center_size(
                Vec3::new(1000.0 + col as f32 * 4.0, 1000.0 + row as f32 * 4.0, 0.0),
                Vec3::splat(1.0),
            ),
        );
    }

    asm
}

fn distant_oriented_cutters_128() -> Assembler {
    let mut asm = Assembler::new();
    asm.solid_box(
        "source",
        Aabb::from_center_size(Vec3::ZERO, Vec3::splat(8.0)),
        MaterialId(1),
    );

    for index in 0..128 {
        let row = index / 16;
        let col = index % 16;
        asm.cut_oriented_box(
            format!("far_oriented_void_{index}"),
            Vec3::new(1000.0 + col as f32 * 4.0, 1000.0 + row as f32 * 4.0, 0.0),
            Vec3::splat(1.0),
            Quat::from_rotation_z(index as f32 * 0.037),
        );
    }

    asm
}
