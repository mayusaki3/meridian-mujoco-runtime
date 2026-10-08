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
                // Both anchors must survive, retain the same parent and remain
                // adjacent. A missing anchor is not silently substituted.
                for anchor in [p.previous_sibling_key, p.next_sibling_key].into_iter().flatten() {
                    let neighbor = new.get(&anchor).ok_or_else(||
                        PlacementError::Conflict("sibling anchor deleted".into()))?;
                    if neighbor.parent_key != Some(parent_key) {
                        return Err(PlacementError::Conflict("sibling moved to another parent".into()));
                    }
                }
                match (p.previous_sibling_key, p.next_sibling_key) {
                    (Some(before), Some(after)) => {
                        if new[&before].next_sibling_key != Some(after)
                            || new[&after].previous_sibling_key != Some(before) {
                            return Err(PlacementError::Conflict("anchors no longer adjacent".into()));
                        }
                    }
                    (Some(before), None) if new[&before].next_sibling_key.is_some() =>
                        return Err(PlacementError::Conflict("trailing anchor no longer last".into())),
                    (None, Some(after)) if new[&after].previous_sibling_key.is_some() =>
                        return Err(PlacementError::Conflict("leading anchor no longer first".into())),
                    (None, None) if new.values().any(|e| e.parent_key == Some(parent_key)) =>
                        return Err(PlacementError::Conflict("unanchored insertion ambiguous".into())),
                    _ => {}
                }
            }
        }
        plan.push(Relocation {
            source_key: item.source_key, parent_key,
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
}
