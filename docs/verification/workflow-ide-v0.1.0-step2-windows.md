# workflow-ide-framework v0.1.0 Step 2 Consumer verification

## Scope

- Consumer: meridian-mujoco-runtime
- OS: Windows 11
- Baseline: `8a1080cd1488bb5a56fdfde51094e473d58ef888`
- Verification branch: `verification/workflow-ide-v0.1.0-step2`
- workflow-ide-framework: `fa13621c37a16f7b0f51894014af9829dad19c2a`

## Meridian Application configuration

- Application ID: `meridian-mujoco-runtime`
- Application name: `Meridian MuJoCo Runtime`
- Window title: `Meridian MuJoCo Runtime`
- Initial Window size: `1280 x 800`
- Minimum Window size: `900 x 600`
- UI scale: `1.0` explicitly configured

The initial size is larger than the Framework sample because Meridian is expected to host a Simulation View and surrounding Dock panels in later steps. The minimum size is provisional until the Dock UI is implemented.

## Verification result

| Item | Windows 11 |
| --- | --- |
| Framework dependency | ○ |
| Meridian build | ○ |
| Meridian executable launch | ○ |
| Meridian Application name | ○ |
| Meridian Application ID | ○ |
| Meridian Window title | ○ |
| Initial Window size | ○ |
| Minimum Window size | ○ |
| UI scale configuration | ○ |
| Consumer avoids direct eframe / egui / OS Window API startup configuration | ○ |
| Linux | ？ |
| macOS | ？ |

The user performed `cargo build` / `cargo run` on Windows 11 and supplied screenshots of the initial Window and the Window reduced to its configured minimum size. The screenshots show the Meridian title, Application name and Application ID, and confirm the initial/minimum Window behavior.

## Reproducibility

This Step 2 branch was created directly from the Meridian pre-Framework baseline. Step 1 was reapplied first, followed by Step 2 ApplicationConfig customization. It was not derived from the Step 1 verification branch.

Therefore the verified path is:

```text
Meridian baseline
  -> Step 1 reapply
  -> Step 2 ApplicationConfig
  -> cargo build
  -> cargo run
  -> Windows GUI verification
```

## Formal integration vs verification-only changes

Formal Meridian integration candidates:

- Cargo dependency on `workflow-ide-framework`, pinned to the Framework revision selected for the integration point.
- Meridian entry point using only Framework public Application APIs.
- Meridian-specific `ApplicationConfig` values listed above.

Verification-only changes:

- The `verification/workflow-ide-v0.1.0-step2` branch itself.
- This verification record.
- Pinning to the exact Step 2 verification commit is a reproducibility constraint for this verification; the revision used by later formal integration may advance through subsequent verified Framework steps.

The Framework host still displays `workflow-ide-framework v0.1.0 Step 1 sample`. This is a Framework-side temporary display already scheduled to be整理 during Step 4 and is not a Meridian Consumer implementation.
