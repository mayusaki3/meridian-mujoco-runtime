[目次](../README.md) > docs/ja-JP > フィードバック > ESP32-WROOM-32 と ESP32-WROVER-E の比較

# ESP32-WROOM-32 と ESP32-WROVER-E の比較

## 概要

ESP32-WROOM-32 と ESP32-WROVER-E は、どちらも初代 ESP32 系 SoC を使用した Wi‑Fi / Bluetooth モジュールである。

主な差分は以下。

- PSRAM の有無
- Flash 容量
- GPIO 使用制限
- メモリ利用可能量
- 画像・GUI・AI用途への適性

CPU や基本的な SDK / Arduino 互換性は高い。

---

# モジュール比較

| 項目 | ESP32-WROOM-32 | ESP32-WROVER-E |
|---|---|---|
| ベース SoC | ESP32 | ESP32 |
| CPU | Xtensa LX6 Dual Core | Xtensa LX6 Dual Core |
| 最大クロック | 240 MHz | 240 MHz |
| Wi‑Fi | 802.11 b/g/n | 802.11 b/g/n |
| Bluetooth Classic | 対応 | 対応 |
| BLE | 対応 | 対応 |
| 内蔵 SRAM | 520 KB | 520 KB |
| PSRAM | なし | あり（通常 8 MB） |
| Flash | 通常 4 MB | 4 / 8 / 16 MB |
| アンテナ | PCB アンテナ | PCB アンテナ |
| サイズ | 小型 | やや大型 |
| 消費電力 | 標準 | やや増加 |
| GPIO 利用自由度 | 高い | 一部制限あり |
| 用途 | 制御・通信 | GUI・画像・AI |

---

# ESP32-WROOM-32 の特徴

## 長所

- 非常に普及している
- Arduino 情報量が多い
- 安価
- GPIO 制約が少ない
- 制御用途に向く

## 短所

- PSRAM がない
- 大容量バッファ用途に弱い
- GUI / 画像処理では RAM 不足になりやすい

## 向いている用途

- サーボ制御
- センサ処理
- UART / SPI / I2C
- CAN(TWAI)
- BLE 通信
- 小規模 WebUI

---

# ESP32-WROVER-E の特徴

## 長所

- PSRAM 搭載
- 大容量メモリ利用可能
- カメラ用途に向く
- GUI / 音声 / AI に向く
- 大型バッファ利用可能

## 短所

- 一部 GPIO が PSRAM 用に使用される
- GPIO 利用自由度が低下
- WROOM より高価

## 向いている用途

- ESP32-CAM 系
- LCD GUI
- 音声処理
- TensorFlow Lite
- 画像処理
- Web サーバー
- 大型 JSON 処理

---

# 重要差分：PSRAM

## WROOM-32

```text
PSRAMなし
```

利用可能 RAM が少ない。

---

## WROVER-E

```text
PSRAMあり
```

通常 8MB PSRAM を搭載。

大量メモリを利用可能。

---

# GPIO / ピンアサイン差分

## 基本

両者とも 38pin モジュールであり、多くの GPIO は共通。

ただし WROVER-E は PSRAM 利用のため、一部 GPIO が内部使用される。

---

# WROOM-32 GPIO

## 比較的自由に使いやすい GPIO

| GPIO |
|---|
| GPIO0 |
| GPIO2 |
| GPIO4 |
| GPIO5 |
| GPIO12 |
| GPIO13 |
| GPIO14 |
| GPIO15 |
| GPIO16 |
| GPIO17 |
| GPIO18 |
| GPIO19 |
| GPIO21 |
| GPIO22 |
| GPIO23 |
| GPIO25 |
| GPIO26 |
| GPIO27 |
| GPIO32 |
| GPIO33 |

---

# WROVER-E GPIO 制限

## PSRAM 使用 GPIO

| GPIO | 用途 |
|---|---|
| GPIO16 | PSRAM |
| GPIO17 | PSRAM |

これらは実質使用不可、または非推奨。

---

# 実務上の GPIO 差分

| GPIO | WROOM-32 | WROVER-E |
|---|---|---|
| GPIO16 | 使用可能 | PSRAM 使用 |
| GPIO17 | 使用可能 | PSRAM 使用 |

---

# GPIO6〜11 の注意

以下は両者共通で Flash 接続済み。

通常利用非推奨。

| GPIO |
|---|
| GPIO6 |
| GPIO7 |
| GPIO8 |
| GPIO9 |
| GPIO10 |
| GPIO11 |

---

# 入力専用 GPIO

以下は両者共通。

入力専用。

| GPIO |
|---|
| GPIO34 |
| GPIO35 |
| GPIO36 |
| GPIO39 |

---

# ブートストラップ注意 GPIO

起動モードに影響する。

| GPIO | 注意 |
|---|---|
| GPIO0 | Boot Mode |
| GPIO2 | Boot 関連 |
| GPIO12 | Flash 電圧関連 |
| GPIO15 | Boot 関連 |

起動時の Pull-up / Pull-down に注意。

---

# ピン互換性

## 多くの基板は物理互換

以下のような基板では差し替え可能な場合が多い。

- DevKitC
- NodeMCU-32S
- ESP32 Dev Module

---

# ただし注意点

## WROOM → WROVER-E 差し替え時

### GPIO16 / 17 使用コード

動作しなくなる可能性あり。

---

## メモリ依存コード

PSRAM 前提コード：

```cpp
ps_malloc();
heap_caps_malloc();
```

は WROOM では失敗する可能性がある。

---

# Arduino IDE 注意

## WROVER-E

Board 設定：

```text
ESP32 Wrover Module
```

推奨。

PSRAM が有効化される。

---

## WROOM-32

Board 設定：

```text
ESP32 Dev Module
```

など。

---

# Meridian 系用途での考察

## 制御用途

以下中心なら WROOM-32 で十分。

- サーボ制御
- Meridian 通信
- CAN(TWAI)
- IMU
- リアルタイム処理

---

## GUI / カメラ追加

以下を行うなら WROVER-E 推奨。

- WebUI
- カメラ
- 音声
- TensorFlow Lite
- 大型状態管理
- ログ保存

---

# 推奨整理

| 用途 | 推奨 |
|---|---|
| 制御専用 | ESP32-WROOM-32 |
| GUI / 画像 | ESP32-WROVER-E |
| AI / 音声 | ESP32-WROVER-E |
| GPIO 多数利用 | ESP32-WROOM-32 |
| RAM 大量利用 | ESP32-WROVER-E |

---

# 参考

- ESP32-WROOM-32 Datasheet
- ESP32-WROVER-E Datasheet
- ESP32 Series Datasheet

参考:
https://documentation.espressif.com/esp32-wroom-32_datasheet_en.html
https://documentation.espressif.com/esp32-wrover-e_esp32-wrover-ie_datasheet_en.html
https://documentation.espressif.com/esp32_datasheet_en.html

---

[目次](../README.md) > docs/ja-JP > フィードバック > ESP32-WROOM-32 と ESP32-WROVER-E の比較
