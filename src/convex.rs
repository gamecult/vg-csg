use bevy_math::{Quat, Vec2, Vec3};

use crate::{Aabb, MaterialId, TriangleMesh};

const EPSILON: f32 = 1.0e-5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Self {
        let normal = normal.normalize_or_zero();
        Self {
            normal,
            distance: -normal.dot(point),
        }
    }

    pub fn signed_distance(self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

#[derive(Clone, Debug)]
pub struct ConvexPolygon {
    pub vertices: Vec<Vec3>,
    pub normal: Vec3,
    pub material: MaterialId,
}

#[derive(Clone, Debug)]
pub struct ConvexSolid {
    pub polygons: Vec<ConvexPolygon>,
    pub clip_planes: Vec<Plane>,
    pub material: MaterialId,
}

impl ConvexSolid {
    pub fn from_aabb(bounds: Aabb, material: MaterialId) -> Self {
        Self::box_from_center_size(bounds.center(), bounds.size(), Quat::IDENTITY, material)
    }

    pub fn box_from_center_size(
        center: Vec3,
        size: Vec3,
        rotation: Quat,
        material: MaterialId,
    ) -> Self {
        let half = size * 0.5;
        let local_corners = [
            Vec3::new(-half.x, -half.y, -half.z),
            Vec3::new(half.x, -half.y, -half.z),
            Vec3::new(half.x, half.y, -half.z),
            Vec3::new(-half.x, half.y, -half.z),
            Vec3::new(-half.x, -half.y, half.z),
            Vec3::new(half.x, -half.y, half.z),
            Vec3::new(half.x, half.y, half.z),
            Vec3::new(-half.x, half.y, half.z),
        ];
        let corners = local_corners.map(|point| center + rotation * point);
        let faces = [
            ([0, 3, 2, 1], Vec3::NEG_Z),
            ([4, 5, 6, 7], Vec3::Z),
            ([0, 1, 5, 4], Vec3::NEG_Y),
            ([3, 7, 6, 2], Vec3::Y),
            ([0, 4, 7, 3], Vec3::NEG_X),
            ([1, 2, 6, 5], Vec3::X),
        ];

        let mut polygons = Vec::with_capacity(6);
        let mut clip_planes = Vec::with_capacity(6);
        for (indices, local_normal) in faces {
            let normal = rotation * local_normal;
            let vertices = indices.map(|index| corners[index]).to_vec();
            clip_planes.push(Plane::from_point_normal(vertices[0], normal));
            polygons.push(ConvexPolygon {
                vertices,
                normal,
                material,
            });
        }

        Self {
            polygons,
            clip_planes,
            material,
        }
    }

    pub fn subtract_convex(&self, cutter: &Self) -> Vec<Self> {
        let mut fragments = Vec::new();
        let mut remainder = Some(self.clone());

        for plane in &cutter.clip_planes {
            let Some(current) = remainder.take() else {
                break;
            };

            match current.split(*plane) {
                SplitResult::Front(front) => {
                    fragments.push(front);
                    break;
                }
                SplitResult::Back(back) => {
                    remainder = Some(back);
                }
                SplitResult::Both { front, back } => {
                    fragments.push(front);
                    remainder = Some(back);
                }
                SplitResult::Coplanar(solid) => {
                    remainder = Some(solid);
                }
            }
        }

        fragments
    }

    pub fn append_to_mesh(&self, mesh: &mut TriangleMesh) {
        for polygon in &self.polygons {
            if polygon.vertices.len() < 3 {
                continue;
            }
            let base = polygon.vertices[0];
            for i in 1..polygon.vertices.len() - 1 {
                mesh.append_triangle(
                    [base, polygon.vertices[i], polygon.vertices[i + 1]],
                    polygon.normal,
                    [Vec2::ZERO, Vec2::X, Vec2::Y],
                    polygon.material,
                );
            }
        }
    }

    fn split(&self, plane: Plane) -> SplitResult {
        let mut saw_front = false;
        let mut saw_back = false;
        for polygon in &self.polygons {
            for vertex in &polygon.vertices {
                let distance = plane.signed_distance(*vertex);
                saw_front |= distance > EPSILON;
                saw_back |= distance < -EPSILON;
            }
        }

        match (saw_front, saw_back) {
            (true, false) => return SplitResult::Front(self.clone()),
            (false, true) => return SplitResult::Back(self.clone()),
            (false, false) => return SplitResult::Coplanar(self.clone()),
            (true, true) => {}
        }

        let mut front_polygons = Vec::new();
        let mut back_polygons = Vec::new();
        let mut cap_points = Vec::new();

        for polygon in &self.polygons {
            let front = clip_polygon(&polygon.vertices, plane, KeepSide::Front, &mut cap_points);
            if front.len() >= 3 {
                front_polygons.push(ConvexPolygon {
                    normal: polygon.normal,
                    vertices: front,
                    material: polygon.material,
                });
            }

            let back = clip_polygon(&polygon.vertices, plane, KeepSide::Back, &mut cap_points);
            if back.len() >= 3 {
                back_polygons.push(ConvexPolygon {
                    normal: polygon.normal,
                    vertices: back,
                    material: polygon.material,
                });
            }
        }

        let cap_points = unique_points_on_plane(cap_points);
        if cap_points.len() >= 3 {
            front_polygons.push(make_cap_polygon(&cap_points, -plane.normal, self.material));
            back_polygons.push(make_cap_polygon(&cap_points, plane.normal, self.material));
        }

        SplitResult::Both {
            front: Self {
                clip_planes: planes_from_polygons(&front_polygons),
                polygons: front_polygons,
                material: self.material,
            },
            back: Self {
                clip_planes: planes_from_polygons(&back_polygons),
                polygons: back_polygons,
                material: self.material,
            },
        }
    }
}

enum SplitResult {
    Front(ConvexSolid),
    Back(ConvexSolid),
    Both {
        front: ConvexSolid,
        back: ConvexSolid,
    },
    Coplanar(ConvexSolid),
}

#[derive(Clone, Copy)]
enum KeepSide {
    Front,
    Back,
}

fn clip_polygon(
    vertices: &[Vec3],
    plane: Plane,
    keep: KeepSide,
    cap_points: &mut Vec<Vec3>,
) -> Vec<Vec3> {
    let mut output = Vec::new();
    if vertices.is_empty() {
        return output;
    }

    let mut previous = *vertices.last().expect("checked non-empty");
    let mut previous_distance = plane.signed_distance(previous);
    let mut previous_inside = inside(previous_distance, keep);

    for &current in vertices {
        let current_distance = plane.signed_distance(current);
        let current_inside = inside(current_distance, keep);

        if current_inside != previous_inside {
            let t = previous_distance / (previous_distance - current_distance);
            let intersection = previous + (current - previous) * t;
            output.push(intersection);
            cap_points.push(intersection);
        }

        if current_inside {
            output.push(current);
        }

        previous = current;
        previous_distance = current_distance;
        previous_inside = current_inside;
    }

    output
}

fn inside(distance: f32, keep: KeepSide) -> bool {
    match keep {
        KeepSide::Front => distance >= -EPSILON,
        KeepSide::Back => distance <= EPSILON,
    }
}

fn make_cap_polygon(points: &[Vec3], normal: Vec3, material: MaterialId) -> ConvexPolygon {
    let center = points.iter().copied().sum::<Vec3>() / points.len() as f32;
    let axis = if normal.z.abs() < 0.9 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let tangent = normal.cross(axis).normalize_or_zero();
    let bitangent = normal.cross(tangent).normalize_or_zero();
    let mut vertices = points.to_vec();

    vertices.sort_by(|a, b| {
        let da = *a - center;
        let db = *b - center;
        let aa = da.dot(bitangent).atan2(da.dot(tangent));
        let ab = db.dot(bitangent).atan2(db.dot(tangent));
        aa.total_cmp(&ab)
    });

    if polygon_normal(&vertices).dot(normal) < 0.0 {
        vertices.reverse();
    }

    ConvexPolygon {
        vertices,
        normal,
        material,
    }
}

fn unique_points_on_plane(points: Vec<Vec3>) -> Vec<Vec3> {
    let mut unique = Vec::new();
    'outer: for point in points {
        for seen in &unique {
            let delta: Vec3 = point - *seen;
            if delta.length_squared() < EPSILON * EPSILON {
                continue 'outer;
            }
        }
        unique.push(point);
    }
    unique
}

