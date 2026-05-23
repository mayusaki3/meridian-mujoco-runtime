# 測定結果から sysid を行う手法

## 概要

実機測定結果から sysid（System Identification）を行う場合、目的は以下である。

```text
実機に入力した指示
↓
実機で観測された応答
↓
シミュレーションモデルの応答
↓
差が最小になるパラメータを推定
```

ここで推定するのは、サーボの設定値そのものではなく、実機挙動を再現するための等価パラメータである。

例：

```text
speed設定値
stretch設定値
punch設定値
deadband設定値
response設定値
damping設定値
```

を直接使うのではなく、それらの結果として現れる、

```text
最大速度
応答遅れ
位置保持剛性
粘性減衰
静止摩擦
最小駆動トルク
電流制限
トルク制限
```

を推定対象にする。

---

# 1. 入力データ

## 1.1 必須データ

sysid に最低限必要なデータは以下である。

```text
時刻
指示値
外部角度
外部速度
サーボ設定値
```

例：

```json
{
  "timestamp": 123456789,
  "servo_id": 1,
  "command": {
    "target_position": 90.0,
    "speed": 80,
    "stretch": 60
  },
  "external": {
    "angle": 87.5,
    "velocity": 120.3
  }
}
```

---

## 1.2 推奨データ

精度を上げるには以下も取得する。

```text
ICS現在角
外部電流
ICS電流
電源電圧
温度
ロードセル値
負荷条件
保護状態
生シリアルパケット
```

---

# 2. 前処理

## 2.1 タイムスタンプ同期

すべての観測値を同じ時間軸に揃える。

```text
指示ログ
ICS取得値
外部エンコーダ
電流センサ
ロードセル
温度センサ
```

を共通タイムスタンプで整列する。

---

## 2.2 リサンプリング

測定周期が異なる場合は、共通周期に補間する。

例：

```text
外部エンコーダ: 1000Hz
電流: 1000Hz
ICS通信: 50〜200Hz
温度: 10Hz
```

この場合、sysid用には例えば 1kHz または 500Hz の共通系列にする。

---

## 2.3 フィルタリング

外部角度から速度・加速度を求める場合、微分ノイズが出やすい。

そのため、以下を行う。

```text
角度の平滑化
速度の平滑化
異常値除去
センサ欠損の補間
```

ただし、パンチや初動応答を見る場合は過度に平滑化しない。

---

## 2.4 区間抽出

測定データ全体を、目的別の試験区間に分ける。

```text
ステップ応答区間
保持負荷区間
微小指令区間
拘束区間
温度上昇区間
回転モード区間
```

---

# 3. 推定対象パラメータ

## 3.1 基本パラメータ

```text
kp
kd
friction
static_friction
armature
torque_limit
velocity_limit
acceleration_limit
response_delay
deadband_width
```

---

## 3.2 サーボ機能別パラメータ

| 機能 | 推定対象 |
|---|---|
| speed | velocity_limit, acceleration_limit |
| stretch | kp, holding_torque_gain |
| punch | minimum_torque, startup_boost |
| deadband | deadband_width |
| response | time_constant, response_delay |
| damping | kd, viscous_damping |
| current limit | torque_limit, current_to_torque_gain |
| protection | stall_threshold, stall_time |
| temperature limit | thermal_time_constant, derating_curve |

---

# 4. 手法1：ステップ応答フィッティング

## 4.1 概要

最も基本的な方法である。

目標角をステップ状に変化させ、実角度の応答波形にシミュレーションを合わせる。

```text
target_angle(t)
↓
actual_angle(t)
```

---

## 4.2 推定しやすいもの

```text
応答遅れ
最大速度
最大加速度
kp
kd
摩擦
オーバーシュート
整定時間
```

---

## 4.3 目的関数

```math
loss = Σ(angle_real(t) - angle_sim(t))^2
```

速度も含める場合：

```math
loss = Σ w_pos(angle_real - angle_sim)^2
     + Σ w_vel(velocity_real - velocity_sim)^2
```

---

## 4.4 適用対象

```text
speed
stretch
response
damping
```

---

# 5. 手法2：負荷応答フィッティング

## 5.1 概要

既知負荷を与えた状態で、角度ズレや保持限界を測定する。

```text
負荷トルク
↓
角度誤差
↓
保持力
```

を推定する。

---

## 5.2 推定しやすいもの

```text
位置保持剛性
最大保持トルク
電流制限
トルク制限
コンプライアンス
```

---

## 5.3 目的関数

```math
loss = Σ(angle_error_real - angle_error_sim)^2
     + Σ(load_real - load_sim)^2
```

---

## 5.4 適用対象

```text
stretch
current limit
torque limit
protection
```

---

# 6. 手法3：初動応答フィッティング

## 6.1 概要

停止状態から小さな指令を与え、動き出しの瞬間を観測する。

