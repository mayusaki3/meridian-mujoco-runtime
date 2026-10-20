//! Explicit, project-scoped registration requests from UI commands to Framework.
use crate::{
    catalog_stage::SharedCatalogStage, robot_registry_integration::register_robot_representation,
    runtime_project::RuntimeProjectAdapter,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use workflow_ide_framework::{
    project::ProjectContext,
    project_controller::RegistryAccess,
    project_resource::{ResourceReference, ResourceScope},
};

#[derive(Clone, Debug)]
pub struct Request {
    pub root: PathBuf,
    pub name: String,
    pub path: &'static str,
    pub format: &'static str,
}

#[derive(Default)]
pub struct Pending {
    pub request: Option<Request>,
}

pub type SharedPending = Arc<Mutex<Pending>>;

pub fn process_pending(
    pending: &SharedPending,
    active_root: &Arc<Mutex<Option<PathBuf>>>,
    registry: &mut RegistryAccess<'_>,
    stage: &SharedCatalogStage,
) {
    let root = active_root.lock().ok().and_then(|guard| guard.clone());
    let Some(root) = root else {
        return;
    };
    let request = {
        let Ok(mut state) = pending.lock() else {
            return;
        };
        match state.request.as_ref() {
            Some(request) if request.root == root => state.request.take(),
            _ => None,
        }
    };
    let Some(request) = request else {
        return;
    };
    let context = ProjectContext::new(root.clone());
    let result = (|| {
        let mut stage_guard = stage
            .lock()
            .map_err(|_| std::io::Error::other("Catalog stage lock poisoned"))?;
        if stage_guard.snapshot(&root).is_none() {
            stage_guard.activate(&root, RuntimeProjectAdapter::load_catalog(&context)?);
        }
        let mut catalog = stage_guard
            .snapshot(&root)
            .expect("stage activated")
            .clone();
        let reference =
            ResourceReference::new(ResourceScope::Project, request.path).map_err(|error| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{error:?}"))
            })?;
        let id = register_robot_representation(
            registry,
            &mut catalog,
            &request.name,
            reference,
            request.format,
            "source",
        )?;
        stage_guard.replace(&root, catalog)?;
        Ok::<String, std::io::Error>(id)
    })();
    match result {
        Ok(id) => {
            workflow_ide_framework::tracing::info!(target: "meridian::catalog", robot_id=%id, "Robot Catalog registration completed")
        }
        Err(error) => {
            workflow_ide_framework::tracing::error!(target: "meridian::catalog", error=%error, "Robot Catalog registration failed; request consumed (explicit retry required)");
        }
    }
}

/// One-shot post-open check, executed only when Framework exposes RegistryAccess.
pub fn diagnose_opened_project(
    pending_root: &Arc<Mutex<Option<PathBuf>>>,
    active_root: &Arc<Mutex<Option<PathBuf>>>,
    registry: &RegistryAccess<'_>,
) {
    let Some(active) = active_root.lock().ok().and_then(|root| root.clone()) else {
        return;
    };
    let should_check = {
        let Ok(mut pending) = pending_root.lock() else {
            return;
        };
        if pending.as_ref() != Some(&active) {
            false
        } else {
            *pending = None;
            true
        }
    };
    if !should_check {
        return;
    }
    let context = ProjectContext::new(active.clone());
    match RuntimeProjectAdapter::load_catalog(&context) {
        Ok(catalog) => {
            let unresolved =
                crate::robot_registry_integration::unresolved_registry_references(&catalog, registry);
            for item in &unresolved {
                workflow_ide_framework::tracing::warn!(
                    target: "meridian::registry",
                    project = %active.display(),
                    robot_id = %item.robot_id,
                    registry_resource_id = %item.registry_resource_id,
                    format = %item.format,
                    role = %item.role,
                    "Robot Catalog contains unresolved Framework Registry reference"
                );
            }
            workflow_ide_framework::tracing::info!(
                target: "meridian::registry",
                project = %active.display(),
                unresolved = unresolved.len(),
                "Robot Catalog Registry reference check completed"
            );
        }
        Err(error) => {
            workflow_ide_framework::tracing::error!(
                target: "meridian::registry",
                project = %active.display(),
                error = %error,
                "Robot Catalog Registry reference check failed"
            );
        }
    }
}
