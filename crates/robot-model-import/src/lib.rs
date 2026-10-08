//! Initial read-only URDF XML inspection. No lossy exporter is exposed.
use roxmltree::Document;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreservedItem {
    pub owner: String,
    pub xml: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectedUrdf {
    pub robot_name: String,
    pub links: Vec<String>,
    pub joints: Vec<String>,
    pub preserved: Vec<PreservedItem>,
    /// Original input retained for exact unchanged-source export.
    pub original_xml: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectError {
    InvalidXml(String),
    InvalidRobot,
    MissingName(String),
}

fn known_child(parent: &str, child: &str) -> bool {
    match parent {
        "robot" => matches!(child, "link" | "joint"),
        "link" => matches!(child, "inertial" | "visual" | "collision"),
        "joint" => matches!(child, "parent" | "child" | "origin" | "axis" | "limit" | "dynamics" | "mimic" | "safety_controller" | "calibration"),
        "inertial" => matches!(child, "origin" | "mass" | "inertia"),
        "visual" | "collision" => matches!(child, "origin" | "geometry" | "material"),
        "geometry" => matches!(child, "box" | "cylinder" | "sphere" | "mesh"),
        _ => false,
    }
}

fn known_attr(element: &str, attr: &str) -> bool {
    match element {
        "robot" | "link" => attr == "name",
        "joint" => matches!(attr, "name" | "type"),
        "parent" | "child" => attr == "link",
        "origin" => matches!(attr, "xyz" | "rpy"),
        "axis" => attr == "xyz",
        "limit" => matches!(attr, "lower" | "upper" | "effort" | "velocity"),
        "mass" => attr == "value",
        "inertia" => matches!(attr, "ixx" | "ixy" | "ixz" | "iyy" | "iyz" | "izz"),
        "box" => attr == "size",
        "cylinder" => matches!(attr, "radius" | "length"),
        "sphere" => attr == "radius",
        "mesh" => matches!(attr, "filename" | "scale"),
        "material" => attr == "name",
        _ => false,
    }
}

pub fn inspect_urdf(xml: &str) -> Result<InspectedUrdf, InspectError> {
    let doc = Document::parse(xml).map_err(|e| InspectError::InvalidXml(e.to_string()))?;
    let root = doc.root_element();
    if root.tag_name().name() != "robot" || root.tag_name().namespace().is_some() {
        return Err(InspectError::InvalidRobot);
    }
    let name = root.attribute("name").ok_or_else(|| InspectError::MissingName("robot".into()))?;
    let mut result = InspectedUrdf {
        robot_name: name.into(),
        links: vec![],
        joints: vec![],
        preserved: vec![],
        original_xml: xml.into(),
    };
    fn visit(node: roxmltree::Node<'_, '_>, owner: &str, xml: &str, result: &mut InspectedUrdf) {
        let tag = node.tag_name().name();
        for attr in node.attributes() {
            if attr.namespace().is_some() || !known_attr(tag, attr.name()) {
                result.preserved.push(PreservedItem {
                    owner: owner.into(),
                    xml: format!("attribute:{}={:?}", attr.name(), attr.value()),
                });
            }
        }
        for child in node.children().filter(|n| n.is_element()) {
            let child_tag = child.tag_name().name();
            if child.tag_name().namespace().is_some() || !known_child(tag, child_tag) {
                result.preserved.push(PreservedItem {
                    owner: owner.into(),
                    xml: xml[child.range()].into(),
                });
                continue;
            }
            let child_owner = if tag == "robot" && matches!(child_tag, "link" | "joint") {
                format!("{child_tag}:{}", child.attribute("name").unwrap_or("<missing>"))
            } else {
                owner.into()
            };
            visit(child, &child_owner, xml, result);
        }
    }
    for child in root.children().filter(|n| n.is_element()) {
        if child.tag_name().namespace().is_some() || !known_child("robot", child.tag_name().name()) {
            result.preserved.push(PreservedItem { owner: "robot".into(), xml: xml[child.range()].into() });
            continue;
        }
        match child.tag_name().name() {
            "link" => result.links.push(child.attribute("name").ok_or_else(|| InspectError::MissingName("link".into()))?.into()),
            "joint" => result.joints.push(child.attribute("name").ok_or_else(|| InspectError::MissingName("joint".into()))?.into()),
            _ => {}
        }
        let owner = format!("{}:{}", child.tag_name().name(), child.attribute("name").unwrap_or(""));
        visit(child, &owner, xml, &mut result);
    }
    for attr in root.attributes() {
        if attr.namespace().is_some() || !known_attr("robot", attr.name()) {
            result.preserved.push(PreservedItem { owner: "robot".into(), xml: format!("attribute:{}={:?}", attr.name(), attr.value()) });
        }
    }
    Ok(result)
}

/// Only safe when no canonical edits have been applied.
pub fn export_unchanged_source(document: &InspectedUrdf) -> &str {
    &document.original_xml
}

#[cfg(test)]
mod tests {
    use super::*;
    const MINIMAL: &str = include_str!("../../../tests/fixtures/robot-model/minimal.urdf");
    const EXTENDED: &str = include_str!("../../../tests/fixtures/robot-model/unknown-extension.urdf");
    const MIXED: &str = include_str!("../../../tests/fixtures/robot-model/mixed-order.urdf");

    #[test]
    fn inspect_minimal() {
        let x = inspect_urdf(MINIMAL).unwrap();
        assert_eq!(x.robot_name, "minimal");
        assert_eq!(x.links, ["base_link", "link_1"]);
        assert_eq!(x.joints, ["joint_1"]);
    }

    #[test]
    fn preserve_unknown_elements_and_attributes() {
        let x = inspect_urdf(EXTENDED).unwrap();
        assert!(x.preserved.iter().any(|p| p.owner == "robot" && p.xml.contains("metadata")));
        assert!(x.preserved.iter().any(|p| p.owner == "link:base_link" && p.xml.contains("calibration")));
        assert!(x.preserved.iter().any(|p| p.owner == "joint:joint_1" && p.xml.contains("control")));
        assert!(x.preserved.iter().any(|p| p.xml.contains("material")));
    }

    #[test]
    fn unchanged_source_roundtrip() {
        let x = inspect_urdf(EXTENDED).unwrap();
        assert_eq!(export_unchanged_source(&x), EXTENDED);
    }

    #[test]
    fn mixed_order_preserved() {
        let x = inspect_urdf(MIXED).unwrap();
        let parts: Vec<_> = x.preserved.iter().filter(|p| p.xml.contains("vendor:between")).collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].xml.contains("first"));
        assert!(parts[1].xml.contains("second"));
    }

    #[test]
    fn preserve_unsupported_top_level_extension() {
        let xml = r#"<robot name="r"><gazebo reference="base"><plugin name="x"/></gazebo><link name="base"/></robot>"#;
        let result = inspect_urdf(xml).unwrap();
        assert!(result.preserved.iter().any(|p| p.owner == "robot" && p.xml.contains("<plugin")));
    }

    #[test]
    fn preserve_unknown_link_attribute() {
        let xml = r#"<robot name="r"><link name="base" custom="value"/></robot>"#;
        let result = inspect_urdf(xml).unwrap();
        assert!(result.preserved.iter().any(|p| p.owner == "link:base" && p.xml.contains("custom")));
    }

    #[test]
    fn reject_invalid_xml() {
        assert!(matches!(inspect_urdf("<robot>"), Err(InspectError::InvalidXml(_))));
    }

    #[test]
    fn reject_invalid_root() {
        assert_eq!(inspect_urdf("<notrobot/>"), Err(InspectError::InvalidRobot));
    }

    #[test]
    fn reject_missing_robot_name() {
        assert_eq!(inspect_urdf("<robot/>"), Err(InspectError::MissingName("robot".into())));
    }
}
