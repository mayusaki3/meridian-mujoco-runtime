//! Regression tests for checked-in URDF fixtures and project roundtrips.
#![cfg(test)]
use crate::{runtime_project::RuntimeProjectAdapter, urdf_io};
use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
use workflow_ide_framework::{project::ProjectContext, project_adapter::ApplicationProjectAdapter};

const VALID: &[(&str, &str)] = &[
    ("01_minimal", include_str!("../tests/fixtures/urdf-regression/valid/01_minimal.urdf")),
    ("02_two_link_revolute", include_str!("../tests/fixtures/urdf-regression/valid/02_two_link_revolute.urdf")),
    ("03_prismatic_fixed_chain", include_str!("../tests/fixtures/urdf-regression/valid/03_prismatic_fixed_chain.urdf")),
    ("04_multi_joint_mimic", include_str!("../tests/fixtures/urdf-regression/valid/04_multi_joint_mimic.urdf")),
    ("05_vendor_extensions", include_str!("../tests/fixtures/urdf-regression/valid/05_vendor_extensions.urdf")),
    ("06_mixed_order_comments", include_str!("../tests/fixtures/urdf-regression/valid/06_mixed_order_comments.urdf")),
    ("07_materials_and_mesh_reference", include_str!("../tests/fixtures/urdf-regression/valid/07_materials_and_mesh_reference.urdf")),
    ("08_preserved_parent_with_namespaces", include_str!("../tests/fixtures/urdf-regression/valid/08_preserved_parent_with_namespaces.urdf")),
];
const INVALID: &[(&str, &str)] = &[
    ("01_malformed_xml", include_str!("../tests/fixtures/urdf-regression/invalid/01_malformed_xml.urdf")),
    ("02_missing_robot_name", include_str!("../tests/fixtures/urdf-regression/invalid/02_missing_robot_name.urdf")),
    ("03_wrong_root", include_str!("../tests/fixtures/urdf-regression/invalid/03_wrong_root.urdf")),
    ("04_missing_link_name", include_str!("../tests/fixtures/urdf-regression/invalid/04_missing_link_name.urdf")),
    ("05_missing_joint_name", include_str!("../tests/fixtures/urdf-regression/invalid/05_missing_joint_name.urdf")),
];

struct TempProject(ProjectContext);
impl TempProject {
    fn new() -> Self {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        Self(ProjectContext::new(std::env::temp_dir().join(format!("meridian-fixtures-{}-{nanos}", std::process::id()))))
    }
}
impl Drop for TempProject {
    fn drop(&mut self) { let _ = fs::remove_dir_all(self.0.root()); }
}

#[test]
fn all_valid_fixtures_roundtrip_exactly_through_project_and_export() {
    for (name, xml) in VALID {
        let project = TempProject::new();
        RuntimeProjectAdapter::import_source(&project.0, xml).unwrap_or_else(|e| panic!("{name}: {e}"));
        let saved = fs::read(project.0.application_directory().join("robot-source.urdf")).unwrap();
        assert_eq!(saved, xml.as_bytes(), "{name}: project bytes differ");
        let mut adapter = RuntimeProjectAdapter;
        adapter.save_project_data(&project.0, "fixture-test").unwrap();
        let reloaded = RuntimeProjectAdapter::load_source(&project.0).unwrap().unwrap();
        let export_path = project.0.root().join("export.urdf");
        urdf_io::export_unchanged_urdf(&reloaded, &export_path).unwrap();
        assert_eq!(fs::read(export_path).unwrap(), xml.as_bytes(), "{name}: export bytes differ");
    }
}

#[test]
fn invalid_fixtures_are_rejected_without_replacing_previous_source() {
    for (name, invalid) in INVALID {
        let project = TempProject::new();
        let original = VALID[0].1;
        RuntimeProjectAdapter::import_source(&project.0, original).unwrap();
        assert!(RuntimeProjectAdapter::import_source(&project.0, invalid).is_err(), "{name}: unexpectedly accepted");
        let saved = fs::read(project.0.application_directory().join("robot-source.urdf")).unwrap();
        assert_eq!(saved, original.as_bytes(), "{name}: previous source was modified");
    }
}

#[test]
fn unresolved_joint_reference_documents_current_semantic_validation_gap() {
    let xml = include_str!("../tests/fixtures/urdf-regression/invalid/06_structurally_invalid_urdf.urdf");
    // Current inspection validates XML syntax and required names, not joint references.
    assert!(urdf_io::ImportedUrdf::parse(xml).is_ok());
}
