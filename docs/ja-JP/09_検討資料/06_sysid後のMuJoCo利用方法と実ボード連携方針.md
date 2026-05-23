# sysid後のMuJoCo利用方法と実ボード連携方針

## 概要

sysidで得たパラメータは、MuJoCo上で以下の3通りに利用できる。

```text
1. MJCFモデルへ反映する
2. 外部制御ループで反映する
3. 実ボードを接続してHIL/SILシミュレーションする
```

MuJoCoでは、`mj_step` によりシミュレーションを1ステップ進め、制御入力は `mjData.ctrl`、外力は `mjData.qfrc_applied` や `mjData.xfrc_applied` に設定できる。制御コールバック `mjcb_control` により、ステップ中に独自制御を差し込むこともできる。  
出典：MuJoCo公式ドキュメント  
https://mujoco.readthedocs.io/en/stable/APIreference/APIfunctions.html  
https://mujoco.readthedocs.io/en/stable/programming/simulation.html

---

# 1. sysid結果の利用先

## 1.1 MJCFに直接反映する項目

以下はMJCFのjoint、actuator、body設定へ反映しやすい。

```text
質量
慣性
重心
関節damping
関節frictionloss
armature
関節可動域
actuator gain
actuator bias
forcerange
ctrlrange
```

例：

```xml
<joint name="joint1"
       type="hinge"
       damping="0.05"
       frictionloss="0.02"
       armature="0.001"
       range="-1.57 1.57" />

<actuator>
  <position name="joint1_pos"
            joint="joint1"
            kp="12.0"
            forcerange="-1.5 1.5"
            ctrlrange="-1.57 1.57" />
</actuator>
```

---

## 1.2 外部制御ループで反映する項目

以下はMJCFだけで表現するより、外部制御コードで扱う方がよい。

```text
デッドバンド
パンチ
レスポンス遅れ
速度制限
加速度制限
電流制限
温度制限
プロテクション
PWMサーボのパルス幅変換
シリアルサーボの内部設定値変換
QDDの電流・トルク制御
```

理由：

```text
不連続な条件分岐がある
状態依存で変化する
温度・電流など内部状態を持つ
デバイスカテゴリごとに挙動が違う
```

---

# 2. 推奨構成

## 2.1 基本構成

```text
MuJoCo model
  ↓
ActuatorBehaviorModel
  ↓
DeviceCategoryModel
  ↓
DeviceSpecificModel
  ↓
mjData.ctrl / qfrc_applied
```

---

## 2.2 処理フロー

```text
1. MuJoCoから現在状態を読む
2. コントローラへセンサー値を渡す
3. コントローラが指令を出す
4. デバイスモデルが指令を変換する
5. MuJoCoへ ctrl または force を設定する
6. mj_step で1ステップ進める
7. MuJoCoのセンサー値を取得する
8. ログへ保存する
```

---

# 3. MuJoCoへの反映方法

## 3.1 位置制御サーボ

### 使う値

```text
target_position
kp
kd
torque_limit
velocity_limit
deadband
response_delay
```

### 実装方法

```text
目標角と現在角の差を計算
↓
デッドバンド判定
↓
速度制限・応答遅れを適用
↓
PD制御でトルク計算
↓
トルク上限でクリップ
↓
mjData.ctrl または qfrc_applied に反映
```

---

## 3.2 PWM角度制御サーボ

### 使う値

```text
pulse_width_us
pulse_to_angle_mapping
deadband
response_delay
torque_limit
velocity_limit
```

### 実装方法

```text
PWMパルス幅
↓
目標角へ変換
↓
位置制御モデルへ入力
↓
MuJoCo jointへ反映
```

例：

```text
1000us → -90°
1500us →   0°
2000us → +90°
```

---

## 3.3 シリアルサーボ

### 使う値

```text
target_position
target_velocity
speed_setting
stretch_setting
punch_setting
current_limit
temperature_limit
protection_state
```

### 実装方法

```text
シリアルサーボ設定値
↓
正規化パラメータへ変換
↓
サーボ内部モデルで出力トルク算出
↓
MuJoCoへ反映
```

---

## 3.4 QDD

### 使う値

```text
target_torque
target_current
torque_constant
current_limit
motor_inertia
joint_inertia
friction
backdrivability
```

