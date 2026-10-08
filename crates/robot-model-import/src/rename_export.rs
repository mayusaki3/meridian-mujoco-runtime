//! Narrow, conflict-safe export PoC: only Link/Joint name changes are supported.
//! Edits are applied to the original XML so unknown nodes and sibling order survive.
use crate::source_mapping::{SourceMappingDocument, SourceOwner};
use roxmltree::Document;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    InvalidSource(String),
    Conflict(String),
}

fn xml_attribute_escape(value: &str, quote: char) -> String {
    let mut s = value.replace('&', "&amp;").replace('<', "&lt;");
    if quote == '"' { s = s.replace('"', "&quot;"); }
    else { s = s.replace('\'', "&apos;"); }
    s
}

/// Supports only rename edits; source XML must be the same document used to
/// construct mapping. Other canonical edits must not call this function.
pub fn export_renamed_source(
    source_xml: &str,
    original: &SourceMappingDocument,
    edited: &SourceMappingDocument,
) -> Result<String, ExportError> {
    if original.robot_id != edited.robot_id || original.preserved != edited.preserved
        || original.robot_name != edited.robot_name || original.schema_version != edited.schema_version
    {
        return Err(ExportError::Conflict("unsupported canonical edit".into()));
    }
    if original.links.len() != edited.links.len() || original.joints.len() != edited.joints.len() {
        return Err(ExportError::Conflict("owner added or deleted".into()));
    }
    let doc = Document::parse(source_xml).map_err(|e| ExportError::InvalidSource(e.to_string()))?;
    let root = doc.root_element();
    if root.tag_name().name() != "robot" || root.attribute("name") != Some(original.robot_name.as_str()) {
        return Err(ExportError::Conflict("source robot mismatch".into()));
    }
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();
    for kind in ["link", "joint"] {
        let (old, new): (Vec<_>, Vec<_>) = if kind == "link" {
            (original.links.iter().map(|v| (v.id.0, v.name.as_str())).collect(),
             edited.links.iter().map(|v| (v.id.0, v.name.as_str())).collect())
        } else {
            (original.joints.iter().map(|v| (v.id.0, v.name.as_str())).collect(),
             edited.joints.iter().map(|v| (v.id.0, v.name.as_str())).collect())
        };
        let unique: HashSet<_> = new.iter().map(|v| v.1).collect();
        if unique.len() != new.len() || new.iter().any(|(_, name)| name.is_empty()) {
            return Err(ExportError::Conflict(format!("invalid or duplicate {kind} name")));
        }
        let source_nodes: Vec<_> = root.children().filter(|n|
            n.is_element() && n.tag_name().namespace().is_none() && n.tag_name().name() == kind
        ).collect();
        if source_nodes.len() != old.len() {
            return Err(ExportError::Conflict(format!("{kind} source count mismatch")));
        }
        for ((id, previous), node) in old.iter().zip(source_nodes) {
            if node.attribute("name") != Some(*previous) {
                return Err(ExportError::Conflict(format!("{kind} source identity mismatch")));
            }
            let updated = new.iter().find(|(other, _)| other == id)
                .ok_or_else(|| ExportError::Conflict(format!("{kind} owner deleted")))?;
            if updated.1 == *previous { continue; }
            // Restrict replacement to the opening tag, never an unknown child.
            let start = node.range().start;
            let text = &source_xml[start..node.range().end];
            let end = text.find('>').ok_or_else(|| ExportError::Conflict("missing start tag".into()))?;
            let head = &text[..end];
            let mut matches = Vec::new();
            for quote in ['"', '\''] {
                let needle = format!("name={quote}{previous}{quote}");
                if let Some(at) = head.find(&needle) {
                    // Verify this is an actual attribute, not part of another name.
                    if at > 0 && head.as_bytes()[at - 1].is_ascii_whitespace() {
                        matches.push((at, needle.len(), quote));
                    }
                }
            }
            if matches.len() != 1 {
                return Err(ExportError::Conflict(format!("{kind} name attribute cannot be safely patched")));
            }
            let (at, len, quote) = matches[0];
            let replacement = format!("name={quote}{}{quote}", xml_attribute_escape(updated.1, quote));
            replacements.push((start + at, start + at + len, replacement));
        }
    }
    // Unknown XML may contain semantic references which cannot be rewritten safely.
    // Only permit renaming if every preserved payload is free of the old name.
    for before in &original.links {
        let after = edited.links.iter().find(|v| v.id == before.id)
            .ok_or_else(|| ExportError::Conflict("link owner deleted".into()))?;
        if before.name == after.name { continue; }
        if original.preserved.iter().any(|p| p.xml.contains(&before.name)) {
            return Err(ExportError::Conflict(format!(
                "preserved XML may reference renamed link: {}", before.name
            )));
        }
    }
    // Patch only known URDF joint parent/child link attributes.
    for joint in root.children().filter(|n| n.is_element()
        && n.tag_name().namespace().is_none() && n.tag_name().name() == "joint") {
        for child in joint.children().filter(|n| n.is_element()
            && n.tag_name().namespace().is_none()
            && matches!(n.tag_name().name(), "parent" | "child")) {
            let Some(link_name) = child.attribute("link") else { continue };
            let Some(before) = original.links.iter().find(|v| v.name == link_name) else { continue };
            let after = edited.links.iter().find(|v| v.id == before.id)
                .ok_or_else(|| ExportError::Conflict("link owner deleted".into()))?;
            if after.name == before.name { continue; }
            let start = child.range().start;
            let text = &source_xml[start..child.range().end];
            let end = text.find('>').ok_or_else(|| ExportError::Conflict("missing reference tag".into()))?;
            let head = &text[..end];
            let mut found = Vec::new();
            for quote in ['"', '\''] {
                let needle = format!("link={quote}{}{quote}", before.name);
                if let Some(at) = head.find(&needle) {
                    if at > 0 && head.as_bytes()[at - 1].is_ascii_whitespace() {
                        found.push((at, needle.len(), quote));
                    }
                }
            }
            if found.len() != 1 {
                return Err(ExportError::Conflict("joint link attribute cannot be safely patched".into()));
            }
            let (at, len, quote) = found[0];
            replacements.push((start + at, start + at + len,
                format!("link={quote}{}{quote}", xml_attribute_escape(&after.name, quote))));
        }
    }
    // Prevent changes to source ownership and unsupported preserved information.
    for item in &edited.preserved {
        let exists = match item.owner {
            SourceOwner::Robot(id) => id == edited.robot_id,
            SourceOwner::Link(id) => edited.links.iter().any(|v| v.id == id),
            SourceOwner::Joint(id) => edited.joints.iter().any(|v| v.id == id),
        };
        if !exists { return Err(ExportError::Conflict("orphaned preserved owner".into())); }
    }
    let mut output = source_xml.to_owned();
    replacements.sort_by_key(|v| std::cmp::Reverse(v.0));
    for (start, end, replacement) in replacements {
        output.replace_range(start..end, &replacement);
    }
    Document::parse(&output).map_err(|e| ExportError::InvalidSource(e.to_string()))?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{inspect_urdf, source_mapping::SourceMappingDocument};

    #[test]
    fn rename_unreferenced_link_preserves_unknown_xml_and_order() {
        let source = r#"<robot name="r" xmlns:v="urn:test"><link name="base"><v:first/><visual><geometry><box size="1 1 1"/></geometry></visual><v:second/></link></robot>"#;
        let original = SourceMappingDocument::from_inspection(&inspect_urdf(source).unwrap()).unwrap();
        let mut edited = original.clone();
        edited.links[0].name = "renamed_base".into();
        let output = export_renamed_source(source, &original, &edited).unwrap();
        assert!(output.contains("name=\"renamed_base\""));
        assert!(output.find("<v:first").unwrap() < output.find("<v:second").unwrap());
        assert!(output.contains("<v:second"));
    }

    #[test]
    fn reject_rename_when_preserved_xml_may_reference_link() {
        let source = r#"<robot name="r" xmlns:v="urn:test"><link name="base"><v:binding target="base"/></link></robot>"#;
        let original = SourceMappingDocument::from_inspection(&inspect_urdf(source).unwrap()).unwrap();
        let mut edited = original.clone();
        edited.links[0].name = "renamed".into();
        assert!(matches!(export_renamed_source(source, &original, &edited), Err(ExportError::Conflict(_))));
    }

    #[test]
    fn rename_link_updates_joint_parent_and_child() {
        let source = r#"<robot name="r"><link name="a"/><link name="b"/><joint name="j" type="fixed"><parent link="a"/><child link="b"/></joint></robot>"#;
        let original = SourceMappingDocument::from_inspection(&inspect_urdf(source).unwrap()).unwrap();
        let mut edited = original.clone();
        edited.links[0].name = "new_a".into();
        edited.links[1].name = "new_b".into();
        let output = export_renamed_source(source, &original, &edited).unwrap();
        assert!(output.contains(r#"<parent link="new_a"/>"#));
        assert!(output.contains(r#"<child link="new_b"/>"#));
    }

    #[test]
    fn swapped_link_names_update_references_by_identity() {
        let source = r#"<robot name="r"><link name="a"/><link name="b"/><joint name="j" type="fixed"><parent link="a"/><child link="b"/></joint></robot>"#;
        let original = SourceMappingDocument::from_inspection(&inspect_urdf(source).unwrap()).unwrap();
        let mut edited = original.clone();
        edited.links[0].name = "b".into();
        edited.links[1].name = "a".into();
        let output = export_renamed_source(source, &original, &edited).unwrap();
        assert!(output.contains(r#"<parent link="b"/>"#));
        assert!(output.contains(r#"<child link="a"/>"#));
    }

    #[test]
    fn reject_deleted_owner() {
        let source = include_str!("../../../tests/fixtures/robot-model/unknown-extension.urdf");
        let original = SourceMappingDocument::from_inspection(&inspect_urdf(source).unwrap()).unwrap();
        let mut edited = original.clone();
        edited.links.clear();
        assert!(matches!(export_renamed_source(source, &original, &edited), Err(ExportError::Conflict(_))));
    }

    #[test]
    fn reject_duplicate_names() {
        let source = include_str!("../../../tests/fixtures/robot-model/mixed-order.urdf");
        let original = SourceMappingDocument::from_inspection(&inspect_urdf(source).unwrap()).unwrap();
        let mut edited = original.clone();
        edited.links[0].name = edited.links[1].name.clone();
        assert!(matches!(export_renamed_source(source, &original, &edited), Err(ExportError::Conflict(_))));
    }
}
