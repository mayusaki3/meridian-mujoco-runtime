# Meridian MuJoCo Runtime — URDF test fixtures

These fixtures are synthetic and created for import/export regression testing. No external mesh is bundled.

## How to test

1. Create a dedicated runtime test project.
2. Data > URDF Import: choose a file in `valid/`.
3. Data > URDF Export: save under a different filename.
4. Compare SHA-256 of source, `application/robot-source.urdf`, and exported file.
5. Save and reopen the project; export again and compare hashes.
6. Use a new test project per case, because the current importer overwrites `robot-source.urdf`.

PowerShell:

```powershell
Get-FileHash .\valid\05_vendor_extensions.urdf -Algorithm SHA256
Get-FileHash 'PATH_TO_PROJECT\application\robot-source.urdf' -Algorithm SHA256
Get-FileHash 'PATH_TO_EXPORT\exported.urdf' -Algorithm SHA256
```

## Expected behavior

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
- File dialogs may be used on the `invalid/` fixtures too; failures should be reported by the runtime console.
