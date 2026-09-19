# workflow-ide-framework v0.1.0 Step 3 Consumer verification

## Scope

- Consumer: meridian-mujoco-runtime
- OS: Windows 11
- Baseline: `8a1080cd1488bb5a56fdfde51094e473d58ef888`
- Verification branch: `verification/workflow-ide-v0.1.0-step3`
- workflow-ide-framework: `0f70f5d523fa8ec889a003f10b043cc3c12f96e2`

## Reapplied configuration

Step 1 and Step 2 were reapplied from the pre-Framework baseline. The Meridian Application configuration remains:

- Application ID: `meridian-mujoco-runtime`
- Application name: `Meridian MuJoCo Runtime`
- Window title: `Meridian MuJoCo Runtime`
- Initial Window size: `1280 x 800`
- Minimum Window size: `900 x 600`
- UI scale: `1.0`

## Meridian Panel declarations

| Panel ID | Name | PanelKind | Initially visible |
| --- | --- | --- | --- |
| `simulation-view` | Simulation View | `GpuViewport` | true |
| `runtime-control` | Runtime Control | `StandardUi` | true |
| `runtime-status` | Runtime Status | `StandardUi` | true |
| `log` | Log | `StandardUi` | true |
| `help` | Help | `Browser` | false |

Panel IDs are stable machine identifiers and are intentionally separated from display names. These IDs are candidates for use by Step 4 initial Dock Layout.

## Windows 11 verification result

| Item | Result |
| --- | --- |
| Meridian cargo build | ○ |
| Meridian cargo run | ○ |
| Meridian Window launch | ○ |
| Step 2 Application configuration retained | ○ |
| Simulation View declaration | ○ |
| Runtime Control declaration | ○ |
| Runtime Status declaration | ○ |
| Log declaration | ○ |
| Help Browser declaration | ○ |
| Stable Panel IDs | ○ |
| PanelKind values | ○ |
| Initial visibility | ○ |
| Help initially_visible=false | ○ |
| Consumer avoids direct backend-specific API initialization | ○ |
| Linux | ？ |
| macOS | ？ |

The user performed the Windows 11 build/run and supplied a GUI screenshot. The Framework verification display showed:

```text
Simulation View [simulation-view] GpuViewport visible=true
Runtime Control [runtime-control] StandardUi visible=true
Runtime Status [runtime-status] StandardUi visible=true
Log [log] StandardUi visible=true
Help [help] Browser visible=false
```

This confirms the Meridian-specific Panel declarations, IDs, kinds, and visibility values are received by the Framework.

## Scope boundary

Step 3 only declares Panels. Meridian does not initialize or connect wgpu, GPU Surface, Browser Surface/CEF, egui backend, or OS Window APIs directly. No Dock layout, Panel content, MuJoCo rendering, Runtime controls/status data, logging implementation, Help URL loading, or persistence is implemented in this step.

## Reproducibility

The Step 3 branch was created directly from the Meridian baseline rather than from the Step 2 verification branch:

```text
Meridian baseline
  -> Step 1 reapply
  -> Step 2 reapply
  -> Step 3 Panel declarations
  -> cargo build
  -> cargo run
  -> Windows GUI verification
```

## Formal integration vs verification-only changes

Formal Meridian integration candidates:

- Framework dependency.
- Meridian ApplicationConfig.
- Stable Meridian Panel declarations and Panel IDs.
- Use of Framework public Application / Panel APIs only.

Verification-only changes:

- `verification/workflow-ide-v0.1.0-step3` branch.
- This verification record.
- Exact revision pin to the Step 3 Framework verification commit is required for this reproducibility test; a later formal integration may advance to a subsequent verified Framework revision.

The Framework host still displays the temporary `workflow-ide-framework v0.1.0 Step 1 sample` label. This is Framework-side temporary verification UI and is outside the Meridian Step 3 implementation.
