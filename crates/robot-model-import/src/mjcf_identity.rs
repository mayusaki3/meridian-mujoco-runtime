//! Conservative, read-only identity reconciliation across MJCF imports.
//! This produces a plan; it does not mutate either RobotModel.
use crate::model::RobotModel;
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
pub fn plan_named_identity_reuse(old: &RobotModel, new: &RobotModel) -> Result<IdentityPlan, ReconcileError> {
    use crate::model::SourceFormat;
    if old.source_format != SourceFormat::Mjcf || new.source_format != SourceFormat::Mjcf {
        return Err(ReconcileError::NotMjcf);
    }
    let all_ids = |m: &RobotModel| m.bodies.iter().map(|b| b.id.clone())
        .chain(m.joints.iter().map(|j| j.id.clone()))
        .chain(m.tendons.iter().map(|t| t.id.clone()))
        .chain(m.actuators.iter().map(|a| a.id.clone())).collect::<Vec<_>>();
    for model in [old, new] {
        let ids = all_ids(model);
        if ids.iter().collect::<HashSet<_>>().len() != ids.len() {
            return Err(ReconcileError::DuplicateId);
        }
    }
    let mut reuse = BTreeMap::new();
    let mut unresolved = Vec::new();
    let old_bodies: HashMap<_, _> = old.bodies.iter().filter(|b| !b.name.starts_with('@'))
        .map(|b| (b.name.as_str(), b)).collect();
    let old_body_counts = old.bodies.iter().filter(|b| !b.name.starts_with('@'))
        .fold(HashMap::new(), |mut m, b| { *m.entry(b.name.as_str()).or_insert(0usize) += 1; m });
    let new_body_counts = new.bodies.iter().filter(|b| !b.name.starts_with('@'))
        .fold(HashMap::new(), |mut m, b| { *m.entry(b.name.as_str()).or_insert(0usize) += 1; m });
    // Parent-first iteration is not required: the final parent identity check
    // uses names of both parents, rejecting unnamed or changed parents.
    for b in &new.bodies {
        let candidate = old_bodies.get(b.name.as_str()).copied().filter(|_| !b.name.starts_with('@'))
            .filter(|_| old_body_counts.get(b.name.as_str()) == Some(&1))
            .filter(|_| new_body_counts.get(b.name.as_str()) == Some(&1));
        let same_parent = |o: &crate::model::Body| {
            match (&o.parent_id, &b.parent_id) {
                (None, None) => true,
                (Some(op), Some(np)) => {
                    let a = old.bodies.iter().find(|x| &x.id == op);
                    let z = new.bodies.iter().find(|x| &x.id == np);
                    matches!((a,z), (Some(a),Some(z)) if !a.name.starts_with('@') && a.name == z.name
                        && old_body_counts.get(a.name.as_str()) == Some(&1)
                        && new_body_counts.get(z.name.as_str()) == Some(&1))
                }
                _ => false,
            }
        };
        if let Some(o) = candidate.filter(|o| same_parent(o)) {
            reuse.insert(b.id.clone(), o.id.clone());
        } else { unresolved.push(b.id.clone()); }
    }
    let old_joints: HashMap<_, _> = old.joints.iter().filter_map(|j| j.name.as_deref().map(|n| (n,j))).collect();
    let count_joints = |m: &RobotModel| m.joints.iter().filter_map(|j| j.name.clone())
        .fold(HashMap::new(), |mut counts, name| { *counts.entry(name).or_insert(0usize) += 1; counts });
    let oc = count_joints(old);
    let nc = count_joints(new);
    for j in &new.joints {
        let candidate = j.name.as_deref().and_then(|n| old_joints.get(n).copied()
            .filter(|_| oc.get(n) == Some(&1) && nc.get(n) == Some(&1)));
        let owner = reuse.get(&j.child_body_id);
        if let Some(o) = candidate.filter(|o| o.kind == j.kind && owner == Some(&o.child_body_id)) {
            reuse.insert(j.id.clone(), o.id.clone());
        } else { unresolved.push(j.id.clone()); }
    }
    // Tendons have required names; actuator identity also requires matching
    // transmission target, so defer actuator reconciliation to a later phase.
    let old_tendons: HashMap<_, _> = old.tendons.iter().map(|t| (t.name.as_str(),t)).collect();
    let count_tendons = |m: &RobotModel| m.tendons.iter().fold(HashMap::new(), |mut counts, t| {
        *counts.entry(t.name.clone()).or_insert(0usize) += 1; counts
    });
    let ot = count_tendons(old);
    let nt = count_tendons(new);
    for t in &new.tendons {
        if let Some(o) = old_tendons.get(t.name.as_str()).filter(|_| ot.get(t.name.as_str()) == Some(&1) && nt.get(t.name.as_str()) == Some(&1)) {
            reuse.insert(t.id.clone(), o.id.clone());
        } else { unresolved.push(t.id.clone()); }
    }
    unresolved.extend(new.actuators.iter().map(|a| a.id.clone()));
    Ok(IdentityPlan { reuse, unresolved })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::from_mjcf;
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
