# Feetech / DYNAMIXEL を共通 sysid 仕組みに載せる検討

## 目的

ICS系シリアルサーボ向けに検討した sysid 仕組みを、Feetech および DYNAMIXEL にも適用できるか検討する。

前提は以下である。

```text
固定の共通部
+
メーカー・シリーズ別の専用部
```

に分離する。

---

# 結論

Feetech と DYNAMIXEL は、どちらも同じ sysid 基盤に載せられる可能性が高い。

理由は、どちらも以下の要素を持つためである。

```text
1. シリアル通信による指令入力
2. 内部レジスタ / Control Table
3. 現在位置・速度・電流・温度などの状態取得
4. 位置制御・速度制御・PWM / 電流系制御
5. パラメータ変更
```

ただし、共通化すべきなのは「sysidの考え方」と「ログ・最適化・評価基盤」であり、通信プロトコルやレジスタ定義は専用部に分けるべきである。

---

# 参考情報

- DYNAMIXEL Xシリーズでは、Present Position、Present Velocity、Present PWM、Present Current が Control Table に格納される。
- DYNAMIXEL では Goal PWM、Goal Current、Profile Velocity、PID Gain、Feedforward Gain などを扱えるモデルがある。
- Feetech Serial Bus Servo では、位置、速度、電流、温度、電圧、負荷などのフィードバックや、複数制御モードを持つシリーズがある。
- Feetech SCS/SMS/ST系では、TTLまたはシリアルバス通信で内部レジスタを読み書きする構成が一般的である。

情報元：

- ROBOTIS DYNAMIXEL e-Manual  
  https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/

- ROBOTIS DYNAMIXEL e-Manual / P Series  
  https://emanual.robotis.com/docs/en/dxl/p/ph42-020-s300-r/

- Feetech Communication Protocol Manual  
  https://files.seeedstudio.com/wiki/robotics/Actuator/feetech/Communication_Protocol_Manual.pdf

- Feetech Bus Servo Technology  
  https://www.feetechrc.com/solution.html

---

# 共通化できる部分

## 1. sysid の基本構造

以下は ICS / Feetech / DYNAMIXEL で共通化できる。

```text
指令ログ
↓
内部状態ログ
↓
外部センサログ
↓
前処理
↓
試験区間抽出
↓
シミュレーション
↓
誤差計算
↓
最適化
↓
検証
```

---

## 2. 観測データ構造

共通ログ形式はメーカー非依存にできる。

```json
{
  "timestamp": 123456789,
  "servo": {
    "vendor": "dynamixel",
    "model": "XM430-W350",
    "id": 1
  },
  "command": {
    "mode": "position",
    "target_position": 90.0,
    "target_velocity": null,
    "target_current": null,
    "target_pwm": null
  },
  "internal": {
    "position": 88.2,
    "velocity": 120.3,
    "current": 1.4,
    "voltage": 12.0,
    "temperature": 42.0,
    "pwm": 230,
    "load": null
  },
  "external": {
    "angle": 87.9,
    "velocity": 119.5,
    "current": 1.6,
    "voltage": 11.9,
    "load_force": 3.2
  },
  "settings": {
    "raw": {}
  }
}
```

---

## 3. 共通の推定対象

以下はメーカーに依存せず共通化できる。

```text
kp
ki
kd
feedforward_gain
velocity_limit
acceleration_limit
torque_limit
current_limit
pwm_limit
deadband_width
static_friction
viscous_friction
response_delay
thermal_time_constant
temperature_limit
stall_threshold
```

---

## 4. 共通の実験種別

以下も共通化できる。

```text
ステップ応答試験
周波数応答試験
負荷保持試験
電流・トルク変換試験
デッドバンド試験
初動応答試験
リミット試験
温度上昇試験
保護動作試験
```

---

## 5. 共通の評価指標

```text
角度RMSE
速度RMSE
電流RMSE
PWM誤差
負荷誤差
整定時間差
オーバーシュート差
保護発生時刻差
温度上昇曲線誤差
```

---

