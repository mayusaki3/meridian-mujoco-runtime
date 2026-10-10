//! Structural URDF link/joint extraction, independent of exact XML preservation.
use roxmltree::Document;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Joint {
    pub name: String,
    pub joint_type: String,
    pub parent: String,
    pub child: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RobotStructure {
    pub name: String,
    pub links: Vec<String>,
    pub joints: Vec<Joint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructureError {
    Xml(String),
    Missing(&'static str),
    DuplicateLink(String),
    DuplicateJoint(String),
    UnknownJointType(String),
    UnknownLink { joint: String, link: String },
    SelfReference(String),
    MultipleParents(String),
}

pub fn parse_structure(xml: &str) -> Result<RobotStructure, StructureError> {
    let doc = Document::parse(xml).map_err(|e| StructureError::Xml(e.to_string()))?;
    let root = doc.root_element();
    if root.tag_name().name() != "robot" || root.tag_name().namespace().is_some() {
        return Err(StructureError::Missing("robot"));
    }
    let name = root.attribute("name").filter(|s| !s.is_empty()).ok_or(StructureError::Missing("robot name"))?.to_owned();
    let mut links = Vec::new();
    let mut link_set = HashSet::new();
    let mut joints = Vec::new();
    let mut joint_set = HashSet::new();
    for node in root.children().filter(|n| n.is_element() && n.tag_name().namespace().is_none()) {
        match node.tag_name().name() {
            "link" => {
                let name = node.attribute("name").filter(|s| !s.is_empty()).ok_or(StructureError::Missing("link name"))?.to_owned();
                if !link_set.insert(name.clone()) { return Err(StructureError::DuplicateLink(name)); }
                links.push(name);
            }
            "joint" => {
                let name = node.attribute("name").filter(|s| !s.is_empty()).ok_or(StructureError::Missing("joint name"))?.to_owned();
                if !joint_set.insert(name.clone()) { return Err(StructureError::DuplicateJoint(name)); }
                let joint_type = node.attribute("type").ok_or(StructureError::Missing("joint type"))?.to_owned();
                if !matches!(joint_type.as_str(), "fixed" | "revolute" | "continuous" | "prismatic" | "floating" | "planar") {
                    return Err(StructureError::UnknownJointType(joint_type));
                }
                let endpoint = |tag| node.children().find(|n| n.is_element() && n.tag_name().namespace().is_none() && n.tag_name().name() == tag)
                    .and_then(|n| n.attribute("link")).filter(|s| !s.is_empty()).map(str::to_owned);
                let parent = endpoint("parent").ok_or(StructureError::Missing("joint parent link"))?;
                let child = endpoint("child").ok_or(StructureError::Missing("joint child link"))?;
                if parent == child { return Err(StructureError::SelfReference(name)); }
                joints.push(Joint { name, joint_type, parent, child });
            }
            _ => {}
        }
    }
    let mut children = HashSet::new();
    for joint in &joints {
        for link in [&joint.parent, &joint.child] {
            if !link_set.contains(link) {
                return Err(StructureError::UnknownLink { joint: joint.name.clone(), link: link.clone() });
            }
        }
        if !children.insert(joint.child.clone()) { return Err(StructureError::MultipleParents(joint.child.clone())); }
    }
    Ok(RobotStructure { name, links, joints })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_fixtures_extract() {
        for xml in [
            include_str!("../../../tests/fixtures/urdf-regression/valid/01_minimal.urdf"),
            include_str!("../../../tests/fixtures/urdf-regression/valid/02_two_link_revolute.urdf"),
            include_str!("../../../tests/fixtures/urdf-regression/valid/03_prismatic_fixed_chain.urdf"),
            include_str!("../../../tests/fixtures/urdf-regression/valid/04_multi_joint_mimic.urdf"),
            include_str!("../../../tests/fixtures/urdf-regression/valid/05_vendor_extensions.urdf"),
            include_str!("../../../tests/fixtures/urdf-regression/valid/06_mixed_order_comments.urdf"),
            include_str!("../../../tests/fixtures/urdf-regression/valid/07_materials_and_mesh_reference.urdf"),
            include_str!("../../../tests/fixtures/urdf-regression/valid/08_preserved_parent_with_namespaces.urdf"),
        ] { assert!(parse_structure(xml).is_ok()); }
    }
    #[test]
    fn missing_joint_link_is_rejected() {
        let xml = include_str!("../../../tests/fixtures/urdf-regression/invalid/06_structurally_invalid_urdf.urdf");
        assert!(matches!(parse_structure(xml), Err(StructureError::UnknownLink { .. })));
    }
    #[test]
    fn duplicate_and_self_reference_are_rejected() {
        assert!(matches!(parse_structure("<robot name='r'><link name='a'/><link name='a'/></robot>"), Err(StructureError::DuplicateLink(_))));
        assert!(matches!(parse_structure("<robot name='r'><link name='a'/><joint name='j' type='fixed'><parent link='a'/><child link='a'/></joint></robot>"), Err(StructureError::SelfReference(_))));
    }
}
