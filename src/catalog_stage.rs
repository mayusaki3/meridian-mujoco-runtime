//! In-memory, project-scoped Robot Catalog staging.
//! The project adapter and Registry command bridge will share this store.
use crate::robot_catalog::RobotCatalog;
use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub struct CatalogStage {
    current: Option<StagedCatalog>,
}

struct StagedCatalog {
    root: PathBuf,
    catalog: RobotCatalog,
    dirty: bool,
}

pub type SharedCatalogStage = Arc<Mutex<CatalogStage>>;

impl CatalogStage {
    /// Never carry a staged catalog into another project.
    pub fn activate(&mut self, root: &Path, catalog: RobotCatalog) {
        self.current = Some(StagedCatalog {
            root: root.to_path_buf(),
            catalog,
            dirty: false,
        });
    }

    pub fn discard(&mut self) {
        self.current = None;
    }

    pub fn snapshot(&self, root: &Path) -> Option<&RobotCatalog> {
        self.current
            .as_ref()
            .filter(|state| state.root == root)
            .map(|state| &state.catalog)
    }

    pub fn is_dirty(&self, root: &Path) -> bool {
        self.current
            .as_ref()
            .is_some_and(|state| state.root == root && state.dirty)
    }

    pub fn replace(&mut self, root: &Path, catalog: RobotCatalog) -> io::Result<()> {
        let state = self
            .current
            .as_mut()
            .filter(|state| state.root == root)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Robot Catalog stage is not active for this project",
                )
            })?;
        // Validate before replacing the staged data.
        catalog.to_json()?;
        state.catalog = catalog;
        state.dirty = true;
        Ok(())
    }

    pub fn mark_saved(&mut self, root: &Path) {
        if let Some(state) = self.current.as_mut().filter(|state| state.root == root) {
            state.dirty = false;
        }
    }
    /// Persist a snapshot during Framework's application-save phase.
    /// The caller must only mark it saved after the whole Framework save succeeds.
    pub fn persist_for_save(
        &self,
        source_root: &Path,
        destination: &workflow_ide_framework::project::ProjectContext,
    ) -> io::Result<bool> {
        let Some(catalog) = self.snapshot(source_root) else {
            return Ok(false);
        };
        crate::runtime_project::RuntimeProjectAdapter::save_catalog(destination, catalog)?;
        Ok(true)
    }

}
