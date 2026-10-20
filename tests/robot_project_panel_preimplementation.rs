//! MER-PP preimplementation tests. Requires FW Project Panel API at f0534f6 or later.
//! This test is intentionally added before src/robot_project_panel.rs implementation.
#[path = "../src/catalog_stage.rs"]
mod catalog_stage;
#[path = "../src/mjcf_project.rs"]
mod mjcf_project;
#[path = "../src/robot_catalog.rs"]
mod robot_catalog;
#[path = "../src/robot_project_panel.rs"]
mod robot_project_panel;
#[path = "../src/runtime_project.rs"]
mod runtime_project;
#[path = "../src/urdf_io.rs"]
mod urdf_io;

use robot_catalog::{RobotCatalog, RobotRepresentation};
use robot_project_panel::catalog_nodes;
use workflow_ide_framework::{
    project_panel_api::{AppPanelNode, AppPanelNodeKind, build_application_project_tree},
    project_resource::{ResourceReference, ResourceScope},
    resource_registry::ResourceRegistry,
};

fn robot_nodes(nodes: &[AppPanelNode]) -> &[AppPanelNode] {
    assert_eq!(nodes.len(), 1);
    assert!(matches!(nodes[0].kind, AppPanelNodeKind::Category { .. }));
    &nodes[0].children
}

#[test]
fn mer_pp_tc_001_empty_catalog_has_robots_category_only() {
    let nodes = catalog_nodes(&RobotCatalog::default());
    assert!(robot_nodes(&nodes).is_empty());
}

#[test]
fn mer_pp_tc_002_two_robots_have_distinct_node_ids_and_correct_robot_ids() {
    let mut catalog = RobotCatalog::default();
    let first = catalog.create("robot01").unwrap();
    let second = catalog.create("robot02").unwrap();
    let nodes = catalog_nodes(&catalog);
    let robots = robot_nodes(&nodes);
    assert_eq!(robots.len(), 2);
    assert_ne!(robots[0].id, robots[1].id);
    assert!(
        matches!(&robots[0].kind, AppPanelNodeKind::LogicalResource { resource_id, .. } if resource_id == &first)
    );
    assert!(
        matches!(&robots[1].kind, AppPanelNodeKind::LogicalResource { resource_id, .. } if resource_id == &second)
    );
}

#[test]
fn mer_pp_tc_003_rename_keeps_robot_and_node_identity() {
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("before").unwrap();
    let before = catalog_nodes(&catalog);
    catalog.rename(&id, "after").unwrap();
    let after = catalog_nodes(&catalog);
    assert_eq!(robot_nodes(&before)[0].id, robot_nodes(&after)[0].id);
    assert_ne!(robot_nodes(&before)[0].label, robot_nodes(&after)[0].label);
    assert!(
        matches!(&robot_nodes(&after)[0].kind, AppPanelNodeKind::LogicalResource { resource_id, .. } if resource_id == &id)
    );
}

#[test]
fn mer_pp_tc_004_representation_references_registry_id_not_robot_id() {
    let mut catalog = RobotCatalog::default();
    let robot_id = catalog.create("robot").unwrap();
    catalog
        .add_representation(
            &robot_id,
            RobotRepresentation {
                registry_resource_id: "registry-urdf-01".into(),
                format: "urdf".into(),
                role: "source".into(),
            },
        )
        .unwrap();
    let nodes = catalog_nodes(&catalog);
    let reps = &robot_nodes(&nodes)[0].children;
    assert_eq!(reps.len(), 1);
    assert_eq!(reps[0].children.len(), 1);
    assert!(
        matches!(&reps[0].children[0].kind, AppPanelNodeKind::RegisteredResource { registry_resource_id } if registry_resource_id == "registry-urdf-01")
    );
    assert_ne!(robot_id, "registry-urdf-01");
}

#[test]
fn mer_pp_tc_005_same_registry_id_may_appear_under_two_robots() {
    let mut catalog = RobotCatalog::default();
    let a = catalog.create("a").unwrap();
    let b = catalog.create("b").unwrap();
    for id in [&a, &b] {
        catalog
            .add_representation(
                id,
                RobotRepresentation {
                    registry_resource_id: "shared-file".into(),
                    format: "urdf".into(),
                    role: "source".into(),
                },
            )
            .unwrap();
    }
    let nodes = catalog_nodes(&catalog);
    let robots = robot_nodes(&nodes);
    let left = &robots[0].children[0].children[0];
    let right = &robots[1].children[0].children[0];
    assert_ne!(left.id, right.id);
    assert_eq!(left.kind, right.kind);
    let mut registry = ResourceRegistry::default();
    registry.register(
        "shared-file",
        ResourceReference::new(ResourceScope::Project, "shared.urdf").unwrap(),
    );
    assert!(build_application_project_tree(&nodes, &registry).is_ok());
}

