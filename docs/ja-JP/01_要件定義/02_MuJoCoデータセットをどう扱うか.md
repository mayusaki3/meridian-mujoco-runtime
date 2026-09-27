<!--
HLDocS:LLM-MANAGED
doc_id: doc-20260521-000003Z-2AE8
lang: ja-JP
document_type: spec
canonical_title: MuJoCoデータセットをどう扱うか
canonical_document: true
-->

[目次](../目次.md) > 要件定義 > MuJoCoデータセットをどう扱うか

# MuJoCoデータセットをどう扱うか

MuJoCoは、ロボットや物理環境を定義して物理シミュレーションを実行するための物理エンジンである。

本システムでは、ロボット、フィールド、アクチュエーター、物理設定等のシミュレーションに必要なデータをProject内で管理し、利用者が必要な対象を選択・組み合わせてシミュレーションを構築できるようにする。

従来「MuJoCoデータセット」と呼んでいた単一のfolder/packageをシステムの基礎単位とはせず、Project、Resource、およびRuntime固有の論理定義として扱う。

## 入力データの要件

利用者はProject内で、用途に応じて次のような対象を扱えること。

- Robot
- Actuator
- Field
- Scenario / Task
- Execution Profile
- MJCF
- URDF
- mesh、texture等の関連Asset
- Runtimeが必要とするその他の入力データ

同一のRobotやFieldについて、Meridian内部定義とMJCF / URDF等の複数表現をProject内に保持できること。

Projectを構成するResourceは明示的に登録され、単にProject directory内にfileが存在することとProjectへの所属を区別できること。

Resourceの論理的な役割と物理的なfile配置を分離して扱えること。

## 選択・組み合わせの要件

利用者は、Projectに登録されたRobot、Field、Actuator等を用途に応じて選択し、組み合わせてシミュレーションを構成できること。

同じRobotを異なるFieldで使用する、同じRobotへ異なるActuator設定を適用する等、定義の再利用が可能であること。

選択・組み合わせの結果から、RuntimeがMuJoCo実行に必要なデータを構築できること。

## Runtime用データ構築の要件

Runtimeは、選択された定義・ResourceからMuJoCoへ渡す実行用データを構築できること。

少なくとも次を扱えること。

- MJCF / URDF等からMuJoCo実行に必要なモデルを構築する
- Meridian固有のRuntime情報を統合する
- mesh等の関連Resourceを解決する
- Resource不足や不整合を検出する

MuJoCo実行用に生成されるデータと、利用者がProjectで管理する元の定義・Resourceを区別できること。

## ResourceとAssetの要件

MJCF / URDF等から参照されるmesh、texture等もProjectを成立させるResourceとして扱えること。

関連Assetが不足している場合は不足を検出できること。

用途上可能な場合は、ダミーmesh等の代替Resourceを生成してProjectへ追加できること。

## Import / Exportの要件

初期段階では、既存の交換形式を利用してProjectへデータをImportし、ProjectからExportできることを目標とする。

対象は次とする。

- URDF
- MJCF（MuJoCo XML）
- 上記から参照され、対象を成立させるために必要なmesh等の関連file

Import時は、URDF / MJCF本体だけでなく必要な関連fileもProjectへ取り込み、利用者が関連fileを一つずつ手動登録しなくても利用可能な状態にできること。

Import元のURDF / MJCFと、そこから生成・変換したMeridian内部定義はProject内で併存できること。

独自のMeridian Exchange Packageや、folderをzip圧縮して独自拡張子へrenameする方式は、初期要件には含めない。

## Projectの自己完結性

通常のProject利用では、Project外のMeridian固有共有directoryや特定PCの絶対pathを必須としないこと。

Project内Resource間の依存関係は、Projectを別の保存場所や別PCへ移動しても解決可能な構成を基本とする。

Project外Resourceへの常時参照や自動同期を、Resource再利用の基本方式とはしない。

## 実行とフィードバック

Runtimeは構築したMuJoCo実行用データを使用してシミュレーションを実行できること。

モーション再生、実マイコンとの中継・同期、センサー出力、sysid等のRuntime機能は、Projectで選択した定義・Resourceを利用できること。

sysid等によってProjectの定義へ反映すべき結果が得られた場合は、Runtime固有の定義としてProjectへ保存できること。

---

[目次](../目次.md) > 要件定義 > MuJoCoデータセットをどう扱うか
