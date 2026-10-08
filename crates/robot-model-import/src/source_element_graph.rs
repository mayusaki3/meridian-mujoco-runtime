//! Source element identity graph. A fresh import creates keys; persisted keys
//! must be reconciled explicitly before using them after structural edits.
use crate::source_mapping::{MappingError, SourceMappingDocument, SourceOwner};
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceElement {
    pub key: Uuid,
    pub owner: SourceOwner,
    pub parent_key: Option<Uuid>,
    pub previous_sibling_key: Option<Uuid>,
    pub next_sibling_key: Option<Uuid>,
    pub path: Vec<usize>,
    pub local_name: String,
    pub namespace_uri: Option<String>,
    pub name_attribute: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceElementGraph {
    pub schema_version: u32,
    pub elements: Vec<SourceElement>,
}

fn owner(node: Node<'_, '_>, mapping: &SourceMappingDocument) -> Result<SourceOwner, MappingError> {
    for ancestor in node.ancestors().filter(|n| n.is_element()) {
        if ancestor.tag_name().namespace().is_some() { continue; }
        match ancestor.tag_name().name() {
            "link" => {
                let name = ancestor.attribute("name").unwrap_or("");
                return mapping.links.iter().find(|l| l.name == name)
                    .map(|l| SourceOwner::Link(l.id))
                    .ok_or_else(|| MappingError::UnknownOwner(format!("link:{name}")));
            }
            "joint" => {
                let name = ancestor.attribute("name").unwrap_or("");
                return mapping.joints.iter().find(|j| j.name == name)
                    .map(|j| SourceOwner::Joint(j.id))
                    .ok_or_else(|| MappingError::UnknownOwner(format!("joint:{name}")));
            }
            "robot" => return Ok(SourceOwner::Robot(mapping.robot_id)),
            _ => {}
        }
    }
    Err(MappingError::UnknownOwner("outside robot".into()))
}

fn visit(
    node: Node<'_, '_>, path: Vec<usize>, parent_key: Option<Uuid>,
    mapping: &SourceMappingDocument, elements: &mut Vec<SourceElement>,
) -> Result<(), MappingError> {
    let key = Uuid::new_v4();
    elements.push(SourceElement {
        key, owner: owner(node, mapping)?, parent_key,
        previous_sibling_key: None, next_sibling_key: None,
        path: path.clone(), local_name: node.tag_name().name().into(),
        namespace_uri: node.tag_name().namespace().map(str::to_owned),
        name_attribute: node.attribute("name").map(str::to_owned),
    });
    let mut sibling_keys = Vec::new();
    for (index, child) in node.children().filter(|n| n.is_element()).enumerate() {
        let mut child_path = path.clone();
        child_path.push(index);
        let offset = elements.len();
        visit(child, child_path, Some(key), mapping, elements)?;
        sibling_keys.push((offset, elements[offset].key));
    }
    for (i, (offset, _)) in sibling_keys.iter().enumerate() {
        if i > 0 { elements[*offset].previous_sibling_key = Some(sibling_keys[i - 1].1); }
        if i + 1 < sibling_keys.len() {
            elements[*offset].next_sibling_key = Some(sibling_keys[i + 1].1);
        }
    }
    Ok(())
}

impl SourceElementGraph {
    pub fn from_source(xml: &str, mapping: &SourceMappingDocument) -> Result<Self, MappingError> {
        if mapping.schema_version != 1 {
            return Err(MappingError::UnsupportedSchema(mapping.schema_version));
        }
        let doc = Document::parse(xml).map_err(|e| MappingError::Json(e.to_string()))?;
        let root = doc.root_element();
        if root.tag_name().name() != "robot" || root.tag_name().namespace().is_some()
            || root.attribute("name") != Some(mapping.robot_name.as_str()) {
            return Err(MappingError::UnknownOwner("source robot mismatch".into()));
        }
        let mut elements = Vec::new();
        visit(root, Vec::new(), None, mapping, &mut elements)?;
        Ok(Self { schema_version: 1, elements })
    }

    /// Reject missing anchors, duplicate keys, broken sibling chains and
    /// parents that do not contain their children. Does not relocate elements.
    pub fn validate(&self) -> Result<(), MappingError> {
        if self.schema_version != 1 {
            return Err(MappingError::UnsupportedSchema(self.schema_version));
        }
        let mut by_key = HashMap::new();
        let mut paths = HashSet::new();
        for e in &self.elements {
            if by_key.insert(e.key, e).is_some() || !paths.insert(&e.path) {
                return Err(MappingError::Json("duplicate element key or path".into()));
            }
        }
        if self.elements.iter().filter(|e| e.parent_key.is_none()).count() != 1 {
            return Err(MappingError::Json("expected one root".into()));
        }
        for e in &self.elements {
            if let Some(parent_key) = e.parent_key {
                let parent = by_key.get(&parent_key)
                    .ok_or_else(|| MappingError::Json("missing parent anchor".into()))?;
                if e.path.len() != parent.path.len() + 1 || !e.path.starts_with(&parent.path) {
                    return Err(MappingError::Json("parent path mismatch".into()));
                }
            } else if !e.path.is_empty() {
                return Err(MappingError::Json("invalid root path".into()));
            }
            for (anchor, is_previous) in [(e.previous_sibling_key, true), (e.next_sibling_key, false)] {
                if let Some(key) = anchor {
                    let sibling = by_key.get(&key)
                        .ok_or_else(|| MappingError::Json("missing sibling anchor".into()))?;
                    if sibling.parent_key != e.parent_key || sibling.key == e.key
                        || sibling.path.len() != e.path.len() || e.path.is_empty() {
                        return Err(MappingError::Json("invalid sibling anchor".into()));
                    }
                    let own_index = e.path[e.path.len() - 1];
                    let sibling_index = sibling.path[sibling.path.len() - 1];
                    if (is_previous && sibling_index.checked_add(1) != Some(own_index))
                        || (!is_previous && own_index.checked_add(1) != Some(sibling_index))
                        || (is_previous && sibling.next_sibling_key != Some(e.key))
                        || (!is_previous && sibling.previous_sibling_key != Some(e.key)) {
                        return Err(MappingError::Json("inconsistent sibling chain".into()));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, MappingError> {
        serde_json::to_string_pretty(self).map_err(|e| MappingError::Json(e.to_string()))
    }

    pub fn from_json(json: &str) -> Result<Self, MappingError> {
        let graph: Self = serde_json::from_str(json)
            .map_err(|e| MappingError::Json(e.to_string()))?;
        graph.validate()?;
        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspect_urdf;

    fn graph() -> SourceElementGraph {
        let xml = r#"<robot name="r"><link name="a"><visual/><vendor/></link><link name="b"/></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        SourceElementGraph::from_source(xml, &mapping).unwrap()
    }

    #[test]
    fn tracks_parent_and_adjacent_siblings() {
        let g = graph();
        g.validate().unwrap();
        let visual = g.elements.iter().find(|e| e.local_name == "visual").unwrap();
        let vendor = g.elements.iter().find(|e| e.local_name == "vendor").unwrap();
        assert_eq!(visual.next_sibling_key, Some(vendor.key));
        assert_eq!(vendor.previous_sibling_key, Some(visual.key));
        assert_eq!(visual.parent_key, vendor.parent_key);
    }

    #[test]
    fn json_roundtrip_preserves_keys() {
        let g = graph();
        assert_eq!(SourceElementGraph::from_json(&g.to_json().unwrap()).unwrap(), g);
    }

    #[test]
    fn missing_parent_is_rejected() {
        let mut g = graph();
        g.elements[1].parent_key = Some(Uuid::new_v4());
        assert!(g.validate().is_err());
    }

    #[test]
    fn missing_sibling_is_rejected() {
        let mut g = graph();
        let i = g.elements.iter().position(|e| e.local_name == "visual").unwrap();
        g.elements[i].next_sibling_key = Some(Uuid::new_v4());
        assert!(g.validate().is_err());
    }

    #[test]
    fn duplicate_key_is_rejected() {
        let mut g = graph();
        g.elements[1].key = g.elements[0].key;
        assert!(g.validate().is_err());
    }
}
