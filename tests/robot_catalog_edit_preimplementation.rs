//! MER-CAT-EDIT-001 stage edit contract tests.
#[path = "../src/robot_catalog.rs"]
mod robot_catalog;
#[path = "../src/catalog_stage.rs"]
mod catalog_stage;
#[path = "../src/catalog_edit.rs"]
mod catalog_edit;

use catalog_edit::{apply, Edit, EditResult};
use catalog_stage::{CatalogStage, SharedCatalogStage};
use robot_catalog::{RobotCatalog, RobotRepresentation};
use std::{path::Path, sync::{Arc, Mutex}};

fn fixture() -> (SharedCatalogStage, String, String) {
    let stage = Arc::new(Mutex::new(CatalogStage::default()));
    let root = Path::new("project-a");
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("Original").unwrap();
    catalog.add_representation(&id, RobotRepresentation {
        registry_resource_id: "shared-resource".into(),
        format: "URDF".into(),
        role: "source".into(),
    }).unwrap();
    stage.lock().unwrap().activate(root, catalog);
    (stage, id, "shared-resource".into())
}

#[test]
fn mer_cat_edit_tc_007_rename_updates_stage_and_preserves_panel_node_identity() {
    let (stage, id, registry_id) = fixture();
    let root = Path::new("project-a");
    let mut refreshes = 0;
    assert_eq!(apply(&stage, root, Edit::Rename {
        robot_id: id.clone(), name: "Renamed".into(),
    }, || refreshes += 1).unwrap(), EditResult::Renamed);
    let guard = stage.lock().unwrap();
    let robot = &guard.snapshot(root).unwrap().robots()[0];
    assert_eq!(robot.robot_id, id);
    assert_eq!(robot.name, "Renamed");
    assert_eq!(robot.representations[0].registry_resource_id, registry_id);
    assert!(guard.is_dirty(root));
    assert_eq!(refreshes, 1);
}

#[test]
fn mer_cat_edit_tc_008_duplicate_shares_registry_reference_and_notifies_once() {
    let (stage, id, _) = fixture();
    let root = Path::new("project-a");
    let mut refreshes = 0;
    let result = apply(&stage, root, Edit::Duplicate {
        robot_id: id.clone(), name: "Copy".into(),
    }, || refreshes += 1).unwrap();
    let EditResult::Duplicated { robot_id: copy } = result else {
        panic!("Expected duplicate result");
    };
    assert_ne!(copy, id);
    let guard = stage.lock().unwrap();
    let robots = guard.snapshot(root).unwrap().robots();
    assert_eq!(robots.len(), 2);
    assert_eq!(robots[0].representations, robots[1].representations);
    assert_eq!(refreshes, 1);
}

#[test]
fn mer_cat_edit_tc_009_remove_keeps_other_robot_and_registry_reference() {
    let (stage, id, _) = fixture();
    let root = Path::new("project-a");
    let copy = match apply(&stage, root, Edit::Duplicate {
        robot_id: id.clone(), name: "Copy".into(),
    }, || {}).unwrap() {
        EditResult::Duplicated { robot_id } => robot_id,
        _ => panic!("Expected duplicate result"),
    };
    let mut refreshes = 0;
    assert_eq!(apply(&stage, root, Edit::Remove { robot_id: id },
        || refreshes += 1).unwrap(), EditResult::Removed);
    let guard = stage.lock().unwrap();
    let robots = guard.snapshot(root).unwrap().robots();
    assert_eq!(robots.len(), 1);
    assert_eq!(robots[0].robot_id, copy);
    assert_eq!(robots[0].representations[0].registry_resource_id, "shared-resource");
    assert_eq!(refreshes, 1);
}

#[test]
fn mer_cat_edit_tc_012_wrong_project_is_rejected_without_notification() {
    let (stage, id, _) = fixture();
    let before = stage.lock().unwrap().snapshot(Path::new("project-a")).unwrap().clone();
    let mut refreshes = 0;
    assert!(apply(&stage, Path::new("project-b"), Edit::Rename {
        robot_id: id, name: "Wrong".into(),
    }, || refreshes += 1).is_err());
    assert_eq!(refreshes, 0);
    let guard = stage.lock().unwrap();
    assert_eq!(guard.snapshot(Path::new("project-a")).unwrap(), &before);
    assert!(!guard.is_dirty(Path::new("project-a")));
}

#[test]
fn mer_cat_edit_tc_013_invalid_edit_does_not_modify_stage_or_notify() {
    let (stage, id, _) = fixture();
    let root = Path::new("project-a");
    let before = stage.lock().unwrap().snapshot(root).unwrap().clone();
    let mut refreshes = 0;
    for edit in [
        Edit::Rename { robot_id: id.clone(), name: " ".into() },
        Edit::Duplicate { robot_id: "missing".into(), name: "Copy".into() },
        Edit::Remove { robot_id: "missing".into() },
    ] {
        assert!(apply(&stage, root, edit, || refreshes += 1).is_err());
    }
    let guard = stage.lock().unwrap();
    assert_eq!(guard.snapshot(root).unwrap(), &before);
    assert!(!guard.is_dirty(root));
    assert_eq!(refreshes, 0);
}

#[test]
fn mer_cat_edit_tc_010_unsaved_edit_is_discarded() {
    let (stage, id, _) = fixture();
    let root = Path::new("project-a");
    let saved = stage.lock().unwrap().snapshot(root).unwrap().clone();
    apply(&stage, root, Edit::Rename { robot_id: id, name: "Unsaved".into() }, || {}).unwrap();
    assert!(stage.lock().unwrap().is_dirty(root));
    let mut guard = stage.lock().unwrap();
    guard.discard();
    guard.activate(root, saved.clone());
    assert_eq!(guard.snapshot(root).unwrap(), &saved);
    assert!(!guard.is_dirty(root));
}

#[test]
fn mer_cat_edit_tc_011_save_callback_preserves_edit_and_identity() {
    let (stage, id, _) = fixture();
    let root = Path::new("project-a");
    apply(&stage, root, Edit::Rename { robot_id: id.clone(), name: "Saved".into() }, || {}).unwrap();
    let mut saved_json = String::new();
    {
        let guard = stage.lock().unwrap();
        assert!(guard.persist_for_save(root, |catalog| {
            saved_json = catalog.to_json()?;
            Ok(())
        }).unwrap());
        assert!(guard.is_dirty(root));
    }
    let loaded = RobotCatalog::from_json(&saved_json).unwrap();
    assert_eq!(loaded.robots()[0].robot_id, id);
    assert_eq!(loaded.robots()[0].name, "Saved");
    assert_eq!(loaded.robots()[0].representations[0].registry_resource_id, "shared-resource");
}
