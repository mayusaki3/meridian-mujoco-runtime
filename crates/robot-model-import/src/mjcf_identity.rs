//! Conservative, read-only identity reconciliation across MJCF imports.
//! This produces a plan; it does not mutate either RobotModel.
use crate::model::{RobotModel, TransmissionTarget};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityPlan {
    /// New domain UUID -> existing domain UUID.
    pub reuse: BTreeMap<String, String>,
    /// New UUIDs that cannot safely reuse an existing identity.
    pub unresolved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileError {
    NotMjcf,
    DuplicateId,
}

/// Phase 1: only unique, explicitly named elements can inherit IDs.
/// Body parents must also be uniquely matched; joint body ownership and
/// joint kind must agree. Unnamed elements are deliberately unresolved.
pub fn plan_named_identity_reuse(
    old: &RobotModel,
    new: &RobotModel,
) -> Result<IdentityPlan, ReconcileError> {
    use crate::model::SourceFormat;
    if old.source_format != SourceFormat::Mjcf || new.source_format != SourceFormat::Mjcf {
        return Err(ReconcileError::NotMjcf);
    }
    let all_ids = |m: &RobotModel| {
        m.bodies
            .iter()
            .map(|b| b.id.clone())
            .chain(m.joints.iter().map(|j| j.id.clone()))
            .chain(m.tendons.iter().map(|t| t.id.clone()))
            .chain(m.actuators.iter().map(|a| a.id.clone()))
            .collect::<Vec<_>>()
    };
    for model in [old, new] {
        let ids = all_ids(model);
        if ids.iter().collect::<HashSet<_>>().len() != ids.len() {
            return Err(ReconcileError::DuplicateId);
        }
    }
    let mut reuse = BTreeMap::new();
    let mut unresolved = Vec::new();
    let old_bodies: HashMap<_, _> = old
        .bodies
        .iter()
        .filter(|b| !b.name.starts_with('@'))
        .map(|b| (b.name.as_str(), b))
        .collect();
    let old_body_counts =
        old.bodies
            .iter()
            .filter(|b| !b.name.starts_with('@'))
            .fold(HashMap::new(), |mut m, b| {
                *m.entry(b.name.as_str()).or_insert(0usize) += 1;
                m
            });
    let new_body_counts =
        new.bodies
            .iter()
            .filter(|b| !b.name.starts_with('@'))
            .fold(HashMap::new(), |mut m, b| {
                *m.entry(b.name.as_str()).or_insert(0usize) += 1;
                m
            });
    // Reconcile parents first. A child is safe only if its parent is itself
    // safely reused; this checks the entire ancestor chain transitively.
    let mut pending: Vec<_> = new.bodies.iter().collect();
    while !pending.is_empty() {
        let mut next = Vec::new();
        let mut progressed = false;
        for b in pending {
            let candidate = old_bodies
                .get(b.name.as_str())
                .copied()
                .filter(|_| !b.name.starts_with('@'))
                .filter(|_| old_body_counts.get(b.name.as_str()) == Some(&1))
                .filter(|_| new_body_counts.get(b.name.as_str()) == Some(&1));
            let parent_ready = b
                .parent_id
                .as_ref()
                .is_none_or(|p| reuse.contains_key(p) || !new.bodies.iter().any(|x| &x.id == p));
            if !parent_ready {
                next.push(b);
                continue;
            }
            progressed = true;
            let valid = candidate.filter(|o| match (&o.parent_id, &b.parent_id) {
                (None, None) => true,
                (Some(op), Some(np)) => reuse.get(np) == Some(op),
                _ => false,
            });
            if let Some(o) = valid {
                reuse.insert(b.id.clone(), o.id.clone());
            } else {
                unresolved.push(b.id.clone());
            }
        }
        if !progressed {
            unresolved.extend(next.into_iter().map(|b| b.id.clone()));
            break;
        }
        pending = next;
    }
    let old_joints: HashMap<_, _> = old
        .joints
        .iter()
        .filter_map(|j| j.name.as_deref().map(|n| (n, j)))
        .collect();
    let count_joints = |m: &RobotModel| {
        m.joints
            .iter()
            .filter_map(|j| j.name.clone())
            .fold(HashMap::new(), |mut counts, name| {
                *counts.entry(name).or_insert(0usize) += 1;
                counts
            })
    };
    let oc = count_joints(old);
    let nc = count_joints(new);
    for j in &new.joints {
        let candidate = j.name.as_deref().and_then(|n| {
            old_joints
                .get(n)
                .copied()
                .filter(|_| oc.get(n) == Some(&1) && nc.get(n) == Some(&1))
        });
        let owner = reuse.get(&j.child_body_id);
        if let Some(o) = candidate.filter(|o| o.kind == j.kind && owner == Some(&o.child_body_id)) {
            reuse.insert(j.id.clone(), o.id.clone());
        } else {
            unresolved.push(j.id.clone());
        }
    }
    // Tendons have required names; actuator identity also requires matching
    // transmission target, so defer actuator reconciliation to a later phase.
    let old_tendons: HashMap<_, _> = old.tendons.iter().map(|t| (t.name.as_str(), t)).collect();
    let count_tendons = |m: &RobotModel| {
        m.tendons.iter().fold(HashMap::new(), |mut counts, t| {
            *counts.entry(t.name.clone()).or_insert(0usize) += 1;
            counts
        })
    };
    let ot = count_tendons(old);
    let nt = count_tendons(new);
    for t in &new.tendons {
        if let Some(o) = old_tendons
            .get(t.name.as_str())
            .filter(|_| ot.get(t.name.as_str()) == Some(&1) && nt.get(t.name.as_str()) == Some(&1))
        {
            reuse.insert(t.id.clone(), o.id.clone());
        } else {
            unresolved.push(t.id.clone());
        }
    }
    // Actuators need a unique explicit name, matching kind, and a target
    // that has already been reconciled (or the same named site).
    let old_actuators: HashMap<_, _> = old
        .actuators
        .iter()
        .filter_map(|a| a.name.as_deref().map(|n| (n, a)))
        .collect();
    let count_actuators = |m: &RobotModel| {
        m.actuators.iter().filter_map(|a| a.name.clone()).fold(
            HashMap::new(),
            |mut counts, name| {
                *counts.entry(name).or_insert(0usize) += 1;
                counts
            },
        )
    };
    let oa = count_actuators(old);
    let na = count_actuators(new);
    for a in &new.actuators {
        let candidate = a.name.as_deref().and_then(|n| {
            old_actuators
                .get(n)
                .copied()
                .filter(|_| oa.get(n) == Some(&1) && na.get(n) == Some(&1))
        });
        let same_target = |other: &crate::model::Actuator| match (&a.target, &other.target) {
            (TransmissionTarget::Joint(n), TransmissionTarget::Joint(o))
            | (TransmissionTarget::Tendon(n), TransmissionTarget::Tendon(o)) => {
                reuse.get(n) == Some(o)
            }
            (TransmissionTarget::Site(n), TransmissionTarget::Site(o)) => n == o,
            _ => false,
        };
        if let Some(o) = candidate.filter(|o| o.kind == a.kind && same_target(o)) {
            reuse.insert(a.id.clone(), o.id.clone());
        } else {
            unresolved.push(a.id.clone());
        }
    }
    Ok(IdentityPlan { reuse, unresolved })
}

/// Errors while validating or applying an identity plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyIdentityError {
    InvalidModel(crate::integrity::IntegrityError),
    InvalidPlan,
}

