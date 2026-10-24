//! MER-RESOURCE-001 implementation-first contract tests.
//! Expected to fail to compile until src/robot_catalog.rs is implemented and exposed.
//! Do not change assertions merely to make the suite green.

#[path = "../src/robot_catalog.rs"]
mod robot_catalog;

use robot_catalog::{RobotCatalog, RobotOrigin, RobotRepresentation};

#[test]
fn mer_res_tc_001_two_robots_have_distinct_ids() {
    let mut catalog = RobotCatalog::default();
    let a = catalog.create("A").unwrap();
    let b = catalog.create("B").unwrap();
    assert_ne!(a, b);
    assert_eq!(catalog.robots().len(), 2);
}

#[test]
fn mer_res_tc_002_rename_and_roundtrip_preserve_identity() {
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("Before").unwrap();
    catalog.rename(&id, "After").unwrap();
    let json = catalog.to_json().unwrap();
    let loaded = RobotCatalog::from_json(&json).unwrap();
    assert_eq!(loaded.robots()[0].robot_id, id);
    assert_eq!(loaded.robots()[0].name, "After");
}

#[test]
fn mer_res_tc_003_duplicate_allocates_new_identity() {
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("Original").unwrap();
    let copy = catalog.duplicate(&id, "Copy").unwrap();
    assert_ne!(id, copy);
    assert_eq!(catalog.robots()[0].name, "Original");
}

#[test]
fn mer_res_tc_004_shared_origin_does_not_merge() {
    let mut catalog = RobotCatalog::default();
    let origin = RobotOrigin {
        source_namespace: "vendor".into(),
        source_id: "model".into(),
    };
    let a = catalog
        .create_with_origin("A", Some(origin.clone()))
        .unwrap();
    let b = catalog.create_with_origin("B", Some(origin)).unwrap();
    assert_ne!(a, b);
    assert_eq!(catalog.robots().len(), 2);
}

#[test]
fn mer_res_tc_005_duplicate_ids_rejected_on_load() {
    let mut catalog = RobotCatalog::default();
    catalog.create("A").unwrap();
    let json = catalog.to_json().unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let duplicate = value["robots"][0].clone();
    value["robots"].as_array_mut().unwrap().push(duplicate);
    assert!(RobotCatalog::from_json(&value.to_string()).is_err());
    assert_eq!(catalog.robots().len(), 1);
}

#[test]
fn mer_res_tc_006_same_id_allowed_in_independent_projects() {
    let mut original = RobotCatalog::default();
    original.create("A").unwrap();
    let json = original.to_json().unwrap();
    let a = RobotCatalog::from_json(&json).unwrap();
    let b = RobotCatalog::from_json(&json).unwrap();
    assert_eq!(a.robots()[0].robot_id, b.robots()[0].robot_id);
}

#[test]
fn mer_res_tc_008_representation_stores_registry_id_not_path() {
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("A").unwrap();
    catalog
        .add_representation(
            &id,
            RobotRepresentation {
                registry_resource_id: "resource-1".into(),
                format: "urdf".into(),
                role: "exchange".into(),
            },
        )
        .unwrap();
    let json = catalog.to_json().unwrap();
    assert!(json.contains("registry_resource_id"));
    assert!(!json.contains("\\\"path\\\""));
    assert!(!json.contains("\\\"scope\\\""));
}

#[test]
fn mer_res_tc_011_no_implicit_urdf_mjcf_merging() {
    let mut catalog = RobotCatalog::default();
    let urdf = catalog.create("URDF").unwrap();
    let mjcf = catalog.create("MJCF").unwrap();
    assert_ne!(urdf, mjcf);
    assert_eq!(catalog.robots().len(), 2);
}

#[test]
fn mer_res_tc_013_generated_ids_are_canonical_uuid_v4() {
    use uuid::{Uuid, Version};
    let mut catalog = RobotCatalog::default();
    for index in 0..32 {
        let id = catalog.create(&format!("Robot {index}")).unwrap();
        let parsed = Uuid::parse_str(&id).expect("Robot ID must parse as UUID");
        assert_eq!(parsed.get_version(), Some(Version::Random));
        assert_eq!(id, parsed.hyphenated().to_string());
    }
}

