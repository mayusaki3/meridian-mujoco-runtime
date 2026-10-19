//! MER-REG GUI bridge regression tests (real Framework RegistryAccess).
#[path = "../src/catalog_registration.rs"]
mod catalog_registration;
#[path = "../src/catalog_stage.rs"]
mod catalog_stage;
#[path = "../src/mjcf_project.rs"]
mod mjcf_project;
#[path = "../src/robot_catalog.rs"]
mod robot_catalog;
#[path = "../src/robot_registry_integration.rs"]
mod robot_registry_integration;
#[path = "../src/runtime_project.rs"]
mod runtime_project;
#[path = "../src/urdf_io.rs"]
mod urdf_io;

use std::sync::{Arc, Mutex};
use workflow_ide_framework::{
    project_controller::ProjectController, project_lifecycle::ProjectSession,
};

fn controller() -> ProjectController {
    let mut controller = ProjectController::new("meridian-mujoco-runtime", "Meridian");
    controller.session = Some(ProjectSession::default());
    controller
}

#[test]
fn mer_reg_gui_tc_001_request_is_not_consumed_without_active_root() {
    let pending = catalog_registration::SharedPending::default();
    let root = std::env::temp_dir().join("meridian-inactive-project");
    pending.lock().unwrap().request = Some(catalog_registration::Request {
        root,
        name: "Robot".into(),
        path: "application/robot-source.urdf",
        format: "URDF",
    });
    let active = Arc::new(Mutex::new(None));
    let mut controller = controller();
    catalog_registration::process_pending(
        &pending,
        &active,
        &mut controller.registry_access().unwrap(),
        &catalog_stage::SharedCatalogStage::default(),
    );
    assert!(pending.lock().unwrap().request.is_some());
    assert!(controller.resource_registry.entries().is_empty());
}

#[test]
fn mer_reg_gui_tc_002_other_project_request_is_not_processed() {
    let pending = catalog_registration::SharedPending::default();
    let root = std::env::temp_dir().join("meridian-project-a");
    pending.lock().unwrap().request = Some(catalog_registration::Request {
        root,
        name: "Robot".into(),
        path: "application/robot-source.urdf",
        format: "URDF",
    });
    let active = Arc::new(Mutex::new(Some(
        std::env::temp_dir().join("meridian-project-b"),
    )));
    let mut controller = controller();
    catalog_registration::process_pending(
        &pending,
        &active,
        &mut controller.registry_access().unwrap(),
    );
    assert!(pending.lock().unwrap().request.is_some());
    assert!(controller.resource_registry.entries().is_empty());
}
