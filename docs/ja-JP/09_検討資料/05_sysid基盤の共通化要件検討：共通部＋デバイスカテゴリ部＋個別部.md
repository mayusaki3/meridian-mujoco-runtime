# sysid基盤の共通化要件検討：共通部＋デバイスカテゴリ部＋個別部

## 目的

シリアルサーボだけでなく、将来的に以下も同じsysid基盤へ載せる。

- PWM角度制御サーボ
- シリアルサーボ
- QDD（Quasi Direct Drive）
- ブラシレスモーター
- DCモーター
- 模型用モーター
- 車輪駆動ロボット
- 関節駆動ロボット

そのため、最初から以下の3層構造を前提にする。

```text
共通部
  ↓
デバイスカテゴリ部
  ↓
個別部
```

---

# 1. 基本方針

## 1.1 共通部に入れるべきもの

共通部は、特定デバイスの通信方式や制御方式に依存しない要素だけを持つ。

```text
観測ログ形式
時刻同期
指令ログ
外部センサログ
前処理
試験区間抽出
損失関数
最適化
検証
レポート生成
単位系
座標系
安全制約
capability定義
```

---

## 1.2 デバイスカテゴリ部に入れるべきもの

デバイスカテゴリ部は、駆動方式ごとの共通性を扱う。

例：

```text
PWMサーボカテゴリ
シリアルサーボカテゴリ
QDDカテゴリ
DCモーターカテゴリ
BLDCモーターカテゴリ
車輪駆動カテゴリ
脚関節カテゴリ
```

---

## 1.3 個別部に入れるべきもの

個別部は、メーカー・型番・プロトコル・実装差分を扱う。

例：

```text
Kondo ICS
Feetech STS3215
DYNAMIXEL XM430
Hobby PWM Servo SG90
ODrive + BLDC
VESC + BLDC
QDD Unit A
自作モータードライバ
```

---

# 2. 3層構造

## 2.1 全体構造

```text
Servo / Motor sysid Platform
  ├─ Common Layer
  │    ├─ Dataset
  │    ├─ TimeSeries
  │    ├─ CommandLog
  │    ├─ SensorLog
  │    ├─ ExperimentPlan
  │    ├─ LossFunction
  │    ├─ Optimizer
  │    ├─ ValidationReport
  │    └─ CapabilityProfile
  │
  ├─ Device Category Layer
  │    ├─ PWM Servo
  │    ├─ Serial Servo
  │    ├─ QDD Actuator
  │    ├─ DC Motor
  │    ├─ BLDC Motor
  │    ├─ Wheel Drive
  │    └─ Joint Actuator
  │
  └─ Device Specific Layer
       ├─ Kondo ICS
       ├─ Feetech
       ├─ DYNAMIXEL
       ├─ PWM Servo Model
       ├─ ODrive
       ├─ VESC
       └─ Custom Driver
```

---

# 3. 共通部の必須要件

## 3.1 指令を抽象化する

共通部では、指令を通信方式ではなく「意味」で表す。

```json
{
  "command": {
    "mode": "position",
    "target_position_rad": 1.57,
    "target_velocity_rad_s": null,
    "target_torque_nm": null,
    "target_current_a": null,
    "target_pwm_norm": null,
    "target_duty_norm": null
  }
}
```

---

## 3.2 指令方式を区別できること

PWMサーボとシリアルサーボでは、同じ「角度指令」でも入力方式が異なる。

そのため、共通部では以下を記録する。

```text
command_semantics:
  position
  velocity
  torque
  current
  pwm
  duty
  mixed

command_transport:
  pwm_pulse
  serial_packet
  can
  uart
  analog
  rc_signal
  direct_gpio
```

---

## 3.3 入力波形を記録できること

PWM角度制御では、指令はパケットではなくパルス幅である。

そのため、共通部に以下を入れる。

```text
pulse_width_us
period_us
duty_ratio
signal_frequency_hz
edge_timestamp
```

例：

```json
{
  "raw_command": {
    "transport": "pwm_pulse",
    "pulse_width_us": 1500,
    "period_us": 20000,
    "frequency_hz": 50
  }
}
```

---