```text
静止状態
↓
微小指令
↓
動き出し
```

---

## 6.2 推定しやすいもの

```text
静止摩擦
最小駆動トルク
パンチ量
初動電流ピーク
起動遅れ
```

---

## 6.3 必要な観測

```text
外部角度
外部速度
外部電流
指示時刻
```

---

## 6.4 適用対象

```text
punch
deadband
static friction
```

---

# 7. 手法4：デッドバンド推定

## 7.1 概要

目標角を微小に変化させ、実軸が反応する閾値を求める。

```text
±0.1°
±0.2°
±0.5°
±1.0°
```

のように入力を変える。

---

## 7.2 推定しやすいもの

```text
deadband_width
neutral_zone
微小誤差時の出力停止条件
```

---

## 7.3 判定例

```text
target差分 < threshold:
  出力なし

target差分 >= threshold:
  出力あり
```

この threshold を sysid パラメータにする。

---

# 8. 手法5：電流・トルク変換推定

## 8.1 概要

外部電流と負荷トルクの関係から、電流と出力トルクの対応を推定する。

```text
current
↓
torque
```

---

## 8.2 推定しやすいもの

```text
current_to_torque_gain
torque_limit
current_limit
saturation
```

---

## 8.3 方法

```text
1. 既知アーム長を使用
2. 既知荷重を加える
3. 外部電流を測定
4. 保持可能限界を測定
5. 電流と推定トルクの関係を回帰する
```

---

## 8.4 トルク計算

```math
τ = m g L
```

またはロードセルを使用する場合：

```math
τ = F L
```

---

# 9. 手法6：保護動作・状態機械推定

## 9.1 概要

プロテクションや脱力動作は、連続パラメータだけではなく状態遷移として扱う。

---

## 9.2 状態例

```text
NORMAL
CURRENT_LIMITED
STALL_DETECTED
PROTECTION_ACTIVE
TORQUE_OFF
RECOVERY
```

---

## 9.3 推定対象

```text
stall_current_threshold
stall_angle_error_threshold
stall_time_threshold
shutdown_delay
recovery_condition
```

---

## 9.4 方法

```text
1. 軽い拘束から始める
2. 電流・角度誤差・時間を記録する
3. 脱力または制限開始タイミングを抽出する
4. 閾値と遅延時間を推定する
```

---

# 10. 手法7：温度モデル推定

## 10.1 概要

温度上昇によるトルク低下や保護動作を、熱モデルとして推定する。

---

## 10.2 簡易熱モデル

```math
T(t+1) = T(t) + a \cdot I(t)^2 - b \cdot (T(t) - T_{ambient})
```

---

## 10.3 推定対象

```text
thermal_gain
cooling_coefficient
thermal_time_constant
temperature_limit_threshold
torque_derating_curve
```

---

## 10.4 適用対象

```text
temperature limit
long duration load
continuous motion
```

---

# 11. 手法8：周波数応答推定

## 11.1 概要

正弦波状の指令を与え、周波数ごとの追従性を測定する。

```text
低周波 → 高周波
```

へ変化させる。

---

## 11.2 推定しやすいもの

```text
制御帯域
位相遅れ
ゲイン低下
応答遅れ
ダンピング
```

---

## 11.3 適用対象

```text
response
damping
speed
control delay
```

---

# 12. 手法9：複数条件同時最適化

## 12.1 概要

単一試験だけでなく、複数試験をまとめて最適化する。

```text
無負荷ステップ応答
負荷ありステップ応答
保持試験
微小指令試験
拘束試験
```

を同時に使う。

---

## 12.2 目的関数例

```math
loss_total =
  w1 * loss_step
+ w2 * loss_load
+ w3 * loss_current
+ w4 * loss_deadband
+ w5 * loss_protection
```

---

## 12.3 利点

```text
特定条件だけに過剰適合しにくい
汎用的なパラメータになりやすい
機能間の相互作用を扱いやすい
```

---

# 13. 最適化アルゴリズム

## 13.1 グリッドサーチ

## 概要

パラメータ範囲を格子状に試す。

## 利点

```text
実装が簡単
局所解に強い
```

## 欠点

```text
パラメータ数が増えると計算量が爆発する
```

## 用途

```text
初期探索
パラメータ範囲確認
```

---

## 13.2 ランダムサーチ

## 概要

範囲内をランダムに試す。

## 利点

```text
高次元でも使いやすい
実装が簡単
```

## 欠点

```text
収束効率は高くない
```

---

## 13.3 Nelder-Mead

## 概要

勾配を使わない局所最適化。

## 利点

```text
シミュレーションが微分不能でも使える
実装が簡単
```

## 欠点

```text
局所解に入りやすい
高次元には弱い
```

---

## 13.4 CMA-ES

## 概要

進化戦略によるブラックボックス最適化。