# 専用部に分けるべき部分

## 1. 通信プロトコル

これは完全に専用部である。

```text
ICS:
  ICSプロトコル

Feetech:
  SCS / STS / SMS 系プロトコル

DYNAMIXEL:
  Protocol 1.0 / Protocol 2.0
```

共通部は、

```text
read_state()
write_command()
write_setting()
sync_read()
sync_write()
```

のような抽象APIだけを見る。

---

## 2. レジスタ / Control Table 定義

メーカー・シリーズ別に異なるため専用部にする。

```text
共通名:
  present_position

専用マッピング:
  DYNAMIXEL XM430:
    address = 132

  Feetech STS:
    address = モデル別定義

  ICS:
    command = 現在値取得
```

---

## 3. 単位変換

内部値の単位はメーカー・モデルで異なる。

専用部で以下を変換する。

```text
内部位置値 → degree / radian
内部速度値 → deg/s / rad/s
内部電流値 → A
内部PWM値 → normalized PWM
内部温度値 → ℃
内部電圧値 → V
```

---

## 4. 制御モード

制御モードも専用部で吸収する。

```text
position
velocity
current
pwm
torque
extended_position
current_based_position
wheel
continuous_rotation
```

共通部では以下のように正規化する。

```text
POSITION_CONTROL
VELOCITY_CONTROL
CURRENT_CONTROL
PWM_CONTROL
HYBRID_POSITION_CURRENT
CONTINUOUS_ROTATION
```

---

## 5. 機能名の差異

同じような意味でもメーカーごとに名称が異なる。

例：

```text
ICS:
  stretch
  speed
  punch
  response
  damping

DYNAMIXEL:
  Position P/I/D Gain
  Velocity P/I Gain
  Feedforward Gain
  Profile Velocity
  Profile Acceleration
  Goal PWM
  Goal Current

Feetech:
  PID parameters
  speed
  acceleration
  torque / load / current
  protection parameters
```

共通部では、意味ベースの正規化名にする。

---

# 共通部の設計案

## ServoSysidCommon

```text
ServoSysidCommon
  ├─ ExperimentPlan
  ├─ CommandLog
  ├─ InternalStateLog
  ├─ ExternalSensorLog
  ├─ DatasetPreprocessor
  ├─ ServoBehaviorModel
  ├─ LossFunction
  ├─ Optimizer
  └─ ValidationReport
```

---

## 共通API案

```python
class ServoAdapter:
    def write_command(self, command):
        pass

    def read_state(self):
        pass

    def write_setting(self, setting):
        pass

    def read_setting(self):
        pass

    def decode_raw_packet(self, packet):
        pass

    def normalize_state(self, raw_state):
        pass

    def normalize_setting(self, raw_setting):
        pass
```

---

# 専用部の設計案

## ICS専用部

```text
IcsAdapter
  ├─ ICS packet encode/decode
  ├─ ICS現在値取得
  ├─ ICS設定値読み書き
  ├─ speed/stretch/punch/deadband変換
  └─ ICS内部値 → 共通単位変換
```

---

## Feetech専用部

```text
FeetechAdapter
  ├─ SCS / STS / SMS protocol encode/decode
  ├─ register map
  ├─ sync read / sync write
  ├─ position / velocity / current / temperature取得
  ├─ PID / acceleration / torque設定
  └─ Feetech内部値 → 共通単位変換
```

---

## DYNAMIXEL専用部

```text
DynamixelAdapter
  ├─ Protocol 1.0 / 2.0 encode/decode
  ├─ Control Table定義
  ├─ GroupSyncRead / GroupSyncWrite
  ├─ Present Position / Velocity / Current / PWM取得
  ├─ PID / Feedforward / Profile設定
  └─ DYNAMIXEL内部値 → 共通単位変換
```

---

# 比較表

