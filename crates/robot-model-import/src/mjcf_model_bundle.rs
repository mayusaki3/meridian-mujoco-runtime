//! ROBOT-Model projection of a resolved MJCF source bundle.
use crate::{
    integrity::{validate_model, IntegrityError},
    mjcf_include::{inspect_mjcf_file, ResolveError},
    model::{from_mjcf, ModelImportError, RobotModel},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::{Path, PathBuf}};

/// Keep original files separate from the generated source-neutral model.
/// The model's original_xml is the expanded snapshot, not the entry source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MjcfModelBundle {
    pub model: RobotModel,
    pub entry_path: PathBuf,
    pub sources: BTreeMap<PathBuf, String>,
}

#[derive(Debug)]
pub enum BundleError {
    Resolve(ResolveError),
    Projection(ModelImportError),
    Integrity(IntegrityError),
}

pub fn from_mjcf_file(path: &Path) -> Result<MjcfModelBundle, BundleError> {
    let resolved = inspect_mjcf_file(path).map_err(BundleError::Resolve)?;
    let model = from_mjcf(&resolved.expanded_xml).map_err(BundleError::Projection)?;
    validate_model(&model).map_err(BundleError::Integrity)?;
    let entry_path = std::fs::canonicalize(path).map_err(|e| BundleError::Resolve(
        ResolveError::Io { path: path.to_owned(), message: e.to_string() }
    ))?;
    Ok(MjcfModelBundle { model, entry_path, sources: resolved.sources })
}

impl MjcfModelBundle {
    /// Validate after JSON deserialization without re-importing or regenerating IDs.
    pub fn validate(&self) -> Result<(), IntegrityError> {
        validate_model(&self.model)
    }
    pub fn entry_xml(&self) -> Option<&str> {
        self.sources.get(&self.entry_path).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SourceFormat, TransmissionTarget};
    use uuid::Uuid;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("mjcf-model-bundle-{}", Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn put(&self, name: &str, xml: &str) {
            let path = self.0.join(name);
            if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).unwrap(); }
            std::fs::write(path, xml).unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }

    #[test]
    fn projects_included_elements_and_retains_originals() {
        let f = Fixture::new();
        let main = r#"<mujoco model="robot"><include file="parts.xml"/><actuator><motor name="drive" joint="hinge" gear="50"/></actuator></mujoco>"#;
        let part = r#"<mujocoinclude><worldbody><body name="base"><joint name="hinge"/></body></worldbody><tendon><fixed name="rope"/></tendon></mujocoinclude>"#;
        f.put("main.xml", main);
        f.put("parts.xml", part);
        let bundle = from_mjcf_file(&f.0.join("main.xml")).unwrap();
        assert_eq!(bundle.model.source_format, SourceFormat::Mjcf);
        assert_eq!(bundle.model.bodies.len(), 1);
        assert_eq!(bundle.model.joints.len(), 1);
        assert_eq!(bundle.model.actuators.len(), 1);
        assert_eq!(bundle.model.tendons.len(), 1);
        assert_eq!(bundle.sources.len(), 2);
        assert_eq!(bundle.entry_xml(), Some(main));
        assert!(bundle.sources.values().any(|v| v == part));
        assert!(matches!(&bundle.model.actuators[0].target, TransmissionTarget::Joint(id) if id == &bundle.model.joints[0].id));
        bundle.validate().unwrap();
    }

    #[test]
    fn json_roundtrip_preserves_all_ids_and_source_files() {
        let f = Fixture::new();
        f.put("main.xml", r#"<mujoco><include file="part.xml"/></mujoco>"#);
        f.put("part.xml", r#"<mujocoinclude><worldbody><body name="b"><joint name="j"/></body></worldbody></mujocoinclude>"#);
        let original = from_mjcf_file(&f.0.join("main.xml")).unwrap();
        let encoded = serde_json::to_string(&original).unwrap();
        let decoded: MjcfModelBundle = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, original);
        decoded.validate().unwrap();
        assert_ne!(from_mjcf_file(&f.0.join("main.xml")).unwrap().model.model_id, original.model.model_id);
    }

    #[test]
    fn rejects_invalid_cross_file_reference_before_projection() {
        let f = Fixture::new();
        f.put("main.xml", r#"<mujoco><include file="part.xml"/><actuator><motor joint="missing"/></actuator></mujoco>"#);
        f.put("part.xml", r#"<mujocoinclude><worldbody><body name="b"><joint name="j"/></body></worldbody></mujocoinclude>"#);
        assert!(matches!(from_mjcf_file(&f.0.join("main.xml")), Err(BundleError::Resolve(_))));
    }
}