/// Validate and apply a reconciliation plan atomically, returning a new model.
/// Unresolved elements retain their freshly imported UUIDs.
pub fn apply_identity_plan(
    old: &RobotModel,
    new: &RobotModel,
    plan: &IdentityPlan,
) -> Result<RobotModel, ApplyIdentityError> {
    use crate::integrity::validate_model;
    use crate::model::SourceFormat;
    validate_model(old).map_err(ApplyIdentityError::InvalidModel)?;
    validate_model(new).map_err(ApplyIdentityError::InvalidModel)?;
    if old.source_format != SourceFormat::Mjcf || new.source_format != SourceFormat::Mjcf {
        return Err(ApplyIdentityError::InvalidPlan);
    }
    // Recompute the authoritative plan: a caller cannot forge a match that
    // bypasses name, ancestry, kind or transmission-target checks.
    let expected =
        plan_named_identity_reuse(old, new).map_err(|_| ApplyIdentityError::InvalidPlan)?;
    if &expected != plan {
        return Err(ApplyIdentityError::InvalidPlan);
    }

    let old_ids: HashSet<&str> = old
        .bodies
        .iter()
        .map(|x| x.id.as_str())
        .chain(old.joints.iter().map(|x| x.id.as_str()))
        .chain(old.tendons.iter().map(|x| x.id.as_str()))
        .chain(old.actuators.iter().map(|x| x.id.as_str()))
        .collect();
    let mut resulting_ids = HashSet::new();
    for id in new
        .bodies
        .iter()
        .map(|x| &x.id)
        .chain(new.joints.iter().map(|x| &x.id))
        .chain(new.tendons.iter().map(|x| &x.id))
        .chain(new.actuators.iter().map(|x| &x.id))
    {
        let result = plan.reuse.get(id).unwrap_or(id);
        if !resulting_ids.insert(result.as_str()) {
            return Err(ApplyIdentityError::InvalidPlan);
        }
        // An unresolved new UUID must not accidentally alias an old identity.
        if !plan.reuse.contains_key(id) && old_ids.contains(id.as_str()) {
            return Err(ApplyIdentityError::InvalidPlan);
        }
    }

    let remap = |id: &str| plan.reuse.get(id).cloned().unwrap_or_else(|| id.to_owned());
    let mut result = new.clone();
    for b in &mut result.bodies {
        b.parent_id = b.parent_id.as_deref().map(&remap);
        b.id = remap(&b.id);
    }
    for j in &mut result.joints {
        j.parent_body_id = j.parent_body_id.as_deref().map(&remap);
        j.child_body_id = remap(&j.child_body_id);
        j.id = remap(&j.id);
    }
    for t in &mut result.tendons {
        t.id = remap(&t.id);
    }
    for a in &mut result.actuators {
        a.target = match &a.target {
            TransmissionTarget::Joint(id) => TransmissionTarget::Joint(remap(id)),
            TransmissionTarget::Tendon(id) => TransmissionTarget::Tendon(remap(id)),
            other => other.clone(),
        };
        a.id = remap(&a.id);
    }
    validate_model(&result).map_err(ApplyIdentityError::InvalidModel)?;
    Ok(result)
}

