# meridian-mujoco-runtime データ構造案

## 目的

`meridian-mujoco-runtime` は、MuJoCo を中核としたシミュレーション実行環境として、以下を組み合わせて動作させる。

```text
ロボット定義
+
使用アクチュエーター定義
+
フィールド定義
+
タスク/シナリオ定義
+
実行プロファイル
```

上位アプリはこれらを選択し、

```text
階段を上る
ドアを開けて入る
車輪で移動する
ロボットバトルを行う
実ボードHILで検証する
sysidを行う
```

のような動作を実行できるようにする。

---

# 1. 全体構造

```text
meridian-mujoco-runtime
  ├─ RuntimeProject
  ├─ RobotDefinition
  ├─ ActuatorDefinition
  ├─ FieldDefinition
  ├─ ScenarioDefinition
  ├─ TaskDefinition
  ├─ SensorDefinition
  ├─ SysidDefinition
  ├─ HilDefinition
  ├─ ExecutionProfile
  ├─ DataFlowDefinition
  └─ RuntimeSession
```

---

# 2. 基本方針

## 2.1 分離する単位

以下を分けて扱う。

```text
ロボット
フィールド
アクチュエーター
センサー
タスク
実行モード
ログ
```

理由：

```text
同じロボットを別フィールドで使える
同じフィールドに別ロボットを置ける
同じロボットに別アクチュエーターを割り当てられる
sysid対象をアクチュエーター単位で扱える
上位アプリが組み合わせを選択しやすい
```

---

# 3. RuntimeProject

## 3.1 役割

シミュレーション全体のプロジェクト単位。

```json
{
  "runtime_project": {
    "id": "project_stair_test_001",
    "name": "Stair Climbing Test",
    "version": "0.1.0",
    "robots": [],
    "actuators": [],
    "fields": [],
    "scenarios": [],
    "execution_profiles": [],
    "data_flows": []
  }
}
```

---

# 4. RobotDefinition

## 4.1 役割

ロボット本体の構造を表す。

ここでは、使用アクチュエーターの具体特性は直接持たず、

```text
どの関節に、どのアクチュエータースロットがあるか
```

を定義する。

---

## 4.2 例

```json
{
  "robot": {
    "id": "robot_humanoid_001",
    "name": "Humanoid Test Robot",
    "model_format": "mjcf",
    "model_uri": "robots/humanoid/model.xml",
    "root_body": "base_link",
    "joints": [
      {
        "id": "left_knee_joint",
        "name": "left_knee",
        "type": "hinge",
        "axis": [0, 1, 0],
        "range_rad": [-2.0, 0.0],
        "actuator_slot": "left_knee_actuator"
      }
    ],
    "actuator_slots": [
      {
        "id": "left_knee_actuator",
        "joint_id": "left_knee_joint",
        "required_category": "serial_servo",
        "required_capabilities": {
          "position_control": true,
          "feedback_position": true
        }
      }
    ],
    "sensor_slots": [
      {
        "id": "base_imu",
        "type": "imu",
        "mount_body": "base_link"
      }
    ]
  }
}
```

---

# 5. ActuatorDefinition

## 5.1 役割

sysid対象にもなるアクチュエーター定義。

アクチュエーターはロボット本体から分離し、以下を持つ。

```text
カテゴリ
個別デバイス情報
capability
sysid結果
MuJoCo反映方法
HIL対応
```

---

## 5.2 カテゴリ

```text
pwm_servo
serial_servo
qdd
dc_motor
bldc_motor
wheel_drive
linear_actuator
custom
```

---

## 5.3 例

```json
{
  "actuator": {
    "id": "knee_servo_001",
    "category": "serial_servo",
    "vendor": "example_vendor",
    "model": "example_servo",
    "capability_profile_id": "serial_servo_position_current_v1",
    "behavior_model_id": "behavior_knee_servo_001",
    "sysid_result_id": "sysid_knee_servo_001",
    "mujoco_mapping": {
      "target_joint": "left_knee_joint",
      "control_type": "external_torque",
      "mujoco_actuator_name": "left_knee_motor",
      "apply_to": "data.ctrl"
    }
  }
}
```

---

# 6. ActuatorBehaviorModel

## 6.1 役割

sysid結果を反映したアクチュエーター挙動モデル。

---

## 6.2 例

