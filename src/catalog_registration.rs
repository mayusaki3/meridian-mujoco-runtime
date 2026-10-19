//! Explicit, project-scoped registration requests from UI commands to Framework.
use std::{path::PathBuf, sync::{Arc, Mutex}};
use workflow_ide_framework::{
    project::ProjectContext,
    project_controller::RegistryAccess,
    project_resource::{ResourceReference, ResourceScope},
};
use crate::{robot_registry_integration::register_robot_representation, runtime_project::RuntimeProjectAdapter};

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

pub fn process_pending(pending: &SharedPending, active_root: &Arc<Mutex<Option<PathBuf>>>, registry: &mut RegistryAccess<'_>) {
    let root = active_root.lock().ok().and_then(|guard| guard.clone());
    let Some(root) = root else { return; };
    let request = {
        let Ok(mut state) = pending.lock() else { return; };
        match state.request.as_ref() {
            Some(request) if request.root == root => state.request.take(),
            _ => None,
        }
    };
    let Some(request) = request else { return; };
    let context = ProjectContext::new(root.clone());
    let result = (|| {
        let mut catalog = RuntimeProjectAdapter::load_catalog(&context)?;
        let reference = ResourceReference::new(ResourceScope::Project, request.path)
            .map_err(std::io::Error::other)?;
        let id = register_robot_representation(registry, &mut catalog, &request.name, reference, request.format, "source")?;
        RuntimeProjectAdapter::save_catalog(&context, &catalog)?;
        Ok::<String, std::io::Error>(id)
    })();
    match result {
        Ok(id) => workflow_ide_framework::tracing::info!(target: "meridian::catalog", robot_id=%id, "Robot Catalog registration completed"),
        Err(error) => {
            workflow_ide_framework::tracing::error!(target: "meridian::catalog", error=%error, "Robot Catalog registration failed; request retained for retry");
            if let Ok(mut state) = pending.lock() {
                if state.request.is_none() {
                    state.request = Some(request);
                }
            }
        }
    }
}