## 3.4 電気入力を共通化する

モーター系では、制御入力が電圧・電流・PWM dutyになる。

共通部で扱えるようにする。

```json
{
  "electrical_input": {
    "voltage_v": 12.0,
    "current_a": 1.2,
    "duty_norm": 0.35,
    "pwm_frequency_hz": 20000
  }
}
```

---

## 3.5 機械出力を共通化する

関節でも車輪でも、最終的には機械出力として扱う。

```json
{
  "mechanical_output": {
    "position_rad": 1.0,
    "velocity_rad_s": 2.0,
    "acceleration_rad_s2": 5.0,
    "torque_nm": 0.8,
    "force_n": null,
    "linear_velocity_m_s": null
  }
}
```

---

# 4. 共通部に必要な抽象モデル

## 4.1 ActuatorModel

すべての駆動装置を以下で抽象化する。

```text
入力
  ↓
制御器
  ↓
電気系
  ↓
機械系
  ↓
出力
```

---

## 4.2 共通構造

```json
{
  "actuator_model": {
    "control": {},
    "electrical": {},
    "mechanical": {},
    "thermal": {},
    "protection": {},
    "sensor": {}
  }
}
```

---

## 4.3 control

```json
{
  "control": {
    "mode": "position",
    "kp": 0.0,
    "ki": 0.0,
    "kd": 0.0,
    "feedforward_velocity": 0.0,
    "feedforward_acceleration": 0.0,
    "command_delay_s": 0.0,
    "control_time_constant_s": 0.0,
    "deadband": 0.0
  }
}
```

---

## 4.4 electrical

```json
{
  "electrical": {
    "resistance_ohm": 0.0,
    "inductance_h": 0.0,
    "back_emf_constant": 0.0,
    "torque_constant_nm_per_a": 0.0,
    "current_limit_a": 0.0,
    "voltage_limit_v": 0.0,
    "pwm_limit_norm": 0.0
  }
}
```

---

## 4.5 mechanical

```json
{
  "mechanical": {
    "gear_ratio": 0.0,
    "efficiency": 0.0,
    "rotor_inertia": 0.0,
    "output_inertia": 0.0,
    "viscous_friction": 0.0,
    "static_friction": 0.0,
    "backlash_rad": 0.0,
    "torque_limit_nm": 0.0,
    "velocity_limit_rad_s": 0.0
  }
}
```

---

## 4.6 thermal

```json
{
  "thermal": {
    "thermal_gain": 0.0,
    "cooling_coefficient": 0.0,
    "ambient_temperature_c": 25.0,
    "derating_start_c": 70.0,
    "shutdown_c": 90.0
  }
}
```

---

## 4.7 protection

```json
{
  "protection": {
    "over_current_threshold_a": 0.0,
    "over_voltage_threshold_v": 0.0,
    "under_voltage_threshold_v": 0.0,
    "stall_current_threshold_a": 0.0,
    "stall_time_threshold_s": 0.0,
    "shutdown_delay_s": 0.0
  }
}
```

---

## 4.8 sensor

```json
{
  "sensor": {
    "position_resolution_rad": 0.0,
    "velocity_resolution_rad_s": 0.0,
    "current_resolution_a": 0.0,
    "sensor_delay_s": 0.0,
    "sensor_noise": 0.0,
    "zero_offset_rad": 0.0
  }
}
```

---

# 5. デバイスカテゴリ部

# 5.1 PWM角度制御サーボカテゴリ

## 特徴

```text
入力:
  PWMパルス幅

内部:
  位置制御はサーボ内部で完結

取得:
  通常は状態取得不可

観測:
  外部センサ依存
```

---

## 共通部へ要求するもの

```text
PWMパルス幅ログ
外部角度ログ
外部電流ログ
外部負荷ログ
電源電圧ログ
```

---

## sysid対象

```text
pulse_to_angle_mapping
deadband
response_delay
velocity_limit
torque_limit
static_friction
holding_stiffness
```

---

## 専用部に置くもの

```text
PWM周期
パルス幅範囲
角度範囲
個体差補正
安全パルス範囲
```

---

# 5.2 シリアルサーボカテゴリ

