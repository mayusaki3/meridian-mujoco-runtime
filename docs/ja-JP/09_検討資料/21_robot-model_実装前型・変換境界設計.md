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

テスト仕様20のRM-001〜RM-024を実装するための初期Rust型・責務境界を定義する。現段階は設計のみとし、既存Meridian Runtime / workflow-ide-frameworkの実装を変更しない。

## 2. Crate境界

- `robot-model`: 中立なRobotDefinition、domain ID、frame、数値型、構造Validation。
- `robot-model-import`: URDF入力、parse、canonical変換、diagnostic。
- `robot-model-persistence`（分離は保留）: canonical JSONの保存・読込。初期はrobot-modelのfeature/moduleとしてもよいが、schemaとdomain型を同一視しない。
- `robot-coordinate-adapter`（分離は保留）: ROS系とSansaXR系の座標変換。初期は純粋関数で検証し、VR runtime依存を入れない。

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

## 6. URDF parser

既存Rust URDF parser crateを調査・評価して採用候補を選定する。自作XML parserは初期既定としない。選定時はlicense、URDF要素の対応範囲、unsupported情報の検出可否、保守状況、依存関係を確認する。

Parser固有型をrobot-model Public APIへ露出しない。ImportResultは成功値と非fatal diagnosticsを同時に返せる構造とする。最低限の分類は `Warning`、`Unsupported`、`MissingAsset`、`Approximated` とし、致命的なparse / structure errorは別途errorで返す。

## 7. Frame変換とVR

ROS系からSansaXR系への基底変換は `(x,y,z) -> (-y,z,x)`、行列 `C` のdeterminantは-1。

- 極性ベクトル: `p_vr = C p_ros`
- 回転行列: `R_vr = C R_ros C^-1`
- 軸性ベクトル: `a_vr = det(C) C a_ros`
- 慣性tensor: `I_vr = C I_ros C^T`
- Mesh三角形: windingと法線の整合を検証

この変換は同じ物理方向を示すframe同士の基底変換であり、Robot配置やWorld原点の差は別のTransformとして合成する。

## 8. 初期実装順序と完了条件

1. 独自最小URDF fixtureとRM-001〜RM-015を実装する。
2. canonical round-tripとImport diagnosticsを確認する。
3. RM-016〜RM-024のframe / unit / VR adapter数学テストを実装する。
4. 全対象テストの成功、追加実装のunit test coverage 100%を検証する。
5. Microban等の第三者Robotを使った統合検証はlicense確認後に別途行う。

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

