//! Initial source-neutral, read-only ROBOT-Model projection.
//! Raw source is retained separately; no edited exporter is provided.
use crate::{mjcf::{inspect_mjcf, MjcfError}, structure::{parse_structure, StructureError}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceFormat { Urdf, Mjcf }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterOrigin { Unknown, Imported, Default, Identified }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    pub raw: Option<String>,
    pub origin: ParameterOrigin,
}

impl Parameter {
    fn imported(raw: Option<String>) -> Self {
        Self { origin: if raw.is_some() { ParameterOrigin::Imported } else { ParameterOrigin::Unknown }, raw }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Body {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Joint {
    pub id: String,
    pub name: Option<String>,
    pub kind: String,
    pub parent_body_id: Option<String>,
    pub child_body_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransmissionTarget {
    Joint(String),
    Tendon(String),
    Site(String),
    Unspecified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actuator {
    pub id: String,
    pub name: Option<String>,
    pub kind: String,
    pub target: TransmissionTarget,
    pub gear: Parameter,
    pub ctrlrange: Parameter,
    pub kp: Parameter,
    pub kv: Parameter,
    pub gainprm: Parameter,
    pub biasprm: Parameter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RobotModel {
    pub schema_version: u32,
    pub source_format: SourceFormat,
    pub name: String,
    pub bodies: Vec<Body>,
    pub joints: Vec<Joint>,
    pub actuators: Vec<Actuator>,
    pub tendons: Vec<String>,
    pub original_xml: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelImportError { Urdf(StructureError), Mjcf(MjcfError), Xml(String) }

pub fn from_urdf(xml: &str) -> Result<RobotModel, ModelImportError> {
    let parsed = parse_structure(xml).map_err(ModelImportError::Urdf)?;
    let bodies = parsed.links.iter().enumerate().map(|(i, name)| Body {
        id: format!("body:{i}"), name: name.clone(),
        parent_id: parsed.joints.iter().find(|j| j.child == *name)
            .and_then(|j| parsed.links.iter().position(|n| n == &j.parent))
            .map(|p| format!("body:{p}")),
    }).collect::<Vec<_>>();
    let joints = parsed.joints.iter().enumerate().map(|(i, joint)| Joint {
        id: format!("joint:{i}"), name: Some(joint.name.clone()), kind: joint.joint_type.clone(),
        parent_body_id: parsed.links.iter().position(|n| n == &joint.parent).map(|p| format!("body:{p}")),
        child_body_id: format!("body:{}", parsed.links.iter().position(|n| n == &joint.child).expect("validated child")),
    }).collect();
    Ok(RobotModel { schema_version: 1, source_format: SourceFormat::Urdf,
        name: parsed.name, bodies, joints, actuators: vec![], tendons: vec![], original_xml: xml.to_owned() })
}

pub fn from_mjcf(xml: &str) -> Result<RobotModel, ModelImportError> {
    let inspected = inspect_mjcf(xml).map_err(ModelImportError::Mjcf)?;
    let doc = roxmltree::Document::parse(xml).map_err(|e| ModelImportError::Xml(e.to_string()))?;
    let mut bodies = vec![];
    let mut joints = vec![];
    let mut named_joint_ids = std::collections::HashMap::new();
    let mut named_tendon_ids = std::collections::HashMap::new();
    let worldbody = doc.root_element().children().find(|n| n.has_tag_name("worldbody"));
    if let Some(worldbody) = worldbody {
        for node in worldbody.descendants().filter(|n| n.is_element() && n.tag_name().namespace().is_none() && n.tag_name().name() == "body") {
            let id = format!("body:{}", bodies.len());
            let parent_id = node.ancestors().skip(1).find(|n| n.is_element() && n.tag_name().name() == "body")
                .and_then(|parent| worldbody.descendants().filter(|n| n.is_element() && n.tag_name().name() == "body").position(|n| n.id() == parent.id()))
                .map(|i| format!("body:{i}"));
            let name = node.attribute("name").map(str::to_owned).unwrap_or_else(|| format!("@{}", node.range().start));
            for joint in node.children().filter(|n| n.is_element() && n.tag_name().namespace().is_none() && n.tag_name().name() == "joint") {
                let joint_id = format!("joint:{}", joints.len());
                let joint_name = joint.attribute("name").map(str::to_owned);
                if let Some(ref name) = joint_name { named_joint_ids.insert(name.clone(), joint_id.clone()); }
                joints.push(Joint { id: joint_id, name: joint_name,
                    kind: joint.attribute("type").unwrap_or("hinge").to_owned(),
                    parent_body_id: parent_id.clone(), child_body_id: id.clone() });
            }
            bodies.push(Body { id, name, parent_id });
        }
    }
    for (i, name) in inspected.tendons.iter().enumerate() { named_tendon_ids.insert(name.clone(), format!("tendon:{i}")); }
    let actuators = inspected.actuators.into_iter().enumerate().map(|(i, a)| {
        let target = if let Some(ref j) = a.joint { TransmissionTarget::Joint(named_joint_ids[j].clone()) }
            else if let Some(ref t) = a.tendon { TransmissionTarget::Tendon(named_tendon_ids[t].clone()) }
            else if let Some(s) = a.site { TransmissionTarget::Site(s) }
            else { TransmissionTarget::Unspecified };
        Actuator { id: format!("actuator:{i}"), name: a.name, kind: a.kind, target,
            gear: Parameter::imported(a.gear), ctrlrange: Parameter::imported(a.ctrlrange),
            kp: Parameter::imported(a.kp), kv: Parameter::imported(a.kv),
            gainprm: Parameter::imported(a.gainprm), biasprm: Parameter::imported(a.biasprm) }
    }).collect();
    Ok(RobotModel { schema_version: 1, source_format: SourceFormat::Mjcf,
        name: inspected.model_name.unwrap_or_default(), bodies, joints, actuators,
        tendons: inspected.tendons, original_xml: xml.to_owned() })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn urdf_projection_does_not_invent_actuators() {
        let xml = include_str!("../../../tests/fixtures/urdf-regression/valid/02_two_link_revolute.urdf");
        let model = from_urdf(xml).unwrap();
        assert_eq!(model.bodies.len(), 2);
        assert_eq!(model.joints.len(), 1);
        assert!(model.actuators.is_empty());
        assert_eq!(model.original_xml, xml);
    }
    #[test]
    fn mjcf_motor_and_servo_parameters_have_imported_provenance() {
        let xml = include_str!("../../../tests/fixtures/mjcf-regression/valid/02_hinge_motor.xml");
        let model = from_mjcf(xml).unwrap();
        assert_eq!(model.bodies.len(), 2);
        assert_eq!(model.actuators.len(), 1);
        assert_eq!(model.actuators[0].gear.raw.as_deref(), Some("100"));
        assert_eq!(model.actuators[0].gear.origin, ParameterOrigin::Imported);
        assert_eq!(model.actuators[0].kp.origin, ParameterOrigin::Unknown);
        assert!(matches!(model.actuators[0].target, TransmissionTarget::Joint(_)));
    }
    #[test]
    fn mjcf_tendon_target_is_preserved() {
        let xml = include_str!("../../../tests/fixtures/mjcf-regression/valid/05_tendon_actuator.xml");
        let model = from_mjcf(xml).unwrap();
        assert_eq!(model.actuators.len(), 1);
        assert!(matches!(model.actuators[0].target, TransmissionTarget::Tendon(_)));
    }
    #[test]
    fn model_serialization_roundtrip() {
        let xml = include_str!("../../../tests/fixtures/mjcf-regression/valid/03_slide_position.xml");
        let model = from_mjcf(xml).unwrap();
        let json = serde_json::to_string(&model).unwrap();
        let restored: RobotModel = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, model);
        assert_eq!(restored.original_xml.as_bytes(), xml.as_bytes());
    }
}
