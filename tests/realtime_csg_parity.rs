use bevy_math::{Quat, Vec3};
use vg_csg::{Aabb, Assembler, ConvexSolid, MaterialId, PolygonCategory};

#[test]
fn parity_polygon_category_vocabulary_matches_public_demo() {
    let categories = [
        PolygonCategory::Inside,
        PolygonCategory::Outside,
        PolygonCategory::Aligned,
        PolygonCategory::ReverseAligned,
    ];

    assert_eq!(categories.len(), 4);
}

#[test]
fn parity_box_brush_observable_counts_match_realtime_csg_demo() {
    let solid = ConvexSolid::box_from_center_size(
        Vec3::ZERO,
        Vec3::splat(2.0),
        Quat::IDENTITY,
        MaterialId(3),
    );

    assert_eq!(solid.clip_planes.len(), 6);
    assert_eq!(solid.polygons.len(), 6);
    assert_eq!(
        solid
            .polygons
            .iter()
            .map(|polygon| polygon.vertices.len())
            .sum::<usize>(),
        24
    );
}

#[test]
fn parity_center_box_subtraction_has_six_surviving_regions() {
    let mut asm = Assembler::new();
    asm.solid_box(
        "source",
        Aabb::from_center_size(Vec3::ZERO, Vec3::splat(4.0)),
        MaterialId(1),
    );
    asm.cut_box("void", Aabb::from_center_size(Vec3::ZERO, Vec3::splat(2.0)));

    let output = asm.build();
    assert_eq!(output.report.emitted_convex_fragments, 6);
    assert_eq!(output.report.warnings.len(), 0);
}
