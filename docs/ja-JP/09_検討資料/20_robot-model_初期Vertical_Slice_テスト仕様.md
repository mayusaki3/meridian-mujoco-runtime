<!--
HLDocS:LLM-MANAGED
doc_id: doc-20261007-000001Z-RMT1
lang: ja-JP
document_type: test
canonical_title: robot-model 初期Vertical Slice テスト仕様
canonical_document: true
-->

[目次](../目次.md) > 検討資料 > robot-model 初期Vertical Slice テスト仕様

# robot-model 初期Vertical Slice テスト仕様

## 1. 目的

URDFを外部交換表現として読み込み、中立なcanonical `RobotDefinition` へ変換し、構造検査・保存・再読込できる最小library境界を検証する。

このテストではworkflow-ide-frameworkのUI、SysID、MuJoCo実行、Meridian flow実機通信は対象外とする。

## 2. 対象

初期対象は次とする。

- `robot-model`: RobotDefinition、Link、Joint、最小Asset参照、domain ID、構造Validation
- `robot-model-import`: URDF parse / conversion、Import diagnostics
- canonical persistence: RobotDefinitionの保存・再読込
- test fixture: 本リポジトリで作成する最小URDF

Microbanは実Robotの統合検証候補として扱うが、初期unit test fixtureへ第三者配布物をコピーしない。Microbanを使用する統合検証は、対象versionとlicense条件を確認した別手順とする。

## 3. 初期fixture

独自の最小URDF fixtureを用意する。

```text
base_link
   |
 joint_1 (revolute)
   |
link_1
```

fixtureには最低限、Robot名、2 Link、1 revolute Joint、parent / child、origin、axis、limit、Link inertialを含める。

Mesh fileを必要とするfixtureは初期必須にしない。Asset参照は別fixtureで検証する。

## 4. テストケース

### RM-001 最小URDF Import

**入力:** 正常な最小URDF fixture。

**期待結果:**

- Importが成功する。
- RobotDefinitionが1件生成される。
- Linkが2件生成される。
- Jointが1件生成される。
- fatal diagnosticがない。

### RM-002 Link identityと参照

**入力:** RM-001の結果。

**期待結果:**

- 各Linkに重複しないdomain IDが存在する。
- Jointのparent / childはLink名やfile pathではなくdomain IDで解決できる。
- Framework resource_idを要求しない。

### RM-003 Joint属性変換

**入力:** revolute Jointにorigin、axis、lower / upper limitを持つfixture。

**期待結果:**

- joint typeが保持される。
- parent / child relationが保持される。
- origin / axisが保持される。
- limitが保持される。

### RM-004 Link inertial変換

**入力:** mass、center of mass、inertia tensorを持つLink。

**期待結果:**

- massが保持される。
- inertial originが保持される。
- inertia tensorが保持される。

### RM-005 構造Validation正常系

**入力:** base_link -> joint_1 -> link_1 の正常RobotDefinition。

**期待結果:** structure validationが成功する。

### RM-006 存在しないparent Link

**入力:** parentが存在しないJointを含むRobotDefinition。

**期待結果:** validationが失敗し、該当Jointとmissing parentを識別可能なdiagnosticを返す。

### RM-007 存在しないchild Link

**入力:** childが存在しないJointを含むRobotDefinition。

**期待結果:** validationが失敗し、該当Jointとmissing childを識別可能なdiagnosticを返す。

### RM-008 domain ID重複

**入力:** 同一LinkIdを持つ複数Link。

**期待結果:** validationまたは追加操作時に重複を拒否する。

### RM-009 canonical round trip

**入力:** RM-001で生成したRobotDefinition。

**操作:** canonical形式へ保存し、再読込する。

**期待結果:**

- domain上同等のRobotDefinitionになる。
- Link / Joint relationが維持される。
- URDFを再parseしなくてもcanonical Definitionだけで復元できる。

byte単位のJSON一致は要求しない。

### RM-010 URDF source非依存

**入力:** Import済みRobotDefinition。

**操作:** 元URDFを参照せずcanonical Definitionを再読込する。

**期待結果:** Robot構造を復元できる。元URDFを正本として要求しない。

### RM-011 unsupported情報のdiagnosticと非破壊保持

**入力:** 初期Importerが対応しないURDF要素または属性を含むfixture。

**期待結果:**

- 対応可能なRobot構造までImportできる場合は成功値とdiagnosticを同時に返せる。
- unsupported内容を識別できる。
- 未対応要素・属性をPreserved Dataとして保持し、同一形式へのExportで復元できる。
- silent discardしない。

### RM-012 malformed URDF

**入力:** XMLまたはURDF構造として不正なfixture。

**期待結果:** Importが失敗し、panicせずerrorを返す。

### RM-013 Asset参照

**入力:** visualまたはcollisionから外部Assetを参照するURDF fixture。

**期待結果:**

- robot-model内ではfilesystem absolute pathやFramework resource_idをcanonical identityとして固定しない。
- Import時にsource Asset参照を解決・登録するために必要な情報をApplication integrationへ渡せる。

Project Resource Registryへの実登録はこのlibrary unit testの対象外とする。

### RM-014 表示名とdomain identityの分離

**入力:** Link名を変更する操作。

**期待結果:** Linkのdomain IDを維持したまま表示名を変更でき、Joint relationが壊れない。

### RM-015 1 Joint = 1 Actuator制約を持たない

**入力:** Actuatorを持たないLink / Joint構造。

**期待結果:** RobotDefinitionとして正常に成立する。robot-modelがJointごとのActuator存在を必須化しない。

