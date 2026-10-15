# Meridian MuJoCo Runtime — URDF test fixtures

These fixtures are synthetic and created for import/export regression testing. No external mesh is bundled.

## Test responsibility and non-duplication policy

- **Automated regression tests** own fixture-by-fixture validation of unchanged import/export byte preservation, project storage/reload, and rejection of invalid input. Keep the fixtures and automated cases: different URDF structures and extension patterns are intentional coverage, not duplicates.
- **GUI smoke tests** verify the UI-to-runtime integration and UI-only behavior. Do **not** repeat the full import/export/reopen/hash procedure for every valid fixture.
- Repeat a GUI case only when the UI command path, dialog behavior, project lifecycle integration, or error handling changes, or when an automated failure requires reproduction.
- The unchanged-source roundtrip does not establish semantic URDF validity, simulation behavior, or canonical ROBOT-Model export after editing.

### Automated tests

Run from the repository root:

```powershell
cargo test --all-targets
```

`src/urdf_fixture_tests.rs` exercises all eight valid fixtures, five rejected invalid fixtures, and documents the current semantic validation gap for invalid case 06. Do not remove these cases as GUI duplicates.

### GUI smoke tests (representative cases only)

1. With a dedicated test project, import a representative valid URDF via **Data > URDF Import**. Confirm the success log and the project-owned `application/robot-source.urdf`.
2. Export via **Data > URDF Export**. Confirm success and compare SHA-256 with the source.
3. Save, close, and reopen the project; export again and compare SHA-256. This is a representative integration check, **not a per-fixture GUI matrix**.
4. Attempt to import one malformed URDF via the dialog; verify the error is reported and the previously imported valid source is not replaced.
5. Cancel the import and export dialogs; verify no file or project data is changed.

Steps 4–5 are pending manual GUI verification as of 2026-10-15. They are not marked PASS by the existing automated tests alone.

PowerShell example (replace the project path):

```powershell
$files = @(
    ".\tests\fixtures\urdf-regression\valid\01_minimal.urdf"
    "PATH_TO_PROJECT\application\robot-source.urdf"
    "PATH_TO_PROJECT\exported.urdf"
    "PATH_TO_PROJECT\exported-reloaded.urdf"
)
Get-FileHash -Path $files -Algorithm SHA256 | Format-Table Hash, Path -AutoSize
```

## GUI verification record (2026-10-15)

| Case | Fixture / project | Import/export | Save/reopen/export | Evidence |
|---|---|---|---|---|
| URDF-RT-001–003 | `valid/01_minimal.urdf` / `D:\WORKPLACE\MELTEST01` | PASS | PASS | Source, stored, exported and re-exported hashes matched: `0BC974966BAD9EF7AB08A39B02E3D2BE1672FD3E52F3D88D73A3ABF37E679B21` |
| URDF-RT-004 | `valid/02_two_link_revolute.urdf` / `D:\WORKPLACE\MELTEST02` | PASS | PASS | Source, stored, exported and re-exported hashes matched: `968F52E3E1149AB529CBE00A6076F63BF61D19DF72FC6773A331D20794993384` |
| URDF-RT-005 | `valid/03_prismatic_fixed_chain.urdf` / `D:\WORKPLACE\MELTEST03` | PASS | PASS | Source, stored, exported and re-exported hashes matched: `29882D3BF0E86E7844996E98B4E82079FBF8D599519AA806838D65F105DC5929` |

The remaining valid fixtures are **automated-regression-only** by design. Previously proposed per-fixture GUI cases URDF-RT-006 through URDF-RT-010 are withdrawn as redundant manual verification; their source fixtures and automated coverage remain intact. The record above reports user-supplied GUI results, not a newly executed CI run.

## Expected behavior — automated fixture matrix

| Fixture | Expected current runtime behavior | Purpose |
|---|---|---|
| valid/01_minimal.urdf | Accept, exact roundtrip | Minimal robot |
| valid/02_two_link_revolute.urdf | Accept, exact roundtrip | Inertia, visual/collision, revolute joint |
| valid/03_prismatic_fixed_chain.urdf | Accept, exact roundtrip | Prismatic + fixed joints |
| valid/04_multi_joint_mimic.urdf | Accept, exact roundtrip | Mimic joint |
| valid/05_vendor_extensions.urdf | Accept, exact roundtrip | Namespaces, unknown attributes, Gazebo extension |
| valid/06_mixed_order_comments.urdf | Accept, exact roundtrip | Order, comments, duplicate extension fragments |
| valid/07_materials_and_mesh_reference.urdf | Accept, exact roundtrip | Global material, external mesh URI (mesh not bundled) |
| valid/08_preserved_parent_with_namespaces.urdf | Accept, exact roundtrip | Nested unknown namespaced parent |
| invalid/01_malformed_xml.urdf | Reject | XML syntax error |
| invalid/02_missing_robot_name.urdf | Reject | Required robot name missing |
| invalid/03_wrong_root.urdf | Reject | Non-robot root |
| invalid/04_missing_link_name.urdf | Reject | Required link name missing |
| invalid/05_missing_joint_name.urdf | Reject | Required joint name missing |
| invalid/06_structurally_invalid_urdf.urdf | **Currently may accept** | Nonexistent joint child reference; exposes missing semantic validation |

## Scope and caveats

- "Exact roundtrip" means byte-identical original XML when the model is **unchanged**; this does not test canonical ROBOT-Model export after edits.
- Current import validates XML and certain names, **not full URDF semantics**.
- Importing an invalid fixture should not replace a previously imported valid fixture.
- `valid/07_materials_and_mesh_reference.urdf` has an intentionally unresolved `package://` mesh path, for XML preservation tests only; visualization or physics loading may require the actual mesh.
- GUI failures should be reported in the runtime console; GUI visibility and cancellation behavior require their own smoke checks.
