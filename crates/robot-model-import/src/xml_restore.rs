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
    Ok(output)
}