## 特徴

```text
入力:
  シリアルコマンド

内部:
  位置・速度・電流などを取得可能な場合がある

観測:
  内部状態 + 外部センサ
```

---

## sysid対象

```text
position_gain
velocity_gain
current_limit
torque_limit
response_delay
deadband
protection
thermal_derating
```

---

## 専用部に置くもの

```text
プロトコル
レジスタ
Control Table
単位変換
機能名マッピング
sync read / sync write
```

---

# 5.3 QDDカテゴリ

## 特徴

```text
低減速比
高トルク
バックドライバブル
トルク制御が重要
外力応答が重要
```

---

## 共通部へ要求するもの

```text
トルク指令
電流指令
外部角度
外部速度
外部トルク
電流
電圧
温度
```

---

## sysid対象

```text
torque_constant
motor_inertia
joint_inertia
viscous_friction
coulomb_friction
gear_efficiency
current_control_delay
torque_bandwidth
backdrivability
```

---

## 専用部に置くもの

```text
CANプロトコル
FOC制御器設定
電流制御モード
トルク制御モード
エンコーダ仕様
安全制限
```

---

# 5.4 DCモーターカテゴリ

## 特徴

```text
入力:
  電圧またはPWM duty

制御:
  外部ドライバ依存

状態取得:
  基本は外部センサ依存
```

---

## sysid対象

```text
resistance
inductance
back_emf_constant
torque_constant
friction
inertia
voltage_to_speed
current_to_torque
```

---

## 専用部に置くもの

```text
ドライバ仕様
PWM周波数
デッドタイム
電流制限
エンコーダ仕様
```

---

# 5.5 BLDCモーターカテゴリ

## 特徴

```text
FOC制御
電流制御
速度制御
位置制御
ESC / VESC / ODrive などに依存
```

---

## sysid対象

```text
phase_resistance
phase_inductance
kv
kt
rotor_inertia
cogging_torque
current_loop_bandwidth
velocity_loop_gain
position_loop_gain
```

---

## 専用部に置くもの

```text
ESCプロトコル
FOCパラメータ
極対数
エンコーダ仕様
キャリブレーション情報
```

---

# 5.6 車輪駆動カテゴリ

## 特徴

```text
回転出力が移動に変換される
床面との接触が重要
スリップが発生する
```

---

## 共通部へ要求するもの

```text
wheel_radius
wheel_base
track_width
ground_contact
friction
slip_ratio
robot_pose
```

---

## sysid対象

```text
wheel_radius_effective
rolling_resistance
ground_friction
slip_coefficient
motor_torque_limit
drive_delay
```

---

## 専用部に置くもの

```text
差動二輪
四輪
オムニホイール
メカナム
クローラ
ステアリング機構
```

---

# 6. capability profile 要件

## 6.1 目的

デバイスごとに、何を指令でき、何を観測できるかを明示する。

---

## 6.2 共通例

```json
{
  "capability_profile": {
    "device_category": "pwm_servo",
    "command": {
      "position": true,
      "velocity": false,
      "torque": false,
      "current": false,
      "pwm_pulse": true,
      "duty": false
    },
    "feedback": {
      "position": false,
      "velocity": false,
      "current": false,
      "voltage": false,
      "temperature": false
    },
    "external_required": {
      "position_sensor": true,
      "current_sensor": true,
      "load_sensor": true
    }
  }
}
```

---

## 6.3 QDD例

```json
{
  "capability_profile": {
    "device_category": "qdd",
    "command": {
      "position": true,
      "velocity": true,
      "torque": true,
      "current": true,
      "duty": false
    },
    "feedback": {
      "position": true,
      "velocity": true,
      "current": true,
      "voltage": true,
      "temperature": true,
      "torque": false
    },
    "external_required": {
      "torque_sensor": true,
      "current_sensor": false,
      "position_sensor": false
    }
  }
}
```

---

# 7. 共通ログ要件

## 7.1 raw と normalized を両方保存する

```text
raw:
  実デバイスから得たそのままの値

normalized:
  共通単位系へ変換した値
```

---

## 7.2 ログ構造