| 項目 | ICS | Feetech | DYNAMIXEL | 共通化可否 |
|---|---|---|---|---|
| シリアル指令 | あり | あり | あり | 可 |
| 現在位置取得 | あり | あり | あり | 可 |
| 速度取得 | 一部/要確認 | あり | あり | 可 |
| 電流取得 | 一部/要確認 | あり | あり | 可 |
| 温度取得 | あり | あり | あり | 可 |
| 電圧取得 | 要確認 | あり | あり | 可 |
| PID調整 | 独自項目 | あり | あり | 可 |
| PWM制御 | 基本なし/要確認 | シリーズ依存 | あり | 専用差分 |
| 電流制御 | 制限中心 | シリーズ依存 | あり | 専用差分 |
| 位置制御 | あり | あり | あり | 可 |
| 速度制御 | 回転モード等 | あり | あり | 可 |
| 保護機能 | あり | あり | あり | 可 |
| 通信プロトコル | 独自 | Feetech系 | DYNAMIXEL | 専用 |

---

# sysid対象としての比較

## 共通して扱えるもの

```text
位置応答
速度応答
保持剛性
トルク制限
電流制限
温度制限
応答遅れ
摩擦
バックラッシュ
デッドバンド
保護動作
```

---

## DYNAMIXELで特に扱いやすいもの

DYNAMIXELは Control Table が整備されており、以下を sysid に載せやすい。

```text
Present Position
Present Velocity
Present Current
Present PWM
Present Input Voltage
Present Temperature
Goal PWM
Goal Current
Profile Velocity
Profile Acceleration
Position PID Gain
Velocity PID Gain
Feedforward Gain
Operating Mode
Hardware Error Status
```

そのため、共通基盤に載せる場合の基準実装として使いやすい。

---

## Feetechで特に注意するもの

Feetechはシリーズ差が大きい可能性がある。

```text
SCS
STS
SMS
SC
SM
```

などで、

- レジスタ
- 制御モード
- 電流取得可否
- PWM制御可否
- PID調整可否

が異なる可能性がある。

そのため、Feetechは「メーカー単位」ではなく「シリーズ・モデル単位」で capability を定義する必要がある。

---

# capability 定義案

## 目的

同じ共通sysid基盤に載せるため、各サーボが何を扱えるかを明示する。

---

## 例

```json
{
  "vendor": "dynamixel",
  "model": "XM430-W350",
  "protocol": "dynamixel_protocol_2",
  "capabilities": {
    "command": {
      "position": true,
      "velocity": true,
      "current": true,
      "pwm": true
    },
    "feedback": {
      "position": true,
      "velocity": true,
      "current": true,
      "pwm": true,
      "voltage": true,
      "temperature": true
    },
    "settings": {
      "pid": true,
      "feedforward": true,
      "profile_velocity": true,
      "profile_acceleration": true,
      "current_limit": true,
      "pwm_limit": true,
      "temperature_limit": true
    },
    "sync": {
      "sync_read": true,
      "sync_write": true
    }
  }
}
```

---

## Feetech例

```json
{
  "vendor": "feetech",
  "model": "STS3215",
  "protocol": "feetech_sts",
  "capabilities": {
    "command": {
      "position": true,
      "velocity": true,
      "current": false,
      "pwm": false
    },
    "feedback": {
      "position": true,
      "velocity": true,
      "current": true,
      "voltage": true,
      "temperature": true,
      "load": true
    },
    "settings": {
      "pid": true,
      "acceleration": true,
      "current_limit": true,
      "temperature_limit": true
    },
    "sync": {
      "sync_read": true,
      "sync_write": true
    }
  }
}
```

---

# 共通ログ設計

## 生データと正規化データを両方保存する

推奨は以下である。

```text
raw_command
raw_response
normalized_command
normalized_internal_state
external_sensor_state
```

---

## 理由

```text
raw:
  後からデコードし直せる
  プロトコル差分を検証できる

normalized:
  共通sysid処理に使える
```

---

# 共通sysid処理への入力