#[test]
fn mer_pp_tc_006_unknown_registry_reference_is_not_silently_registered() {
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("robot").unwrap();
    catalog
        .add_representation(
            &id,
            RobotRepresentation {
                registry_resource_id: "missing-id".into(),
                format: "mjcf".into(),
                role: "generated".into(),
            },
        )
        .unwrap();
    let registry = ResourceRegistry::default();
    let result = build_application_project_tree(&catalog_nodes(&catalog), &registry);
    assert!(
        matches!(result, Err(workflow_ide_framework::project_panel_api::AppPanelTreeError::UnknownRegisteredResource(ref id)) if id == "missing-id")
    );
    assert!(registry.entries().is_empty());
}

#[test]
fn mer_pp_tc_008_provider_does_not_carry_catalog_across_projects() {
    use robot_project_panel::RobotProjectPanelProvider;
    use runtime_project::RuntimeProjectAdapter;
    use workflow_ide_framework::{
        project::ProjectContext, project_panel_api::ApplicationProjectPanelProvider,
    };
    let first = ProjectContext::new(
        std::env::temp_dir().join(format!("meridian-pp-first-{}", uuid::Uuid::new_v4())),
    );
    let second = ProjectContext::new(
        std::env::temp_dir().join(format!("meridian-pp-second-{}", uuid::Uuid::new_v4())),
    );
    let mut catalog = RobotCatalog::default();
    catalog.create("first robot").unwrap();
    RuntimeProjectAdapter::save_catalog(&first, &catalog).unwrap();
    let provider = RobotProjectPanelProvider;
    assert_eq!(robot_nodes(&provider.project_panel_roots(&first)).len(), 1);
    assert!(robot_nodes(&provider.project_panel_roots(&second)).is_empty());
    std::fs::remove_dir_all(first.root()).unwrap();
}

#[test]
fn mer_pp_tc_009_provider_returns_not_handled_for_unimplemented_actions() {
    use robot_project_panel::RobotProjectPanelProvider;
    use workflow_ide_framework::{
        project::ProjectContext,
        project_panel_api::{
            AppPanelAction, AppPanelActionResult, ApplicationProjectPanelProvider,
        },
    };
    let mut provider = RobotProjectPanelProvider;
    let context = ProjectContext::new("example");
    assert_eq!(
        provider.handle_project_panel_action(
            &context,
            AppPanelAction::Activated {
                node_id: "meridian.robots".into()
            }
        ),
        AppPanelActionResult::NotHandled
    );
}

#[test]
fn mer_pp_tc_010_staged_provider_displays_unsaved_robot_without_disk_write() {
    use robot_project_panel::StagedRobotProjectPanelProvider;
    use workflow_ide_framework::{
        project::ProjectContext, project_panel_api::ApplicationProjectPanelProvider,
    };
    let project = ProjectContext::new(
        std::env::temp_dir().join(format!("meridian-pp-stage-{}", uuid::Uuid::new_v4())),
    );
    let stage = catalog_stage::SharedCatalogStage::default();
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("unsaved robot").unwrap();
    {
        let mut guard = stage.lock().unwrap();
        guard.activate(project.root(), RobotCatalog::default());
        guard.replace(project.root(), catalog).unwrap();
    }
    let provider = StagedRobotProjectPanelProvider { stage };
    let nodes = provider.project_panel_roots(&project);
    assert_eq!(robot_nodes(&nodes).len(), 1);
    assert!(matches!(&robot_nodes(&nodes)[0].kind, AppPanelNodeKind::LogicalResource { resource_id, .. } if resource_id == &id));
    assert!(!project.application_directory().join("robot-catalog.json").exists());
}

#[test]
fn mer_pp_tc_011_staged_provider_isolated_by_project_and_discard() {
    use robot_project_panel::StagedRobotProjectPanelProvider;
    use workflow_ide_framework::{
        project::ProjectContext, project_panel_api::ApplicationProjectPanelProvider,
    };
    let first = ProjectContext::new(
        std::env::temp_dir().join(format!("meridian-pp-stage-a-{}", uuid::Uuid::new_v4())),
    );
    let second = ProjectContext::new(
        std::env::temp_dir().join(format!("meridian-pp-stage-b-{}", uuid::Uuid::new_v4())),
    );
    let stage = catalog_stage::SharedCatalogStage::default();
    let mut catalog = RobotCatalog::default();
    catalog.create("first robot").unwrap();
    stage.lock().unwrap().activate(first.root(), catalog);
    let provider = StagedRobotProjectPanelProvider {
        stage: stage.clone(),
    };
    assert_eq!(robot_nodes(&provider.project_panel_roots(&first)).len(), 1);
    assert!(robot_nodes(&provider.project_panel_roots(&second)).is_empty());
    stage.lock().unwrap().discard();
    assert!(robot_nodes(&provider.project_panel_roots(&first)).is_empty());
}
