# robot-model independent URDF fixtures

These fixtures were authored for meridian-mujoco-runtime and contain no third-party robot models.

- `minimal.urdf`: RM-001–RM-015, RM-024 base case.
- `unknown-extension.urdf`: RM-011, RM-025–RM-030 source for import/edit/conflict tests.
- `mixed-order.urdf`: RM-026, RM-027, RM-032 order and namespace tests.

`renamed-owner`, `deleted-owner`, and `conflicting-attribute` are **test operations on imported fixtures**, not necessarily distinct source files. Export validation is not yet implemented. These fixtures alone do not prove lossless preservation.

Expected source-owned extensions in unknown-extension.urdf:
- robot: `vendor:revision`, `vendor:metadata`
- base_link: `vendor:material`, `vendor:calibration`
- joint_1: `vendor:mode`, `vendor:control`

Expected sibling ordering in mixed-order.urdf: `visual`, first `vendor:between`, `collision`, second `vendor:between`.