/// Outcome of reimporting an MJCF snapshot while preserving safe identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReimportResult {
    pub model: RobotModel,
    pub plan: IdentityPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReimportError {
    Import(crate::model::ModelImportError),
    Reconcile(ReconcileError),
    Apply(ApplyIdentityError),
}

/// Reimport MJCF source into an existing MJCF model without modifying it.
/// The existing model ID is retained; unmatched elements keep fresh IDs.
/// On any failure, the previous model remains unchanged.
pub fn reimport_mjcf_with_identity(
    previous: &RobotModel,
    xml: &str,
) -> Result<ReimportResult, ReimportError> {
    use crate::model::SourceFormat;
    if previous.source_format != SourceFormat::Mjcf {
        return Err(ReimportError::Reconcile(ReconcileError::NotMjcf));
    }
    let imported = crate::model::from_mjcf(xml).map_err(ReimportError::Import)?;
    let plan = plan_named_identity_reuse(previous, &imported).map_err(ReimportError::Reconcile)?;
    let mut model =
        apply_identity_plan(previous, &imported, &plan).map_err(ReimportError::Apply)?;
    model.model_id = previous.model_id.clone();
    Ok(ReimportResult { model, plan })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::from_mjcf;
    #[test]
    fn reimport_keeps_model_and_matched_ids_with_new_xml() {
        let before = r#"<mujoco><worldbody><body name="b"><joint name="j"/></body></worldbody><actuator><motor name="drive" joint="j"/></actuator></mujoco>"#;
        let after = r#"<mujoco> <worldbody><body name="b"><joint name="j"/></body><body name="added"/></worldbody><actuator><motor name="drive" joint="j"/></actuator></mujoco>"#;
        let previous = from_mjcf(before).unwrap();
        let outcome = reimport_mjcf_with_identity(&previous, after).unwrap();
        assert_eq!(outcome.model.model_id, previous.model_id);
        assert_eq!(outcome.model.original_xml, after);
        assert_eq!(outcome.model.bodies[0].id, previous.bodies[0].id);
        assert_eq!(outcome.model.joints[0].id, previous.joints[0].id);
        assert_eq!(outcome.model.actuators[0].id, previous.actuators[0].id);
        assert_eq!(outcome.model.joints[0].child_body_id, previous.bodies[0].id);
        assert_eq!(
            outcome.model.actuators[0].target,
            previous.actuators[0].target
        );
        assert_eq!(outcome.plan.reuse.len(), 3);
        assert_eq!(
            outcome.plan.unresolved,
            vec![outcome.model.bodies[1].id.clone()]
        );
        assert_ne!(outcome.model.bodies[1].id, previous.bodies[0].id);
        crate::integrity::validate_model(&outcome.model).unwrap();
    }
    #[test]
    fn reimport_rejects_invalid_xml_and_non_mjcf_without_changing_previous() {
        let previous =
            from_mjcf(r#"<mujoco><worldbody><body name="b"/></worldbody></mujoco>"#).unwrap();
        let snapshot = previous.clone();
        assert!(matches!(
            reimport_mjcf_with_identity(&previous, "<mujoco>"),
            Err(ReimportError::Import(_))
        ));
        assert_eq!(previous, snapshot);
        let urdf =
            crate::model::from_urdf(r#"<robot name="r"><link name="base"/></robot>"#).unwrap();
        assert_eq!(
            reimport_mjcf_with_identity(&urdf, &previous.original_xml),
            Err(ReimportError::Reconcile(ReconcileError::NotMjcf))
        );
    }
    #[test]
    fn reimport_keeps_new_identity_for_changed_joint_and_actuator() {
        let before = r#"<mujoco><worldbody><body name="b"><joint name="j"/></body></worldbody><actuator><motor name="drive" joint="j"/></actuator></mujoco>"#;
        let after = r#"<mujoco><worldbody><body name="b"><joint name="j" type="slide"/></body></worldbody><actuator><motor name="drive" joint="j"/></actuator></mujoco>"#;
        let previous = from_mjcf(before).unwrap();
        let outcome = reimport_mjcf_with_identity(&previous, after).unwrap();
        assert_eq!(outcome.model.bodies[0].id, previous.bodies[0].id);
        assert_ne!(outcome.model.joints[0].id, previous.joints[0].id);
        assert_ne!(outcome.model.actuators[0].id, previous.actuators[0].id);
        assert_eq!(outcome.plan.reuse.len(), 1);
        assert_eq!(outcome.plan.unresolved.len(), 2);
        assert_eq!(
            outcome.model.actuators[0].target,
            TransmissionTarget::Joint(outcome.model.joints[0].id.clone())
        );
    }
    #[test]
    fn applying_plan_preserves_references_and_source() {
        let xml = r#"<mujoco><worldbody><body name="b"><joint name="j"/></body></worldbody><actuator><motor name="drive" joint="j"/></actuator></mujoco>"#;
        let old = from_mjcf(xml).unwrap();
        let new = from_mjcf(xml).unwrap();
        let plan = plan_named_identity_reuse(&old, &new).unwrap();
        let applied = apply_identity_plan(&old, &new, &plan).unwrap();
        assert_eq!(applied.bodies[0].id, old.bodies[0].id);
        assert_eq!(applied.joints[0].id, old.joints[0].id);
        assert_eq!(applied.actuators[0].id, old.actuators[0].id);
        assert_eq!(applied.joints[0].child_body_id, old.bodies[0].id);
        assert_eq!(applied.actuators[0].target, old.actuators[0].target);
        assert_eq!(applied.original_xml, new.original_xml);
        assert_eq!(new.original_xml, xml);
    }
    #[test]
    fn applying_plan_rejects_forgery_and_collision_without_mutation() {
        let xml =
            r#"<mujoco><worldbody><body name="b"><joint name="j"/></body></worldbody></mujoco>"#;
        let old = from_mjcf(xml).unwrap();
        let new = from_mjcf(xml).unwrap();
        let untouched = new.clone();
        let mut plan = plan_named_identity_reuse(&old, &new).unwrap();
        plan.reuse
            .insert(new.joints[0].id.clone(), old.bodies[0].id.clone());
        assert_eq!(
            apply_identity_plan(&old, &new, &plan),
            Err(ApplyIdentityError::InvalidPlan)
        );
        assert_eq!(new, untouched);
        let mut plan = plan_named_identity_reuse(&old, &new).unwrap();
        plan.unresolved.push(new.bodies[0].id.clone());
        assert_eq!(
            apply_identity_plan(&old, &new, &plan),
            Err(ApplyIdentityError::InvalidPlan)
        );
    }
    #[test]
    fn ancestor_change_blocks_descendant_reuse() {
        let a = from_mjcf(r#"<mujoco><worldbody><body name="a"><body name="middle"><body name="leaf"/></body></body><body name="b"/></worldbody></mujoco>"#).unwrap();
        let b = from_mjcf(r#"<mujoco><worldbody><body name="a"/><body name="b"><body name="middle"><body name="leaf"/></body></body></worldbody></mujoco>"#).unwrap();
        let plan = plan_named_identity_reuse(&a, &b).unwrap();
        assert_eq!(plan.reuse.len(), 2);
        assert_eq!(plan.unresolved.len(), 2);
    }
    #[test]
    fn named_actuator_requires_reconciled_target() {
        let a = from_mjcf(r#"<mujoco><worldbody><body name="b"><joint name="j"/></body></worldbody><actuator><motor name="drive" joint="j"/></actuator></mujoco>"#).unwrap();
        let b = from_mjcf(r#"<mujoco> <worldbody><body name="b"><joint name="j"/></body></worldbody><actuator><motor name="drive" joint="j"/></actuator></mujoco>"#).unwrap();
        let plan = plan_named_identity_reuse(&a, &b).unwrap();
        assert_eq!(plan.reuse.len(), 3);
        assert!(plan.unresolved.is_empty());
        let c = from_mjcf(r#"<mujoco><worldbody><body name="b"><joint name="j" type="slide"/></body></worldbody><actuator><motor name="drive" joint="j"/></actuator></mujoco>"#).unwrap();
        let plan = plan_named_identity_reuse(&a, &c).unwrap();
        assert_eq!(plan.reuse.len(), 1);
        assert_eq!(plan.unresolved.len(), 2);
    }
    #[test]
    fn named_body_joint_tendon_survive_whitespace_changes() {
        let a = from_mjcf(r#"<mujoco><worldbody><body name="arm"><joint name="hinge"/></body></worldbody><tendon><fixed name="rope"/></tendon></mujoco>"#).unwrap();
        let b = from_mjcf(r#"<mujoco> <worldbody><body name="arm"><joint name="hinge"/></body></worldbody><tendon><fixed name="rope"/></tendon></mujoco>"#).unwrap();
        let plan = plan_named_identity_reuse(&a, &b).unwrap();
        assert_eq!(plan.reuse.len(), 3);
        assert!(plan.unresolved.is_empty());
    }
    #[test]
    fn unnamed_and_changed_joint_kind_are_not_reused() {
        let a = from_mjcf(r#"<mujoco><worldbody><body name="b"><joint name="j"/><joint/></body><body/></worldbody></mujoco>"#).unwrap();
        let b = from_mjcf(r#"<mujoco><worldbody><body name="b"><joint name="j" type="slide"/><joint/></body><body/></worldbody></mujoco>"#).unwrap();
        let plan = plan_named_identity_reuse(&a, &b).unwrap();
        assert_eq!(plan.reuse.len(), 1);
        assert_eq!(plan.unresolved.len(), 3);
    }
    #[test]
    fn changed_parent_prevents_child_identity_reuse() {
        let a = from_mjcf(r#"<mujoco><worldbody><body name="a"><body name="child"/></body><body name="b"/></worldbody></mujoco>"#).unwrap();
        let b = from_mjcf(r#"<mujoco><worldbody><body name="a"/><body name="b"><body name="child"/></body></worldbody></mujoco>"#).unwrap();
        let plan = plan_named_identity_reuse(&a, &b).unwrap();
        assert_eq!(plan.reuse.len(), 2);
        assert_eq!(plan.unresolved.len(), 1);
    }
}
