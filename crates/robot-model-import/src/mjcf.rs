//! Read-only MJCF inspection and exact unchanged-source retention.
//! This is deliberately not a full MuJoCo compiler or canonical exporter.
use roxmltree::{Document, Node};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MjcfJoint {
    pub name: Option<String>,
    pub joint_type: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MjcfActuator {
    pub name: Option<String>,
    pub kind: String,
    pub joint: Option<String>,
    pub tendon: Option<String>,
    pub site: Option<String>,
    pub gear: Option<String>,
    pub ctrlrange: Option<String>,
    pub kp: Option<String>,
    pub kv: Option<String>,
    pub gainprm: Option<String>,
    pub biasprm: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectedMjcf {
    pub model_name: Option<String>,
    pub bodies: Vec<String>,
    pub joints: Vec<MjcfJoint>,
    pub actuators: Vec<MjcfActuator>,
    pub tendons: Vec<String>,
    pub original_xml: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MjcfError {
    InvalidXml(String),
    InvalidRoot,
    DuplicateBody(String),
    DuplicateJoint(String),
    DuplicateTendon(String),
    DuplicateSite(String),
    InvalidJointType(String),
    UnknownActuatorTarget { actuator: String, target: String },
}

fn element<'a, 'input>(node: Node<'a, 'input>, name: &str) -> bool {
    node.is_element() && node.tag_name().namespace().is_none() && node.tag_name().name() == name
}

pub fn inspect_mjcf(xml: &str) -> Result<InspectedMjcf, MjcfError> {
    let doc = Document::parse(xml).map_err(|e| MjcfError::InvalidXml(e.to_string()))?;
    let root = doc.root_element();
    if !element(root, "mujoco") { return Err(MjcfError::InvalidRoot); }
    let mut result = InspectedMjcf {
        model_name: root.attribute("model").map(str::to_owned),
        bodies: vec![], joints: vec![], actuators: vec![], tendons: vec![],
        original_xml: xml.to_owned(),
    };
    let mut body_names = HashSet::new();
    let mut joint_names = HashSet::new();
    let mut tendon_names = HashSet::new();
    let mut site_names = HashSet::new();
    if let Some(worldbody) = root.children().find(|n| element(*n, "worldbody")) {
        for site in worldbody.descendants().filter(|n| element(*n, "site")) {
            if let Some(name) = site.attribute("name") {
                if !site_names.insert(name.to_owned()) {
                    return Err(MjcfError::DuplicateSite(name.to_owned()));
                }
            }
        }
        for body in worldbody.descendants().filter(|n| element(*n, "body")) {
            let body_name = body.attribute("name").map(str::to_owned).unwrap_or_else(|| format!("@{}", body.range().start));
            if !body_names.insert(body_name.clone()) { return Err(MjcfError::DuplicateBody(body_name)); }
            result.bodies.push(body_name.clone());
            for joint in body.children().filter(|n| element(*n, "joint")) {
                let kind = joint.attribute("type").unwrap_or("hinge").to_owned();
                if !matches!(kind.as_str(), "hinge" | "slide" | "ball" | "free") {
                    return Err(MjcfError::InvalidJointType(kind));
                }
                let name = joint.attribute("name").map(str::to_owned);
                if let Some(ref n) = name {
                    if !joint_names.insert(n.clone()) { return Err(MjcfError::DuplicateJoint(n.clone())); }
                }
                result.joints.push(MjcfJoint { name, joint_type: kind, body: body_name.clone() });
            }
        }
    }
    if let Some(tendon) = root.children().find(|n| element(*n, "tendon")) {
        for node in tendon.children().filter(|n| n.is_element() && n.tag_name().namespace().is_none()) {
            if let Some(name) = node.attribute("name") {
                if !tendon_names.insert(name.to_owned()) { return Err(MjcfError::DuplicateTendon(name.to_owned())); }
                result.tendons.push(name.to_owned());
            }
        }
    }
    if let Some(actuators) = root.children().find(|n| element(*n, "actuator")) {
        for node in actuators.children().filter(|n| n.is_element() && n.tag_name().namespace().is_none()) {
            let name = node.attribute("name").map(str::to_owned);
            let label = name.clone().unwrap_or_else(|| format!("@{}", node.range().start));
            for (attr, known) in [("joint", &joint_names), ("tendon", &tendon_names), ("site", &site_names)] {
                if let Some(target) = node.attribute(attr) {
                    if !known.contains(target) {
                        return Err(MjcfError::UnknownActuatorTarget { actuator: label, target: target.to_owned() });
                    }
                }
            }
            result.actuators.push(MjcfActuator {
                name, kind: node.tag_name().name().to_owned(),
                joint: node.attribute("joint").map(str::to_owned),
                tendon: node.attribute("tendon").map(str::to_owned),
                site: node.attribute("site").map(str::to_owned),
                gear: node.attribute("gear").map(str::to_owned),
                ctrlrange: node.attribute("ctrlrange").map(str::to_owned),
                kp: node.attribute("kp").map(str::to_owned),
                kv: node.attribute("kv").map(str::to_owned),
                gainprm: node.attribute("gainprm").map(str::to_owned),
                biasprm: node.attribute("biasprm").map(str::to_owned),
            });
        }
    }
    Ok(result)
}

pub fn export_unchanged_mjcf(model: &InspectedMjcf) -> &str { &model.original_xml }

#[cfg(test)]
mod tests {
    use super::*;
    const VALID: &[&str] = &[
        include_str!("../../../tests/fixtures/mjcf-regression/valid/01_minimal.xml"),
        include_str!("../../../tests/fixtures/mjcf-regression/valid/02_hinge_motor.xml"),
        include_str!("../../../tests/fixtures/mjcf-regression/valid/03_slide_position.xml"),
        include_str!("../../../tests/fixtures/mjcf-regression/valid/04_velocity_general.xml"),
        include_str!("../../../tests/fixtures/mjcf-regression/valid/05_tendon_actuator.xml"),
        include_str!("../../../tests/fixtures/mjcf-regression/valid/06_defaults_contact_sensor.xml"),
        include_str!("../../../tests/fixtures/mjcf-regression/valid/07_extension_comments.xml"),
        include_str!("../../../tests/fixtures/mjcf-regression/valid/08_nested_bodies_inertial.xml"),
    ];
    #[test]
    fn valid_fixtures_inspect_and_roundtrip_exactly() {
        for xml in VALID {
            let model = inspect_mjcf(xml).unwrap();
            assert_eq!(export_unchanged_mjcf(&model).as_bytes(), xml.as_bytes());
        }
    }
    #[test]
    fn actuator_characteristics_are_extracted() {
        let motor = inspect_mjcf(VALID[1]).unwrap();
        assert_eq!(motor.actuators[0].joint.as_deref(), Some("shoulder"));
        assert_eq!(motor.actuators[0].gear.as_deref(), Some("100"));
        let servo = inspect_mjcf(VALID[2]).unwrap();
        assert_eq!(servo.actuators[0].kp.as_deref(), Some("120"));
        assert_eq!(servo.actuators[0].kv.as_deref(), Some("5"));
        let coupled = inspect_mjcf(VALID[4]).unwrap();
        assert_eq!(coupled.actuators[0].tendon.as_deref(), Some("coupled"));
    }
    #[test]
    fn duplicate_named_joints_are_rejected() {
        let xml = r#"<mujoco><worldbody><body name="a"><joint name="j"/></body><body name="b"><joint name="j"/></body></worldbody></mujoco>"#;
        assert_eq!(inspect_mjcf(xml), Err(MjcfError::DuplicateJoint("j".into())));
    }
    #[test]
    fn duplicate_named_tendons_are_rejected() {
        let xml = r#"<mujoco><tendon><fixed name="t"/><fixed name="t"/></tendon></mujoco>"#;
        assert_eq!(inspect_mjcf(xml), Err(MjcfError::DuplicateTendon("t".into())));
    }
    #[test]
    fn missing_actuator_site_is_rejected() {
        let xml = r#"<mujoco><worldbody><body name="a"/></worldbody><actuator><motor name="m" site="missing"/></actuator></mujoco>"#;
        assert!(matches!(inspect_mjcf(xml), Err(MjcfError::UnknownActuatorTarget { target, .. }) if target == "missing"));
    }
    #[test]
    fn duplicate_sites_and_valid_site_actuator() {
        let valid = r#"<mujoco><worldbody><site name="anchor"/><body name="a"><site name="tip"/></body></worldbody><actuator><motor name="m" site="tip"/></actuator></mujoco>"#;
        assert_eq!(inspect_mjcf(valid).unwrap().actuators[0].site.as_deref(), Some("tip"));
        let duplicate = r#"<mujoco><worldbody><site name="s"/><body name="a"><site name="s"/></body></worldbody></mujoco>"#;
        assert_eq!(inspect_mjcf(duplicate), Err(MjcfError::DuplicateSite("s".into())));
    }
    #[test]
    fn invalid_fixtures_are_rejected() {
        for xml in [
            include_str!("../../../tests/fixtures/mjcf-regression/invalid/01_malformed_xml.xml"),
            include_str!("../../../tests/fixtures/mjcf-regression/invalid/02_wrong_root.xml"),
            include_str!("../../../tests/fixtures/mjcf-regression/invalid/03_missing_actuator_target.xml"),
            include_str!("../../../tests/fixtures/mjcf-regression/invalid/04_duplicate_body_name.xml"),
            include_str!("../../../tests/fixtures/mjcf-regression/invalid/05_unknown_joint_type.xml"),
            include_str!("../../../tests/fixtures/mjcf-regression/invalid/06_missing_tendon_reference.xml"),
        ] { assert!(inspect_mjcf(xml).is_err(), "unexpectedly accepted: {xml}"); }
    }
}
