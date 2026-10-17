//! Read-only projection of Meridian Robot Catalog into FW Project Panel nodes.
//! Robot identity, Registry identity and display-node identity are distinct.
use crate::robot_catalog::RobotCatalog;
use workflow_ide_framework::project_panel_api::{AppPanelNode, AppPanelNodeKind};

pub fn catalog_nodes(catalog: &RobotCatalog) -> Vec<AppPanelNode> {
    let robots = catalog
        .robots()
        .iter()
        .map(|robot| {
            let representations = robot
                .representations
                .iter()
                .enumerate()
                .map(|(index, representation)| AppPanelNode {
                    id: format!("robot:{}:representation:{index}", robot.robot_id),
                    label: format!("{} ({})", representation.format, representation.role),
                    kind: AppPanelNodeKind::RegisteredResource {
                        registry_resource_id: representation.registry_resource_id.clone(),
                    },
                    children: Vec::new(),
                })
                .collect();
            AppPanelNode {
                id: format!("robot:{}", robot.robot_id),
                label: robot.name.clone(),
                kind: AppPanelNodeKind::LogicalResource {
                    resource_id: robot.robot_id.clone(),
                    type_id: "meridian.robot".into(),
                },
                children: vec![AppPanelNode {
                    id: format!("robot:{}:representations", robot.robot_id),
                    label: "Representations".into(),
                    kind: AppPanelNodeKind::Category {
                        type_id: "meridian.representations".into(),
                    },
                    children: representations,
                }],
            }
        })
        .collect();
    vec![AppPanelNode {
        id: "meridian.robots".into(),
        label: "Robots".into(),
        kind: AppPanelNodeKind::Category {
            type_id: "meridian.robots".into(),
        },
        children: robots,
    }]
}

use workflow_ide_framework::{
    project::ProjectContext,
    project_panel_api::{AppPanelAction, AppPanelActionResult, ApplicationProjectPanelProvider},
};

/// Loads a fresh catalog for each requested project snapshot; no cross-project cache.
#[derive(Default)]
pub struct RobotProjectPanelProvider;

impl ApplicationProjectPanelProvider for RobotProjectPanelProvider {
    fn project_panel_roots(&self, project: &ProjectContext) -> Vec<AppPanelNode> {
        match crate::runtime_project::RuntimeProjectAdapter::load_catalog(project) {
            Ok(catalog) => catalog_nodes(&catalog),
            Err(error) => {
                workflow_ide_framework::tracing::error!(
                    target: "meridian::project_panel",
                    project = %project.root().display(),
                    error = %error,
                    "Robot Catalog could not be loaded for Project Panel"
                );
                Vec::new()
            }
        }
    }

    fn handle_project_panel_action(
        &mut self,
        _project: &ProjectContext,
        _action: AppPanelAction,
    ) -> AppPanelActionResult {
        AppPanelActionResult::NotHandled
    }
}
