//! Portable, relative-path representation of an imported MJCF bundle.
//! This is a separate opt-in snapshot; it does not change the legacy bundle schema.
use crate::{
    integrity::{IntegrityError, validate_model},
    mjcf_include::{ResolveError, inspect_mjcf_file},
    mjcf_model_bundle::MjcfModelBundle,
    model::RobotModel,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableMjcfBundle {
    pub schema_version: u32,
    pub model: RobotModel,
    pub entry_path: PathBuf,
    pub sources: BTreeMap<PathBuf, String>,
}

#[derive(Debug)]
pub enum PortableError {
    UnsupportedVersion(u32),
    InvalidPath(PathBuf),
    SourceOutsideRoot(PathBuf),
    MissingEntry(PathBuf),
    DestinationExists(PathBuf),
    Io { path: PathBuf, message: String },
    Integrity(IntegrityError),
    Resolve(ResolveError),
    ExpandedMismatch,
}

fn safe_relative(path: &Path) -> bool {
    let mut any = false;
    for component in path.components() {
        match component {
            Component::Normal(_) => any = true,
            _ => return false,
        }
    }
    any
}

impl PortableMjcfBundle {
    /// Convert canonical source paths into paths relative to a chosen source root.
    /// Files outside that root must be handled explicitly by the caller.
    pub fn from_bundle(
        bundle: &MjcfModelBundle,
        source_root: &Path,
    ) -> Result<Self, PortableError> {
        let root = fs::canonicalize(source_root).map_err(|e| PortableError::Io {
            path: source_root.to_owned(),
            message: e.to_string(),
        })?;
        let relative = |path: &Path| -> Result<PathBuf, PortableError> {
            let value = path
                .strip_prefix(&root)
                .map_err(|_| PortableError::SourceOutsideRoot(path.to_owned()))?
                .to_owned();
            if !safe_relative(&value) {
                return Err(PortableError::InvalidPath(value));
            }
            Ok(value)
        };
        let entry_path = relative(&bundle.entry_path)?;
        let mut sources = BTreeMap::new();
        for (path, xml) in &bundle.sources {
            sources.insert(relative(path)?, xml.clone());
        }
        let portable = Self {
            schema_version: 1,
            model: bundle.model.clone(),
            entry_path,
            sources,
        };
        portable.validate()?;
        Ok(portable)
    }

    pub fn validate(&self) -> Result<(), PortableError> {
        if self.schema_version != 1 {
            return Err(PortableError::UnsupportedVersion(self.schema_version));
        }
        if !safe_relative(&self.entry_path) {
            return Err(PortableError::InvalidPath(self.entry_path.clone()));
        }
        for path in self.sources.keys() {
            if !safe_relative(path) {
                return Err(PortableError::InvalidPath(path.clone()));
            }
        }
        if !self.sources.contains_key(&self.entry_path) {
            return Err(PortableError::MissingEntry(self.entry_path.clone()));
        }
        validate_model(&self.model).map_err(PortableError::Integrity)
    }

    /// Restore into a *new, empty* destination directory, without overwriting files.
    /// Re-inspects the restored MJCF to verify its expanded XML matches the saved snapshot.
    /// On validation failure the destination is left in place for inspection.
    pub fn restore_to(&self, destination: &Path) -> Result<PathBuf, PortableError> {
        self.validate()?;
        if destination.exists() {
            return Err(PortableError::DestinationExists(destination.to_owned()));
        }
        fs::create_dir(destination).map_err(|e| PortableError::Io {
            path: destination.to_owned(),
            message: e.to_string(),
        })?;
        for (relative, xml) in &self.sources {
            let path = destination.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| PortableError::Io {
                    path: parent.to_owned(),
                    message: e.to_string(),
                })?;
            }
            fs::write(&path, xml).map_err(|e| PortableError::Io {
                path: path.clone(),
                message: e.to_string(),
            })?;
        }
        let entry = destination.join(&self.entry_path);
        let inspected = inspect_mjcf_file(&entry).map_err(PortableError::Resolve)?;
        if inspected.expanded_xml != self.model.original_xml {
            return Err(PortableError::ExpandedMismatch);
        }
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mjcf_model_bundle::from_mjcf_file;
    use uuid::Uuid;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("mjcf-portable-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn put(&self, name: &str, xml: &str) {
            let path = self.0.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, xml).unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn portable_json_roundtrip_restores_nested_sources_and_ids() {
        let f = Fixture::new();
        f.put("main.xml", r#"<mujoco><include file="parts/one.xml"/><actuator><motor joint="j"/></actuator></mujoco>"#);
        f.put(
            "parts/one.xml",
            r#"<mujocoinclude><include file="two.xml"/></mujocoinclude>"#,
        );
        f.put("parts/two.xml", r#"<mujocoinclude><worldbody><body name="b"><joint name="j"/></body></worldbody></mujocoinclude>"#);
        let bundle = from_mjcf_file(&f.0.join("main.xml")).unwrap();
        let portable = PortableMjcfBundle::from_bundle(&bundle, &f.0).unwrap();
        assert_eq!(portable.sources.len(), 3);
        let json = serde_json::to_string(&portable).unwrap();
        let decoded: PortableMjcfBundle = serde_json::from_str(&json).unwrap();
        let target = f.0.join("moved");
        let entry = decoded.restore_to(&target).unwrap();
        assert_eq!(entry, target.join("main.xml"));
        assert_eq!(decoded.model.model_id, bundle.model.model_id);
        assert_eq!(decoded.model.joints[0].id, bundle.model.joints[0].id);
        assert_eq!(
            fs::read_to_string(target.join("parts/one.xml")).unwrap(),
            portable.sources[Path::new("parts/one.xml")]
        );
        assert_eq!(decoded, portable);
    }

    #[test]
    fn rejects_traversal_missing_entry_and_existing_destination() {
        let f = Fixture::new();
        f.put("main.xml", "<mujoco/>");
        let bundle = from_mjcf_file(&f.0.join("main.xml")).unwrap();
        let mut portable = PortableMjcfBundle::from_bundle(&bundle, &f.0).unwrap();
        portable.entry_path = PathBuf::from("../escape.xml");
        assert!(matches!(
            portable.validate(),
            Err(PortableError::InvalidPath(_))
        ));
        portable.entry_path = PathBuf::from("absent.xml");
        assert!(matches!(
            portable.validate(),
            Err(PortableError::MissingEntry(_))
        ));
        portable.entry_path = PathBuf::from("main.xml");
        assert!(matches!(
            portable.restore_to(&f.0),
            Err(PortableError::DestinationExists(_))
        ));
    }

    #[test]
    fn rejects_sources_outside_selected_root() {
        let f = Fixture::new();
        f.put(
            "main.xml",
            r#"<mujoco><include file="parts/one.xml"/></mujoco>"#,
        );
        f.put("parts/one.xml", "<mujocoinclude/>");
        let bundle = from_mjcf_file(&f.0.join("main.xml")).unwrap();
        let narrow = f.0.join("parts");
        assert!(matches!(
            PortableMjcfBundle::from_bundle(&bundle, &narrow),
            Err(PortableError::SourceOutsideRoot(_))
        ));
    }
}