```json
{
  "actuator_behavior_model": {
    "id": "behavior_knee_servo_001",
    "control": {
      "mode": "position",
      "kp": 12.0,
      "ki": 0.0,
      "kd": 0.15,
      "deadband_rad": 0.005,
      "response_delay_s": 0.012,
      "control_time_constant_s": 0.04
    },
    "limits": {
      "velocity_limit_rad_s": 8.0,
      "acceleration_limit_rad_s2": 120.0,
      "torque_limit_nm": 1.5,
      "current_limit_a": 2.0
    },
    "mechanical": {
      "static_friction_nm": 0.03,
      "viscous_friction_nms": 0.002,
      "backlash_rad": 0.004,
      "gear_efficiency": 0.75
    },
    "thermal": {
      "thermal_gain": 0.02,
      "cooling_coefficient": 0.005,
      "derating_start_c": 70.0,
      "shutdown_c": 90.0
    },
    "protection": {
      "stall_current_threshold_a": 2.2,
      "stall_error_threshold_rad": 0.3,
      "stall_time_threshold_s": 1.0
    }
  }
}
```

---

# 7. RobotActuatorBinding

## 7.1 役割

ロボットのアクチュエータースロットに、実際のアクチュエーター定義を割り当てる。

---

## 7.2 例

```json
{
  "robot_actuator_binding": {
    "id": "binding_humanoid_servo_set_001",
    "robot_id": "robot_humanoid_001",
    "bindings": [
      {
        "slot_id": "left_knee_actuator",
        "actuator_id": "knee_servo_001"
      },
      {
        "slot_id": "right_knee_actuator",
        "actuator_id": "knee_servo_002"
      }
    ]
  }
}
```

---

# 8. FieldDefinition

## 8.1 役割

ロボットが動作する環境を定義する。

```text
地形
階段
床材
摩擦
障害物
ドア
スロープ
接触対象
照明
カメラ
```

---

## 8.2 例

```json
{
  "field": {
    "id": "field_house_entry_001",
    "name": "House Entry Field",
    "model_format": "mjcf",
    "model_uri": "fields/house_entry/model.xml",
    "objects": [
      {
        "id": "stairs_001",
        "type": "stairs",
        "pose": {
          "position": [1.0, 0.0, 0.0],
          "rotation_quat": [1, 0, 0, 0]
        },
        "parameters": {
          "step_height_m": 0.15,
          "step_depth_m": 0.28,
          "step_count": 5,
          "friction": 0.8
        }
      },
      {
        "id": "door_001",
        "type": "door",
        "joint": {
          "type": "hinge",
          "axis": [0, 0, 1],
          "range_rad": [0.0, 1.57],
          "damping": 0.1
        },
        "handle": {
          "position": [0.0, -0.4, 0.9],
          "required_force_n": 8.0
        }
      }
    ]
  }
}
```

---

# 9. ScenarioDefinition

## 9.1 役割

ロボット、アクチュエーター、フィールド、タスク、実行プロファイルを組み合わせる。

---

## 9.2 例

```json
{
  "scenario": {
    "id": "scenario_open_door_001",
    "name": "Open Door And Enter",
    "robot_id": "robot_humanoid_001",
    "robot_actuator_binding_id": "binding_humanoid_servo_set_001",
    "field_id": "field_house_entry_001",
    "task_id": "task_open_door_and_enter",
    "execution_profile_id": "profile_realtime_hil",
    "initial_state": {
      "robot_pose": {
        "position": [0.0, 0.0, 0.0],
        "rotation_quat": [1, 0, 0, 0]
      }
    }
  }
}
```

---

# 10. TaskDefinition

## 10.1 役割

上位アプリが扱う目的・達成条件を定義する。

---

## 10.2 例：階段を上る

```json
{
  "task": {
    "id": "task_climb_stairs",
    "type": "locomotion",
    "goal": {
      "type": "reach_region",
      "target_region": "top_of_stairs"
    },
    "success_conditions": [
      {
        "type": "robot_base_in_region",
        "region_id": "top_of_stairs"
      },
      {
        "type": "no_fall",
        "duration_s": 10.0
      }
    ],
    "failure_conditions": [
      {
        "type": "fall_detected"
      },
      {
        "type": "timeout",
        "time_s": 60.0
      }
    ]
  }
}
```

---

## 10.3 例：ドアを開けて入る

