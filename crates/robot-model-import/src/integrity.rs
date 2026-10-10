//! Referential integrity checks for source-neutral ROBOT-Model snapshots.
use crate::model::{RobotModel, TransmissionTarget};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityError {
    UnsupportedSchema(u32),
    DuplicateId(String),
    InvalidId(String),
    MissingReference { owner: String, target: String },
    SelfParent(String),
    BodyCycle(String),
}

pub fn validate_model(model: &RobotModel) -> Result<(), IntegrityError> {
    if model.schema_version != 1 { return Err(IntegrityError::UnsupportedSchema(model.schema_version)); }
    let mut all: HashSet<String> = HashSet::new();
    if uuid::Uuid::parse_str(&model.model_id).is_err() { return Err(IntegrityError::InvalidId(model.model_id.clone())); }
    let mut bodies = HashSet::new();
    let mut joints = HashSet::new();
    let mut tendons = HashSet::new();
    for body in &model.bodies {
        if !all.insert(body.id.clone()) { return Err(IntegrityError::DuplicateId(body.id.clone())); }
        bodies.insert(body.id.as_str());
    }
    for joint in &model.joints {
        if !all.insert(joint.id.clone()) { return Err(IntegrityError::DuplicateId(joint.id.clone())); }
        joints.insert(joint.id.as_str());
    }
    for actuator in &model.actuators {
        if !all.insert(actuator.id.clone()) { return Err(IntegrityError::DuplicateId(actuator.id.clone())); }
    }
    for tendon in &model.tendons {
        let id = tendon.id.clone();
        if !all.insert(id.clone()) { return Err(IntegrityError::DuplicateId(id)); }
        tendons.insert(id);
    }
    // Site targets are retained by source name until sites are modeled explicitly.
    for id in &all { if uuid::Uuid::parse_str(id).is_err() { return Err(IntegrityError::InvalidId(id.clone())); } }
    for body in &model.bodies {
        if let Some(parent) = &body.parent_id {
            if parent == &body.id { return Err(IntegrityError::SelfParent(body.id.clone())); }
            if !bodies.contains(parent.as_str()) {
                return Err(IntegrityError::MissingReference { owner: body.id.clone(), target: parent.clone() });
            }
        }
        let mut visited = HashSet::new();
        let mut cursor = Some(body.id.as_str());
        while let Some(id) = cursor {
            if !visited.insert(id) { return Err(IntegrityError::BodyCycle(body.id.clone())); }
            cursor = model.bodies.iter().find(|b| b.id == id).and_then(|b| b.parent_id.as_deref());
        }
    }
    for joint in &model.joints {
        if !bodies.contains(joint.child_body_id.as_str()) {
            return Err(IntegrityError::MissingReference { owner: joint.id.clone(), target: joint.child_body_id.clone() });
        }
        if let Some(parent) = &joint.parent_body_id {
            if !bodies.contains(parent.as_str()) {
                return Err(IntegrityError::MissingReference { owner: joint.id.clone(), target: parent.clone() });
            }
            if parent == &joint.child_body_id { return Err(IntegrityError::SelfParent(joint.child_body_id.clone())); }
        }
    }
    for actuator in &model.actuators {
        let target = match &actuator.target {
            TransmissionTarget::Joint(id) if !joints.contains(id.as_str()) => Some(id),
            TransmissionTarget::Tendon(id) if !tendons.contains(id) => Some(id),
            TransmissionTarget::Site(_) | TransmissionTarget::Unspecified => None,
            _ => None,
        };
        if let Some(target) = target {
            return Err(IntegrityError::MissingReference { owner: actuator.id.clone(), target: target.clone() });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{from_mjcf, from_urdf, SourceFormat};
    #[test]
    fn valid_fixtures_pass_integrity() {
        let urdf = include_str!("../../../tests/fixtures/urdf-regression/valid/02_two_link_revolute.urdf");
        let mjcf = include_str!("../../../tests/fixtures/mjcf-regression/valid/05_tendon_actuator.xml");
        assert_eq!(from_urdf(urdf).unwrap().source_format, SourceFormat::Urdf);
        validate_model(&from_urdf(urdf).unwrap()).unwrap();
        validate_model(&from_mjcf(mjcf).unwrap()).unwrap();
    }
    #[test]
    fn rejects_missing_body_reference() {
        let mut model = from_urdf(include_str!("../../../tests/fixtures/urdf-regression/valid/02_two_link_revolute.urdf")).unwrap();
        model.joints[0].child_body_id = "body:missing".into();
        assert!(matches!(validate_model(&model), Err(IntegrityError::MissingReference { .. })));
    }
    #[test]
    fn rejects_duplicate_id_and_cycle() {
        let mut model = from_urdf(include_str!("../../../tests/fixtures/urdf-regression/valid/02_two_link_revolute.urdf")).unwrap();
        model.bodies[1].id = model.bodies[0].id.clone();
        assert!(matches!(validate_model(&model), Err(IntegrityError::DuplicateId(_))));
        let mut model = from_urdf(include_str!("../../../tests/fixtures/urdf-regression/valid/02_two_link_revolute.urdf")).unwrap();
        model.bodies[0].parent_id = Some(model.bodies[1].id.clone());
        assert!(matches!(validate_model(&model), Err(IntegrityError::BodyCycle(_))));
    }
    #[test]
    fn rejects_broken_actuator_reference_and_version() {
        let mut model = from_mjcf(include_str!("../../../tests/fixtures/mjcf-regression/valid/02_hinge_motor.xml")).unwrap();
        model.actuators[0].target = TransmissionTarget::Joint("joint:missing".into());
        assert!(matches!(validate_model(&model), Err(IntegrityError::MissingReference { .. })));
        model.schema_version = 2;
        assert_eq!(validate_model(&model), Err(IntegrityError::UnsupportedSchema(2)));
    }
}
