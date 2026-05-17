[目次](../README.md) > docs/ja-JP > フィードバック > MuJoCo VR リアルタイム同期検討

# MuJoCo VR リアルタイム同期検討

## 概要

MuJoCo を authoritative physics とし、VR 側は visualization / interaction を担当する構成を検討する。

本検討では以下を目的とする。

- MuJoCo による物理演算
- 実機制御コードとの共通化
- VR 側への状態同期
- 実時間格闘シミュレーション
- deterministic simulation
- simulation time ベース制御
- adaptive degradation
- HIL/SIL 対応

---

## PlatformIO と MuJoCo の連携

PlatformIO と MuJoCo を連携し、以下を実現する。

- 実機制御
- シミュレーション
- HIL
- SIL
- sysid
- PID tuning
- VR 可視化

---

## 基本構成

```text
MuJoCo
  ↑↓
通信層(TCP/UDP/Serial/Ethernet)
  ↑↓
PlatformIO firmware
  ↑↓
ESP32 / STM32 / サーボ
```

---

## 推奨最小構成

```text
PC
 ├─ MuJoCo
 ├─ Python bridge
 └─ UDP socket
        ↕
ESP32 / STM32
 └─ PlatformIO firmware
```

---

## 制御ロジック分離

制御ロジックとハードウェア依存部分を分離する。

推奨構成：

```text
lib/
  control/
  communication/
  hardware/
  simulator/
```

---

## Hardware Abstraction Layer

例：

```cpp
class IJoint {
public:
    virtual float getAngle() = 0;
    virtual void setTorque(float t) = 0;
};
```

実機：

```cpp
class RealJoint : public IJoint
```

MuJoCo：

```cpp
class SimJoint : public IJoint
```

これにより同じ制御コードを実機とシミュレータで共通利用可能。

---

## 通信方式

推奨順位：

| 通信 | 推奨 |
|---|---|
| UDP | ◎ |
| Ethernet | ◎ |
| TCP | ○ |
| USB Serial | ○ |
| CAN | ○ |

---

## UDP 推奨理由

- 実装容易
- Python と相性が良い
- ESP32 実装容易
- デバッグ容易

---

## sysid

以下の流れを実現する。

```text
実機ログ取得
→ MuJoCo parameter optimization
→ simulator reproduction
```

---

## simulation time ベース制御

実時間(real time)ではなく simulation time を基準とする。

例：

```text
実時間 10ms
= simulation 0.1ms
```

でも成立可能。

重要なのは wall clock ではなく simulation clock を基準とすること。

---

## deterministic simulation

simulation step を基準とした同期を行う。

例：

```text
step 1000
```

を同期単位として扱う。

これにより以下を実現可能。

- 再現性
- rollback
- replay
- sysid
- deterministic debug

---

## tick driven architecture

重要なのは timer interrupt ではなく tick 駆動。

例：

```cpp
controller.tick(dt);
```

---

## timer interrupt 非依存

割り込み無しでも以下で動作可能。

```cpp
while (true) {
    controller.tick(dt);
}
```

interrupt 使用時：

```cpp
ISR(timer) {
    controller.tick(dt);
}
```

---

## PlatformIO 依存除去

制御コアは PlatformIO や timer interrupt に依存しない構成を推奨。

時間供給と制御ロジックを分離する。

---

## 抽象化例

```cpp
class IClock {
public:
    virtual uint64_t now() = 0;
};
```

---

## time source

| 環境 | time source |
|---|---|
| ESP32 | hardware timer |
| Linux | std::chrono |
| MuJoCo | simulation step |
| unit test | fake clock |

---

## MuJoCo authoritative physics

```text
MuJoCo
  ↓ state sync
VR System
```

MuJoCo を物理演算の正本(authoritative physics)とする。

VR 側では以下を行う。

- 表示
- 人間入力
- UI
- VR interaction
- 補間

VR 側は基本的に physics authoritative にならない。

---

## 状態同期方式

### 同期対象

最低限必要な情報は以下。

| 項目 | 必要性 |
|---|---|
| root position | 必須 |
| root rotation | 必須 |
| joint angle | 必須 |
| joint velocity | 推奨 |
| torque | 任意 |

---

## 構造一致要件

MuJoCo モデルと VR モデルは以下が一致している必要がある。

- joint hierarchy
- joint axis
- coordinate system
- root definition

例：

```text
root
 ├─ hip
 │   ├─ knee
 │   └─ ankle
```

joint 構造が一致していることで、角度情報と位置情報の同期のみで VR 側に物理演算結果を反映可能。

---

## 実時間格闘シミュレーション

人間入力による格闘シミュレーションでは実時間性が重要。

しかし高精度 physics により計算負荷が増大する可能性がある。

そのため、負荷時には adaptive degradation を行う。

---

## adaptive degradation

### 例

| Level | 内容 |
|---|---|
| 0 | full physics |
| 1 | simplified contact |
| 2 | kinematic assist |
| 3 | emergency bypass |

---

## bypass mode

高負荷時には full dynamics を一部停止し、簡略モードへ移行する。

例：

- finger collision disable
- cloth disable
- contact simplification
- pose interpolation
- substep reduction

---

## physics LOD

負荷に応じて physics detail を変更する。

例：

| 通常 | 軽量 |
|---|---|
| full contact | simplified contact |
| tendon | disable |
| finger collision | disable |
| cloth | disable |

---

## 可変 simulation frequency

通常：

```text
1000Hz
```

高負荷時：

```text
250Hz
```

へ低下。

controller 側は可変 dt に対応する。

---

## VR 同期

MuJoCo と VR は一般的に周波数が異なる。

例：

| System | Frequency |
|---|---|
| MuJoCo | 1000Hz |
| VR Render | 90Hz |

このため interpolation が必要。

---

## interpolation

step 間を補間する。

例：

```text
step100
step101
```

間を slerp 等で補間。

これにより滑らかな VR 表示を実現する。

---

## authoritative model

推奨構成：

```text
MuJoCo Server
  ↓ state packet
VR Client
```

状態同期を行い、入力同期ではなく state replication を行う。

---

## 推奨アーキテクチャ

```text
Input Layer
Physics Layer
Control Layer
Rendering Layer
Network Layer
```

---

## 推奨構成

初期構成：

```text
ESP32
+ UDP
+ Python
+ MuJoCo
```

将来的には以下へ拡張可能。

- Ethernet
- CAN
- ROS2
- HIL
- SIL
- rollback simulation
- distributed simulation

---

## 結論

以下の方向性は成立可能。

- simulation time 基準制御
- deterministic simulation
- MuJoCo authoritative physics
- VR visualization synchronization
- adaptive degradation
- bypass mode
- state replication
- PlatformIO 非依存 control core

特に、joint structure が一致していれば、角度情報および位置情報の同期のみで MuJoCo の物理演算結果を VR 側へ反映可能。

---
[目次](../README.md) > docs/ja-JP > フィードバック > MuJoCo VR リアルタイム同期検討