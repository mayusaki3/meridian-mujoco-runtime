//! Versioned identity mapping for preserved, source-owned data.
//! This is an intermediate model, not the final robot-model canonical schema.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RobotId(pub Uuid);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LinkId(pub Uuid);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JointId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum SourceOwner {
    Robot(RobotId),
    Link(LinkId),
    Joint(JointId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedLink {
    pub id: LinkId,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedJoint {
    pub id: JointId,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappedPreservedItem {
    pub owner: SourceOwner,
    pub xml: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMappingDocument {
    pub schema_version: u32,
    pub robot_id: RobotId,
    pub robot_name: String,
    pub links: Vec<NamedLink>,
    pub joints: Vec<NamedJoint>,
    pub preserved: Vec<MappedPreservedItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingError {
    UnknownOwner(String),
    DuplicateName(String),
    UnsupportedSchema(u32),
    Json(String),
}

impl SourceMappingDocument {
    /// Converts a fresh inspection into a domain-ID mapping.
    /// Reimport identity reconciliation is deliberately not implemented.
    pub fn from_inspection(input: &crate::InspectedUrdf) -> Result<Self, MappingError> {
        use std::collections::HashSet;
        let mut names = HashSet::new();
        for name in &input.links {
            if !names.insert(name.as_str()) {
                return Err(MappingError::DuplicateName(format!("link:{name}")));
            }
        }
        names.clear();
        for name in &input.joints {
            if !names.insert(name.as_str()) {
                return Err(MappingError::DuplicateName(format!("joint:{name}")));
            }
        }
        let robot_id = RobotId(Uuid::new_v4());
        let links: Vec<_> = input.links.iter().map(|name| NamedLink {
            id: LinkId(Uuid::new_v4()), name: name.clone(),
        }).collect();
        let joints: Vec<_> = input.joints.iter().map(|name| NamedJoint {
            id: JointId(Uuid::new_v4()), name: name.clone(),
        }).collect();
        let mut preserved = Vec::with_capacity(input.preserved.len());
        for item in &input.preserved {
            let owner = if item.owner == "robot" {
                SourceOwner::Robot(robot_id)
            } else if let Some(name) = item.owner.strip_prefix("link:") {
                let link = links.iter().find(|l| l.name == name)
                    .ok_or_else(|| MappingError::UnknownOwner(item.owner.clone()))?;
                SourceOwner::Link(link.id)
            } else if let Some(name) = item.owner.strip_prefix("joint:") {
                let joint = joints.iter().find(|j| j.name == name)
                    .ok_or_else(|| MappingError::UnknownOwner(item.owner.clone()))?;
                SourceOwner::Joint(joint.id)
            } else {
                return Err(MappingError::UnknownOwner(item.owner.clone()));
            };
            preserved.push(MappedPreservedItem { owner, xml: item.xml.clone() });
        }
        Ok(Self {
            schema_version: 1, robot_id, robot_name: input.robot_name.clone(),
            links, joints, preserved,
        })
    }

    pub fn to_json(&self) -> Result<String, MappingError> {
        serde_json::to_string_pretty(self).map_err(|e| MappingError::Json(e.to_string()))
    }

    pub fn from_json(json: &str) -> Result<Self, MappingError> {
        let document: Self = serde_json::from_str(json)
            .map_err(|e| MappingError::Json(e.to_string()))?;
        if document.schema_version != 1 {
            return Err(MappingError::UnsupportedSchema(document.schema_version));
        }
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extended() -> SourceMappingDocument {
        let xml = include_str!("../../../tests/fixtures/robot-model/unknown-extension.urdf");
        SourceMappingDocument::from_inspection(&crate::inspect_urdf(xml).unwrap()).unwrap()
    }

    #[test]
    fn preserved_items_bind_to_domain_ids() {
        let x = extended();
        let link_id = x.links.iter().find(|l| l.name == "base_link").unwrap().id;
        assert!(x.preserved.iter().any(|p|
            p.owner == SourceOwner::Link(link_id) && p.xml.contains("calibration")));
    }

    #[test]
    fn rename_does_not_change_preserved_owner() {
        let mut x = extended();
        let id = x.links[0].id;
        x.links[0].name = "renamed".into();
        assert!(x.preserved.iter().any(|p| p.owner == SourceOwner::Link(id)));
    }

    #[test]
    fn canonical_mapping_json_roundtrip() {
        let x = extended();
        let restored = SourceMappingDocument::from_json(&x.to_json().unwrap()).unwrap();
        assert_eq!(restored, x);
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let mut x = extended();
        x.schema_version = 2;
        assert_eq!(SourceMappingDocument::from_json(&x.to_json().unwrap()),
            Err(MappingError::UnsupportedSchema(2)));
    }

    #[test]
    fn rejects_duplicate_link_names() {
        let mut x = crate::inspect_urdf("<robot name=\"r\"><link name=\"a\"/><link name=\"a\"/></robot>").unwrap();
        assert!(matches!(SourceMappingDocument::from_inspection(&x),
            Err(MappingError::DuplicateName(_))));
        x.links.clear();
    }

    #[test]
    fn rejects_unmapped_owner() {
        let mut x = crate::inspect_urdf("<robot name=\"r\"/>").unwrap();
        x.preserved.push(crate::PreservedItem { owner: "link:missing".into(), xml: "x".into() });
        assert_eq!(SourceMappingDocument::from_inspection(&x),
            Err(MappingError::UnknownOwner("link:missing".into())));
    }
}
