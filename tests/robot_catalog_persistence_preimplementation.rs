//! MER-RESOURCE-001 persistence contract tests.
//! The adapter methods are intentionally introduced after these tests.
#[path = "../src/catalog_stage.rs"]
mod catalog_stage;
#[path = "../src/mjcf_project.rs"]
mod mjcf_project;
#[path = "../src/robot_catalog.rs"]
mod robot_catalog;
#[path = "../src/runtime_project.rs"]
mod runtime_project;
#[path = "../src/urdf_io.rs"]
mod urdf_io;

use robot_catalog::RobotCatalog;
use runtime_project::RuntimeProjectAdapter;
use std::fs;
use workflow_ide_framework::{
    project::ProjectContext, project_adapter::ApplicationProjectAdapter,
    project_resource::ProjectDataConsistency,
};

fn context() -> ProjectContext {
    ProjectContext::new(std::env::temp_dir().join(format!(
        "meridian-robot-catalog-persistence-{}",
        uuid::Uuid::new_v4()
    )))
}

#[test]
fn mer_res_tc_002_catalog_survives_save_and_open_with_rename() {
    let project = context();
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("Before").unwrap();
    catalog.rename(&id, "After").unwrap();
    RuntimeProjectAdapter::save_catalog(&project, &catalog).unwrap();
    let mut adapter = RuntimeProjectAdapter;
    adapter.save_project_data(&project, "save").unwrap();
    let loaded = RuntimeProjectAdapter::load_catalog(&project).unwrap();
    assert_eq!(loaded.robots()[0].robot_id, id);
    assert_eq!(loaded.robots()[0].name, "After");
    fs::remove_dir_all(project.root()).unwrap();
}

#[test]
fn mer_res_tc_010_legacy_project_without_catalog_stays_compatible() {
    let project = context();
    let mut adapter = RuntimeProjectAdapter;
    adapter.save_project_data(&project, "legacy").unwrap();
    assert!(
        RuntimeProjectAdapter::load_catalog(&project)
            .unwrap()
            .robots()
            .is_empty()
    );
    assert!(
        !project
            .application_directory()
            .join("robot-catalog.json")
            .exists()
    );
    fs::remove_dir_all(project.root()).unwrap();
}

#[test]
fn mer_res_tc_011_legacy_urdf_does_not_infer_robot_identity() {
    let project = context();
    RuntimeProjectAdapter::import_source(
        &project,
        r#"<robot name="legacy"><link name="base"/></robot>"#,
    )
    .unwrap();
    assert!(
        RuntimeProjectAdapter::load_catalog(&project)
            .unwrap()
            .robots()
            .is_empty()
    );
    fs::remove_dir_all(project.root()).unwrap();
}

#[test]
fn mer_res_tc_012_invalid_catalog_does_not_replace_previous_data() {
    let project = context();
    let mut catalog = RobotCatalog::default();
    catalog.create("Valid").unwrap();
    RuntimeProjectAdapter::save_catalog(&project, &catalog).unwrap();
    let file = project.application_directory().join("robot-catalog.json");
    let previous = fs::read(&file).unwrap();
    let mut corrupted: serde_json::Value = serde_json::from_slice(&previous).unwrap();
    let duplicate = corrupted["robots"][0].clone();
    corrupted["robots"].as_array_mut().unwrap().push(duplicate);
    let invalid = RobotCatalog::from_json(&corrupted.to_string());
    assert!(invalid.is_err());
    assert_eq!(fs::read(&file).unwrap(), previous);
    fs::remove_dir_all(project.root()).unwrap();
}

#[test]
fn mer_res_tc_014_save_as_copies_catalog_without_changing_ids() {
    let source = context();
    let destination = context();
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("Original").unwrap();
    RuntimeProjectAdapter::save_catalog(&source, &catalog).unwrap();
    let mut adapter = RuntimeProjectAdapter;
    adapter
        .save_project_data_as(Some(&source), &destination, "save-as")
        .unwrap();
    assert_eq!(
        RuntimeProjectAdapter::load_catalog(&destination)
            .unwrap()
            .robots()[0]
            .robot_id,
        id
    );
    assert_eq!(
        RuntimeProjectAdapter::load_catalog(&source).unwrap(),
        catalog
    );
    fs::remove_dir_all(source.root()).unwrap();
    fs::remove_dir_all(destination.root()).unwrap();
}

#[test]
fn mer_res_tc_015_corrupt_catalog_is_reported_as_inconsistent() {
    let project = context();
    fs::create_dir_all(project.application_directory()).unwrap();
    fs::write(
        project.application_directory().join("robot-catalog.json"),
        "{invalid",
    )
    .unwrap();
    let mut adapter = RuntimeProjectAdapter;
    assert!(matches!(
        adapter.check_project_consistency(&project).unwrap(),
        ProjectDataConsistency::Inconsistent {
            can_open: false,
            ..
        }
    ));
    fs::remove_dir_all(project.root()).unwrap();
}

#[test]
fn mer_reg_persist_tc_017_staged_catalog_is_written_only_by_save_hook() {
    let project = context();
    let stage = catalog_stage::SharedCatalogStage::default();
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("Staged").unwrap();
    {
        let mut guard = stage.lock().unwrap();
        guard.activate(project.root(), RobotCatalog::default());
        guard.replace(project.root(), catalog).unwrap();
    }
    let file = project.application_directory().join("robot-catalog.json");
    assert!(!file.exists());
    let mut adapter = runtime_project::StagedRuntimeProjectAdapter { stage: stage.clone() };
    adapter.save_project_data(&project, "save").unwrap();
    assert_eq!(RuntimeProjectAdapter::load_catalog(&project).unwrap().robots()[0].robot_id, id);
    assert!(stage.lock().unwrap().is_dirty(project.root()));
    stage.lock().unwrap().mark_saved(project.root());
    assert!(!stage.lock().unwrap().is_dirty(project.root()));
    fs::remove_dir_all(project.root()).unwrap();
}

#[test]
fn mer_reg_persist_tc_018_save_as_uses_staged_catalog_and_preserves_robot_id() {
    let source = context();
    let destination = context();
    let stage = catalog_stage::SharedCatalogStage::default();
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("Staged").unwrap();
    {
        let mut guard = stage.lock().unwrap();
        guard.activate(source.root(), RobotCatalog::default());
        guard.replace(source.root(), catalog).unwrap();
    }
    let mut adapter = runtime_project::StagedRuntimeProjectAdapter { stage: stage.clone() };
    adapter.save_project_data_as(Some(&source), &destination, "save-as").unwrap();
    assert_eq!(RuntimeProjectAdapter::load_catalog(&destination).unwrap().robots()[0].robot_id, id);
    assert!(!source.application_directory().join("robot-catalog.json").exists());
    assert!(stage.lock().unwrap().is_dirty(source.root()));
    fs::remove_dir_all(source.root()).unwrap();
    fs::remove_dir_all(destination.root()).unwrap();
}
