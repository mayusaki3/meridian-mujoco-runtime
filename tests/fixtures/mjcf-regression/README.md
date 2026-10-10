# MJCF regression fixtures

Synthetic MuJoCo XML test fixtures for a future MJCF ↔ ROBOT-Model pipeline. Checked into Git alongside URDF fixtures.

## Status

**Fixture data only**. No MJCF import/export or validation is implemented by this commit. The invalid cases are *expected targets* for future tests, not claims about current behavior.

| Fixture | Purpose |
|---|---|
| valid/01_minimal.xml | Minimal body and geometry |
| valid/02_hinge_motor.xml | Nested bodies, hinge joint, motor gear/control range |
| valid/03_slide_position.xml | Slide joint and position servo |
| valid/04_velocity_general.xml | Velocity/general actuator types; multiple actuators on one joint |
| valid/05_tendon_actuator.xml | Fixed tendon coupling two joints |
| valid/06_defaults_contact_sensor.xml | Defaults, contact friction, simulation options, sensors |
| valid/07_extension_comments.xml | Comments, custom data, vendor namespace/attributes and source preservation |
| valid/08_nested_bodies_inertial.xml | Body hierarchy, ball joint, inertial parameters |
| invalid/01_malformed_xml.xml | Broken XML syntax |
| invalid/02_wrong_root.xml | Wrong document root |
| invalid/03_missing_actuator_target.xml | Actuator targets nonexistent joint |
| invalid/04_duplicate_body_name.xml | Duplicate body names |
| invalid/05_unknown_joint_type.xml | Unknown joint type |
| invalid/06_missing_tendon_reference.xml | Actuator targets nonexistent tendon |

## Planned validation

1. Parse MJCF and preserve original XML bytes.
2. Extract bodies, joints, actuators, tendons, sensors, and physical parameters.
3. Validate references and identifiers.
4. Save and reload canonical ROBOT-Model plus source mapping.
5. Export unchanged MJCF byte-identically.
6. After edits, reconstruct MJCF without losing unknown extensions or source-specific information.

**Caution:** MJCF is not interchangeable with URDF. A body can contain multiple joints, actuator transmissions can target tendons/sites, and MJCF default classes influence effective values. Some fixtures intentionally use external extension namespaces; compatibility with the MuJoCo engine is not asserted until engine-level tests are run.
