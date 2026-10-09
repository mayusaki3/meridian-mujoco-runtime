//! First-stage project URDF I/O: validate and retain an exact source copy.
//! Canonical ROBOT-Model serialization is intentionally not implemented here.
use robot_model_import::{inspect_urdf, InspectedUrdf, InspectError};
use std::{fs, io, path::Path};

#[derive(Debug, Clone)]
pub struct ImportedUrdf {
    pub inspection: InspectedUrdf,
}

impl ImportedUrdf {
    pub fn parse(source: &str) -> Result<Self, InspectError> {
        Ok(Self { inspection: inspect_urdf(source)? })
    }

    /// Export is only valid while no model edits have been applied.
    pub fn unchanged_xml(&self) -> &str {
        robot_model_import::export_unchanged_source(&self.inspection)
    }
}

pub fn import_urdf(path: &Path) -> Result<ImportedUrdf, io::Error> {
    let source = fs::read_to_string(path)?;
    ImportedUrdf::parse(&source)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, format!("{error:?}")))
}

pub fn export_unchanged_urdf(model: &ImportedUrdf, path: &Path) -> io::Result<()> {
    // Avoid truncating the target if validation fails.
    ImportedUrdf::parse(model.unchanged_xml())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, format!("{error:?}")))?;
    workflow_ide_framework::project_io::atomic_write(path, model.unchanged_xml().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_exact_source_with_unknown_extension() {
        let xml = "<robot name=\"test\"><link name=\"base\"/><vendor value=\"x\"/></robot>";
        let model = ImportedUrdf::parse(xml).unwrap();
        assert_eq!(model.unchanged_xml(), xml);
        assert!(model.inspection.preserved.iter().any(|p| p.xml.contains("vendor")));
    }

    #[test]
    fn rejects_invalid_source() {
        assert!(ImportedUrdf::parse("<robot>").is_err());
    }
}
