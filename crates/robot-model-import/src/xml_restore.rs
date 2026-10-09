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
    // Attributes are restored separately after element insertion. Namespaced
    // attributes remain fail-closed until prefix identity can be verified.
    if snapshot.items.iter().any(|i| matches!(&i.payload,
        PreservedPayload::Attribute { namespace_uri: Some(_), .. })) {
        return Err(conflict("namespaced attribute restoration not supported"));
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
    // Verify that graph identity, structural anchors and owner assignments
    // describe the actual edited XML, not just matching element names.
    let by_key: HashMap<_, _> = edited.elements.iter().map(|e| (e.key, e)).collect();
    for element in &edited.elements {
        let node = nodes[&element.key];
        if let Some(parent_key) = element.parent_key {
            let parent = nodes.get(&parent_key).ok_or_else(|| conflict("edited parent missing"))?;
            if node.parent().filter(|n| n.is_element()) != Some(*parent) {
                return Err(conflict("edited parent relationship mismatch"));
            }
            let parent_element = by_key[&parent_key];
            let expected_owner = if element.namespace_uri.is_none() && element.local_name == "link" {
                element.owner.clone()
            } else if element.namespace_uri.is_none() && element.local_name == "joint" {
                element.owner.clone()
            } else {
                parent_element.owner.clone()
            };
            if element.owner != expected_owner {
                return Err(conflict("edited owner relationship mismatch"));
            }
        }
        let actual_previous = node.prev_siblings().filter(|n| n.is_element()).nth(1);
        let actual_next = node.next_siblings().filter(|n| n.is_element()).nth(1);
        if actual_previous.map(|n| n.range().start) != element.previous_sibling_key
            .and_then(|k| nodes.get(&k).map(|n| n.range().start))
            || actual_next.map(|n| n.range().start) != element.next_sibling_key
                .and_then(|k| nodes.get(&k).map(|n| n.range().start)) {
            return Err(conflict("edited sibling anchors mismatch"));
        }
    }
    let old: HashMap<_, _> = original.elements.iter().map(|e| (e.key, e)).collect();
    let preserved: HashSet<_> = snapshot.items.iter().map(|i| i.source_key).collect();
    let mut insertions: HashMap<usize, Vec<(Vec<usize>, &str)>> = HashMap::new();
    for entry in &plan {
        let item = snapshot.items.iter().find(|i| i.source_key == entry.source_key)
            .ok_or_else(|| conflict("preserved item missing"))?;
        let PreservedPayload::Element { xml } = &item.payload else {
            continue;
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
    // Insert unqualified attributes at their keyed element's start tag.
    // XML escaping is explicit and no existing attribute may be overwritten.
    let mut attr_patches: Vec<(usize, String)> = Vec::new();
    {
        let intermediate = Document::parse(&output).map_err(|_| conflict("restored XML invalid"))?;
        let mut used = HashSet::new();
        for item in &snapshot.items {
            let PreservedPayload::Attribute { local_name, namespace_uri: None, value } = &item.payload else { continue };
            if !used.insert((item.placement.parent_key, local_name.as_str())) {
                return Err(conflict("duplicate preserved attribute"));
            }
            let parent_key = item.placement.parent_key.ok_or_else(|| conflict("attribute parent key absent"))?;
            let parent_graph = edited.elements.iter().find(|e| e.key == parent_key)
                .ok_or_else(|| conflict("attribute parent missing"))?;
            let node = node_at(intermediate.root_element(), &parent_graph.path)
                .ok_or_else(|| conflict("attribute parent path missing"))?;
            if node.attribute(local_name.as_str()).is_some() {
                return Err(conflict("attribute already exists"));
            }
            if !valid_xml_name(local_name) { return Err(conflict("invalid attribute name")); }
            let head = &output[node.range()];
            let opening_end = opening_tag_end(head).ok_or_else(|| conflict("invalid start tag"))?;
            let insertion = node.range().start + opening_end;
            let escaped = escape_attribute(value);
            attr_patches.push((insertion, format!(" {local_name}=\\\"{escaped}\\\"")));
        }
    }
    attr_patches.sort_by_key(|(position, _)| *position);
    for (position, fragment) in attr_patches.into_iter().rev() {
        output.insert_str(position, &fragment);
    }
    let result = Document::parse(&output).map_err(|_| conflict("restored XML invalid"))?;
    // Verify every preserved fragment occurs exactly once as an element's
    // original serialized subtree; no silent loss or duplication.
    for item in &snapshot.items {
        let PreservedPayload::Element { xml } = &item.payload else { continue };
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
        if !matches!(item.payload, PreservedPayload::Element { .. }) { continue; }
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
    // Attribute value and owner verification after reparsing.
    for item in &snapshot.items {
        let PreservedPayload::Attribute { local_name, namespace_uri: None, value } = &item.payload else { continue };
        let key = item.placement.parent_key.ok_or_else(|| conflict("attribute parent absent"))?;
        let parent = edited.elements.iter().find(|e| e.key == key)
            .ok_or_else(|| conflict("attribute parent graph absent"))?;
        let node = node_at(result_root, &parent.path).ok_or_else(|| conflict("attribute output parent absent"))?;
        if node.attribute(local_name.as_str()) != Some(value.as_str()) || item.owner != parent.owner {
            return Err(conflict("restored attribute value or owner mismatch"));
        }
    }
    Ok(output)
}

fn valid_xml_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        && !name.to_ascii_lowercase().starts_with("xml")
}
fn escape_attribute(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;")
        .replace('"', "&quot;").replace('\r', "&#13;")
        .replace('\n', "&#10;").replace('\t', "&#9;")
}
fn opening_tag_end(raw: &str) -> Option<usize> {
    let mut quote = None;
    for (index, c) in raw.char_indices() {
        if let Some(q) = quote {
            if c == q { quote = None; }
        } else if c == '\'' || c == '"' { quote = Some(c); }
        else if c == '>' {
            let mut pos = index;
            while pos > 0 && raw.as_bytes()[pos - 1].is_ascii_whitespace() { pos -= 1; }
            if pos > 0 && raw.as_bytes()[pos - 1] == b'/' { pos -= 1; }
            return Some(pos);
        }
    }
    None
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
    fn rejects_existing_preserved_attribute_collision() {
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
        *edited.elements[visual].path.last_mut().unwrap() = 1;
        *edited.elements[collision].path.last_mut().unwrap() = 0;
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

    #[test]
    fn restores_unqualified_attribute_with_xml_escaping() {
        let source = r#"<robot name="r"><link name="a" vendor="a&amp;b&quot;c"/></robot>"#;
        let edited_xml = r#"<robot name="r"><link name="a"/></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(source).unwrap()).unwrap();
        let original = SourceElementGraph::from_source(source, &mapping).unwrap();
        let snapshot = PlacementSnapshot::from_source_with_graph(source, &mapping, &original).unwrap();
        let edited = original.clone();
        let output = restore_elements(edited_xml, &snapshot, &original, &edited).unwrap();
        assert!(output.contains(r#"vendor="a&amp;b&quot;c""#));
    }

    #[test]
    fn rejects_namespaced_attribute_until_prefix_reconciliation() {
        let source = r#"<robot name="r" xmlns:v="urn:vendor"><link name="a" v:flag="yes"/></robot>"#;
        let edited_xml = r#"<robot name="r" xmlns:v="urn:vendor"><link name="a"/></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(source).unwrap()).unwrap();
        let original = SourceElementGraph::from_source(source, &mapping).unwrap();
        let snapshot = PlacementSnapshot::from_source_with_graph(source, &mapping, &original).unwrap();
        assert!(restore_elements(edited_xml, &snapshot, &original, &original).is_err());
    }

}