### 実装方法

```text
電流指令またはトルク指令
↓
トルクへ変換
↓
遅れ・電流制限を適用
↓
MuJoCo joint torqueへ反映
```

QDDでは、位置制御よりもトルク制御・外力応答が重要になる。

---

## 3.5 車輪駆動

### 使う値

```text
wheel_velocity
wheel_torque
wheel_radius
rolling_resistance
ground_friction
slip_ratio
drive_delay
```

### 実装方法

```text
車輪トルクまたは速度指令
↓
ホイールjointへ反映
↓
床面接触で移動
↓
姿勢・速度・スリップを観測
```

---

# 4. sysid結果ファイル案

## 4.1 例

```json
{
  "sysid_result": {
    "device_category": "serial_servo",
    "vendor": "example",
    "model": "example_servo",
    "model_version": "1.0",
    "control": {
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

# 5. MuJoCoでの実装イメージ

## 5.1 疑似コード

```python
while running:
    # 1. MuJoCo状態取得
    qpos = data.qpos.copy()
    qvel = data.qvel.copy()
    sensor = data.sensordata.copy()

    # 2. コントローラ入力生成
    command = controller.update(
        qpos=qpos,
        qvel=qvel,
        sensor=sensor
    )

    # 3. sysid済みデバイスモデルで出力計算
    torque = actuator_model.compute_torque(
        command=command,
        position=qpos[joint_index],
        velocity=qvel[joint_index],
        dt=model.opt.timestep
    )

    # 4. MuJoCoへ反映
    data.ctrl[actuator_index] = torque

    # 5. シミュレーションを進める
    mujoco.mj_step(model, data)
```

---

# 6. 実ボードを使ったシミュレーション

## 6.1 可能か

可能である。

ただし、目的により以下の2種類に分ける。

```text
SIL:
  Software In the Loop
  実ボードを使わず、制御ソフトだけをMuJoCoと接続する

HIL:
  Hardware In the Loop
  実ボードをMuJoCoと接続し、ボードが実機のように制御する
```

---

# 7. HIL構成

## 7.1 構成例

```text
MuJoCo
  ↓ センサー値出力
PC側ブリッジ
  ↓ UART / CAN / USB / Ethernet
実ボード
  ↓ 制御指令
PC側ブリッジ
  ↓
MuJoCo
```

---

## 7.2 データの流れ

```text
MuJoCo:
  仮想センサー値を生成

実ボード:
  実ロボットから来た値として受信

実ボード:
  モーター指令を出力

MuJoCo:
  その指令を受け取り、仮想アクチュエータへ反映
```

---

# 8. MuJoCo側からセンサー値を出力できるか

## 8.1 結論

可能である。

MuJoCoはモデル内にセンサーを定義でき、シミュレーション中に `mjData.sensordata` からセンサー値を取得できる。さらに、Python APIではMuJoCoのC APIに対応した操作が可能で、独自callbackも利用できる。  
出典：MuJoCo公式Pythonドキュメント  
https://mujoco.readthedocs.io/en/stable/python.html

---

## 8.2 出力できる代表値

```text
joint position
joint velocity
joint force / torque
body position
body orientation
body velocity
accelerometer
gyro
touch / force
actuator force
camera
rangefinder
```

---

## 8.3 実ボードへ出力する場合

MuJoCoセンサー値を、実ボードが期待する形式へ変換する。

例：

```text
MuJoCo joint angle [rad]
↓
DYNAMIXEL Present Position raw value

MuJoCo joint velocity [rad/s]
↓
Present Velocity raw value

MuJoCo current estimate [A]
↓
Present Current raw value