```json
{
  "timestamp": 0.001,
  "device": {
    "category": "pwm_servo",
    "vendor": "generic",
    "model": "sg90",
    "id": 1
  },
  "raw_command": {
    "transport": "pwm_pulse",
    "pulse_width_us": 1500,
    "period_us": 20000
  },
  "normalized_command": {
    "mode": "position",
    "target_position_rad": 0.0
  },
  "internal_state": {},
  "external_state": {
    "position_rad": 0.01,
    "velocity_rad_s": 0.2,
    "current_a": 0.3,
    "voltage_v": 5.0,
    "load_torque_nm": 0.0
  }
}
```

---

# 8. 共通部に最初から入れるべき要件

## 8.1 command abstraction

```text
位置指令
速度指令
トルク指令
電流指令
PWMパルス指令
PWM duty指令
電圧指令
```

---

## 8.2 observation abstraction

```text
内部観測
外部観測
推定観測
欠損観測
```

---

## 8.3 unit normalization

```text
角度: rad
角速度: rad/s
角加速度: rad/s^2
トルク: Nm
力: N
電流: A
電圧: V
温度: ℃
時間: s
PWM: normalized [-1, 1] or [0, 1]
```

---

## 8.4 transport abstraction

```text
pwm_pulse
uart
rs485
ttl_serial
can
i2c
spi
analog
gpio
```

---

## 8.5 device category abstraction

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

## 8.6 capability-driven processing

共通部は、デバイス名ではなく capability を見て処理する。

```text
このデバイスは現在角を内部取得できるか
このデバイスは外部角度センサが必要か
このデバイスはトルク指令可能か
このデバイスは電流値を取得可能か
```

---

# 9. sysid手法への影響

## 9.1 PWMサーボ

内部状態が取れないため、sysidは外部センサ主導になる。

```text
入力:
  pulse_width_us

出力:
  external_angle
  external_current
  load_torque
```

---

## 9.2 シリアルサーボ

内部状態と外部状態を比較できる。

```text
入力:
  serial command

出力:
  internal position
  external position
  internal current
  external current
```

---

## 9.3 QDD

トルク・電流・外力応答が重要になる。

```text
入力:
  target_torque
  target_current

出力:
  joint_angle
  joint_velocity
  current
  external_torque
```

---

## 9.4 車輪駆動

関節角だけでは不十分で、ロボット位置姿勢が必要になる。

```text
入力:
  wheel_velocity
  wheel_torque

出力:
  wheel_angle
  wheel_velocity
  robot_pose
  slip
```

---

# 10. 最終構成案

```text
Common Layer
  ├─ TimeSeries
  ├─ Command
  ├─ Observation
  ├─ UnitSystem
  ├─ CapabilityProfile
  ├─ ExperimentPlan
  ├─ Dataset
  ├─ LossFunction
  ├─ Optimizer
  ├─ ValidationReport
  └─ SafetyRule

Device Category Layer
  ├─ PWMServoCategory
  ├─ SerialServoCategory
  ├─ QDDCategory
  ├─ DCMotorCategory
  ├─ BLDCMotorCategory
  ├─ WheelDriveCategory
  └─ JointActuatorCategory

Device Specific Layer
  ├─ KondoICS
  ├─ Feetech
  ├─ DYNAMIXEL
  ├─ HobbyPWMServo
  ├─ ODrive
  ├─ VESC
  └─ CustomDriver
```

---

# 11. まとめ

将来的にPWMサーボ、QDD、模型用モーター、車輪駆動ロボットまで対応するなら、初期段階から共通部には以下を入れておくべきである。

```text
1. 指令抽象化
2. 観測抽象化
3. raw / normalized 両対応ログ
4. 単位正規化
5. transport抽象化
6. device category抽象化
7. capability profile
8. 外部センサ前提の観測設計
9. 電気系・機械系・熱系・保護系の共通モデル枠
10. 車輪駆動用の線形運動・姿勢観測枠
```

この構成により、

```text
PWMサーボ
シリアルサーボ
QDD
DCモーター
BLDCモーター
車輪駆動
```

を同じsysid基盤に載せつつ、差分はデバイスカテゴリ部と個別部へ分離できる。
