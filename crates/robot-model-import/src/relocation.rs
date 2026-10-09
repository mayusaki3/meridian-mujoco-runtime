//! Conservative relocation planning for preserved XML. This module does not
//! mutate XML; consumers must apply and verify a plan separately.
use crate::placement::{PlacementError, PlacementSnapshot, PreservedPayload};
use crate::source_element_graph::SourceElementGraph;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relocation {
    pub source_key: Uuid,
    pub parent_key: Uuid,
    pub insert_before: Option<Uuid>,
    pub insert_after: Option<Uuid>,
}

pub fn plan_relocation(
    snapshot: &PlacementSnapshot,
    original: &SourceElementGraph,
    edited: &SourceElementGraph,
) -> Result<Vec<Relocation>, PlacementError> {
    original.validate().map_err(PlacementError::InvalidSource)?;
    edited.validate().map_err(PlacementError::InvalidSource)?;
    if snapshot.schema_version != 1 {
        return Err(PlacementError::Conflict("unsupported placement schema".into()));
    }
    let old: HashMap<_, _> = original.elements.iter().map(|e| (e.key, e)).collect();
    let new: HashMap<_, _> = edited.elements.iter().map(|e| (e.key, e)).collect();
    let mut seen = HashSet::new();
    let preserved: HashSet<_> = snapshot.items.iter()
        .filter(|item| matches!(item.payload, PreservedPayload::Element { .. }))
        .map(|item| item.source_key).collect();
    let mut plan = Vec::new();
    for item in &snapshot.items {
        if !seen.insert(item.source_key) {
            return Err(PlacementError::Conflict("duplicate preserved key".into()));
        }
        let p = &item.placement;
        let parent_key = p.parent_key.ok_or_else(||
            PlacementError::Conflict("unkeyed parent".into()))?;
        let original_parent = old.get(&parent_key).ok_or_else(||
            PlacementError::Conflict("original parent missing".into()))?;
        let new_parent = new.get(&parent_key).ok_or_else(||
            PlacementError::Conflict("edited parent deleted".into()))?;
        if original_parent.owner != new_parent.owner
            || original_parent.local_name != new_parent.local_name
            || original_parent.namespace_uri != new_parent.namespace_uri {
            return Err(PlacementError::Conflict("parent identity changed".into()));
        }
        match &item.payload {
            PreservedPayload::Attribute { .. } => {
                if p.sibling_index.is_some() || p.previous_sibling_key.is_some()
                    || p.next_sibling_key.is_some() {
                    return Err(PlacementError::Conflict("attribute has sibling anchors".into()));
                }
            }
            PreservedPayload::Element { .. } => {
                let element = old.get(&item.source_key).ok_or_else(||
                    PlacementError::Conflict("original preserved element missing".into()))?;
                if element.parent_key != Some(parent_key)
                    || element.previous_sibling_key != p.previous_sibling_key
                    || element.next_sibling_key != p.next_sibling_key {
                    return Err(PlacementError::Conflict("original anchors mismatch".into()));
                }
                if new.contains_key(&item.source_key) {
                    return Err(PlacementError::Conflict("preserved element already present in edited graph".into()));
                }
                // Walk through adjacent preserved siblings until a surviving
                // anchor is found. Cycles and missing preserved entries fail.
                let resolve = |forward: bool| -> Result<Option<Uuid>, PlacementError> {
                    let mut cursor = if forward { p.next_sibling_key } else { p.previous_sibling_key };
                    let mut visited = HashSet::new();
                    while let Some(key) = cursor {
                        if !visited.insert(key) {
                            return Err(PlacementError::Conflict("anchor cycle".into()));
                        }
                        if let Some(neighbor) = new.get(&key) {
                            if neighbor.parent_key != Some(parent_key) {
                                return Err(PlacementError::Conflict("anchor moved to another parent".into()));
                            }
                            return Ok(Some(key));
                        }
                        if !preserved.contains(&key) {
                            return Err(PlacementError::Conflict("sibling anchor deleted".into()));
                        }
                        let neighbor = old.get(&key).ok_or_else(||
                            PlacementError::Conflict("preserved anchor missing in original".into()))?;
                        if neighbor.parent_key != Some(parent_key) {
                            return Err(PlacementError::Conflict("preserved sibling parent changed".into()));
                        }
                        cursor = if forward { neighbor.next_sibling_key } else { neighbor.previous_sibling_key };
                    }
                    Ok(None)
                };
                let before = resolve(false)?;
                let after = resolve(true)?;
                match (before, after) {
                    (Some(left), Some(right)) => {
                        if new[&left].next_sibling_key != Some(right)
                            || new[&right].previous_sibling_key != Some(left) {
                            return Err(PlacementError::Conflict("surviving anchors no longer adjacent".into()));
                        }
                    }
                    (Some(left), None) if new[&left].next_sibling_key.is_some() =>
                        return Err(PlacementError::Conflict("trailing anchor no longer last".into())),
                    (None, Some(right)) if new[&right].previous_sibling_key.is_some() =>
                        return Err(PlacementError::Conflict("leading anchor no longer first".into())),
                    (None, None) if new.values().any(|e| e.parent_key == Some(parent_key)) =>
                        return Err(PlacementError::Conflict("unanchored insertion ambiguous".into())),
                    _ => {}
                }
            }
        }
        plan.push(Relocation {
            source_key: item.source_key, parent_key,
            // Immediate anchors may themselves be preserved and therefore
            // restored by another entry in the same plan.
            insert_before: p.next_sibling_key,
            insert_after: p.previous_sibling_key,
        });
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{inspect_urdf, placement::PlacementSnapshot, source_mapping::SourceMappingDocument};

    fn setup() -> (PlacementSnapshot, SourceElementGraph) {
        let xml = r#"<robot name="r"><link name="a"><visual/><vendor/><collision/></link></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let graph = SourceElementGraph::from_source(xml, &mapping).unwrap();
        let snapshot = PlacementSnapshot::from_source_with_graph(xml, &mapping, &graph).unwrap();
        (snapshot, graph)
    }

    fn without_preserved(graph: &SourceElementGraph, key: Uuid) -> SourceElementGraph {
        let mut edited = graph.clone();
        let item = edited.elements.iter().find(|e| e.key == key).unwrap().clone();
        for element in &mut edited.elements {
            if element.next_sibling_key == Some(key) { element.next_sibling_key = item.next_sibling_key; }
            if element.previous_sibling_key == Some(key) { element.previous_sibling_key = item.previous_sibling_key; }
            if element.parent_key == item.parent_key && element.path.last().copied().unwrap_or(0) >
                item.path.last().copied().unwrap_or(0) {
                *element.path.last_mut().unwrap() -= 1;
            }
        }
        edited.elements.retain(|e| e.key != key);
        edited
    }

    #[test]
    fn plans_reinsertion_between_stable_anchors() {
        let (snapshot, original) = setup();
        let key = snapshot.items[0].source_key;
        let edited = without_preserved(&original, key);
        edited.validate().unwrap();
        let plan = plan_relocation(&snapshot, &original, &edited).unwrap();
        assert_eq!(plan.len(), 1);
        assert!(plan[0].insert_before.is_some());
        assert!(plan[0].insert_after.is_some());
    }

    #[test]
    fn rejects_deleted_parent() {
        let (snapshot, original) = setup();
        let key = snapshot.items[0].source_key;
        let mut edited = without_preserved(&original, key);
        let parent = snapshot.items[0].placement.parent_key.unwrap();
        edited.elements.retain(|e| e.key != parent);
        assert!(plan_relocation(&snapshot, &original, &edited).is_err());
    }

    #[test]
    fn rejects_missing_sibling_anchor() {
        let (snapshot, original) = setup();
        let key = snapshot.items[0].source_key;
        let mut edited = without_preserved(&original, key);
        let anchor = snapshot.items[0].placement.previous_sibling_key.unwrap();
        edited.elements.retain(|e| e.key != anchor);
        assert!(plan_relocation(&snapshot, &original, &edited).is_err());
    }

    #[test]
    fn rejects_ambiguous_anchor_order() {
        let (snapshot, original) = setup();
        let key = snapshot.items[0].source_key;
        let mut edited = without_preserved(&original, key);
        let previous = snapshot.items[0].placement.previous_sibling_key.unwrap();
        let next = snapshot.items[0].placement.next_sibling_key.unwrap();
        let p = edited.elements.iter().position(|e| e.key == previous).unwrap();
        let n = edited.elements.iter().position(|e| e.key == next).unwrap();
        edited.elements[p].next_sibling_key = None;
        edited.elements[n].previous_sibling_key = None;
        assert!(plan_relocation(&snapshot, &original, &edited).is_err());
    }
    #[test]
    fn plans_adjacent_preserved_elements_in_original_order() {
        let xml = r#"<robot name="r"><link name="a"><visual/><vendor_a/><vendor_b/><collision/></link></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let original = SourceElementGraph::from_source(xml, &mapping).unwrap();
        let snapshot = PlacementSnapshot::from_source_with_graph(xml, &mapping, &original).unwrap();
        assert_eq!(snapshot.items.len(), 2);
        let first = snapshot.items[0].source_key;
        let second = snapshot.items[1].source_key;
        let edited = without_preserved(&without_preserved(&original, first), second);
        edited.validate().unwrap();
        let plan = plan_relocation(&snapshot, &original, &edited).unwrap();
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].insert_before, Some(second));
        assert_eq!(plan[1].insert_after, Some(first));
    }

    #[test]
    fn rejects_incomplete_adjacent_preserved_group() {
        let xml = r#"<robot name="r"><link name="a"><visual/><vendor_a/><vendor_b/><collision/></link></robot>"#;
        let mapping = SourceMappingDocument::from_inspection(&inspect_urdf(xml).unwrap()).unwrap();
        let original = SourceElementGraph::from_source(xml, &mapping).unwrap();
        let mut snapshot = PlacementSnapshot::from_source_with_graph(xml, &mapping, &original).unwrap();
        let first = snapshot.items[0].source_key;
        let second = snapshot.items[1].source_key;
        let edited = without_preserved(&without_preserved(&original, first), second);
        snapshot.items.remove(1);
        assert!(matches!(plan_relocation(&snapshot, &original, &edited), Err(PlacementError::Conflict(_))));
    }

}