MuJoCo temperature model [℃]
↓
Present Temperature raw value
```

---

# 9. 実ボード接続時の注意点

## 9.1 時間同期

最重要である。

```text
MuJoCo timestep
実ボード制御周期
通信周期
```

を揃える必要がある。

例：

```text
MuJoCo timestep: 1ms
制御周期: 2ms
通信周期: 2ms
```

---

## 9.2 リアルタイム性

HILでは、MuJoCoを実時間に近い速度で動かす必要がある。

```text
シミュレーションが遅れる
↓
ボード側制御が破綻する
```

可能性がある。

---

## 9.3 通信遅延

通信遅延はsysid済みモデルへ含めるか、HILブリッジで補正する。

```text
command_delay_s
sensor_delay_s
transport_delay_s
```

---

## 9.4 センサー値の量子化

実ボードが整数値を期待する場合、MuJoCo値を量子化する。

```text
rad → raw position
A → raw current
℃ → raw temperature
```

この量子化も、実機再現には重要である。

---

## 9.5 ノイズ付与

理想値をそのまま渡すと、実機よりきれいすぎる。

必要に応じて以下を加える。

```text
センサーノイズ
分解能
遅延
ドロップ
通信周期
飽和
```

---

# 10. 実ボードでシミュレーションする場合の方式

## 10.1 実ボードを「コントローラ」として使う

最も現実的な方式。

```text
MuJoCo = 仮想ロボット
実ボード = 制御器
```

流れ：

```text
MuJoCoセンサー値
↓
実ボード
↓
実ボードのモーター指令
↓
MuJoCo
```

この場合、実ボード上の制御ロジックをそのまま検証できる。

---

## 10.2 実ボードを「仮想デバイスI/O」として使う

ボードがサーボ通信やモータードライバ通信を担当する。

```text
MuJoCo
↓
仮想センサー値
↓
ボード
↓
サーボ風の応答
```

これは、上位制御ソフトのテストに向く。

---

## 10.3 実ボードと実モーターを一部使う

一部だけ実機、残りをMuJoCoにする。

```text
実モーター1軸
+
MuJoCoロボット全体
```

これは精度は高いが、同期と安全制御が難しい。

---

# 11. MuJoCo側センサー出力の設計要件

## 11.1 共通出力形式

```json
{
  "timestamp": 1.234,
  "sensor": {
    "joint_position_rad": 0.52,
    "joint_velocity_rad_s": 1.2,
    "joint_torque_nm": 0.8,
    "imu": {
      "accel_m_s2": [0.0, 0.0, 9.8],
      "gyro_rad_s": [0.0, 0.1, 0.0]
    },
    "contact": {
      "force_n": [0.0, 0.0, 12.0]
    }
  }
}
```

---

## 11.2 デバイス別変換

```text
common sensor
↓
device category mapping
↓
device specific raw value
```

例：

```text
joint_position_rad
↓
serial servo present_position
↓
DYNAMIXEL raw position
```

---

# 12. sysid結果を使ったMuJoCo運用モード

## 12.1 オフライン検証

```text
測定ログ
↓
MuJoCo再生
↓
実機ログと比較
```

用途：

```text
sysid結果の検証
モデル品質確認
```

---

## 12.2 通常シミュレーション

```text
MuJoCo
↓
仮想ロボット制御
```

用途：

```text
制御アルゴリズム開発
動作検証
```

---

## 12.3 SIL

```text
MuJoCo
+
実制御ソフト
```

用途：

```text
制御ソフト検証
実機なし開発
```

---

## 12.4 HIL

```text
MuJoCo
+
実制御ボード
```

用途：

```text
ボード実装検証
通信処理検証
リアルタイム制御検証
```

---

# 13. 共通部に入れるべき要件

将来HILまで考えるなら、共通部には以下を入れておくべきである。

```text
1. sysid結果パラメータ形式
2. MuJoCo反映マッピング
3. MJCF反映可能項目
4. 外部制御ループ反映項目
5. センサー出力抽象化
6. デバイス別raw値変換
7. 遅延・ノイズ・量子化モデル
8. HIL用通信ブリッジ抽象化
9. シミュレーション時刻管理
10. 実時間同期モード
```

---

# 14. まとめ

sysid後のMuJoCo利用方法は、以下に分けるとよい。

```text
MJCFへ固定値として反映するもの
外部制御モデルとして反映するもの
HIL/SIL接続で実ボードとやり取りするもの
```

また、MuJoCo側からセンサー値を出力することは可能である。

その場合は、

```text
MuJoCo内部値
↓
共通センサー表現
↓
デバイスカテゴリ別変換
↓
個別デバイスraw値
↓
実ボードへ送信
```

という流れにする。

特にHILを想定するなら、

```text
センサー値の遅延
量子化
ノイズ
通信周期
時刻同期
```

を最初から共通部の要件に入れておくべきである。
