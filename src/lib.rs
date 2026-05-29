//! Lean constructive geometry tools for VibeGeometry.
//!
//! This crate borrows RealtimeCSG's useful public shape: ordered brushes,
//! operation types, dirtied generations, prefix checkpoints, and rebuildable
//! output meshes. The hidden native RealtimeCSG kernel is not public, so this
//! starts with the smallest honest CSG organ: exact box subtraction against
//! additive boxes, plus additive procedural primitives for the habitat forms we
//! keep needing.

mod assembler;
mod brush;
mod convex;
mod cult_geometry;
mod domain;
mod dsl;
mod frontier;
mod mesh;
mod primitives;
mod tree;

pub use assembler::{Assembler, BuildOutput, BuildReport, BuildWarning};
pub use brush::{Aabb, Brush, BrushId, BrushOp, MaterialId, PolygonCategory, Primitive};
pub use convex::{CategorizedPolygons, ConvexPolygon, ConvexSolid, Plane, PolygonRouteScratch};
pub use cult_geometry::{
    CultGeometryBuildRequest, CultGeometryChunkArtifact, CultGeometryContributionRow,
    CultGeometryDomainDocument, CultGeometryDomainNode, CultGeometryFeatureClaim,
    CultGeometrySelectedCutManifest, CultGeometryTriangleMesh, GEOMETRY_BUILD_REQUEST_SCHEMA,
    GEOMETRY_CHUNK_ARTIFACT_SCHEMA, GEOMETRY_DOMAIN_SCHEMA, GEOMETRY_SELECTED_CUT_SCHEMA,
};
pub use domain::{
    ClaimLoweringTarget, ContributionManifest, ContributionRow, CsgClaimLowering, DomainChunkBuild,
    DomainFrame, DomainKey, DomainKind, DomainNode, DomainNodeDocument, DomainNodeSpec,
    DomainQuery, DomainSpecDocument, DomainSummary, FeatureClaim, FeatureClaimDocument,
    FeatureClaimKind, FeatureClaimSpec, FeatureLoweringPolicy, FieldEncoding, FieldLayer,
    SelectedCut, SelectedCutManifest, TriangleChunk, TriangleChunkManifest, build_domain_chunks,
    lower_feature_claims_to_csg_tree, lower_selected_cut, lower_selected_cut_chunks,
    ragnarok_column_fixture, ragnarok_column_spec, select_domain_cut,
};
pub use dsl::LevelDsl;
pub use frontier::{DemandFrontier, DemandPair, DirtyDemandFrontier};
pub use mesh::TriangleMesh;
pub use primitives::{
    DomeCapZSpec, FloretArmSpec, append_cylinder_z, append_dome_cap_z, append_floret_arm,
};
pub use tree::{
    CsgBranchOp, CsgNode, CsgNodeId, CsgOperationType, CsgTree, CsgTreeArena, CsgTreeBranch,
    CsgTreeBrush,
};