fn planes_from_polygons(polygons: &[ConvexPolygon]) -> Vec<Plane> {
    polygons
        .iter()
        .filter_map(|polygon| {
            polygon
                .vertices
                .first()
                .map(|point| Plane::from_point_normal(*point, polygon.normal))
        })
        .collect()
}

fn polygon_normal(vertices: &[Vec3]) -> Vec3 {
    if vertices.len() < 3 {
        return Vec3::ZERO;
    }
    (vertices[1] - vertices[0])
        .cross(vertices[2] - vertices[0])
        .normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_box_parity(solid: &ConvexSolid, material: MaterialId) {
        assert_eq!(solid.polygons.len(), 6);
        assert_eq!(solid.clip_planes.len(), 6);
        assert!(
            solid
                .polygons
                .iter()
                .all(|polygon| polygon.vertices.len() == 4)
        );
        assert!(
            solid
                .polygons
                .iter()
                .all(|polygon| polygon.material == material)
        );
    }

    #[test]
    fn parity_box_brush_from_planes_has_six_quad_polygons() {
        let solid = ConvexSolid::box_from_center_size(
            Vec3::ZERO,
            Vec3::splat(2.0),
            Quat::IDENTITY,
            MaterialId(9),
        );

        assert_box_parity(&solid, MaterialId(9));
    }

    #[test]
    fn subtract_center_box_emits_six_convex_fragments() {
        let source = ConvexSolid::box_from_center_size(
            Vec3::ZERO,
            Vec3::splat(4.0),
            Quat::IDENTITY,
            MaterialId(1),
        );
        let cutter = ConvexSolid::box_from_center_size(
            Vec3::ZERO,
            Vec3::splat(2.0),
            Quat::IDENTITY,
            MaterialId(0),
        );

        let fragments = source.subtract_convex(&cutter);
        assert_eq!(fragments.len(), 6);
        assert!(
            fragments
                .iter()
                .all(|fragment| !fragment.polygons.is_empty())
        );
    }

    #[test]
    fn subtract_rotated_box_creates_non_axis_aligned_faces() {
        let source = ConvexSolid::box_from_center_size(
            Vec3::ZERO,
            Vec3::splat(4.0),
            Quat::IDENTITY,
            MaterialId(1),
        );
        let cutter = ConvexSolid::box_from_center_size(
            Vec3::ZERO,
            Vec3::new(2.0, 5.0, 2.0),
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
            MaterialId(0),
        );

        let fragments = source.subtract_convex(&cutter);
        assert!(fragments.len() > 2);
        assert!(fragments.iter().any(|fragment| {
            fragment
                .polygons
                .iter()
                .any(|polygon| polygon.normal.x.abs() > 0.01 && polygon.normal.y.abs() > 0.01)
        }));
    }
}
