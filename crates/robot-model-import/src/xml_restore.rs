//! Fail-closed element-only XML restoration. Attributes and self-closing
//! insertion parents are intentionally unsupported until lossless handling exists.
use crate::placement::{PlacementError, PlacementSnapshot, PreservedPayload};
use crate::relocation::{plan_relocation, validate_relocation_plan};
use crate::source_element_graph::SourceElementGraph;
use roxmltree::{Document, Node};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

fn conflict(message: &str) -> PlacementError {
    PlacementError::Conflict(message.into())
}

fn node_at<'a, 'input>(root: Node<'a, 'input>, path: &[usize]) -> Option<Node<'a, 'input>> {
    let mut node = root;
    for index in path {
        node = node.children().filter(|n| n.is_element()).nth(*index)?;
    }
    Some(node)
}

/// Restore preserved elements into an edited XML document. The edited graph
/// must retain original keys for surviving elements and accurately describe
/// edited_xml. No XML is returned if any validation fails.
pub fn restore_elements(
    edited_xml: &str,
    snapshot: &PlacementSnapshot,
    original: &SourceElementGraph,
    edited: &SourceElementGraph,
) -> Result<String, PlacementError> {
    let plan = plan_relocation(snapshot, original, edited)?;
    validate_relocation_plan(snapshot, edited, &plan)?;
    if snapshot.items.iter().any(|i| !matches!(i.payload, PreservedPayload::Element { .. })) {
        return Err(conflict("attribute restoration not supported"));
    }
    let doc = Document::parse(edited_xml).map_err(|_| conflict("invalid edited XML"))?;
    let root = doc.root_element();
    if root.tag_name().name() != "robot" || root.tag_name().namespace().is_some() {
        return Err(conflict("edited XML root is not robot"));
    }
    let nodes: HashMap<Uuid, Node<'_, '_>> = edited.elements.iter().map(|element| {
        let node = node_at(root, &element.path).ok_or_else(|| conflict("edited graph path missing"))?;
        if node.tag_name().name() != element.local_name
            || node.tag_name().namespace() != element.namespace_uri.as_deref()
            || node.attribute("name") != element.name_attribute.as_deref() {
            return Err(conflict("edited graph does not match XML"));
        }
        Ok((element.key, node))
    }).collect::<Result<_, PlacementError>>()?;
    if nodes.len() != doc.descendants().filter(|n| n.is_element()).count() {
        return Err(conflict("edited graph element count mismatch"));
    }
    let old: HashMap<_, _> = original.elements.iter().map(|e| (e.key, e)).collect();
    let preserved: HashSet<_> = snapshot.items.iter().map(|i| i.source_key).collect();
    let mut insertions: HashMap<usize, Vec<(Vec<usize>, &str)>> = HashMap::new();
    for entry in &plan {
        let item = snapshot.items.iter().find(|i| i.source_key == entry.source_key)
            .ok_or_else(|| conflict("preserved item missing"))?;
        let PreservedPayload::Element { xml } = &item.payload else {
            return Err(conflict("non-element payload"));
        };
        let source = old.get(&entry.source_key).ok_or_else(|| conflict("source element missing"))?;
        // Walk right through a consecutive preserved run. The insertion point
        // is the first surviving sibling, or the parent's closing tag.
        let mut next = source.next_sibling_key;
        let mut visited = HashSet::new();
        while let Some(key) = next {
            if !visited.insert(key) { return Err(conflict("sibling cycle")); }
            if !preserved.contains(&key) { break; }
            next = old.get(&key).ok_or_else(|| conflict("source sibling missing"))?.next_sibling_key;
        }
        let parent = nodes.get(&entry.parent_key).ok_or_else(|| conflict("parent missing"))?;
        let position = if let Some(key) = next {
            let neighbor = nodes.get(&key).ok_or_else(|| conflict("surviving sibling missing"))?;
            if neighbor.parent().filter(|n| n.is_element()) != Some(*parent) {
                return Err(conflict("sibling parent mismatch"));
            }
            neighbor.range().start
        } else {
            // Locate the actual closing tag, rejecting self-closing parents.
            let raw = &edited_xml[parent.range()];
            let closing = format!("</{}", parent.tag_name().name());
            let relative = raw.rfind(&closing).ok_or_else(|| conflict("self-closing parent not supported"))?;
            parent.range().start + relative
        };
        insertions.entry(position).or_default().push((source.path.clone(), xml));
    }
    let mut positions: Vec<_> = insertions.into_iter().collect();
    positions.sort_by_key(|(position, _)| *position);
    let mut output = edited_xml.to_owned();
    for (position, mut fragments) in positions.into_iter().rev() {
        fragments.sort_by(|a, b| a.0.cmp(&b.0));
        let combined: String = fragments.into_iter().map(|(_, xml)| xml).collect();
        output.insert_str(position, &combined);
    }
    let result = Document::parse(&output).map_err(|_| conflict("restored XML invalid"))?;
    // Verify every preserved fragment occurs exactly once as an element's
    // original serialized subtree; no silent loss or duplication.
    for item in &snapshot.items {
        let PreservedPayload::Element { xml } = &item.payload else { unreachable!() };
        let count = result.descendants().filter(|n| n.is_element())
            .filter(|n| &output[n.range()] == xml).count();
        if count != 1 { return Err(conflict("restored fragment missing or duplicated")); }
    }
    // Verify placement against the edited graph after reparsing. Each
    // preserved subtree must occupy the expected child slot and retain its
    // original owner. Checking only serialized fragments is insufficient.
    let result_root = result.root_element();
    let mut expected: HashMap<Uuid, Vec<(usize, Uuid)>> = HashMap::new();
    for item in &snapshot.items {
        let element = old.get(&item.source_key).ok_or_else(|| conflict("source key absent"))?;
        let parent_key = element.parent_key.ok_or_else(|| conflict("source parent absent"))?;
        let sibling_index = element.path.last().copied().ok_or_else(|| conflict("source index absent"))?;
        expected.entry(parent_key).or_default().push((sibling_index, item.source_key));
    }
    for (parent_key, mut entries) in expected {
        entries.sort_by_key(|(index, _)| *index);
        let parent = nodes.get(&parent_key).ok_or_else(|| conflict("edited parent absent"))?;
        let output_parent = node_at(result_root, &edited.elements.iter()
            .find(|e| e.key == parent_key)
            .ok_or_else(|| conflict("parent graph key absent"))?.path)
            .ok_or_else(|| conflict("output parent absent"))?;
        if output_parent.tag_name() != parent.tag_name() {
            return Err(conflict("output parent changed"));
        }
        let saved_children: HashMap<_, _> = entries.iter().map(|(_, key)| {
            let item = snapshot.items.iter().find(|i| i.source_key == *key).unwrap();
            (*key, item)
        }).collect();
        let original_order: Vec<_> = original.elements.iter()
            .filter(|e| e.parent_key == Some(parent_key))
            .collect();
        let mut original_order = original_order;
        original_order.sort_by_key(|e| e.path.last().copied().unwrap_or(0));
        let mut expected_keys: Vec<_> = original_order.iter()
            .filter(|e| saved_children.contains_key(&e.key) || nodes.contains_key(&e.key))
            .map(|e| e.key).collect();
        // Newly added edited siblings are not represented in the original
        // graph. Current planner only permits insertion between adjacent
        // surviving anchors; reject rather than guessing their placement.
        let edited_children: HashSet<_> = edited.elements.iter()
            .filter(|e| e.parent_key == Some(parent_key))
            .map(|e| e.key).collect();
        if edited_children.iter().any(|key| !original_order.iter().any(|e| e.key == *key)) {
            return Err(conflict("new siblings require explicit reconciliation"));
        }
        let actual_children: Vec<_> = output_parent.children().filter(|n| n.is_element()).collect();
        if actual_children.len() != expected_keys.len() {
            return Err(conflict("restored sibling count mismatch"));
        }
        for (index, key) in expected_keys.drain(..).enumerate() {
            let child = actual_children[index];
            if let Some(item) = saved_children.get(&key) {
                let PreservedPayload::Element { xml } = &item.payload else { unreachable!() };
                if &output[child.range()] != xml {
                    return Err(conflict("restored sibling content or order mismatch"));
                }
                if item.owner != old[&key].owner || item.owner != edited.elements.iter()
                    .find(|e| e.key == parent_key)
                    .ok_or_else(|| conflict("edited owner absent"))?.owner {
                    return Err(conflict("restored owner mismatch"));
                }
            } else {
                let survivor = nodes.get(&key).ok_or_else(|| conflict("surviving sibling missing"))?;
                if child.tag_name() != survivor.tag_name()
                    || child.attribute("name") != survivor.attribute("name") {
                    return Err(conflict("surviving sibling order mismatch"));
                }
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{inspect_urdf, source_mapping::SourceMappingDocument};

    fn fixture() -> (PlacementSnapshot, SourceElementGraph, SourceElementGraph, String) {
        let source = r#"<robot name="r"><link name="a"><visual/><vendor_a/><vendor_b/><collision/></link></robot>"#;
        let edited_xml = r#"<robot name="r"><link name="a"><visual/><collision/></link></robot>"#.to_owned();
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(source).unwrap()).unwrap();
        let original = SourceElementGraph::from_source(source, &mapping).unwrap();
        let snapshot = PlacementSnapshot::from_source_with_graph(source, &mapping, &original).unwrap();
        let mut edited = original.clone();
        let preserved: HashSet<_> = snapshot.items.iter().map(|i| i.source_key).collect();
        edited.elements.retain(|e| !preserved.contains(&e.key));
        let visual = edited.elements.iter().position(|e| e.local_name == "visual").unwrap();
        let collision = edited.elements.iter().position(|e| e.local_name == "collision").unwrap();
        let left = edited.elements[visual].key;
        let right = edited.elements[collision].key;
        edited.elements[visual].next_sibling_key = Some(right);
        edited.elements[collision].previous_sibling_key = Some(left);
        *edited.elements[collision].path.last_mut().unwrap() = 1;
        (snapshot, original, edited, edited_xml)
    }

    #[test]
    fn restores_consecutive_unknown_elements_without_reordering() {
        let (snapshot, original, edited, xml) = fixture();
        let output = restore_elements(&xml, &snapshot, &original, &edited).unwrap();
        assert!(output.contains("<visual/><vendor_a/><vendor_b/><collision/>"));
    }

    #[test]
    fn rejects_xml_that_does_not_match_edited_graph() {
        let (snapshot, original, edited, xml) = fixture();
        let changed = xml.replace("<collision/>", "<inertial/>");
        assert!(restore_elements(&changed, &snapshot, &original, &edited).is_err());
    }

    #[test]
    fn rejects_unsupported_preserved_attributes() {
        let xml = r#"<robot name="r" vendor="x"/>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let original = SourceElementGraph::from_source(xml, &mapping).unwrap();
        let snapshot = PlacementSnapshot::from_source_with_graph(xml, &mapping, &original).unwrap();
        assert!(restore_elements(xml, &snapshot, &original, &original).is_err());
    }
    #[test]
    fn rejects_changed_owner_identity() {
        let (snapshot, original, mut edited, xml) = fixture();
        let link = edited.elements.iter().find(|e| e.local_name == "link").unwrap().owner.clone();
        let collision = edited.elements.iter_mut().find(|e| e.local_name == "collision").unwrap();
        assert!(matches!(link, crate::source_mapping::SourceOwner::Link(_)));
        collision.owner = crate::source_mapping::SourceOwner::Robot(edited_dummy_robot_id(&original));
        assert!(restore_elements(&xml, &snapshot, &original, &edited).is_err());
    }

    fn edited_dummy_robot_id(graph: &SourceElementGraph) -> crate::source_mapping::RobotId {
        match graph.elements[0].owner {
            crate::source_mapping::SourceOwner::Robot(id) => id,
            _ => unreachable!(),
        }
    }

    #[test]
    fn rejects_changed_parent_key() {
        let (snapshot, original, mut edited, xml) = fixture();
        let root_key = edited.elements[0].key;
        edited.elements.iter_mut().find(|e| e.local_name == "collision").unwrap().parent_key = Some(root_key);
        assert!(restore_elements(&xml, &snapshot, &original, &edited).is_err());
    }

    #[test]
    fn rejects_reordered_surviving_siblings() {
        let (snapshot, original, mut edited, xml) = fixture();
        let visual = edited.elements.iter().position(|e| e.local_name == "visual").unwrap();
        let collision = edited.elements.iter().position(|e| e.local_name == "collision").unwrap();
        edited.elements[visual].path[2] = 1;
        edited.elements[collision].path[2] = 0;
        let visual_key = edited.elements[visual].key;
        let collision_key = edited.elements[collision].key;
        edited.elements[visual].previous_sibling_key = Some(collision_key);
        edited.elements[visual].next_sibling_key = None;
        edited.elements[collision].previous_sibling_key = None;
        edited.elements[collision].next_sibling_key = Some(visual_key);
        assert!(restore_elements(&xml, &snapshot, &original, &edited).is_err());
    }

    #[test]
    fn rejects_duplicate_preserved_item() {
        let (mut snapshot, original, edited, xml) = fixture();
        snapshot.items.push(snapshot.items[0].clone());
        assert!(restore_elements(&xml, &snapshot, &original, &edited).is_err());
    }

    #[test]
    fn rejects_missing_preserved_item_in_group() {
        let (mut snapshot, original, edited, xml) = fixture();
        snapshot.items.pop();
        assert!(restore_elements(&xml, &snapshot, &original, &edited).is_err());
    }

    #[test]
    fn rejects_duplicate_fragment_already_in_edited_xml() {
        let (snapshot, original, edited, xml) = fixture();
        let xml = xml.replace("<collision/>", "<vendor_a/><collision/>");
        assert!(restore_elements(&xml, &snapshot, &original, &edited).is_err());
    }

}