#[test]
fn mer_cat_edit_tc_001_rename_preserves_identity_origin_and_registry_references() {
    let mut catalog = RobotCatalog::default();
    let origin = RobotOrigin {
        source_namespace: "vendor".into(),
        source_id: "model".into(),
    };
    let id = catalog.create_with_origin("Before", Some(origin)).unwrap();
    catalog
        .add_representation(
            &id,
            RobotRepresentation {
                registry_resource_id: "resource-shared".into(),
                format: "URDF".into(),
                role: "source".into(),
            },
        )
        .unwrap();
    let before = catalog.robots()[0].clone();
    catalog.rename(&id, "After").unwrap();
    let after = &catalog.robots()[0];
    assert_eq!(after.robot_id, before.robot_id);
    assert_eq!(after.origin, before.origin);
    assert_eq!(after.representations, before.representations);
    assert_eq!(after.name, "After");
}

#[test]
fn mer_cat_edit_tc_002_invalid_rename_does_not_mutate_catalog() {
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("Robot").unwrap();
    let before = catalog.clone();
    assert!(catalog.rename(&id, "  ").is_err());
    assert!(catalog.rename("missing", "Changed").is_err());
    assert_eq!(catalog, before);
}

#[test]
fn mer_cat_edit_tc_003_duplicate_shares_registry_id_but_not_robot_id() {
    let mut catalog = RobotCatalog::default();
    let origin = RobotOrigin {
        source_namespace: "vendor".into(),
        source_id: "model".into(),
    };
    let id = catalog
        .create_with_origin("Original", Some(origin))
        .unwrap();
    catalog
        .add_representation(
            &id,
            RobotRepresentation {
                registry_resource_id: "resource-shared".into(),
                format: "URDF".into(),
                role: "source".into(),
            },
        )
        .unwrap();
    let copy = catalog.duplicate(&id, "Copy").unwrap();
    assert_ne!(id, copy);
    assert_eq!(catalog.robots().len(), 2);
    assert_eq!(catalog.robots()[0].origin, catalog.robots()[1].origin);
    assert_eq!(
        catalog.robots()[0].representations,
        catalog.robots()[1].representations
    );
    assert_eq!(catalog.robots()[1].name, "Copy");
}

#[test]
fn mer_cat_edit_tc_004_invalid_duplicate_does_not_mutate_catalog() {
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("Robot").unwrap();
    let before = catalog.clone();
    assert!(catalog.duplicate(&id, "  ").is_err());
    assert!(catalog.duplicate("missing", "Copy").is_err());
    assert_eq!(catalog, before);
}

#[test]
fn mer_cat_edit_tc_005_remove_only_target_robot_and_preserve_shared_reference() {
    let mut catalog = RobotCatalog::default();
    let id = catalog.create("Original").unwrap();
    catalog
        .add_representation(
            &id,
            RobotRepresentation {
                registry_resource_id: "resource-shared".into(),
                format: "URDF".into(),
                role: "source".into(),
            },
        )
        .unwrap();
    let copy = catalog.duplicate(&id, "Copy").unwrap();
    catalog.remove(&id).unwrap();
    assert_eq!(catalog.robots().len(), 1);
    assert_eq!(catalog.robots()[0].robot_id, copy);
    assert_eq!(
        catalog.robots()[0].representations[0].registry_resource_id,
        "resource-shared"
    );
    let loaded = RobotCatalog::from_json(&catalog.to_json().unwrap()).unwrap();
    assert_eq!(loaded, catalog);
}

#[test]
fn mer_cat_edit_tc_006_unknown_remove_does_not_mutate_catalog() {
    let mut catalog = RobotCatalog::default();
    catalog.create("Robot").unwrap();
    let before = catalog.clone();
    assert!(catalog.remove("missing").is_err());
    assert_eq!(catalog, before);
}
