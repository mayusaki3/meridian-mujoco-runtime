//! MER-REG integration tests: Framework owns Registry; Meridian owns Robot Catalog.
#[path = "../src/robot_catalog.rs"]
mod robot_catalog;
#[path = "../src/robot_registry_integration.rs"]
mod robot_registry_integration;

use robot_catalog::RobotCatalog;
use robot_registry_integration::register_robot_representation;
use workflow_ide_framework::{
    project_controller::ProjectController,
    project_lifecycle::ProjectSession,
    project_resource::{ResourceReference, ResourceScope},
};

fn controller() -> ProjectController {
    let mut controller = ProjectController::new("meridian-mujoco-runtime", "Meridian");
    controller.session = Some(ProjectSession::default());
    controller
}

#[test]
fn mer_reg_tc_001_registry_id_is_framework_generated_and_catalog_references_it() {
    let mut controller = controller();
    let mut catalog = RobotCatalog::default();
    let reference =
        ResourceReference::new(ResourceScope::Project, "application/robot-source.urdf").unwrap();
    let robot_id = {
        let mut access = controller.registry_access().unwrap();
        register_robot_representation(
            &mut access,
            &mut catalog,
            "Robot A",
            reference.clone(),
            "URDF",
            "source",
        )
        .unwrap()
    };
    let robot = catalog
        .robots()
        .iter()
        .find(|robot| robot.robot_id == robot_id)
        .unwrap();
    assert_eq!(robot.representations.len(), 1);
    assert_eq!(controller.resource_registry.entries().len(), 1);
    assert_eq!(
        robot.representations[0].registry_resource_id,
        controller.resource_registry.entries()[0].resource_id
    );
    assert_eq!(
        controller
            .resource_registry
            .find_by_reference(&reference)
            .unwrap()
            .resource_id,
        robot.representations[0].registry_resource_id
    );
}

#[test]
fn mer_reg_tc_002_repeated_reference_reuses_resource_id_but_not_robot_id() {
    let mut controller = controller();
    let mut catalog = RobotCatalog::default();
    let reference =
        ResourceReference::new(ResourceScope::Project, "application/robot-source.urdf").unwrap();
    let (first, second) = {
        let mut access = controller.registry_access().unwrap();
        let first = register_robot_representation(
            &mut access,
            &mut catalog,
            "Robot A",
            reference.clone(),
            "URDF",
            "source",
        )
        .unwrap();
        let second = register_robot_representation(
            &mut access,
            &mut catalog,
            "Robot B",
            reference,
            "URDF",
            "source",
        )
        .unwrap();
        (first, second)
    };
    assert_ne!(first, second);
    assert_eq!(controller.resource_registry.entries().len(), 1);
    assert_eq!(catalog.robots().len(), 2);
    assert_eq!(
        catalog.robots()[0].representations[0].registry_resource_id,
        catalog.robots()[1].representations[0].registry_resource_id
    );
}

#[test]
fn mer_reg_tc_003_invalid_robot_name_does_not_register_resource() {
    let mut controller = controller();
    let mut catalog = RobotCatalog::default();
    let reference =
        ResourceReference::new(ResourceScope::Project, "application/robot-source.urdf").unwrap();
    let result = {
        let mut access = controller.registry_access().unwrap();
        register_robot_representation(&mut access, &mut catalog, "", reference, "URDF", "source")
    };
    assert!(result.is_err());
    assert!(catalog.robots().is_empty());
    assert!(controller.resource_registry.entries().is_empty());
}

#[test]
fn mer_reg_tc_004_invalid_representation_fields_leave_registry_and_catalog_unchanged() {
    for (format, role) in [("", "source"), ("URDF", ""), ("  ", "source")] {
        let mut controller = controller();
        let mut catalog = RobotCatalog::default();
        let reference =
            ResourceReference::new(ResourceScope::Project, "application/robot-source.urdf")
                .unwrap();
        let result = {
            let mut access = controller.registry_access().unwrap();
            register_robot_representation(
                &mut access,
                &mut catalog,
                "Robot A",
                reference,
                format,
                role,
            )
        };
        assert!(result.is_err());
        assert!(catalog.robots().is_empty());
        assert!(controller.resource_registry.entries().is_empty());
    }
}

#[test]
fn mer_reg_tc_005_registration_requires_active_framework_project() {
    let mut controller = ProjectController::new("meridian-mujoco-runtime", "Meridian");
    assert!(controller.registry_access().is_err());
    assert!(controller.resource_registry.entries().is_empty());
}

#[test]
fn mer_reg_tc_006_registration_does_not_implicitly_persist_catalog() {
    let mut controller = controller();
    let mut catalog = RobotCatalog::default();
    let reference =
        ResourceReference::new(ResourceScope::Project, "application/robot-source.urdf").unwrap();
    {
        let mut access = controller.registry_access().unwrap();
        register_robot_representation(
            &mut access,
            &mut catalog,
            "Robot A",
            reference,
            "URDF",
            "source",
        )
        .unwrap();
    }
    // RegistryAccess changes Framework state; the application still owns catalog persistence.
    assert_eq!(catalog.robots().len(), 1);
    assert!(controller.session.as_ref().unwrap().dirty.framework);
}

#[test]
fn mer_reg_persist_tc_005_orphan_registry_reference_is_reported_without_mutation() {
    use robot_catalog::RobotRepresentation;
    use robot_registry_integration::unresolved_registry_references;
    let mut controller = controller();
    let mut catalog = RobotCatalog::default();
    let robot_id = catalog.create("Orphan").unwrap();
    catalog
        .add_representation(
            &robot_id,
            RobotRepresentation {
                registry_resource_id: "missing-registry-id".into(),
                format: "URDF".into(),
                role: "source".into(),
            },
        )
        .unwrap();
    let before = catalog.clone();
    let missing = unresolved_registry_references(&catalog, &controller.registry_access().unwrap());
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].robot_id, robot_id);
    assert_eq!(missing[0].registry_resource_id, "missing-registry-id");
    assert_eq!(missing[0].format, "URDF");
    assert_eq!(missing[0].role, "source");
    assert_eq!(catalog, before);
    assert!(controller.resource_registry.entries().is_empty());
}

#[test]
fn mer_reg_persist_tc_002_registered_references_are_resolved() {
    use robot_registry_integration::unresolved_registry_references;
    let mut controller = controller();
    let mut catalog = RobotCatalog::default();
    let reference =
        ResourceReference::new(ResourceScope::Project, "application/robot-source.urdf").unwrap();
    {
        let mut access = controller.registry_access().unwrap();
        register_robot_representation(
            &mut access,
            &mut catalog,
            "Robot",
            reference,
            "URDF",
            "source",
        )
        .unwrap();
    }
    let before = catalog.clone();
    assert!(
        unresolved_registry_references(&catalog, &controller.registry_access().unwrap()).is_empty()
    );
    assert_eq!(catalog, before);
    assert_eq!(controller.resource_registry.entries().len(), 1);
}