```json
{
  "task": {
    "id": "task_open_door_and_enter",
    "type": "manipulation_and_locomotion",
    "goal": {
      "type": "door_open_and_enter",
      "door_id": "door_001",
      "target_region": "inside_room"
    },
    "success_conditions": [
      {
        "type": "door_angle_greater_than",
        "door_id": "door_001",
        "angle_rad": 1.0
      },
      {
        "type": "robot_base_in_region",
        "region_id": "inside_room"
      }
    ],
    "failure_conditions": [
      {
        "type": "door_not_opened",
        "timeout_s": 20.0
      },
      {
        "type": "fall_detected"
      }
    ]
  }
}
```

---

# 11. ExecutionProfile

## 11.1 役割

シミュレーションの正確性重視・リアルタイム性重視を切り替える。

---

## 11.2 モード

```text
accuracy
realtime
balanced
hil_sync
fast_batch
```

---

## 11.3 例：正確性重視

```json
{
  "execution_profile": {
    "id": "profile_accuracy",
    "mode": "accuracy",
    "mujoco": {
      "timestep_s": 0.0005,
      "solver": "Newton",
      "iterations": 100,
      "tolerance": 1e-10,
      "contact_model": "high_accuracy"
    },
    "runtime": {
      "real_time_sync": false,
      "allow_slowdown": true,
      "sensor_noise": false,
      "logging_level": "full"
    }
  }
}
```

---

## 11.4 例：リアルタイム重視

```json
{
  "execution_profile": {
    "id": "profile_realtime",
    "mode": "realtime",
    "mujoco": {
      "timestep_s": 0.002,
      "solver": "PGS",
      "iterations": 20,
      "tolerance": 1e-6,
      "contact_model": "fast"
    },
    "runtime": {
      "real_time_sync": true,
      "allow_slowdown": false,
      "sensor_noise": true,
      "logging_level": "standard"
    }
  }
}
```

---

## 11.5 例：HIL同期

```json
{
  "execution_profile": {
    "id": "profile_hil_sync",
    "mode": "hil_sync",
    "mujoco": {
      "timestep_s": 0.001,
      "solver": "Newton",
      "iterations": 50
    },
    "hil": {
      "clock_master": "mujoco",
      "step_sync": true,
      "step_token_required": true,
      "timeout_ms": 10,
      "on_timeout": "safe_stop"
    }
  }
}
```

---

# 12. SysidDefinition

## 12.1 役割

sysid対象、測定データ、推定結果を管理する。

---

## 12.2 例

```json
{
  "sysid": {
    "id": "sysid_knee_servo_001",
    "target_actuator_id": "knee_servo_001",
    "datasets": [
      "dataset_step_response_001",
      "dataset_load_test_001"
    ],
    "estimated_parameters": {
      "kp": 12.0,
      "kd": 0.15,
      "torque_limit_nm": 1.5,
      "velocity_limit_rad_s": 8.0
    },
    "quality": {
      "angle_rmse_rad": 0.01,
      "velocity_rmse_rad_s": 0.12,
      "current_rmse_a": 0.08
    },
    "status": "validated"
  }
}
```

---

# 13. DataFlowDefinition

## 13.1 役割

上位アプリ・実ボード・MuJoCo・ログ・制御器間のデータ流通を定義する。

---

## 13.2 データ流通の基本

```text
上位アプリ
  ↓ scenario選択
Runtime
  ↓ robot + actuator + field 合成
MuJoCo
  ↓ sensor出力
Controller / Board
  ↓ command出力
Runtime
  ↓ MuJoCoへ反映
Logger
  ↓ dataset保存
上位アプリ
  ↓ 結果参照
```

---

## 13.3 例

```json
{
  "data_flow": {
    "id": "dataflow_hil_001",
    "inputs": [
      {
        "name": "scenario_request",
        "source": "upper_app",
        "type": "scenario_select"
      },
      {
        "name": "hil_command",
        "source": "board",
        "type": "actuator_command"
      }
    ],
    "outputs": [
      {
        "name": "sensor_packet",
        "target": "board",
        "type": "hil_sensor"
      },
      {
        "name": "runtime_state",
        "target": "upper_app",
        "type": "state_stream"
      },
      {
        "name": "simulation_log",
        "target": "logger",
        "type": "dataset"
      }
    ],
    "streams": [
      {
        "name": "runtime_state_stream",
        "frequency_hz": 60,
        "payload": [
          "robot_pose",
          "joint_state",
          "task_status"
        ]
      },
      {
        "name": "hil_sensor_stream",
        "frequency_hz": 1000,
        "payload": [
          "joint_position",
          "joint_velocity",
          "imu",
          "contact"
        ]
      }
    ]
  }
}
```

