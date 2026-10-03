<!--
HLDocS:LLM-MANAGED
doc_id: doc-20261003-000001Z-MBSI
lang: ja-JP
document_type: research
canonical_title: Microban・BAMを用いたRobot各部SysIDサンプル検討
canonical_document: true
-->

[目次](../目次.md) > [検討資料](./検討資料目次.md) > Microban・BAMを用いたRobot各部SysIDサンプル検討

# Microban・BAMを用いたRobot各部SysIDサンプル検討

## 1. 目的

Robot各部を選択してSysIDするMeridianの代表サンプルとして、RhobanのMicrobanを参照する。

Microban固有の実装をMeridianへ移植することを目的とせず、実在する小型humanoid Robotを使って、Robot構成からMotor / Servo / 機構を選択し、SysID Definitionを編集してReal / Sim双方を実行するworkflowを検証する。

Servo actuatorのSystem Identification手法についてはRhoban BAM (Better Actuator Models)も参考資料とする。

## 2. サンプルworkflow

```text
Microban Robot Definition
  -> Robot構成を表示
  -> 対象Servo / Joint / 機構を選択
  -> SysID Unitを編集
  -> SysID Definitionを編集
       - Motion Pattern
       - Real実行設定
       - Sim実行設定
       - Measurement設定
       - 実装値
       - fixed / estimate parameter
       - Calculation Model
       - Identification / Validation設定
  -> Realへ変換・実行
  -> Measurement Datasetを保存
  -> Simへ変換・実行
  -> Identification
  -> Validation
  -> HistoryへResultを追加
```

最初はServo単体をRobot内Instanceとして選択するケースを優先し、その後Servo + reduction / Joint / Link等を含むMechanism-level SysIDへ広げる。

同一製品Servoを単体登録してProduct-level SysIDした結果を、Microban内の同一製品Instanceへ適用するケースも検証対象とする。

## 3. BAMから参考にする範囲

BAMはservo-actuatorの実測trajectoryからfriction model等を同定し、MuJoCoで利用する既存事例として参照する。

MeridianではBAMのfile formatやAPIをSysID共通仕様に固定せず、actuatorへ与えるmotion / excitation、load等の実験条件、position / velocity / control signal等の観測、Measurement Dataset、Calculation Model、estimate対象parameter、Simulation、実測とSimulationの比較・Validation、同定済みmodelのRobot simulationへの適用を汎用SysID Definition / Historyへ対応付けて検討する。

BAM互換機能を初期要件とはしない。

## 4. Meridian側で一般化する点

Microban / BAMをそのままdomain modelにしない。

MeridianのSysID TargetはMicroban以外のRobot、Motor、Servo、機構にも適用可能とし、Robot構成とSysID Unit境界を分離する。

特定Servo製品専用のparameter名やCalculation ModelをSysID共通層へ固定せず、device categoryまたは個別Calculation Modelが必要なparameterと処理を定義できる構造を検討する。

## 5. ライセンス上の扱い

### Microban

Microban repositoryはmulti-licenseである。README / LICENSEではHardware / CAD / documentationがCC BY-NC-SA 4.0、control softwareがGNU GPL v3として示されている。

Meridianでは初期段階でMicrobanのsource code、CAD、STL、documentation本文等をコピー・改変して取り込まない。Robot各部SysIDのユースケース、構成上の考え方、実機検証候補として参照する。

将来Microban由来のsoftware codeをコピー・改変してMeridianへ組み込む場合はGPLv3の条件を個別に確認する。CAD / documentation等を再配布・改変する場合はCC BY-NC-SA 4.0のAttribution、NonCommercial、ShareAlike等の条件を個別に確認する。

単なる参照を超えてMicroban由来materialをProject sampleへ同梱する判断は、実装前にlicense reviewを行う。

### BAM

BAM repositoryはApache License 2.0として公開されている。

MeridianではBAMのSysID workflowやmodeling approachを参考にし、Meridian自身のdomain model / API / implementationとして設計する。

将来BAM source codeを直接利用・改変・再配布する場合は、Apache-2.0のcopyright / license notice、NOTICEの有無、変更表示等、実際に利用するversionとfileのlicense条件を確認してから導入する。

論文由来の手法を実装する場合も、software licenseとは別にcitationおよび関連する権利・特許等を実装採用時に再確認する。

## 6. 現時点の方針

1. MicrobanをRobot各部SysIDの代表サンプルとして使用する。
2. BAMをservo-actuator SysIDの参考実装・参考手法として使用する。
3. MeridianのSysID仕様をMicroban / BAM専用にはしない。
4. 初期検討では外部repositoryのcode / CAD / documentationをMeridianへコピーしない。
5. 外部成果物を実際に取り込む場合は、取り込み単位ごとにlicenseを再確認する。
6. Microban / BAM由来であることが分かる検討・実装には出典を記録する。
7. SysID固有のExport / Import共有形式はこのサンプル検討では定義しない。

## 7. 参照

- Rhoban/microban repository / LICENSE / README
- Rhoban/bam repository / LICENSE / documentation
- Extended Friction Models for the Physics Simulation of Servo Actuators (ICRA 2025)

---

[目次](../目次.md) > [検討資料](./検討資料目次.md) > Microban・BAMを用いたRobot各部SysIDサンプル検討
