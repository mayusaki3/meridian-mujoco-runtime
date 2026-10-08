//! Structured placement snapshot for unknown URDF XML data.
//! Read-only metadata: reinsertion into a regenerated document is not yet supported.
use crate::source_mapping::{MappingError, SourceMappingDocument, SourceOwner};
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PreservedPayload {
    Element { xml: String },
    Attribute { local_name: String, namespace_uri: Option<String>, value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    /// Element-child indices from the robot root to the containing element.
    /// [] is the robot root; [0] is its first element child.
    pub parent_path: Vec<usize>,
    /// Index among element children; None for an attribute on parent_path.
    pub sibling_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacedPreservedItem {
    pub source_key: Uuid,
    pub owner: SourceOwner,
    pub placement: Placement,
    pub payload: PreservedPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacementSnapshot {
    pub schema_version: u32,
    pub items: Vec<PlacedPreservedItem>,
}

fn owner_for(
    node: Node<'_, '_>,
    mapping: &SourceMappingDocument,
) -> Result<SourceOwner, MappingError> {
    for ancestor in node.ancestors() {
        if ancestor.tag_name().namespace().is_some() { continue; }
        match ancestor.tag_name().name() {
            "link" => {
                let name = ancestor.attribute("name").unwrap_or("");
                return mapping.links.iter().find(|v| v.name == name)
                    .map(|v| SourceOwner::Link(v.id))
                    .ok_or_else(|| MappingError::UnknownOwner(format!("link:{name}")));
            }
            "joint" => {
                let name = ancestor.attribute("name").unwrap_or("");
                return mapping.joints.iter().find(|v| v.name == name)
                    .map(|v| SourceOwner::Joint(v.id))
                    .ok_or_else(|| MappingError::UnknownOwner(format!("joint:{name}")));
            }
            "robot" => return Ok(SourceOwner::Robot(mapping.robot_id)),
            _ => {}
        }
    }
    Err(MappingError::UnknownOwner("outside robot".into()))
}

fn collect(
    node: Node<'_, '_>,
    path: Vec<usize>,
    xml: &str,
    mapping: &SourceMappingDocument,
    items: &mut Vec<PlacedPreservedItem>,
) -> Result<(), MappingError> {
    let owner = owner_for(node, mapping)?;
    for attr in node.attributes() {
        if attr.namespace().is_some() || !crate::known_attr(node.tag_name().name(), attr.name()) {
            items.push(PlacedPreservedItem {
                source_key: Uuid::new_v4(),
                owner: owner.clone(),
                placement: Placement { parent_path: path.clone(), sibling_index: None },
                payload: PreservedPayload::Attribute {
                    local_name: attr.name().into(),
                    namespace_uri: attr.namespace().map(str::to_owned),
                    value: attr.value().into(),
                },
            });
        }
    }
    for (index, child) in node.children().filter(|n| n.is_element()).enumerate() {
        if child.tag_name().namespace().is_some()
            || !crate::known_child(node.tag_name().name(), child.tag_name().name())
        {
            items.push(PlacedPreservedItem {
                source_key: Uuid::new_v4(),
                owner: owner.clone(),
                placement: Placement { parent_path: path.clone(), sibling_index: Some(index) },
                payload: PreservedPayload::Element { xml: xml[child.range()].into() },
            });
        } else {
            let mut child_path = path.clone();
            child_path.push(index);
            collect(child, child_path, xml, mapping, items)?;
        }
    }
    Ok(())
}

impl PlacementSnapshot {
    pub fn from_source(xml: &str, mapping: &SourceMappingDocument) -> Result<Self, MappingError> {
        if mapping.schema_version != 1 {
            return Err(MappingError::UnsupportedSchema(mapping.schema_version));
        }
        let doc = Document::parse(xml).map_err(|e| MappingError::Json(e.to_string()))?;
        let root = doc.root_element();
        if root.tag_name().name() != "robot" || root.tag_name().namespace().is_some()
            || root.attribute("name") != Some(mapping.robot_name.as_str())
        {
            return Err(MappingError::UnknownOwner("source robot mismatch".into()));
        }
        let mut items = Vec::new();
        collect(root, Vec::new(), xml, mapping, &mut items)?;
        Ok(Self { schema_version: 1, items })
    }

    pub fn to_json(&self) -> Result<String, MappingError> {
        serde_json::to_string_pretty(self).map_err(|e| MappingError::Json(e.to_string()))
    }

    pub fn from_json(json: &str) -> Result<Self, MappingError> {
        let snapshot: Self = serde_json::from_str(json)
            .map_err(|e| MappingError::Json(e.to_string()))?;
        if snapshot.schema_version != 1 {
            return Err(MappingError::UnsupportedSchema(snapshot.schema_version));
        }
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspect_urdf;

    #[test]
    fn unknown_siblings_keep_original_element_positions() {
        let xml = include_str!("../../../tests/fixtures/robot-model/mixed-order.urdf");
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let snapshot = PlacementSnapshot::from_source(xml, &mapping).unwrap();
        let between: Vec<_> = snapshot.items.iter().filter(|item|
            matches!(&item.payload, PreservedPayload::Element { xml } if xml.contains("vendor:between"))
        ).collect();
        assert_eq!(between.len(), 2);
        assert_eq!(between[0].placement.parent_path, between[1].placement.parent_path);
        assert!(between[0].placement.sibling_index < between[1].placement.sibling_index);
    }

    #[test]
    fn namespaced_attributes_store_uri_and_value() {
        let xml = include_str!("../../../tests/fixtures/robot-model/unknown-extension.urdf");
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let snapshot = PlacementSnapshot::from_source(xml, &mapping).unwrap();
        assert!(snapshot.items.iter().any(|item|
            matches!(&item.payload, PreservedPayload::Attribute { namespace_uri: Some(uri), .. }
                if uri == "urn:meridian:test:vendor")));
    }

    #[test]
    fn owner_ids_survive_rename_and_json_roundtrip() {
        let xml = include_str!("../../../tests/fixtures/robot-model/unknown-extension.urdf");
        let mut mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let snapshot = PlacementSnapshot::from_source(xml, &mapping).unwrap();
        let link_id = mapping.links[0].id;
        mapping.links[0].name = "renamed".into();
        let restored = PlacementSnapshot::from_json(&snapshot.to_json().unwrap()).unwrap();
        assert_eq!(restored, snapshot);
        assert!(restored.items.iter().any(|item| item.owner == SourceOwner::Link(link_id)));
    }

    #[test]
    fn rejects_wrong_source_robot() {
        let xml = r#"<robot name="r"/>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        assert!(PlacementSnapshot::from_source(r#"<robot name="other"/>"#, &mapping).is_err());
    }

    #[test]
    fn rejects_unsupported_snapshot_version() {
        let snapshot = PlacementSnapshot { schema_version: 2, items: Vec::new() };
        assert_eq!(PlacementSnapshot::from_json(&snapshot.to_json().unwrap()),
            Err(MappingError::UnsupportedSchema(2)));
    }
}
