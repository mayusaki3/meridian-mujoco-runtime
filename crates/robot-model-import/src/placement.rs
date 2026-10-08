//! Structured placement snapshot for unknown URDF XML data.
//! Read-only metadata: reinsertion into a regenerated document is not yet supported.
use crate::source_mapping::{MappingError, SourceMappingDocument, SourceOwner};
use crate::source_element_graph::SourceElementGraph;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_key: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_sibling_key: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_sibling_key: Option<Uuid>,
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
                placement: Placement { parent_path: path.clone(), sibling_index: None, parent_key: None, previous_sibling_key: None, next_sibling_key: None },
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
                placement: Placement { parent_path: path.clone(), sibling_index: Some(index), parent_key: None, previous_sibling_key: None, next_sibling_key: None },
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



    /// Build placement with element keys shared with a previously constructed
    /// source graph. Unknown element keys are the graph's own keys; unknown
    /// attributes retain independent keys and anchor to their containing element.
    pub fn from_source_with_graph(
        xml: &str,
        mapping: &SourceMappingDocument,
        graph: &SourceElementGraph,
    ) -> Result<Self, MappingError> {
        graph.validate()?;
        let doc = Document::parse(xml).map_err(|e| MappingError::Json(e.to_string()))?;
        let root = doc.root_element();
        if root.tag_name().name() != "robot" || root.tag_name().namespace().is_some()
            || root.attribute("name") != Some(mapping.robot_name.as_str()) {
            return Err(MappingError::UnknownOwner("source robot mismatch".into()));
        }
        let mut nodes = std::collections::HashMap::new();
        fn index_nodes<'a, 'input>(
            node: Node<'a, 'input>, path: Vec<usize>,
            result: &mut std::collections::HashMap<Vec<usize>, Node<'a, 'input>>,
        ) {
            result.insert(path.clone(), node);
            for (i, child) in node.children().filter(|n| n.is_element()).enumerate() {
                let mut child_path = path.clone();
                child_path.push(i);
                index_nodes(child, child_path, result);
            }
        }
        index_nodes(root, Vec::new(), &mut nodes);
        if graph.elements.len() != nodes.len() {
            return Err(MappingError::Json("source graph element count mismatch".into()));
        }
        let mut by_path = std::collections::HashMap::new();
        for element in &graph.elements {
            let node = nodes.get(&element.path)
                .ok_or_else(|| MappingError::Json("graph path missing in source".into()))?;
            if node.tag_name().name() != element.local_name
                || node.tag_name().namespace() != element.namespace_uri.as_deref()
                || node.attribute("name") != element.name_attribute.as_deref()
                || owner_for(*node, mapping)? != element.owner {
                return Err(MappingError::Json("graph source identity mismatch".into()));
            }
            by_path.insert(element.path.clone(), element);
        }
        let mut snapshot = Self::from_source(xml, mapping)?;
        for item in &mut snapshot.items {
            let parent = by_path.get(&item.placement.parent_path)
                .ok_or_else(|| MappingError::Json("placement parent missing".into()))?;
            item.placement.parent_key = Some(parent.key);
            if let Some(index) = item.placement.sibling_index {
                let mut path = item.placement.parent_path.clone();
                path.push(index);
                let element = by_path.get(&path)
                    .ok_or_else(|| MappingError::Json("preserved element missing".into()))?;
                item.source_key = element.key;
                item.placement.previous_sibling_key = element.previous_sibling_key;
                item.placement.next_sibling_key = element.next_sibling_key;
            }
        }
        Ok(snapshot)
    }

    /// Validate a snapshot against its source. Never infer a new position when
    /// the original element index or parent path no longer matches.
    pub fn validate_against_source(
        &self,
        xml: &str,
        mapping: &SourceMappingDocument,
    ) -> Result<(), MappingError> {
        if self.schema_version != 1 {
            return Err(MappingError::UnsupportedSchema(self.schema_version));
        }
        let current = Self::from_source(xml, mapping)?;
        if self.items.len() != current.items.len() {
            return Err(MappingError::Json("placement item count changed".into()));
        }
        let mut keys = std::collections::HashSet::new();
        for (saved, observed) in self.items.iter().zip(&current.items) {
            if !keys.insert(saved.source_key) {
                return Err(MappingError::Json("duplicate source key".into()));
            }
            if saved.owner != observed.owner
                || saved.placement != observed.placement
                || saved.payload != observed.payload
            {
                return Err(MappingError::Json("placement or preserved payload changed".into()));
            }
        }
        Ok(())
    }


    /// Validate persisted, keyed placement against the exact source graph.
    /// No key regeneration or implicit reanchoring is allowed.
    pub fn validate_with_graph(
        &self,
        xml: &str,
        mapping: &SourceMappingDocument,
        graph: &SourceElementGraph,
    ) -> Result<(), MappingError> {
        if self.schema_version != 1 {
            return Err(MappingError::UnsupportedSchema(self.schema_version));
        }
        graph.validate()?;
        let current = Self::from_source_with_graph(xml, mapping, graph)?;
        if self.items.len() != current.items.len() {
            return Err(MappingError::Json("placement item count changed".into()));
        }
        let mut keys = std::collections::HashSet::new();
        let graph_keys: std::collections::HashSet<_> =
            graph.elements.iter().map(|e| e.key).collect();
        for (saved, observed) in self.items.iter().zip(&current.items) {
            if !keys.insert(saved.source_key) {
                return Err(MappingError::Json("duplicate preserved source key".into()));
            }
            if saved.owner != observed.owner
                || saved.placement != observed.placement
                || saved.payload != observed.payload {
                return Err(MappingError::Json("preserved placement or anchor conflict".into()));
            }
            match &saved.payload {
                PreservedPayload::Element { .. } => {
                    if saved.source_key != observed.source_key
                        || !graph_keys.contains(&saved.source_key) {
                        return Err(MappingError::Json("preserved element key conflict".into()));
                    }
                }
                PreservedPayload::Attribute { .. } => {
                    if graph_keys.contains(&saved.source_key) {
                        return Err(MappingError::Json("attribute key collides with element".into()));
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
    #[test]
    fn validation_accepts_original_source() {
        let xml = r#"<robot name="r"><link name="a"><visual/><extra/></link></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let snapshot = PlacementSnapshot::from_source(xml, &mapping).unwrap();
        assert!(snapshot.validate_against_source(xml, &mapping).is_ok());
    }

    #[test]
    fn validation_rejects_shifted_sibling() {
        let xml = r#"<robot name="r"><link name="a"><visual/><extra/></link></robot>"#;
        let changed = r#"<robot name="r"><link name="a"><extra/><visual/></link></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let snapshot = PlacementSnapshot::from_source(xml, &mapping).unwrap();
        assert!(snapshot.validate_against_source(changed, &mapping).is_err());
    }

    #[test]
    fn validation_rejects_modified_payload_and_duplicate_keys() {
        let xml = r#"<robot name="r"><link name="a"><extra x="1"/><extra y="2"/></link></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let snapshot = PlacementSnapshot::from_source(xml, &mapping).unwrap();
        let changed = xml.replace("x=\"1\"", "x=\"3\"");
        assert!(snapshot.validate_against_source(&changed, &mapping).is_err());
        let mut duplicated = snapshot.clone();
        duplicated.items[1].source_key = duplicated.items[0].source_key;
        assert!(duplicated.validate_against_source(xml, &mapping).is_err());
    }

    #[test]
    fn graph_and_placement_share_unknown_element_keys() {
        let xml = r#"<robot name="r"><link name="a"><visual/><vendor x="1"/></link></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let graph = SourceElementGraph::from_source(xml, &mapping).unwrap();
        let snapshot = PlacementSnapshot::from_source_with_graph(xml, &mapping, &graph).unwrap();
        let vendor = graph.elements.iter().find(|e| e.local_name == "vendor").unwrap();
        let item = snapshot.items.iter().find(|i| matches!(&i.payload, PreservedPayload::Element { xml } if xml.contains("<vendor"))).unwrap();
        assert_eq!(item.source_key, vendor.key);
        assert_eq!(item.placement.parent_key, vendor.parent_key);
        assert_eq!(item.placement.previous_sibling_key, vendor.previous_sibling_key);
        assert_eq!(PlacementSnapshot::from_json(&snapshot.to_json().unwrap()).unwrap(), snapshot);
    }

    #[test]
    fn graph_mismatch_is_rejected() {
        let xml = r#"<robot name="r"><link name="a"><vendor/></link></robot>"#;
        let changed = r#"<robot name="r"><link name="a"><other/></link></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let graph = SourceElementGraph::from_source(xml, &mapping).unwrap();
        assert!(PlacementSnapshot::from_source_with_graph(changed, &mapping, &graph).is_err());
    }

    #[test]
    fn keyed_validation_accepts_original_and_json_roundtrip() {
        let xml = r#"<robot name="r" custom="x"><link name="a"><visual/><vendor/></link></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let graph = SourceElementGraph::from_source(xml, &mapping).unwrap();
        let snapshot = PlacementSnapshot::from_source_with_graph(xml, &mapping, &graph).unwrap();
        let restored = PlacementSnapshot::from_json(&snapshot.to_json().unwrap()).unwrap();
        restored.validate_with_graph(xml, &mapping, &graph).unwrap();
    }

    #[test]
    fn keyed_validation_rejects_modified_parent_and_sibling_anchors() {
        let xml = r#"<robot name="r"><link name="a"><visual/><vendor/></link></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let graph = SourceElementGraph::from_source(xml, &mapping).unwrap();
        let snapshot = PlacementSnapshot::from_source_with_graph(xml, &mapping, &graph).unwrap();
        let mut bad_parent = snapshot.clone();
        bad_parent.items[0].placement.parent_key = Some(Uuid::new_v4());
        assert!(bad_parent.validate_with_graph(xml, &mapping, &graph).is_err());
        let mut bad_sibling = snapshot.clone();
        bad_sibling.items[0].placement.previous_sibling_key = None;
        assert!(bad_sibling.validate_with_graph(xml, &mapping, &graph).is_err());
    }

    #[test]
    fn keyed_validation_rejects_changed_element_key_and_source() {
        let xml = r#"<robot name="r"><link name="a"><visual/><vendor/></link></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let graph = SourceElementGraph::from_source(xml, &mapping).unwrap();
        let snapshot = PlacementSnapshot::from_source_with_graph(xml, &mapping, &graph).unwrap();
        let mut changed_key = snapshot.clone();
        changed_key.items[0].source_key = Uuid::new_v4();
        assert!(changed_key.validate_with_graph(xml, &mapping, &graph).is_err());
        let changed_xml = xml.replace("<vendor/>", "<other/>");
        assert!(snapshot.validate_with_graph(&changed_xml, &mapping, &graph).is_err());
    }

    #[test]
    fn keyed_validation_rejects_duplicate_attribute_key() {
        let xml = r#"<robot name="r" custom="x" other="y"/>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let graph = SourceElementGraph::from_source(xml, &mapping).unwrap();
        let mut snapshot = PlacementSnapshot::from_source_with_graph(xml, &mapping, &graph).unwrap();
        assert_eq!(snapshot.items.len(), 2);
        snapshot.items[1].source_key = snapshot.items[0].source_key;
        assert!(snapshot.validate_with_graph(xml, &mapping, &graph).is_err());
    }

}
