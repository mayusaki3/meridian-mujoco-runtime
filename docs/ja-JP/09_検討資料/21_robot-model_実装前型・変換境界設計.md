<!--
HLDocS:LLM-MANAGED
doc_id: doc-20261008-000001Z-RMAP
lang: ja-JP
document_type: research
canonical_title: robot-model 実装前型・変換境界設計
canonical_document: true
-->

[目次](../目次.md) > 検討資料 > robot-model 実装前型・変換境界設計

# robot-model 実装前型・変換境界設計

## 1. 対象

テスト仕様20に定義する初期検証を実施するための初期Rust型・責務境界を定義する。現段階は設計のみとし、既存Meridian Runtime / workflow-ide-frameworkの実装を変更しない。

## 2. Crate境界

- `robot-model`: 中立なRobotDefinition、domain ID、frame、数値型、構造Validation。
- `robot-model-import`: URDF入力、XML構文解析、URDF意味検証、SourceDocument生成、canonical変換、diagnostic。
- `robot-model-persistence`（分離は保留）: canonical JSONの保存・読込。初期はrobot-modelのfeature/moduleとしてもよいが、schemaとdomain型を同一視しない。
- `robot-coordinate-adapter`（分離は保留）: ROS系とSansaXR系の座標変換。初期は純粋関数で検証し、VR runtime依存を入れない。

SourceDocumentとRobotDefinitionは独立して保持し、管理層が関連付けと永続化を担当する。管理層の具体的なcrate配置は未確定とする。

依存は `robot-model-import -> robot-model` とし、robot-modelはImporter、MuJoCo、Meridian、workflow-ide-frameworkへ依存しない。

## 3. Domain ID

`RobotId`、`LinkId`、`JointId`、`ComponentId`、`ConnectionId` を型で区別する。表示名は変更可能で、IDは変更しない。

初期内部表現はUUIDを候補とするが、URDF Importで毎回ランダムIDを振ると同一sourceの再Import差分比較が困難になる。初回Import時のID生成と、既存Definitionへの再Importでの対応付けは別責務として扱う。Project所属のFramework `resource_id` とdomain IDは共有しない。

## 4. 数値・座標

- 内部数値は原則 `f64`。
- 位置・長さm、角度rad、質量kg、時間s。慣性tensorはkg·m²、角速度rad/s、トルクN·m。
- canonical body frameは右手系、+X前、+Y左、+Z上。
- `Vector3`、`Rotation3`、`Transform3`、`Inertia3`等の型を用途別に区別する。
- Transformは「どのframeからどのframeへの変換か」をAPI・命名で明示する。単なる3要素配列を位置・回転軸・角速度で混用しない。
- 角度やJoint axisは対応frameに属する値であり、すべてworld frameに固定しない。
- 姿勢表現は単位Quaternionを第一候補とし、matrix / RPYとの変換を境界で扱う。正規化、非有限値、特異点・丸め誤差の検証を行う。

## 5. canonical JSON

初期形式はJSONを候補とするが、Rust structの自動serialization出力をそのまま長期schema契約にしない。保存するのはdomain ID、metadata、Link / Joint / Component / Connection、frame・物理量、Asset参照であり、Framework resource_idやfilesystem絶対pathは正本として埋め込まない。

Schema migrationはRuntime Application data_versionとの責務関係を確認してから確定する。保存・再読込でdomain構造が一致することをRM-009で検証する。

## 6. URDF解析・意味検証

既存Rust URDF parser crateを調査・評価して採用候補を選定する。自作XML parserは初期既定としない。選定時はlicense、URDF要素の対応範囲、unsupported情報の検出可否、保守状況、依存関係を確認する。

Parser固有型をrobot-model Public APIへ露出しない。

### 6.1 処理境界

1. XML構文解析を行う。構文エラーの場合は通常Importを失敗とし、診断を返す。
2. 構文解析成功後、元XMLと形式固有情報をSourceDocumentとして保持し、URDF意味検証を行う。
3. 意味検証エラーがある場合はSourceDocumentと診断結果を保持し、RobotDefinitionを生成しない。シミュレーション対象としない。
4. 意味検証エラーがなく、canonical変換に成功した場合にRobotDefinitionを生成する。警告のみの場合は生成を妨げない。
5. Source Mappingとdomain IDの対応を確立し、管理層でSourceDocumentとRobotDefinitionを関連付ける。
6. Simulator固有の対応可否・変換検証は、対象Simulatorを使用する段階で別途行う。

SourceDocument修正後は構文解析・意味検証を再実行し、診断結果を更新する。意味検証エラーが解消した場合にRobotDefinitionを生成する。編集方法は限定しない。

### 6.2 診断

ImportResultはSourceDocument、任意のRobotDefinition、診断結果を区別して扱える構造とする。XML構文解析失敗は別途errorで返す。意味検証では検出可能な複数の問題を収集する。

既存の診断分類候補 `Warning`、`Unsupported`、`MissingAsset`、`Approximated` を維持し、XML構文エラー、URDF意味検証エラー、Simulator未対応、Simulator変換エラーを区別する。具体的な型・コード体系は別途確定する。

Simulator固有の未対応機能を理由に、URDF自体の意味検証エラーと判定しない。個別の意味検証規則と期待結果はテスト仕様20と整合させる。

## 7. Frame変換とVR

ROS系からSansaXR系への基底変換は `(x,y,z) -> (-y,z,x)`、行列 `C` のdeterminantは-1。

- 極性ベクトル: `p_vr = C p_ros`
- 回転行列: `R_vr = C R_ros C^-1`
- 軸性ベクトル: `a_vr = det(C) C a_ros`
- 慣性tensor: `I_vr = C I_ros C^T`
- Mesh三角形: windingと法線の整合を検証

