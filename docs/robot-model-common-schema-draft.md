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

## MJCF explicit-value validation stage

The read-only MJCF inspector validates selected **explicit** physical numeric attributes: positive mass and diagonal inertia, finite six-component full inertia, ordered joint/actuator ranges, and nonnegative damping, armature, friction loss, kp and kv. Missing attributes remain permissible at this stage. Invalid numeric tokens, nonfinite values and incorrect component counts are rejected.

This is intentionally **not** a MuJoCo compiler equivalence guarantee. In particular, defaults/class resolution, includes, inertia positive-definiteness and triangle inequalities, actuator subtype-specific restrictions, and full model physics validation require later stages. Raw XML is still retained for exact unchanged-source export.

### MJCF default classes and unresolved includes

Static inspection checks duplicate named `<default class="...">` declarations and unknown `class` / `childclass` references in the provided XML document. It does not calculate inherited attribute values or expand includes. `inspect_mjcf_standalone` rejects unresolved `<include>` elements; `inspect_mjcf` remains a partial, read-only inspection API for compatibility. Neither API validates file existence, include cycles, transitive references or preserves an external-file bundle. These require a source-aware resolver and later MuJoCo compiler validation.

### Resolved MJCF to ROBOT-Model bundle

`mjcf_model_bundle::from_mjcf_file(path)` resolves includes, validates the expanded MJCF, projects a new UUID-identified `RobotModel`, and runs `validate_model`. The `MjcfModelBundle` holds the model, canonical entry path and an exact-text map of all source XML files. `RobotModel.original_xml` contains the **expanded snapshot**, whereas `MjcfModelBundle.entry_xml()` returns the unmodified entry file. JSON roundtrip preserves model and element IDs as well as source text. Re-import intentionally creates fresh IDs.

This is not yet a relocatable project package: canonical absolute paths are stored. Nor does it provide edited multi-file export, element-to-source provenance mapping, or MuJoCo compiler-equivalent expansion.

### Portable MJCF source snapshot (opt-in)

`mjcf_portable::PortableMjcfBundle::from_bundle(bundle, source_root)` converts canonical source paths into relative paths rooted at the selected directory. A source outside this root is rejected, not silently copied or rewritten. The serialized snapshot includes the UUID-preserving `RobotModel`, relative entry path, and original source XML files.

`restore_to(destination)` requires a **nonexistent** destination directory, writes the original XML file tree, re-resolves the MJCF includes and checks the expanded XML against the saved model snapshot. It does not re-import or regenerate model IDs. Path components such as `..`, absolute paths and platform prefixes are rejected. Failed restore may leave a partially written destination, so this API is not yet transactional. Original source XML is stored as UTF-8 strings; arbitrary binary assets and external mesh dependencies are not packaged. Element-to-source provenance is a separate future step.

### Read-only MJCF element provenance

`mjcf_provenance::map_named_elements(portable_bundle)` produces a separate `MjcfProvenance` mapping domain UUIDs to relative source path, XML tag, explicit name, and byte offset in the original source. It covers named Body, Joint, Actuator and Tendon elements, including included files. The map can be serialized to JSON and checked against the same portable bundle.

This is **not** an editable XML source mapping: offsets become stale if source text changes. Unnamed bodies/joints/actuators are currently rejected, and source name collisions are rejected rather than guessed. The existing `RobotModel` and portable bundle schemas are unchanged. Future work must use stable source-element identities and handle unnamed elements before enabling edits.

### Single-source unnamed MJCF provenance (opt-in)

`mjcf_provenance::map_single_source_elements` maps Body, Joint, Tendon, and Actuator UUIDs to original element offsets by the same traversal order used in projection. It accepts unnamed Body, Joint and Actuator elements, recording an empty `source_name` for them. It requires exactly one source file and exact equality between the entry XML and `RobotModel.original_xml`; it rejects multi-file includes and changed source. This is read-only and separate from `map_named_elements`.

This ordering-based mapping must **not** be applied to included sources. Source spans through include expansion are required before unnamed multi-file elements can be mapped safely. `validate_against` and `validate_source_locations` currently use the named-element API and are not validators for this opt-in unnamed mapping.

### Nested MJCF include source spans (opt-in)

`mjcf_source_spans::source_byte_map` reconstructs the existing include expansion and records an original relative file path and byte offset for each expanded XML byte. It requires exact equality with the saved `RobotModel.original_xml`. `mjcf_provenance::map_included_elements` then associates projected Body, Joint, Tendon and Actuator UUIDs with the original opening tag, including unnamed elements in nested includes. This is read-only; original files and the existing bundle schema are unchanged.

The span-based map is reconstructed from the stored source snapshot. Its offsets are not durable identifiers across source edits, and `MjcfProvenance::validate_against` remains the older named-element validator; do not use it to validate span-based unnamed mappings. Future work: dedicated validation of span-based provenance and explicit stable identity/reconciliation across source edits.
