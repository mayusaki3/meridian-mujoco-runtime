//! Safe reimport of a portable MJCF source bundle with provenance regeneration.
use crate::{
    mjcf_identity::{apply_identity_plan, plan_named_identity_reuse, ApplyIdentityError, IdentityPlan, ReconcileError},
    mjcf_model_bundle::{from_mjcf_file, BundleError},
    mjcf_portable::{PortableError, PortableMjcfBundle},
    mjcf_provenance::{map_included_elements, MjcfProvenance, ProvenanceError},
};
use std::path::Path;

#[derive(Debug)]
pub enum BundleReimportError {
    OldSnapshot(PortableError),
    OldProvenance(ProvenanceError),
    Import(BundleError),
    Portable(PortableError),
    Reconcile(ReconcileError),
    Apply(ApplyIdentityError),
    Provenance(ProvenanceError),
}

#[derive(Debug)]
pub struct BundleReimportResult {
    pub bundle: PortableMjcfBundle,
    pub provenance: MjcfProvenance,
    pub plan: IdentityPlan,
}

/// Read updated sources from disk and return a validated, self-contained snapshot.
/// The old bundle and provenance are never mutated. No files are written.
/// source_root must contain every source referenced by the new entry file.
pub fn reimport_portable_mjcf_with_identity(
    previous: &PortableMjcfBundle,
    previous_provenance: &MjcfProvenance,
    entry_file: &Path,
    source_root: &Path,
) -> Result<BundleReimportResult, BundleReimportError> {
    previous.validate().map_err(BundleReimportError::OldSnapshot)?;
    previous_provenance.validate_included_elements(previous)
        .map_err(BundleReimportError::OldProvenance)?;
    let imported = from_mjcf_file(entry_file).map_err(BundleReimportError::Import)?;
    let mut portable = PortableMjcfBundle::from_bundle(&imported, source_root)
        .map_err(BundleReimportError::Portable)?;
    // Prevent accidentally replacing one entry document with another.
    if portable.entry_path != previous.entry_path {
        return Err(BundleReimportError::Portable(PortableError::InvalidPath(portable.entry_path)));
    }
    let plan = plan_named_identity_reuse(&previous.model, &portable.model)
        .map_err(BundleReimportError::Reconcile)?;
    portable.model = apply_identity_plan(&previous.model, &portable.model, &plan)
        .map_err(BundleReimportError::Apply)?;
    portable.model.model_id = previous.model.model_id.clone();
    portable.validate().map_err(BundleReimportError::Portable)?;
    let provenance = map_included_elements(&portable).map_err(BundleReimportError::Provenance)?;
    provenance.validate_included_elements(&portable)
        .map_err(BundleReimportError::Provenance)?;
    Ok(BundleReimportResult { bundle: portable, provenance, plan })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mjcf_model_bundle::from_mjcf_file, model::TransmissionTarget};
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!("mjcf-reimport-{}", Uuid::new_v4()));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn put(&self, path: &str, xml: &str) {
            let p = self.0.join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, xml).unwrap();
        }
        fn snapshot(&self) -> (PortableMjcfBundle, MjcfProvenance) {
            let imported = from_mjcf_file(&self.0.join("main.xml")).unwrap();
            let portable = PortableMjcfBundle::from_bundle(&imported, &self.0).unwrap();
            let provenance = map_included_elements(&portable).unwrap();
            (portable, provenance)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    #[test]
    fn nested_include_reimport_preserves_named_ids_and_refreshes_provenance() {
        let f = Fixture::new();
        f.put("main.xml", r#"<mujoco><include file="parts/a.xml"/><actuator><motor name="drive" joint="j"/></actuator></mujoco>"#);
        f.put("parts/a.xml", r#"<mujocoinclude><include file="b.xml"/></mujocoinclude>"#);
        f.put("parts/b.xml", r#"<mujocoinclude><worldbody><body name="b"><joint name="j"/></body><body/></worldbody></mujocoinclude>"#);
        let (previous, provenance) = f.snapshot();
        f.put("parts/b.xml", r#"<mujocoinclude> <worldbody><body name="b"><joint name="j"/></body><body/><body name="added"/></worldbody></mujocoinclude>"#);
        let result = reimport_portable_mjcf_with_identity(&previous, &provenance, &f.0.join("main.xml"), &f.0).unwrap();
        assert_eq!(result.bundle.model.model_id, previous.model.model_id);
        assert_eq!(result.bundle.model.bodies[0].id, previous.model.bodies[0].id);
        assert_eq!(result.bundle.model.joints[0].id, previous.model.joints[0].id);
        assert_eq!(result.bundle.model.actuators[0].id, previous.model.actuators[0].id);
        assert_ne!(result.bundle.model.bodies[1].id, previous.model.bodies[1].id);
        assert_eq!(result.plan.reuse.len(), 3);
        assert_eq!(result.plan.unresolved.len(), 2);
        assert!(matches!(&result.bundle.model.actuators[0].target, TransmissionTarget::Joint(id) if id == &result.bundle.model.joints[0].id));
        result.provenance.validate_included_elements(&result.bundle).unwrap();
        assert_ne!(result.provenance.origins[&previous.model.bodies[0].id].byte_offset, provenance.origins[&previous.model.bodies[0].id].byte_offset);
        assert_eq!(previous.sources.len(), 3);
    }

    #[test]
    fn invalid_updated_include_does_not_mutate_previous_snapshot() {
        let f = Fixture::new();
        f.put("main.xml", r#"<mujoco><include file="parts.xml"/></mujoco>"#);
        f.put("parts.xml", r#"<mujocoinclude><worldbody><body name="b"/></worldbody></mujocoinclude>"#);
        let (previous, provenance) = f.snapshot();
        let saved = previous.clone();
        f.put("parts.xml", "<mujocoinclude>");
        assert!(matches!(reimport_portable_mjcf_with_identity(&previous, &provenance, &f.0.join("main.xml"), &f.0), Err(BundleReimportError::Import(_))));
        assert_eq!(previous, saved);
    }

    #[test]
    fn tampered_old_provenance_is_rejected() {
        let f = Fixture::new();
        f.put("main.xml", r#"<mujoco><worldbody><body name="b"/></worldbody></mujoco>"#);
        let (previous, mut provenance) = f.snapshot();
        provenance.origins.clear();
        assert!(matches!(reimport_portable_mjcf_with_identity(&previous, &provenance, &f.0.join("main.xml"), &f.0), Err(BundleReimportError::OldProvenance(_))));
    }
}
