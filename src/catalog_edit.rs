//! Project-scoped, transactional Robot Catalog edits.
//! Registry resources and source files are intentionally not mutated here.
use crate::catalog_stage::SharedCatalogStage;
use std::{io, path::Path};
use workflow_ide_framework::project_panel_api::ProjectPanelRefreshHandle;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    Rename { robot_id: String, name: String },
    Duplicate { robot_id: String, name: String },
    Remove { robot_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditResult {
    Renamed,
    Duplicated { robot_id: String },
    Removed,
}

/// Apply to a clone, commit only after validation, and notify only on success.
/// An inactive or mismatched project is an error, never an implicit activation.
pub fn apply(
    stage: &SharedCatalogStage,
    root: &Path,
    edit: Edit,
    on_success: impl FnOnce(),
) -> io::Result<EditResult> {
    let result = {
        let mut guard = stage
            .lock()
            .map_err(|_| io::Error::other("Catalog stage lock poisoned"))?;
        let mut candidate = guard
            .snapshot(root)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Catalog stage is not active for project"))?
            .clone();
        let result = match edit {
            Edit::Rename { robot_id, name } => {
                candidate.rename(&robot_id, &name)?;
                EditResult::Renamed
            }
            Edit::Duplicate { robot_id, name } => EditResult::Duplicated {
                robot_id: candidate.duplicate(&robot_id, &name)?,
            },
            Edit::Remove { robot_id } => {
                candidate.remove(&robot_id)?;
                EditResult::Removed
            }
        };
        guard.replace(root, candidate)?;
        result
    };
    on_success();
    Ok(result)
}

/// Production adapter for the Framework Project Panel refresh signal.
pub fn apply_with_panel_refresh(
    stage: &SharedCatalogStage,
    root: &Path,
    edit: Edit,
    panel_refresh: &ProjectPanelRefreshHandle,
) -> io::Result<EditResult> {
    apply(stage, root, edit, || {
        panel_refresh.invalidate_application_project_panel();
    })
}
