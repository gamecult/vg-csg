use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use bevy_math::Vec3;
use vg_csg::{
    Aabb, CultGeometryBuildRequest, CultGeometryChunkArtifact, CultGeometryDomainDocument,
    CultGeometrySelectedCutManifest, DomainQuery, GEOMETRY_BUILD_REQUEST_SCHEMA,
    GEOMETRY_CHUNK_ARTIFACT_SCHEMA, GEOMETRY_DOMAIN_SCHEMA, GEOMETRY_SELECTED_CUT_SCHEMA,
    build_domain_chunks, ragnarok_column_spec,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/ragnarok-cultcache"));
    fs::create_dir_all(&output_dir)?;

    let spec = ragnarok_column_spec();
    let root = spec.compile_root();
    let query = high_lod_query();
    let domain =
        CultGeometryDomainDocument::from_spec(&spec, "vg-csg", "2026-05-29T00:00:00.0000000Z");
    let domain_key = domain.record_key();
    let request = CultGeometryBuildRequest::from_query(
        "ragnarok-column-high",
        domain_key.clone(),
        "ragnarok-column-workers",
        &query,
        "2026-05-29T00:00:01.0000000Z",
    );
    let request_key = request.record_key();
    let build = build_domain_chunks(&root, &query);
    let cut = CultGeometrySelectedCutManifest::from_cut(&build.cut, request_key.clone());
    let cut_key = cut.record_key();

    write_document(
        &output_dir,
        GEOMETRY_DOMAIN_SCHEMA,
        &domain_key,
        &domain.to_msgpack()?,
    )?;
    write_document(
        &output_dir,
        GEOMETRY_BUILD_REQUEST_SCHEMA,
        &request_key,
        &request.to_msgpack()?,
    )?;
    write_document(
        &output_dir,
        GEOMETRY_SELECTED_CUT_SCHEMA,
        &cut_key,
        &cut.to_msgpack()?,
    )?;

    for chunk in &build.chunks {
        let artifact = CultGeometryChunkArtifact::from_chunk(chunk, cut_key.clone());
        write_document(
            &output_dir,
            GEOMETRY_CHUNK_ARTIFACT_SCHEMA,
            &artifact.record_key(),
            &artifact.to_msgpack()?,
        )?;
    }

    println!(
        "wrote {} CultCache geometry documents to {}",
        3 + build.chunks.len(),
        output_dir.display()
    );
    Ok(())
}

fn high_lod_query() -> DomainQuery {
    DomainQuery {
        camera_position: Vec3::new(36.0, -42.0, 30.0),
        frustum: Aabb::new(Vec3::new(-75.0, -75.0, -30.0), Vec3::new(75.0, 75.0, 120.0)),
        viewport_height_px: 1080.0,
        vertical_fov_radians: std::f32::consts::FRAC_PI_3,
        target_error: 0.01,
        triangle_budget: 10_000,
        collider_budget: 10_000,
        semantic_filter: Vec::new(),
        requested_chunk_keys: Vec::new(),
        dirty_domain_keys: Vec::new(),
    }
}

fn write_document(output_dir: &Path, schema: &str, key: &str, payload: &[u8]) -> io::Result<()> {
    let path = output_dir.join(format!("{}__{}.msgpack", schema, file_key(key)));
    fs::write(&path, payload)?;
    println!("{}\t{}\t{}", schema, key, path.display());
    Ok(())
}

fn file_key(key: &str) -> String {
    key.chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || value == '-' || value == '_' {
                value
            } else {
                '_'
            }
        })
        .collect()
}
