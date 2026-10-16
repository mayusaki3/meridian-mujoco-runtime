//! Application-owned, validated MJCF snapshot persistence.
use robot_model_import::{
    mjcf_bundle_reimport::{BundleReimportResult, reimport_portable_mjcf_with_identity},
    mjcf_model_bundle::from_mjcf_file,
    mjcf_portable::PortableMjcfBundle,
    mjcf_provenance::{MjcfProvenance, map_included_elements},
};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};
use workflow_ide_framework::project::ProjectContext;

const SNAPSHOT_FILE: &str = "mjcf-snapshot.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MjcfProjectSnapshot {
    pub bundle: PortableMjcfBundle,
    pub provenance: MjcfProvenance,
}

impl MjcfProjectSnapshot {
    pub fn validate(&self) -> io::Result<()> {
        self.bundle.validate().map_err(invalid)?;
        self.provenance
            .validate_included_elements(&self.bundle)
            .map_err(invalid)
    }
}

fn invalid(error: impl std::fmt::Debug) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("{error:?}"))
}

fn snapshot_path(context: &ProjectContext) -> std::path::PathBuf {
    context.application_directory().join(SNAPSHOT_FILE)
}

/// Store the bundle and provenance as one atomically replaced JSON file.
/// Validate before writing; on validation failure the previous file is intact.
pub fn save_snapshot(context: &ProjectContext, snapshot: &MjcfProjectSnapshot) -> io::Result<()> {
    snapshot.validate()?;
    let json = serde_json::to_vec_pretty(snapshot).map_err(invalid)?;
    workflow_ide_framework::project_io::atomic_write(&snapshot_path(context), &json)
}

pub fn load_snapshot(context: &ProjectContext) -> io::Result<Option<MjcfProjectSnapshot>> {
    let json = match fs::read(snapshot_path(context)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let snapshot: MjcfProjectSnapshot = serde_json::from_slice(&json).map_err(invalid)?;
    snapshot.validate()?;
    Ok(Some(snapshot))
}

/// Import a new portable MJCF source tree. The snapshot is committed only
/// after all source and provenance checks succeed.
pub fn import_snapshot(
    context: &ProjectContext,
    entry_file: &Path,
    source_root: &Path,
) -> io::Result<MjcfProjectSnapshot> {
    let imported = from_mjcf_file(entry_file).map_err(invalid)?;
    let bundle = PortableMjcfBundle::from_bundle(&imported, source_root).map_err(invalid)?;
    let provenance = map_included_elements(&bundle).map_err(invalid)?;
    let snapshot = MjcfProjectSnapshot { bundle, provenance };
    save_snapshot(context, &snapshot)?;
    Ok(snapshot)
}

/// Reimport updated MJCF sources while preserving safely matched identities.
/// If validation fails, the existing saved snapshot is not replaced.
pub fn reimport_snapshot(
    context: &ProjectContext,
    entry_file: &Path,
    source_root: &Path,
) -> io::Result<BundleReimportResult> {
    let previous = load_snapshot(context)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "MJCF snapshot is missing"))?;
    let result = reimport_portable_mjcf_with_identity(
        &previous.bundle,
        &previous.provenance,
        entry_file,
        source_root,
    )
    .map_err(invalid)?;
    save_snapshot(
        context,
        &MjcfProjectSnapshot {
            bundle: result.bundle.clone(),
            provenance: result.provenance.clone(),
        },
    )?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        root: PathBuf,
        context: ProjectContext,
    }
    impl Fixture {
        fn new() -> Self {
            let id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir()
                .join(format!("meridian-mjcf-project-{}-{id}", std::process::id()));
            fs::create_dir_all(&root).unwrap();
            let context = ProjectContext::new(root.join("project"));
            Self { root, context }
        }
        fn put(&self, file: &str, xml: &str) {
            let path = self.root.join("sources").join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, xml).unwrap();
        }
        fn entry(&self) -> PathBuf {
            self.root.join("sources/main.xml")
        }
        fn source_root(&self) -> PathBuf {
            self.root.join("sources")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn snapshot_roundtrip_preserves_ids_and_original_sources() {
        let f = Fixture::new();
        f.put(
            "main.xml",
            r#"<mujoco><include file="parts.xml"/></mujoco>"#,
        );
        f.put("parts.xml", r#"<mujocoinclude><worldbody><body name="b"><joint name="j"/></body></worldbody></mujocoinclude>"#);
        let saved = import_snapshot(&f.context, &f.entry(), &f.source_root()).unwrap();
        let loaded = load_snapshot(&f.context).unwrap().unwrap();
        assert_eq!(saved, loaded);
        assert_eq!(
            saved.bundle.model.bodies[0].id,
            loaded.bundle.model.bodies[0].id
        );
        assert_eq!(saved.bundle.sources, loaded.bundle.sources);
    }

    #[test]
    fn reimport_preserves_ids_and_updates_provenance_atomically() {
        let f = Fixture::new();
        f.put(
            "main.xml",
            r#"<mujoco><include file="parts.xml"/></mujoco>"#,
        );
        f.put(
            "parts.xml",
            r#"<mujocoinclude><worldbody><body name="b"/></worldbody></mujocoinclude>"#,
        );
        let original = import_snapshot(&f.context, &f.entry(), &f.source_root()).unwrap();
        f.put("parts.xml", r#"<mujocoinclude> <worldbody><body name="b"/><body name="new"/></worldbody></mujocoinclude>"#);
        let result = reimport_snapshot(&f.context, &f.entry(), &f.source_root()).unwrap();
        let loaded = load_snapshot(&f.context).unwrap().unwrap();
        assert_eq!(result.bundle, loaded.bundle);
        assert_eq!(result.provenance, loaded.provenance);
        assert_eq!(loaded.bundle.model.model_id, original.bundle.model.model_id);
        assert_eq!(
            loaded.bundle.model.bodies[0].id,
            original.bundle.model.bodies[0].id
        );
        assert_eq!(result.plan.unresolved.len(), 1);
    }

    #[test]
    fn failed_reimport_and_invalid_snapshot_do_not_overwrite_previous() {
        let f = Fixture::new();
        f.put(
            "main.xml",
            r#"<mujoco><worldbody><body name="b"/></worldbody></mujoco>"#,
        );
        let original = import_snapshot(&f.context, &f.entry(), &f.source_root()).unwrap();
        let before = fs::read(snapshot_path(&f.context)).unwrap();
        f.put("main.xml", "<mujoco>");
        assert!(reimport_snapshot(&f.context, &f.entry(), &f.source_root()).is_err());
        assert_eq!(fs::read(snapshot_path(&f.context)).unwrap(), before);
        let mut tampered = original.clone();
        tampered.provenance.origins.clear();
        assert!(save_snapshot(&f.context, &tampered).is_err());
        assert_eq!(fs::read(snapshot_path(&f.context)).unwrap(), before);
    }
}
