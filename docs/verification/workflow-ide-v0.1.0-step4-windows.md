# workflow-ide-framework v0.1.0 Step 4 Consumer verification

## Scope

- Consumer: meridian-mujoco-runtime
- OS: Windows 11
- Baseline: `8a1080cd1488bb5a56fdfde51094e473d58ef888`
- Verification branch: `verification/workflow-ide-v0.1.0-step4`
- workflow-ide-framework: `3c17786aeef0600db9aa5a7590f5992e230c490d`

## Reproduction path

```text
Meridian baseline
  -> Step 1 Framework dependency
  -> Step 2 ApplicationConfig
  -> Step 3 PanelDefinition
  -> Step 3.5 WFIDE Logging
  -> Step 4 LayoutConfig
  -> cargo build
  -> cargo run
  -> Windows GUI verification
```

The branch was created directly from the pre-Framework baseline rather than from an earlier verification branch.

## ApplicationConfig

- Application ID: `meridian-mujoco-runtime`
- Application name: `Meridian MuJoCo Runtime`
- Window title: `Meridian MuJoCo Runtime`
- Initial Window: `1280 x 800`
- Minimum Window: `900 x 600`
- UI scale: `1.0`

## PanelDefinition

| Panel ID | Name | PanelKind | Initially visible |
| --- | --- | --- | --- |
| `simulation-view` | Simulation View | `GpuViewport` | true |
| `runtime-control` | Runtime Control | `StandardUi` | true |
| `runtime-status` | Runtime Status | `StandardUi` | true |
| `log` | Log | `StandardUi` | true |
| `help` | Help | `Browser` | false |

## LayoutConfig

The Consumer uses only Framework `LayoutConfig` and stable Panel IDs.

- root: `simulation-view`
- left of Simulation View: `runtime-control`, fraction 0.25
- below Runtime Control: `runtime-status`, fraction 0.50
- below Simulation View: `log`, fraction 0.75
- selected Panel: `simulation-view`
- `help` is initially hidden and is not referenced by the initial Layout

The intent is to keep Simulation View as the largest work area, reserve approximately one quarter of the width for Runtime controls/status, divide that left area between control and status, and reserve approximately one quarter of the main right area height for multi-line logs.

Meridian does not use `egui_dock::DockState`, Tree APIs, NodeIndex, backend split nodes, eframe Window APIs, or OS-specific Window APIs directly.

## WFIDE Logging configuration

- directory: `logs/meridian`
- prefix: `meridian`
- application ID suffix: `meridian-mujoco-runtime`
- retention: 7 files
- rotation: daily, provided by WFIDE
- Consumer tracing target: `meridian::application`

The Consumer does not install its own tracing subscriber. A verification-only Consumer probe uses the public WFIDE logging API and `wfide::tracing` to verify Consumer event routing into console/file/in-memory logging.

Observed Consumer event:

```text
INFO meridian::application: Meridian application logging probe
MERIDIAN_WFIDE_LOGGING consumer_in_memory=true
```

TRACE output from eframe/wgpu was also observed during normal Application execution, as expected from the current Framework TRACE-level configuration. Consumer-side filtering was not added.

## Windows 11 verification

| Item | Result |
| --- | --- |
| baselineからStep 1→4再現 | ○ |
| cargo build | ○ |
| cargo run | ○ |
| Meridian Window起動 | ○ |
| Step 2 ApplicationConfig維持 | ○ |
| Step 3 PanelDefinition維持 | ○ |
| Initial Dock Layout | ○ |
| horizontal split | ○ |
| vertical split | ○ |
| nested split | ○ |
| Simulation View主要領域 | ○ |
| Runtime Control配置 | ○ |
| Runtime Status配置 | ○ |
| Log配置 | ○ |
| Help初期非表示 | ○ |
| selected Simulation View | ○ |
| split resize | ？ |
| Window resize後のDock維持 | ？ |
| backend Dock API非依存 | ○ |
| wfide::tracing Consumer利用 | ○ |
| Meridian固有ログ出力 | ○ |
| console logging | ○ |
| file logging | ？ |
| rotation設定適用 | ○ |
| Log PanelとLoggingの分離 | ○ |
| Linux | ？ |
| macOS | ？ |

The supplied GUI screenshot confirms the expected Meridian initial nested Dock layout and that Help is not present. It also shows Simulation View as the main work area and the expected Panel IDs/kinds in each placeholder.

The supplied console output confirms normal Framework tracing and the Meridian Consumer logging probe. File logging is intentionally left unverified until the generated file is directly checked. Split drag resize and Window resize persistence are also left unverified until explicitly exercised.

Daily rotation and maximum seven-file retention are verified as configuration/API application only. Actual day-boundary rotation, eighth-generation deletion, cross-restart retention and continued logging after rotation are deferred to the formal v0.1.0 test specification.

## Scope boundary

Step 4 does not implement MuJoCo rendering, GPU Surface connection, Browser Surface connection, Help content, Runtime Control UI, Runtime Status data, Log Panel Viewer, Layout persistence, Workspace restoration, direct Dock backend manipulation, or Runtime process separation.

WFIDE Logging is independent of the Log Panel. The Log Panel remains a placeholder; connecting the in-memory buffer to a viewer is a later Panel implementation/API step.

## Formal integration candidates

- workflow-ide-framework dependency
- Meridian ApplicationConfig
- Meridian stable PanelDefinition set
- Meridian LayoutConfig based only on stable Panel IDs
- Meridian WFIDE LoggingConfig
- Consumer use of `wfide::tracing` once application lifecycle APIs provide an appropriate post-logging-init execution point

## Verification-only changes

- `verification/workflow-ide-v0.1.0-step4` branch
- this verification record
- exact Step 4 Framework revision pin for reproducibility
- `examples/wfide_logging_consumer_probe.rs`

The probe is used because the current `Application::run()` initializes WFIDE logging internally immediately before entering the Framework host. Logging before `run()` occurs before subscriber initialization, while independently initializing the subscriber in normal Consumer application startup would conflict with Framework-owned initialization. A later Consumer lifecycle/API may provide a natural post-init location for application startup events.