## 5. Integration検証候補

初期unit test完了後、Microban等の公開Robot URDFを利用したintegration検証を別途行う。

確認項目は、複数Link / Jointのtopology、mesh Asset参照、慣性情報、実Robot規模でのdiagnostic、Import後canonical保存・再読込とする。

第三者fixtureをrepositoryへ含める場合は、その時点のsource、license、attribution、再配布条件を確認する。URLから取得して検証する方式についても、test再現性とupstream変更の影響を考慮して別途決定する。

## 6. 実装開始条件

実装開始前に次を確定する。

- 初期crate名
- domain IDの内部表現
- transform / vector / inertiaの数値型と単位
- canonical persistenceの初期schema方針
- URDF parserを自作するか既存crateを利用するか
- Import diagnosticsの最低限の分類

上記確定後、RM-001から順にtestを実装し、対象scopeのunit test coverage 100%を目標とする。

## 7. 座標・単位・frame変換テスト

### RM-016 基底軸の変換

**入力:** ROS body frameの単位基底 (+X前、+Y左、+Z上)。

**期待結果:** SansaXR座標ではそれぞれ (+Z前、-X左、+Y上) となる。

### RM-017 位置変換の逆変換

**入力:** 任意の位置ベクトル。

**期待結果:** ROS -> VR -> ROSの往復で元の値へ許容誤差内で復元できる。

### RM-018 姿勢変換

**入力:** 既知のJoint/Robot姿勢を表す回転行列。

**期待結果:** `R_vr = C R_ros C^-1` で変換され、変換後も正規直交性とdet=+1を維持する。

### RM-019 軸性ベクトル

**入力:** ROS frameの角速度またはトルク。

**期待結果:** 右手系から左手系への変換で `det(C) C` を適用し、物理的な回転向きが一致する。

### RM-020 Frame階層

**入力:** World -> Robot Base -> Link -> Jointの非自明な並進・回転を持つfixture。

**期待結果:** frame transformの合成と逆変換が成立し、Joint axisを誤ったframeへ解釈しない。

### RM-021 Joint回転方向

**入力:** 正方向へ回転するrevolute Joint。

**期待結果:** ROS側の正方向とVR表示側の動きが物理的に一致する。軸性ベクトルの符号反転を考慮する。

### RM-022 Mesh面の向き

**入力:** 向きが既知の三角形mesh。

**期待結果:** handedness変更後も表示面の表裏が意図どおりで、必要な頂点順序または描画設定の変換が行われる。

### RM-023 単位系

**入力:** 長さ、角度、質量、時間、および外部protocolの別単位で表された値。

**期待結果:** canonical値はm、rad、kg、sとなり、変換の適用箇所が明示される。単位を持たないraw packet値をcanonical物理量として誤認しない。

### RM-024 URDF frame意味の保持

**入力:** Link visual / collision / inertial originおよびJoint origin / axisが異なるfixture。

**期待結果:** 各要素のframe意味を維持してImportでき、URDF全体へ不要な固定90度回転を加えない。

これらのうちVR描画やmesh表示のend-to-end確認はadapter統合テストに分類する。robot-model単体では変換数学とframe関係を検証する。

## 8. 非破壊Import / Exportテスト

### RM-025 未対応要素・属性の保持

**入力:** 独自属性と未知子要素を持つURDF fixture。

**期待結果:** Import後、元XMLがなくてもPreserved Dataと所有要素の対応を参照できる。診断のみで情報を破棄しない。

### RM-026 canonical保存・再読込後の復元

**入力:** RM-025のRobotDefinition。

**操作:** canonical保存、再読込、URDF Export。

**期待結果:** 未対応要素・属性の意味と階層・配置が維持される。元URDFファイルは不要。

### RM-027 既知属性編集後のExport

**入力:** 未対応情報を持つLink / Joint。

**操作:** 既知のmass / limit等を編集してExport。

**期待結果:** 既知属性は更新値、未対応属性・要素は保持値で出力される。

### RM-028 名称変更とID対応

**入力:** 未対応情報を持つLink。

**操作:** Link表示名を変更してExport。

**期待結果:** Source Mappingがdomain IDに追従し、未対応情報が正しいLinkに復元される。未知断片内の旧名参照は検出可能な場合に診断する。

### RM-029 所有要素削除時のConflict

**入力:** 未対応情報を持つLink / Joint。

**操作:** 所有要素を削除してExport。

**期待結果:** 未対応情報を黙って破棄・別要素へ移動せず、復元不能のdiagnosticを返す。通常のlossless Export成功とはしない。

### RM-030 復元先の衝突・曖昧さ

**入力:** 変更後のRobot構造でSource Mappingの復元先が一意に決まらないfixture。

**期待結果:** Conflictを報告し、自動的な誤配置をしない。

### RM-031 異形式Export

**入力:** URDF固有のPreserved Dataを含むRobotDefinition。

**操作:** 将来のMJCF等へのExportを想定。

**期待結果:** URDF固有断片を無条件にMJCFへ混入しない。未表現情報の診断・明示的なlossy扱いを要求する。

### RM-032 XML保持境界

**入力:** コメント、CDATA、名前空間、属性順序、空白等を含むfixture。

**期待結果:** 保持保証範囲をformat adapterの仕様で明示し、保証対象を保存・復元できる。byte一致を保証しない情報は診断・仕様で明確化する。

実装順序はRM-025〜RM-030をURDF Import / Exportの初期完了条件へ含める。RM-031は異形式Exporter導入時のgate、RM-032はXML保持保証範囲確定後に期待値を具体化する。
