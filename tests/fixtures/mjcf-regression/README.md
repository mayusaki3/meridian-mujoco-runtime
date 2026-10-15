# MJCF regression fixtures

Synthetic MuJoCo XML fixtures for MJCF parsing, original-source retention, and model import tests. Keep these fixtures for automated coverage; they are not a per-fixture GUI checklist.

## Current implementation and test ownership

The original fixture note described these as fixture data only. That statement is outdated: the repository now implements MJCF inspection and unchanged-source retention in `crates/robot-model-import/src/mjcf.rs`, and project snapshot import, load, save, and reimport in `src/mjcf_project.rs`.

- Automated tests own detailed valid/invalid fixture coverage and the snapshot persistence/reimport logic.
- GUI smoke tests own command wiring, file dialogs, notifications, and integration with an open project.
- Do not duplicate every MJCF fixture in manual GUI testing.
- The current GUI offers **MJCF Import** and **MJCF Reimport**; there is no corresponding MJCF Export GUI command in the reviewed `src/main.rs`. Do not invent an export GUI roundtrip test.
- Original-source retention and snapshot persistence do not by themselves prove a canonical edited-model exporter or successful MuJoCo engine compilation.

## Fixture inventory

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

## Automated regression

Run from the repository root:

```powershell
cargo test --all-targets
cargo test --manifest-path crates/robot-model-import/Cargo.toml --all-targets
```

The fixture-backed tests in `crates/robot-model-import/src/mjcf.rs` exercise inspection and original XML retention. The `src/mjcf_project.rs` tests cover snapshot persistence, reimport identity/provenance, and non-overwrite on failure. This is not a claim that every fixture is exercised through the project snapshot pipeline or that full MJCF semantics are validated.

## GUI smoke test plan — pending

Use a dedicated test project; do not overwrite a project containing valuable MJCF snapshots.

| Case | UI-only verification | Expected outcome |
|---|---|---|
| MJCF-GUI-001 | Import a representative valid MJCF through Data > MJCF Import | Success notification and project snapshot created |
| MJCF-GUI-002 | Save/close/reopen project, then use MJCF Reimport with a valid source | Reimport success notification; snapshot remains loadable |
| MJCF-GUI-003 | Attempt malformed MJCF import/reimport as applicable | Error notification; previous snapshot unchanged |
| MJCF-GUI-004 | Cancel import/reimport file dialog | No change and no error |
| MJCF-GUI-005 | Invoke import when a snapshot already exists | Existing-source protection or warning according to current command behavior |

Record actual logs and snapshot hashes when executing; none of these GUI cases is marked PASS by the automated tests alone.

## Limitations

- MJCF is not interchangeable with URDF: bodies can have multiple joints, actuators may target tendons or sites, and default classes affect effective values.
- External extension namespaces and references in fixtures do not guarantee MuJoCo engine compatibility.
- Full semantic validation and edited canonical export remain separate development scopes.