この変換は同じ物理方向を示すframe同士の基底変換であり、Robot配置やWorld原点の差は別のTransformとして合成する。

## 8. 初期実装順序と完了条件

1. 資料20・22と本資料の整合を確認し、設計・テスト仕様を確定する。
2. 対象テストを先に作成し、その後に対象実装を変更する。
3. canonical round-tripとImport diagnosticsを確認する。
4. RM-016〜RM-024のframe / unit / VR adapter数学テストを実装する。
5. 全対象テストの成功、追加実装のunit test coverage 100%を検証する。
6. Microban等の第三者Robotを使った統合検証はlicense確認後に別途行う。

## 9. 未決事項

- UUID生成・再Import時のID対応規則
- 数学crateの採否
- URDF parser crateの選定
- canonical JSON schemaの正式field名
- persistenceを独立crateとするか
- Asset参照解決のApplication integration契約

これらは実装前に必要な範囲で決める。Meridian Console / URDF Kitchenの内部実装へ直接依存する設計にはしない。

## 10. 初期技術選定（実装候補）

- URDF Parser: `urdf-rs` 0.10系を第一候補とする。Link / Joint中心のparserであるため、未知要素・拡張属性の検出が十分かをfixtureで確認する。parserが未知要素を読み捨てる場合、別途XMLレベルの事前検査を設ける。silent discardは認めない。
- 数学: `nalgebra` を第一候補とする。Vector3、UnitQuaternion、Isometry3、Matrix3等を内部計算に使用する。ただしdomain Public APIとcanonical JSONへライブラリ固有serialization形式を無条件に露出させない。
- Domain ID: `uuid` 1系のUUID v4を新規entity生成の第一候補とする。再Importで既存entityとの対応を必要とする場合は名前だけの自動上書きではなく、source identityと既存domain IDの照合手順を別途定義する。
- Persistence: `serde` / `serde_json` を初期候補とし、versioned DTOをdomainと分離する。初期JSON field名とmigration責務はschema確定時に決定する。
- Crate分離: 初期は `robot-model` と `robot-model-import` を独立crate候補とし、persistenceとcoordinate adapterの別crate化は実装規模を見て判断する。

`nalgebra` 0.35.0の公表MSRVはRust 1.89.0。実装環境のtoolchainが満たすか確認する。依存versionはCargo.lock等で検証時に固定する。

この選定は資料20のテストケースを満たすことを条件とする。特にRM-011のunsupported diagnosticsとRM-024のURDF frame意味の保持をparser選定のgateとする。ライセンス、依存関係、Rust toolchain適合は実装時に最終確認する。


## 11. 未対応データの非破壊保持とExport復元

SansaVRMと同様に、Import時に未対応情報を捨てず、canonical保存・再読込・既知項目の編集後も保持し、同一形式へのExportで復元できることを基本要件とする。診断だけ出して破棄する方式は不可。

### 11.1 モデルと形式固有情報の分離

- Known Data: robot-modelが解釈するLink、Joint、Frame、Inertial等。
- Preserved Data: importerが意味を解釈しない要素・属性・拡張情報。source formatとformat version（判別可能な場合）、元のXML断片またはlossless表現、順序・配置情報を保持する。
- Source Mapping: source要素の安定した参照先とdomain IDの対応を保持する。Link / Jointの表示名変更だけで紐付けを失わない。
- SourceDocumentはRobotDefinitionから独立して保持する。意味検証エラー時も元XMLと診断結果を保持し、domain IDとの対応が未確定でも保存できる。
- 永続化ではRobotDefinition、SourceDocument、Preserved Data、Source Mappingおよび関連付けを保持する。元ファイルの存在を復元の前提としない。

形式固有のXML構造をrobot-modelの物理domain属性へ無理に変換しない。URDF、将来のMJCF等の保持・復元規則は各format adapterが担当する。

### 11.2 Export規則

通常のモデルExportは正常なRobotDefinitionを前提とする。SourceDocumentのみの保持・編集とは区別する。

1. 既知項目は現在のcanonical modelから出力する。
2. 未対応項目はSource Mappingに従って元の位置・順序・所有要素へ復元する。
3. Link / Jointの名称変更後もdomain IDに紐付いた未対応項目を復元する。未対応断片内部の文字列参照を自動的に書き換えられるとは限らないため、参照の曖昧さは診断する。
4. 削除された所有要素、構造変更による配置先不明、名前衝突等はConflict / Unresolvedとして扱い、黙って破棄・別要素へ移動しない。
5. 完全復元できない場合、lossless Export成功と報告しない。通常Exportは失敗または明示的な利用者判断を必要とする。lossy Exportは別操作として扱い、失われる内容を列挙する。

保証するのは未対応情報の意味と構造の保持であり、既知項目編集後のXML全体のbyte一致ではない。コメント、CDATA、名前空間、属性順序、空白等をどこまでbyte保存するかはformatごとに明示する。

### 11.3 Parser選定への影響

`urdf-rs` は既知URDF構造の解釈候補として維持するが、それだけでlossless preservationを保証しない。原文XMLの構造・未対応要素・属性を把握できるXMLイベント/ツリー解析層（例: `quick-xml`）を併用する案を採用候補とする。両解析結果の要素対応と、変更箇所への復元を検証する。

Importerは、未対応情報を発見したときdiagnosticとpreserved payloadを同時に返す。形式固有の未対応情報を単なるWarningに置き換えない。

### 11.4 追加検証

テスト仕様20のRM-011を「検出・保持・再Export」へ強化し、RM-025以降でcanonical round-trip、既知属性編集、名称変更、要素削除、参照衝突、異形式Export時の扱いを検証する。

---

[目次](../目次.md) > 検討資料 > robot-model 実装前型・変換境界設計
