# MuJoCo sysid によるシリアルサーボ特性再現

## 概要

シリアルサーボ（例：ICS3.5/3.6）の各種設定機能によって変化する挙動は、MuJoCo の system identification（sysid）により再現可能である。

ただし重要なのは、

- 「サーボ内部挙動モデル」を先に定義する
- sysid はそのモデルのパラメータを推定する

という構成になる点である。

sysid が直接再現するのは「設定値」ではなく、

- 実機の応答特性
- トルク特性
- 遅延
- 摩擦
- 制御応答

などの「観測可能な挙動」である。

---

# sysid の基本構造

## 目的

実機挙動とシミュレーション挙動の差を最小化する。

## 最適化対象

以下のようなパラメータを推定する：

- 剛性（kp）
- ダンピング（kd）
- 摩擦
- 最大トルク
- 速度制限
- 応答時定数
- デッドバンド幅
- 初動補正
- 温度モデル
- 電流制限

## 目的関数

```math
\min_{\theta}\sum_t \|x_{real}(t)-x_{sim}(t,\theta)\|^2
```

- \(x_{real}\)：実測値
- \(x_{sim}\)：シミュレーション値
- \(\theta\)：推定対象パラメータ

---

# ICS系機能の再現可能性

| 機能 | sysid再現 | 備考 |
|---|---|---|
| スピード | 可能 | 速度上限・レート制限 |
| ストレッチ | 可能 | 位置保持剛性 |
| パンチ | 条件付き可能 | 初動トルク補正 |
| デッドバンド | 可能 | 微小誤差時出力停止 |
| プロテクション | 可能 | 状態機械が必要 |
| リミッタ | 可能 | 可動域制限 |
| 温度制限 | 条件付き可能 | 熱モデル追加 |
| 電流制限 | 可能 | トルク上限 |
| リバース | 可能 | 入力符号反転 |
| スレーブ | 不要 | 通信機能 |
| 回転モード | 可能 | 速度制御 |
| レスポンス | 可能 | 応答時定数 |
| ダンピング | 可能 | 粘性制御 |
| ユーザーオフセット | 可能 | ゼロ点補正 |
| シリアル専用 | 不要 | 通信仕様 |
| 現在値取得 | 不要 | センサI/O |

---

# MuJoCo 側で必要な構成

## 標準MJCFだけで可能な項目

以下は標準機能のみで比較的再現しやすい：

- damping
- frictionloss
- armature
- ctrlrange
- forcerange
- joint range

---

## カスタム制御が必要な項目

以下は外部制御ロジックを持つ方が望ましい：

- デッドバンド
- パンチ
- プロテクション
- 温度制御
- 電流制限
- レスポンス調整

---

# 推奨アーキテクチャ

## 基本構成

```text
制御入力
  ↓
サーボ内部モデル
  ↓
MuJoCo actuator
  ↓
剛体・関節シミュレーション
```

---

# サーボ内部モデル例

```json
{
  "servo": {
    "mode": "position",
    "parameters": {
      "speed": 80,
      "stretch": 60,
      "punch": 20,
      "deadband": 3,
      "response": 40,
      "damping": 30,
      "current_limit": 1.8,
      "temperature_limit": 70
    }
  }
}
```

---

# sysid 対象パラメータ例

```json
{
  "sysid": {
    "optimizable": [
      "kp",
      "kd",
      "torque_limit",
      "velocity_limit",
      "deadband_width",
      "response_time_constant",
      "static_friction",
      "thermal_time_constant"
    ]
  }
}
```

---

# 重要な注意点

## 設定値そのものは再現対象ではない

例えば：

```text
ストレッチ = 60
```

が、

```text
kp = 1234
```

へ直接変換できるとは限らない。

必要なのは：

```text
設定値
  ↓
実機挙動
  ↓
等価物理パラメータ
```

への変換である。

---

# 実測データとして必要なもの

最低限：

- 時刻
- 目標角
- 現在角
- サーボ設定値

推奨：

- 電流
- 電圧
- 温度
- 外部負荷
- リンク重量
- 慣性

---

# 実装イメージ

## Python例

```python
def loss(params):
    apply_params(model, params)

    sim_data = run_simulation()

    return np.mean((real_data - sim_data) ** 2)

result = scipy.optimize.minimize(loss, x0)
```

---

# まとめ

sysid により、

- サーボ応答
- トルク特性
- 遅延
- 摩擦
- 保持特性
- 初動特性

などは再現可能である。

ただし、

- 通信仕様
- シリアルモード
- I/O取得機能

などは物理特性ではないため、sysid対象ではない。

また、温度制限や保護機能のような状態依存挙動は、

- 状態機械
- 熱モデル
- 制御ロジック

を追加することで再現精度を高められる。

---

# 参考

- MuJoCo Documentation
  https://mujoco.readthedocs.io/

- MuJoCo MPC
  https://github.com/google-deepmind/mujoco_mpc

- ICS3.5/3.6 機能説明
  https://kondo-robot.com/faq/ics3-5-explain
  