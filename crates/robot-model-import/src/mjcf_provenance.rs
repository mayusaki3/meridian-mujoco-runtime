//! Read-only source provenance for explicitly named MJCF domain elements.
//! This intentionally rejects ambiguous/unnamed elements instead of guessing.
use crate::{mjcf_portable::PortableMjcfBundle, model::RobotModel};
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, HashMap, HashSet}, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementOrigin {
    pub source_path: PathBuf,
    pub element_tag: String,
    pub source_name: String,
    /// Byte offset of the opening element in the *original* source XML.
    pub byte_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MjcfProvenance {
    pub model_id: String,
    pub origins: BTreeMap<String, ElementOrigin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceError {
    InvalidXml(PathBuf),
    UnnamedElement(String),
    DuplicateSourceName { kind: String, name: String },
    MissingSourceElement { kind: String, name: String },
    AmbiguousSourceElement { kind: String, name: String },
    InvalidModelId,
}

fn collect_source_elements(bundle: &PortableMjcfBundle) -> Result<HashMap<(String, String), Vec<ElementOrigin>>, ProvenanceError> {
    let mut result: HashMap<(String, String), Vec<ElementOrigin>> = HashMap::new();
    for (path, xml) in &bundle.sources {
        let doc = roxmltree::Document::parse(xml).map_err(|_| ProvenanceError::InvalidXml(path.clone()))?;
        for node in doc.root_element().descendants().filter(|n| n.is_element() && n.tag_name().namespace().is_none()) {
            let kind = if node.has_tag_name("body") && node.ancestors().any(|a| a.has_tag_name("worldbody")) {
                "body"
            } else if node.has_tag_name("joint") && node.parent().is_some_and(|p| p.has_tag_name("body")) && node.ancestors().any(|a| a.has_tag_name("worldbody")) {
                "joint"
            } else if node.parent().is_some_and(|p| p.has_tag_name("tendon")) {
                "tendon"
            } else if node.parent().is_some_and(|p| p.has_tag_name("actuator")) {
                "actuator"
            } else { continue };
            if let Some(name) = node.attribute("name") {
                result.entry((kind.to_owned(), name.to_owned())).or_default().push(ElementOrigin {
                    source_path: path.clone(),
                    element_tag: node.tag_name().name().to_owned(),
                    source_name: name.to_owned(),
                    byte_offset: node.range().start,
                });
            }
        }
    }
    Ok(result)
}

/// Build UUID -> original source file/element location for named elements.
/// Unnamed elements cannot yet be mapped reliably across include expansion.
pub fn map_named_elements(bundle: &PortableMjcfBundle) -> Result<MjcfProvenance, ProvenanceError> {
    let sources = collect_source_elements(bundle)?;
    let model: &RobotModel = &bundle.model;
    let mut origins = BTreeMap::new();
    let mut seen = HashSet::new();
    let mut bind = |id: &str, kind: &str, name: &str| -> Result<(), ProvenanceError> {
        if !seen.insert((kind.to_owned(), name.to_owned())) {
            return Err(ProvenanceError::DuplicateSourceName { kind: kind.to_owned(), name: name.to_owned() });
        }
        let matches = sources.get(&(kind.to_owned(), name.to_owned()))
            .ok_or_else(|| ProvenanceError::MissingSourceElement { kind: kind.to_owned(), name: name.to_owned() })?;
        if matches.len() != 1 {
            return Err(ProvenanceError::AmbiguousSourceElement { kind: kind.to_owned(), name: name.to_owned() });
        }
        origins.insert(id.to_owned(), matches[0].clone());
        Ok(())
    };
    for body in &model.bodies {
        // Synthetic @offset names are generated for unnamed bodies.
        if body.name.starts_with('@') { return Err(ProvenanceError::UnnamedElement("body".into())); }
        bind(&body.id, "body", &body.name)?;
    }
    for joint in &model.joints {
        let name = joint.name.as_deref().ok_or_else(|| ProvenanceError::UnnamedElement("joint".into()))?;
        bind(&joint.id, "joint", name)?;
    }
    for actuator in &model.actuators {
        let name = actuator.name.as_deref().ok_or_else(|| ProvenanceError::UnnamedElement("actuator".into()))?;
        bind(&actuator.id, "actuator", name)?;
    }
    for tendon in &model.tendons { bind(&tendon.id, "tendon", &tendon.name)?; }
    Ok(MjcfProvenance { model_id: model.model_id.clone(), origins })
}

impl MjcfProvenance {

    /// Verify that each stored byte offset still points to the same named
    /// XML element in the original source. This is a guard, not an edit API.
    pub fn validate_source_locations(&self, bundle: &PortableMjcfBundle) -> Result<(), ProvenanceError> {
        self.validate_against(bundle)?;
        for origin in self.origins.values() {
            let xml = bundle.sources.get(&origin.source_path)
                .ok_or_else(|| ProvenanceError::InvalidXml(origin.source_path.clone()))?;
            let doc = roxmltree::Document::parse(xml)
                .map_err(|_| ProvenanceError::InvalidXml(origin.source_path.clone()))?;
            let found = doc.descendants().any(|node|
                node.is_element()
                && node.range().start == origin.byte_offset
                && node.tag_name().name() == origin.element_tag
                && node.attribute("name") == Some(origin.source_name.as_str())
            );
            if !found { return Err(ProvenanceError::InvalidModelId); }
        }
        Ok(())
    }
    pub fn validate_against(&self, bundle: &PortableMjcfBundle) -> Result<(), ProvenanceError> {
        if self.model_id != bundle.model.model_id { return Err(ProvenanceError::InvalidModelId); }
        let expected = map_named_elements(bundle)?;
        if self != &expected { return Err(ProvenanceError::InvalidModelId); }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mjcf_model_bundle::from_mjcf_file, mjcf_portable::PortableMjcfBundle};
    use std::{fs, path::Path};
    use uuid::Uuid;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!("mjcf-origin-{}", Uuid::new_v4()));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn put(&self, name: &str, xml: &str) {
            let path = self.0.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, xml).unwrap();
        }
        fn portable(&self) -> PortableMjcfBundle {
            let bundle = from_mjcf_file(&self.0.join("main.xml")).unwrap();
            PortableMjcfBundle::from_bundle(&bundle, &self.0).unwrap()
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

    #[test]
    fn maps_named_elements_across_nested_include_and_json() {
        let f = Fixture::new();
        f.put("main.xml", r#"<mujoco><include file="parts.xml"/><actuator><motor name="drive" joint="hinge"/></actuator></mujoco>"#);
        f.put("parts.xml", r#"<mujocoinclude><worldbody><body name="arm"><joint name="hinge"/></body></worldbody><tendon><fixed name="rope"/></tendon></mujocoinclude>"#);
        let portable = f.portable();
        let provenance = map_named_elements(&portable).unwrap();
        assert_eq!(provenance.origins.len(), 4);
        assert_eq!(provenance.origins[&portable.model.joints[0].id].source_path, Path::new("parts.xml"));
        assert_eq!(provenance.origins[&portable.model.actuators[0].id].source_path, Path::new("main.xml"));
        let json = serde_json::to_string(&provenance).unwrap();
        let restored: MjcfProvenance = serde_json::from_str(&json).unwrap();
        restored.validate_against(&portable).unwrap();
    }

    #[test]
    fn source_locations_detect_xml_reordering_and_offset_tampering() {
        let f = Fixture::new();
        f.put("main.xml", r#"<mujoco><worldbody><body name="b"><joint name="j"/></body></worldbody></mujoco>"#);
        let portable = f.portable();
        let original = map_named_elements(&portable).unwrap();
        original.validate_source_locations(&portable).unwrap();
        let mut altered = original.clone();
        altered.origins.get_mut(&portable.model.joints[0].id).unwrap().byte_offset += 1;
        assert!(altered.validate_source_locations(&portable).is_err());
        let mut changed = portable.clone();
        changed.sources.insert(PathBuf::from("main.xml"),
            r#"<mujoco><worldbody> <body name="b"><joint name="j"/></body></worldbody></mujoco>"#.into());
        assert!(original.validate_source_locations(&changed).is_err());
    }

    #[test]
    fn rejects_unnamed_joint_instead_of_guessing() {
        let f = Fixture::new();
        f.put("main.xml", r#"<mujoco><worldbody><body name="b"><joint/></body></worldbody></mujoco>"#);
        assert!(matches!(map_named_elements(&f.portable()), Err(ProvenanceError::UnnamedElement(_))));
    }

    #[test]
    fn rejects_ambiguous_source_element() {
        let f = Fixture::new();
        f.put("main.xml", r#"<mujoco><worldbody><body name="b"><joint name="j"/></body></worldbody><default><joint name="j"/></default></mujoco>"#);
        // A default joint is not a domain joint and must not cause a false match.
        let p = f.portable();
        assert!(map_named_elements(&p).is_ok());
        let mut altered = p.clone();
        altered.sources.insert(PathBuf::from("extra.xml"), r#"<mujocoinclude><worldbody><body name="b"/></worldbody></mujocoinclude>"#.into());
        assert!(matches!(map_named_elements(&altered), Err(ProvenanceError::AmbiguousSourceElement { .. })));
    }
}
