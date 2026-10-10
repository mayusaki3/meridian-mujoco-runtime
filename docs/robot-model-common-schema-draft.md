# ROBOT-Model common schema — design draft

Status: design draft (not yet the persisted format or implemented converter).

## Goal

One canonical model shared by URDF import/export, MJCF import/export and SysId. Preserve each source document separately to permit lossless unchanged-source export.

## Proposed entities

- `RobotModel`: schema_version, model_id, name, bodies, joints, geometries, actuators, tendons, sensors, simulation, parameter_metadata, sources.
- `Body`: stable ID, display name, parent attachment(s), inertial properties, visual and collision geometry references.
- `Joint`: stable ID, type, parent and child body IDs, pose/axis, limits, damping/friction. A joint is **not** an actuator.
- `Actuator`: stable ID, actuator type, transmission target (joint, tendon, site, or other supported target), gear/transmission parameters, control/force limits, gain/bias and dynamic parameters.
- `Tendon`: stable ID, routing or fixed-joint coefficients.
- `Sensor`: stable ID, sensor type and measurement target.
- `Simulation`: timestep, gravity, contact and solver options; source-specific settings can remain in source mapping.
- `ParameterMetadata`: path to parameter, origin (unknown/imported/default/identified), units, optional uncertainty, bounds and SysId evidence reference.
- `SourceDocument`: format (URDF/MJCF), original XML bytes, source-specific element identity/ordering, preserved unsupported fragments and source mapping.

## Constraints

1. Stable IDs must not depend on names or XML order.
2. An MJCF body may have multiple joints; avoid a forced URDF-like one-joint-per-body assumption.
3. Actuator-to-joint is not 1:1; tendon/site transmissions must be representable.
4. Preserve raw source values separately from effective values (MJCF defaults/classes may change effective properties).
5. Unknown/unsupported XML must not be discarded. Edited export may be refused when safe reconstruction is impossible.
6. Do not silently invent actuator parameters missing from URDF; distinguish unknown from default and identified.
7. The model must not assume URDF and MJCF coordinate, joint, or contact semantics are identical.
8. Keep exact unchanged-source export separate from regenerated export after model edits.

## Conversion stages

1. URDF/MJCF source inspection and unchanged roundtrip tests.
2. Source-neutral structural graph and source mapping.
3. Canonical physical and actuator fields with explicit provenance.
4. Model persistence and reload.
5. Edited export with preservation and conflict checks.
6. SysId parameter updates and simulation integration.

## Fixtures

- `tests/fixtures/urdf-regression/`
- `tests/fixtures/mjcf-regression/`

MJCF fixtures are committed as data first. Parser, validator, and MuJoCo engine compatibility tests are follow-up tasks.

## Agreed identity, import, merge, and SysId policy (2026-10-10)

- Identity is two-level: `Model ID + Element ID`. Both are UUIDs owned by ROBOT-Model, not URDF/MJCF XML names or positions.
- Element IDs are unique within one model; different models may contain identical Element IDs. Global references must use the pair.
- Rename, reorder, parameter editing, and persistence retain IDs. Importing as a new model issues a new Model ID and new Element IDs.
- If distributed models contain colliding identifiers, user-directed re-identification will be supported in a later version, including remapping all internal references.
- Each URDF/MJCF import **adds** an independent ROBOT-Model to the project. Import does not silently reconcile or merge existing models.
- Future merge: user selects two existing models, compares them side-by-side, explicitly resolves element correspondence and conflicts, and creates a **third, new model**. The two input models remain unchanged, enabling retries.
- Merge and reimport reconciliation are **out of scope for the first version**. Merge identity remapping, revision provenance, and collision policy must be designed before implementing merge.
- After a ROBOT-Model is prepared, the user will select/extract a SysId model from it. The SysId selection must refer back to the source Model ID and stable Element IDs; it is not an implicit replacement of the complete ROBOT-Model.
- Current UUID assignment is a first-import identity mechanism, not an implemented merge, reimport, or SysId extraction workflow.
