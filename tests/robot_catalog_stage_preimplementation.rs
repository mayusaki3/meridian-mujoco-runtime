//! Pre-integration tests for project-scoped Robot Catalog staging.
#[path = "../src/catalog_stage.rs"]
mod catalog_stage;
#[path = "../src/robot_catalog.rs"]
mod robot_catalog;

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

#[test]
fn mer_reg_persist_tc_013_save_callback_writes_snapshot_but_does_not_clear_dirty() {
    let mut stage = CatalogStage::default();
    let root = Path::new("project-a");
    stage.activate(root, RobotCatalog::default());
    let mut candidate = stage.snapshot(root).unwrap().clone();
    let id = candidate.create("Robot A").unwrap();
    stage.replace(root, candidate).unwrap();
    let mut persisted = None;
    assert!(stage.persist_for_save(root, |catalog| {
        persisted = Some(catalog.clone());
        Ok(())
    }).unwrap());
    assert_eq!(persisted.unwrap().robots()[0].robot_id, id);
    assert!(stage.is_dirty(root), "Framework save completion must clear dirty, not app write");
}

#[test]
fn mer_reg_persist_tc_014_failed_write_preserves_staged_identity() {
    let mut stage = CatalogStage::default();
    let root = Path::new("project-a");
    stage.activate(root, RobotCatalog::default());
    let mut candidate = stage.snapshot(root).unwrap().clone();
    let id = candidate.create("Robot A").unwrap();
    stage.replace(root, candidate).unwrap();
    let error = stage.persist_for_save(root, |_| Err(std::io::Error::other("disk full")));
    assert!(error.is_err());
    assert!(stage.is_dirty(root));
    assert_eq!(stage.snapshot(root).unwrap().robots()[0].robot_id, id);
}

#[test]
fn mer_reg_persist_tc_015_save_as_preserves_ids_without_mutating_source_stage() {
    let mut stage = CatalogStage::default();
    let source = Path::new("project-a");
    stage.activate(source, RobotCatalog::default());
    let mut candidate = stage.snapshot(source).unwrap().clone();
    let id = candidate.create("Robot A").unwrap();
    stage.replace(source, candidate).unwrap();
    let mut destination = None;
    assert!(stage.persist_for_save(source, |catalog| {
        destination = Some(catalog.clone());
        Ok(())
    }).unwrap());
    assert_eq!(destination.unwrap().robots()[0].robot_id, id);
    assert_eq!(stage.snapshot(source).unwrap().robots()[0].robot_id, id);
}

#[test]
fn mer_reg_persist_tc_016_no_active_stage_does_not_invoke_save_callback() {
    let stage = CatalogStage::default();
    let mut called = false;
    assert!(!stage.persist_for_save(Path::new("project-a"), |_| {
        called = true;
        Ok(())
    }).unwrap());
    assert!(!called);
}