---

# 14. RuntimeSession

## 14.1 役割

実行中のセッション状態。

---

## 14.2 例

```json
{
  "runtime_session": {
    "id": "session_20260510_001",
    "scenario_id": "scenario_open_door_001",
    "status": "running",
    "sim_time_s": 12.345,
    "wall_time_s": 12.350,
    "real_time_factor": 0.99,
    "current_step": 12345,
    "robot_state": {
      "base_pose": {
        "position": [0.5, 0.1, 0.0],
        "rotation_quat": [1, 0, 0, 0]
      },
      "joints": []
    },
    "task_state": {
      "status": "in_progress",
      "progress": 0.45
    }
  }
}
```

---

# 15. 上位アプリから見た選択単位

## 15.1 選択構造

上位アプリは以下を選択する。

```text
1. ロボット
2. アクチュエーターセット
3. フィールド
4. タスク
5. 実行プロファイル
```

---

## 15.2 例

```json
{
  "simulation_request": {
    "robot_id": "robot_humanoid_001",
    "robot_actuator_binding_id": "binding_humanoid_servo_set_001",
    "field_id": "field_house_entry_001",
    "task_id": "task_open_door_and_enter",
    "execution_profile_id": "profile_hil_sync"
  }
}
```

---

# 16. データ構造の責務分離

## 16.1 RobotDefinition

```text
ロボット形状
リンク
関節
センサースロット
アクチュエータースロット
```

---

## 16.2 ActuatorDefinition

```text
アクチュエーター種別
能力
sysid結果
MuJoCo反映方法
HIL変換
```

---

## 16.3 FieldDefinition

```text
地形
建物
障害物
ドア
階段
摩擦
接触物
```

---

## 16.4 ScenarioDefinition

```text
どのロボットを
どのアクチュエーターで
どのフィールドに置き
何を実行するか
```

---

## 16.5 ExecutionProfile

```text
正確性優先
リアルタイム優先
HIL同期
高速バッチ
ログ粒度
```

---

# 17. 精度優先とリアルタイム優先の切替

## 17.1 accuracy mode

用途：

```text
sysid検証
接触精度検証
階段昇降の詳細評価
ドア操作の力学評価
```

特徴：

```text
小さいtimestep
高反復solver
詳細ログ
非リアルタイム可
```

---

## 17.2 realtime mode

用途：

```text
上位アプリ連携
VR表示
インタラクティブ操作
大まかな動作確認
```

特徴：

```text
実時間同期
軽量solver
低頻度ログ
センサー値の間引き
```

---

## 17.3 hil_sync mode

用途：

```text
実ボード制御検証
制御周期同期
通信遅延検証
```

特徴：

```text
MuJoCoがクロックマスター
step token方式
ボードは1 stepごとに待機
timeout時safe_stop
```

---

# 18. 最小データ構造

初期実装では、最低限以下があればよい。

```text
RobotDefinition
ActuatorDefinition
RobotActuatorBinding
FieldDefinition
ScenarioDefinition
ExecutionProfile
RuntimeSession
```

---

# 19. 推奨ファイル構成

```text
meridian-mujoco-runtime/
  projects/
    project_stair_test_001.json

  robots/
    humanoid_001/
      robot.json
      model.xml

  actuators/
    serial_servo/
      knee_servo_001.json

  fields/
    house_entry_001/
      field.json
      model.xml

  scenarios/
    open_door_001.json

  profiles/
    accuracy.json
    realtime.json
    hil_sync.json

  sessions/
    session_20260510_001/
      session.json
      logs/
```

---

# 20. まとめ

`meridian-mujoco-runtime` のデータ構造は、以下を基本単位にするとよい。

```text
RobotDefinition
ActuatorDefinition
FieldDefinition
ScenarioDefinition
ExecutionProfile
RuntimeSession
```

特に重要なのは、

```text
ロボット本体
アクチュエーター
フィールド
```

を分離することである。

これにより、

```text
同じロボットで別フィールドを試す
同じロボットに別アクチュエーターを割り当てる
同じフィールドで別ロボットを試す
sysid結果だけ差し替える
精度優先とリアルタイム優先を切り替える
HIL同期へ切り替える
```

ことが可能になる。
