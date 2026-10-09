//! Application-owned project storage for the current unchanged-source URDF stage.
use std::{fs, io};
use workflow_ide_framework::{
    project::ProjectContext,
    project_adapter::ApplicationProjectAdapter,
    project_resource::{ProjectDataCompatibility, ProjectDataConsistency},
    project_save::ApplicationSaveResult,
};

use crate::urdf_io::ImportedUrdf;

const DATA_VERSION: &str = "urdf-source-v1";
const SOURCE_FILE: &str = "robot-source.urdf";

#[derive(Default)]
pub struct RuntimeProjectAdapter;

impl RuntimeProjectAdapter {
    fn source_path(context: &ProjectContext) -> std::path::PathBuf {
        context.application_directory().join(SOURCE_FILE)
    }

    /// Stage a validated URDF source into the application-owned project directory.
    /// This does not yet create the canonical ROBOT-Model representation.
    pub fn import_source(context: &ProjectContext, source: &str) -> io::Result<()> {
        let imported = ImportedUrdf::parse(source)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, format!("{error:?}")))?;
        workflow_ide_framework::project_io::atomic_write(
            &Self::source_path(context),
            imported.unchanged_xml().as_bytes(),
        )
    }

    pub fn load_source(context: &ProjectContext) -> io::Result<Option<ImportedUrdf>> {
        let source = match fs::read_to_string(Self::source_path(context)) {
            Ok(source) => source,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        ImportedUrdf::parse(&source)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, format!("{error:?}")))
    }
}

impl ApplicationProjectAdapter for RuntimeProjectAdapter {
    type Error = io::Error;

    fn initialize_project(&mut self, context: &ProjectContext) -> Result<(), Self::Error> {
        fs::create_dir_all(context.application_directory())
    }

    fn inspect_project_data(
        &mut self,
        _context: &ProjectContext,
        stored_data_version: Option<&str>,
    ) -> Result<ProjectDataCompatibility, Self::Error> {
        Ok(match stored_data_version {
            None | Some(DATA_VERSION) => ProjectDataCompatibility::Compatible,
            Some(other) => ProjectDataCompatibility::Incompatible {
                reason: Some(format!("unsupported runtime data version: {other}")),
                handled: false,
            },
        })
    }

    fn check_project_consistency(
        &mut self,
        context: &ProjectContext,
    ) -> Result<ProjectDataConsistency, Self::Error> {
        match Self::load_source(context) {
            Ok(_) => Ok(ProjectDataConsistency::Consistent),
            Err(error) => Ok(ProjectDataConsistency::Inconsistent {
                reason: Some(error.to_string()),
                can_open: false,
                can_recover: false,
                handled: false,
            }),
        }
    }

    fn save_project_data(
        &mut self,
        context: &ProjectContext,
        _save_id: &str,
    ) -> Result<ApplicationSaveResult, Self::Error> {
        fs::create_dir_all(context.application_directory())?;
        Self::load_source(context)?;
        Ok(ApplicationSaveResult { data_version: Some(DATA_VERSION.into()) })
    }

    fn save_project_data_as(
        &mut self,
        source: Option<&ProjectContext>,
        destination: &ProjectContext,
        save_id: &str,
    ) -> Result<ApplicationSaveResult, Self::Error> {
        if let Some(source) = source {
            if let Some(imported) = Self::load_source(source)? {
                Self::import_source(destination, imported.unchanged_xml())?;
            }
        }
        self.save_project_data(destination, save_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_context() -> ProjectContext {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        ProjectContext::new(std::env::temp_dir().join(format!("meridian-project-test-{}-{unique}", std::process::id())))
    }

    #[test]
    fn source_survives_save_and_reload() {
        let context = temporary_context();
        let xml = "<robot name=\"r\"><link name=\"base\"/><vendor x=\"1\"/></robot>";
        RuntimeProjectAdapter::import_source(&context, xml).unwrap();
        let mut adapter = RuntimeProjectAdapter;
        adapter.save_project_data(&context, "test").unwrap();
        assert_eq!(RuntimeProjectAdapter::load_source(&context).unwrap().unwrap().unchanged_xml(), xml);
        fs::remove_dir_all(context.root()).unwrap();
    }

    #[test]
    fn invalid_import_does_not_replace_existing_source() {
        let context = temporary_context();
        let xml = "<robot name=\"r\"><link name=\"base\"/></robot>";
        RuntimeProjectAdapter::import_source(&context, xml).unwrap();
        assert!(RuntimeProjectAdapter::import_source(&context, "<robot>").is_err());
        assert_eq!(RuntimeProjectAdapter::load_source(&context).unwrap().unwrap().unchanged_xml(), xml);
        fs::remove_dir_all(context.root()).unwrap();
    }

    #[test]
    fn save_as_copies_source() {
        let source = temporary_context();
        let destination = temporary_context();
        let xml = "<robot name=\"r\"><link name=\"base\"/></robot>";
        RuntimeProjectAdapter::import_source(&source, xml).unwrap();
        let mut adapter = RuntimeProjectAdapter;
        adapter.save_project_data_as(Some(&source), &destination, "test").unwrap();
        assert_eq!(RuntimeProjectAdapter::load_source(&destination).unwrap().unwrap().unchanged_xml(), xml);
        fs::remove_dir_all(source.root()).unwrap();
        fs::remove_dir_all(destination.root()).unwrap();
    }
}
