# HILにおける実ボード制御周期同期方式

## 目的

HILでは、実ボード側の制御周期をMuJoCoのシミュレーション周期に同期させたい。

想定方式：

```text
実ボードは1制御周期ごとに処理を進める
ただし、MuJoCo側から同期信号が来るまで待つ
MuJoCo側の1 step完了後、次周期の開始信号を送る
```

---

# 基本方針

## 通常の実機制御

```text
ボード内タイマ割り込み
↓
周期ごとに制御処理
↓
モーター指令出力
```

## HIL同期制御

```text
MuJoCo step
↓
センサー値送信
↓
ボードが1周期分制御
↓
ボードが指令を返す
↓
MuJoCoへ反映
↓
次のMuJoCo step
```

つまり、HIL時はボード側の自律タイマではなく、

```text
MuJoCo側クロック
```

を制御周期の基準にする。

---

# 推奨構成

```text
MuJoCo PC
  ├─ simulation clock
  ├─ sensor packet生成
  ├─ step token送信
  ├─ command packet受信
  └─ mj_step実行

実ボード
  ├─ step token受信待ち
  ├─ sensor packet読込
  ├─ 1周期分の制御計算
  ├─ actuator command生成
  └─ done応答送信
```

---

# 通信シーケンス

```text
[MuJoCo] step N の状態を取得
[MuJoCo] sensor_packet(N) + step_token(N) を送信
[Board ] step_token(N) を受信
[Board ] control_tick(N) を1回だけ実行
[Board ] command_packet(N) を返信
[MuJoCo] command_packet(N) を適用
[MuJoCo] mj_step()
[MuJoCo] step N+1 へ進む
```

---

# プロトコル例

## MuJoCo → Board

```json
{
  "type": "hil_step",
  "step_index": 12345,
  "sim_time_s": 12.345,
  "dt_s": 0.001,
  "sensors": {
    "joint_position_rad": [0.1, 0.2],
    "joint_velocity_rad_s": [1.0, 1.2],
    "imu_accel_m_s2": [0.0, 0.0, 9.8],
    "imu_gyro_rad_s": [0.0, 0.0, 0.0]
  }
}
```

## Board → MuJoCo

```json
{
  "type": "hil_command",
  "step_index": 12345,
  "commands": {
    "joint_torque_nm": [0.3, 0.4],
    "joint_target_position_rad": [0.5, 0.6],
    "motor_pwm_norm": [0.2, 0.1]
  },
  "status": {
    "control_executed": true,
    "fault": false
  }
}
```

---

# ボード側の制御ループ

## 通常モード

```c
while (true) {
    wait_timer_interrupt();
    read_sensors();
    control_tick();
    write_actuators();
}
```

## HIL同期モード

```c
while (true) {
    wait_hil_step_packet();

    read_virtual_sensors_from_packet();

    control_tick_once();

    send_hil_command_packet();
}
```

---

# 重要要件

## 1. step_indexを必ず持つ

```text
MuJoCo側step_index
Board側step_index
```

を一致させる。

目的：

```text
パケット取り違え防止
遅延検出
欠落検出
再送判定
ログ同期
```

---

## 2. ボードは勝手に複数周期進めない

HIL同期モードでは、以下を禁止する。

```text
ボード内タイマだけでcontrol_tickを連続実行する
古いsensor値で複数回制御する
MuJoCoからのstep_tokenなしに進む
```

---

## 3. timeoutを設ける

MuJoCo側から信号が来ない場合、ボードは永久待機しない。

例：

```text
timeout発生
↓
HIL_FAULT
↓
出力ゼロ
↓
安全停止
```

---

## 4. 実時間同期と非実時間同期を分ける

## 非実時間HIL

```text
MuJoCoが遅くてもよい
ボードはMuJoCoを待つ
シミュレーション時間だけが進む
```

用途：

```text
再現性重視
デバッグ
高負荷モデル
```

## 実時間HIL

```text
MuJoCoは実時間以内にstepを返す必要がある
ボード側timeoutを短くする
```

用途：

```text
実制御周期検証
通信遅延検証
実機近似
```

---

# 制御周期の扱い

## 1:1同期

```text
MuJoCo timestep = ボード制御周期
```

例：

```text
MuJoCo dt = 1ms
Board control period = 1ms
```

最も単純で推奨。

---

## N:1同期

```text
MuJoCo timestep < ボード制御周期
```

例：

```text
MuJoCo dt = 0.5ms
Board control period = 2ms
```

この場合：

```text
MuJoCo 4 stepごとにBoardへstep_token送信
Board指令は4 step保持
```

---

## 1:N同期

```text
MuJoCo timestep > ボード制御周期
```

例：

```text
MuJoCo dt = 2ms
Board control period = 1ms
```

非推奨。

理由：

```text
ボード側が1ms周期で動きたいのに
MuJoCoセンサーが2msごとにしか更新されない
```

対応する場合は：

```text
センサー補間
複数control_tick実行
予測値生成
```

が必要になる。

---

# 推奨設計

## HILクロックマスター

```text
MuJoCoをクロックマスターにする
```

理由：

```text
シミュレーション状態を決めるのはMuJoCo
再現性を確保しやすい
ログとstepが一致しやすい
非実時間実行が可能
```

---

# HIL同期モードの状態機械

```text
DISCONNECTED
  ↓
CONNECTED
  ↓
SYNC_WAIT
  ↓
STEP_RECEIVED
  ↓
CONTROL_EXECUTING
  ↓
COMMAND_SENT
  ↓
SYNC_WAIT
```

---

# ボード側状態

```text
NORMAL_REAL_DEVICE_MODE
HIL_SYNC_MODE
HIL_TIMEOUT
HIL_FAULT
SAFE_STOP
```

---

# 安全設計

## timeout時

```text
出力0
トルクOFF
PWM停止
エラー通知
ログ保存
```

---

## step_index不一致時

```text
古いパケット破棄
欠落検出
必要なら再同期
```

---

## dt不一致時

```text
制御計算に使うdtはpacket内のdt_sを優先
```

---

# 実装上の注意

## 1. 割り込み制御

HIL同期モードでは、通常の周期タイマ割り込みを以下のどちらかにする。

```text
無効化
または
タイマは監視専用にする
```

制御処理そのものは、step packet受信で起動する。

---

## 2. 通信遅延

HILでは以下をログに残す。

```text
packet_send_time
packet_receive_time
control_start_time
control_end_time
command_send_time
command_receive_time
```

---

## 3. 実ボード処理時間

ボード側は毎stepで以下を返す。

```text
control_execution_time_us
```

これにより、実制御周期内に処理できるか確認できる。

---

# 共通部に入れるべき要件

```text
1. HIL step token
2. step_index
3. sim_time_s
4. dt_s
5. sensor packet
6. command packet
7. timeout policy
8. sync mode
9. real-time / non-real-time mode
10. clock master設定
11. control execution time計測
12. packet latency計測
13. fault時安全出力
```

---

# まとめ

HILで実ボード側制御周期をMuJoCoに同期する場合、推奨は以下である。

```text
MuJoCoをクロックマスターにする
MuJoCoが1 stepごとにstep_tokenを送る
ボードはstep_tokenを受け取るまで待つ
ボードは1 tokenにつきcontrol_tickを1回だけ実行する
ボードは指令を返す
MuJoCoはその指令を使って次stepへ進む
```

この方式により、

```text
MuJoCo時間
制御周期
センサー値
制御指令
ログ
```

をすべてstep単位で同期できる。
