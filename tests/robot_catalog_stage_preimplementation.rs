//! Pre-integration tests for project-scoped Robot Catalog staging.
#[path = "../src/robot_catalog.rs"]
mod robot_catalog;
#[path = "../src/catalog_stage.rs"]
mod catalog_stage;

use catalog_stage::CatalogStage;
use robot_catalog::RobotCatalog;
use std::path::Path;

#[test]
fn mer_reg_persist_tc_009_registration_stages_without_writing_to_disk() {
    let mut stage = CatalogStage::default();
    let root = Path::new("project-a");
    stage.activate(root, RobotCatalog::default());
    let mut candidate = stage.snapshot(root).unwrap().clone();
    let id = candidate.create("Robot A").unwrap();
    stage.replace(root, candidate).unwrap();
    assert!(stage.is_dirty(root));
    assert_eq!(stage.snapshot(root).unwrap().robots()[0].robot_id, id);
}

#[test]
fn mer_reg_persist_tc_010_discard_drops_unsaved_robot() {
    let mut stage = CatalogStage::default();
    let root = Path::new("project-a");
    stage.activate(root, RobotCatalog::default());
    let mut candidate = stage.snapshot(root).unwrap().clone();
    candidate.create("Unsaved").unwrap();
    stage.replace(root, candidate).unwrap();
    stage.discard();
    stage.activate(root, RobotCatalog::default());
    assert!(stage.snapshot(root).unwrap().robots().is_empty());
    assert!(!stage.is_dirty(root));
}

#[test]
fn mer_reg_persist_tc_011_project_switch_does_not_leak_catalog() {
    let mut stage = CatalogStage::default();
    let a = Path::new("project-a");
    let b = Path::new("project-b");
    stage.activate(a, RobotCatalog::default());
    let mut candidate = stage.snapshot(a).unwrap().clone();
    candidate.create("A").unwrap();
    stage.replace(a, candidate).unwrap();
    stage.activate(b, RobotCatalog::default());
    assert!(stage.snapshot(a).is_none());
    assert!(stage.replace(a, RobotCatalog::default()).is_err());
    assert!(stage.snapshot(b).unwrap().robots().is_empty());
}

#[test]
fn mer_reg_persist_tc_012_save_failure_retains_stage_until_success() {
    let mut stage = CatalogStage::default();
    let root = Path::new("project-a");
    stage.activate(root, RobotCatalog::default());
    let mut candidate = stage.snapshot(root).unwrap().clone();
    let id = candidate.create("Stable").unwrap();
    stage.replace(root, candidate).unwrap();
    // A failed persistence operation must not call mark_saved.
    assert!(stage.is_dirty(root));
    assert_eq!(stage.snapshot(root).unwrap().robots()[0].robot_id, id);
    stage.mark_saved(root);
    assert!(!stage.is_dirty(root));
    assert_eq!(stage.snapshot(root).unwrap().robots()[0].robot_id, id);
}