```json
{
  "dataset_id": "step_response_001",
  "servo_identity": {
    "vendor": "dynamixel",
    "model": "XM430-W350",
    "id": 1
  },
  "capability_profile": "dynamixel_xm430_w350",
  "time_series": [
    {
      "t": 0.000,
      "command": {
        "mode": "position",
        "target_position_rad": 0.0
      },
      "internal": {
        "position_rad": 0.0,
        "velocity_rad_s": 0.0,
        "current_a": 0.1,
        "pwm_norm": 0.02,
        "temperature_c": 30.0
      },
      "external": {
        "angle_rad": 0.0,
        "velocity_rad_s": 0.0,
        "current_a": 0.12,
        "voltage_v": 12.0,
        "load_torque_nm": 0.0
      }
    }
  ]
}
```

---

# 共通 ServoBehaviorModel 案

## 共通パラメータ

```json
{
  "behavior_model": {
    "position_control": {
      "kp": 0.0,
      "ki": 0.0,
      "kd": 0.0,
      "feedforward_velocity": 0.0,
      "feedforward_acceleration": 0.0
    },
    "limits": {
      "velocity_limit_rad_s": 0.0,
      "acceleration_limit_rad_s2": 0.0,
      "torque_limit_nm": 0.0,
      "current_limit_a": 0.0,
      "pwm_limit_norm": 0.0
    },
    "nonlinear": {
      "deadband_rad": 0.0,
      "static_friction_nm": 0.0,
      "viscous_friction_nms": 0.0,
      "backlash_rad": 0.0
    },
    "delay": {
      "command_delay_s": 0.0,
      "sensor_delay_s": 0.0,
      "control_time_constant_s": 0.0
    },
    "thermal": {
      "thermal_gain": 0.0,
      "cooling_coefficient": 0.0,
      "derating_start_c": 0.0,
      "shutdown_c": 0.0
    },
    "protection": {
      "stall_current_threshold_a": 0.0,
      "stall_error_threshold_rad": 0.0,
      "stall_time_threshold_s": 0.0
    }
  }
}
```

---

# 共通部と専用部の責務分離

## 共通部

```text
ログ形式
単位系
実験計画
前処理
試験区間抽出
損失関数
最適化
検証
レポート生成
ServoBehaviorModel
```

---

## 専用部

```text
通信プロトコル
レジスタ定義
Control Table定義
生値デコード
生値エンコード
単位変換
制御モード変換
機能対応表
安全制約
```

---

# 推奨方針

## 1. DYNAMIXELを先に基準実装にする

理由：

```text
Control Tableが明確
公式ドキュメントが整っている
Present Position / Velocity / Current / PWM が扱いやすい
制御モードが明確
SDKが整っている
```

---

## 2. Feetechはシリーズ別capabilityを作る

理由：

```text
シリーズ差が大きい
販売元・派生品の情報差がある
モデルごとに扱える値が異なる可能性がある
```

---

## 3. ICSは独自機能名を正規化する

例：

```text
stretch → kp / holding_gain
punch → startup_boost / minimum_torque
speed → velocity_limit
response → control_time_constant
damping → kd / viscous_damping
```

---

# 最終判断

Feetech / DYNAMIXEL / ICS は、同じ sysid 仕組みに載せられる。

ただし、実装は以下の形が望ましい。

```text
共通sysid基盤
  ├─ 共通ログ形式
  ├─ 共通単位系
  ├─ 共通実験定義
  ├─ 共通ServoBehaviorModel
  ├─ 共通最適化
  └─ 共通検証

メーカー・シリーズ専用部
  ├─ ICS Adapter
  ├─ Feetech SCS Adapter
  ├─ Feetech STS Adapter
  ├─ Feetech SMS Adapter
  ├─ DYNAMIXEL Protocol 1 Adapter
  └─ DYNAMIXEL Protocol 2 Adapter
```

共通化の単位は、

```text
メーカー名
```

ではなく、

```text
capability profile
```

にするのが良い。

これにより、

```text
同じsysid手順
同じログ形式
同じ最適化処理
同じ検証レポート
```

を使いながら、

```text
通信仕様
レジスタ
制御モード
単位変換
機能差
```

だけを専用部に分離できる。
