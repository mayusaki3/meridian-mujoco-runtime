//! Bridge from Framework-owned Resource Registry to Meridian-owned Robot Catalog.
//! This helper does not persist the catalog or perform file imports.
use std::io;
use workflow_ide_framework::{
    project_controller::RegistryAccess,
    project_resource::ResourceReference,
};
use crate::robot_catalog::{RobotCatalog, RobotRepresentation};

/// Create a distinct logical Robot, referencing the Framework-issued Resource ID.
/// Validate catalog input before registering a resource to avoid avoidable partial changes.
pub fn register_robot_representation(
    registry: &mut RegistryAccess<'_>,
    catalog: &mut RobotCatalog,
    name: &str,
    reference: ResourceReference,
    format: &str,
    role: &str,
) -> io::Result<String> {
    if name.trim().is_empty() || format.trim().is_empty() || role.trim().is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Robot name, format and role must be nonempty"));
    }
    let mut candidate = catalog.clone();
    let robot_id = candidate.create(name)?;
    // Framework owns the registry identity. Registration may mark the project dirty.
    let registry_resource_id = registry.register(reference)
        .map_err(|message| io::Error::other(message))?;
    candidate.add_representation(&robot_id, RobotRepresentation {
        registry_resource_id,
        format: format.into(),
        role: role.into(),
    })?;
    *catalog = candidate;
    Ok(robot_id)
}