## 利点

```text
非線形
ノイズあり
局所解あり
```

の問題に比較的強い。

## 欠点

```text
シミュレーション回数が多い
```

## 用途

```text
サーボ特性全体のフィッティング
複数条件同時最適化
```

---

## 13.5 ベイズ最適化

## 概要

少ない試行回数で良いパラメータを探す。

## 利点

```text
実機試験回数を減らせる
高コスト実験向き
```

## 欠点

```text
高次元にはやや弱い
実装が複雑
```

## 用途

```text
実機での追加測定計画
高コスト試験
```

---

## 13.6 勾配法

## 概要

微分可能なモデルに対して勾配を使って最適化する。

## 利点

```text
収束が速い
高次元に強い
```

## 欠点

```text
モデルが滑らかである必要がある
deadbandやprotectionのような不連続要素に弱い
```

---

# 14. 推奨手順

## Step 1：単純なモデルから始める

最初は以下だけを推定する。

```text
kp
kd
velocity_limit
torque_limit
friction
```

---

## Step 2：ステップ応答で合わせる

無負荷ステップ応答で基本パラメータを推定する。

---

## Step 3：負荷試験を追加する

保持負荷・既知トルク試験で、

```text
kp
torque_limit
current_to_torque_gain
```

を調整する。

---

## Step 4：初動・デッドバンドを追加する

微小指令試験で、

```text
deadband_width
static_friction
minimum_torque
startup_boost
```

を推定する。

---

## Step 5：保護・温度モデルを分離して扱う

保護動作や温度制限は、通常応答とは別に状態機械として推定する。

---

## Step 6：全試験をまとめて再最適化する

最後に複数条件を同時に使い、過剰適合を避ける。

---

# 15. 推奨パラメータ分離

## 15.1 連続制御パラメータ

```text
kp
kd
friction
velocity_limit
torque_limit
response_delay
```

通常の最適化で推定する。

---

## 15.2 不連続パラメータ

```text
deadband_width
protection_threshold
current_limit_threshold
```

閾値推定として扱う。

---

## 15.3 状態依存パラメータ

```text
temperature_limit
stall_protection
recovery_condition
```

状態機械として扱う。

---

# 16. 評価指標

## 16.1 角度誤差

```math
RMSE_angle = sqrt(mean((angle_real - angle_sim)^2))
```

---

## 16.2 速度誤差

```math
RMSE_velocity = sqrt(mean((velocity_real - velocity_sim)^2))
```

---

## 16.3 電流誤差

```math
RMSE_current = sqrt(mean((current_real - current_sim)^2))
```

---

## 16.4 イベント誤差

保護動作などは、イベント時刻の差を見る。

```text
実機保護時刻 - シミュレーション保護時刻
```

---

# 17. 検証方法

## 17.1 学習用データと検証用データを分ける

同じ測定データだけに合わせると過剰適合する。

そのため、

```text
sysid用データ
検証用データ
```

を分ける。

---

## 17.2 未使用条件で検証する

例：

```text
sysid:
  speed = 40, 80
  stretch = 40, 80

検証:
  speed = 60
  stretch = 60
```

このように中間条件で確認する。

---

# 18. 推奨する実装構成

## 18.1 処理フロー

```text
測定ログ読み込み
↓
前処理
↓
試験区間抽出
↓
初期パラメータ設定
↓
シミュレーション実行
↓
誤差計算
↓
最適化
↓
検証
↓
パラメータ保存
```

---

## 18.2 疑似コード

```python
def loss(params, datasets):
    total_loss = 0.0

    for dataset in datasets:
        sim_result = run_simulation(params, dataset.command_log)

        total_loss += compare_angle(dataset.external_angle, sim_result.angle)
        total_loss += compare_velocity(dataset.external_velocity, sim_result.velocity)
        total_loss += compare_current(dataset.current, sim_result.current)

    return total_loss


result = optimizer.minimize(
    objective=loss,
    initial_params=initial_params,
    bounds=parameter_bounds,
    datasets=train_datasets
)
```

---

# 19. まとめ

測定結果から sysid を行う場合、考えられる手法は以下である。

```text
1. ステップ応答フィッティング
2. 負荷応答フィッティング
3. 初動応答フィッティング
4. デッドバンド推定
5. 電流・トルク変換推定
6. 保護動作の状態機械推定
7. 温度モデル推定
8. 周波数応答推定
9. 複数条件同時最適化
```

推奨は、

```text
単純な連続モデル
↓
負荷モデル
↓
初動・デッドバンド
↓
保護・温度モデル
↓
複数条件同時最適化
```

の順に進めることである。

最初からすべてを同時に推定すると、原因分離が難しくなる。

そのため、実機sysidでは、

```text
測定条件を分ける
推定対象を分ける
最後に統合する
```

という手順が望ましい。