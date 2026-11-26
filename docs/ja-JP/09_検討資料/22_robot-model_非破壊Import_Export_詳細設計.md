<!--
HLDocS:LLM-MANAGED
doc_id: doc-20261008-000002Z-RMPR
lang: ja-JP
document_type: research
canonical_title: robot-model 非破壊Import Export 詳細設計
canonical_document: true
-->

[目次](../目次.md) > 検討資料 > robot-model 非破壊Import Export 詳細設計

# robot-model 非破壊Import / Export 詳細設計

## 1. 目的と保証境界

資料21の非破壊保持方針を、実装可能なデータ構造と処理順序に具体化する。URDFの未知要素・未知属性を検出・保持し、canonical保存・再読込・既知項目編集後の同形式Exportで復元する。初期保証は**意味・構造上のround-trip**とし、元XML全体のbyte一致は保証しない。

未対応情報を解釈したことにしない。復元不能・衝突時は成功を返さず、失われる情報を診断可能にする。

## 2. データ構造（Rust擬似定義）

```rust
struct RobotDefinition {
    id: RobotId,
    name: String,
    links: Vec<Link>,
    joints: Vec<Joint>,
    // 形式固有情報は所有しない
}

struct SourceDocument {
    id: SourceDocumentId,
    format: SourceFormat,           // 初期: Urdf
    format_version: Option<String>,
    original_xml: String,          // 元XMLを独立して保持
    preserved: Vec<PreservedNode>,
    mappings: Vec<SourceMapping>,   // 意味検証成功後にdomain IDと対応
    diagnostics: Vec<Diagnostic>,
}

// 管理層が保持する関連付けの概念型
struct SourceAssociation {
    source_document_id: SourceDocumentId,
    robot_id: RobotId,
}

struct SourceMapping {
    source_key: SourceNodeKey,
    owner: SourceOwner,             // Robot / Link(LinkId) / Joint(JointId) / ...
    placement: Placement,           // 親source keyと兄弟順序の情報
}

struct PreservedNode {
    source_key: SourceNodeKey,
    payload: PreservedPayload,      // XML subtreeまたは未知attribute
    provenance: SourceProvenance,   // 入力documentと位置（任意）
}

enum PreservedPayload {
    XmlElement { /* 名前空間を含む要素木、属性、子、text */ },
    XmlAttribute { /* namespace, name, value */ },
    XmlText { /* 必要なtext / CDATA等 */ },
}

enum ExportOutcome {
    Complete { bytes: Vec<u8>, diagnostics: Vec<Diagnostic> },
    Conflict { diagnostics: Vec<Diagnostic> },
}
```

これは**概念的な型**であり、正確なRust API・serde schemaを確定したものではない。特に `SourceNodeKey` は元ファイルの位置情報だけをIDとして使用せず、Import時に生成した安定識別子をcanonicalへ保存する。domain IDとSourceNodeKeyは異なる。

SourceDocumentとRobotDefinitionは独立して保存する。管理層が関連付けを保持し、RobotDefinition未生成時は関連付けを作成しない。直接作成したRobotDefinitionはSourceDocumentを必須としない。SourceAssociationは責務を示す概念型であり、一つのSourceDocumentから複数RobotDefinitionへの関連付けを許容する。具体的な永続化形式・crate配置および逆方向の多重度は未確定とする。

## 3. 所有関係・配置

- `SourceOwner` はdomain IDでLink / Jointを参照する。表示名では参照しない。意味検証成功後に対応を確立する。
- 意味検証エラー時はdomain IDへの対応を必須とせず、元XMLとsource keyに基づく情報および診断をSourceDocumentに保持する。
- `Placement` は既知要素を含む元の兄弟順序を復元できるよう、親要素のSourceNodeKey、兄弟アンカー、順序を保持する。
- 未知要素の**内部**は順序・属性・子要素を保持する。未知要素内の参照文字列を、意味が不明なまま自動変更しない。
- 既知要素に付いた未知属性も、対応する既知要素へ再付与する。既知属性と同じ展開名（namespace URI + local name）で衝突した場合はConflictとする。
- 削除したLink / Jointに属するPreservedNodeは、ユーザーが明示的に破棄しない限りcanonicalから削除しない。

## 4. Import処理

1. 元XMLを構文解析し、要素・属性・順序・namespaceとsource keyを取得する。XML構文エラー時は通常Importを失敗とし、診断を返す。
2. 元XMLと形式固有情報をSourceDocumentとして保持する。未知要素・属性はPreserved Dataとして保持する。
3. URDF意味検証を行い、検出可能な複数の問題を診断する。
4. 意味検証エラーがある場合はSourceDocumentと診断結果を保持し、RobotDefinitionを生成しない。シミュレーション対象としない。
5. 意味検証エラーがなくcanonical変換に成功した場合にRobotDefinitionを生成する。警告のみの場合は生成を妨げない。
6. XML source keyとdomain IDを対応付け、管理層でSourceDocumentとRobotDefinitionを関連付ける。
7. 対応付け不能または保持失敗の場合は完全な非破壊Import成功とはしない。
8. Simulator固有の対応可否・変換検証は対象Simulatorを使用する段階で別途行う。

Parserが未知XMLを捨てる場合でも、**元XMLを先に解析して差分を検出**する。parserのdeserialize成功だけで非破壊Import成功とは判定しない。

### 4.1 SourceDocument修正後の再検証

意味検証エラーによりRobotDefinitionが未生成の場合、保持したSourceDocumentを修正し、XML構文解析とURDF意味検証を再実行する。診断結果を更新し、意味検証エラーが解消してcanonical変換に成功した場合にRobotDefinitionを生成し、Source Mappingと関連付けを確立する。編集方法は限定しない。

既存RobotDefinitionがある場合の再Import時のID対応および更新規則は別途確定する。

### 4.2 複数ツリーの分割Import

通常Importでは独立した複数Linkツリーを意味検証エラーとして診断する。利用者の承認後に限り、各独立ツリーを個別に意味検証・canonical変換する。正常なツリーのみRobotDefinitionを生成し、部分成功を許容する。未承認・拒否時はSourceDocumentのみ保持する。元XMLを変更せず、成功した各RobotDefinitionについてSource Mappingと関連付けを確立する。ツリー間Mimic参照は分割後の意味検証対象とする。所属先不明の拡張情報は自動割当せず保持・診断する。

## 5. Export処理

通常のモデルExportは正常な単一RobotDefinitionを前提とする。意味検証エラーによりRobotDefinitionが未生成の場合は通常のモデルExportを行わない。SourceDocumentの保持・編集はモデルExportと区別する。

1. canonical modelから最新の既知URDF要素・属性を構築する。
2. SourceMappingで所有domain IDを解決する。
3. 元の親・兄弟アンカーを利用して未知属性・要素を挿入する。
4. 既知属性との衝突、所有要素削除、復元位置不明、未知情報内の明白な旧名参照等を診断する。
5. Conflictがあれば通常の完全Exportを拒否する。部分出力を完全Exportと表示しない。
6. 生成したXMLを再parseし、URDF意味検証と既知データ・Preserved Dataの復元を確認してから完全Export成功とする。

異形式（URDF→MJCF等）へのExportでは、元形式のPreservedNodeを無条件に出力しない。情報が表現できない場合は明示的なlossy操作とdiagnosticを要求する。

### 5.1 分割Import後の単独URDF Export

対象RobotDefinitionから単一ツリーのURDFを生成する。元の複数ツリーを再構築しない。複数ロボットのマージは別ツールの責務とし、初版の通常Exportには含めない。

| 情報 | 扱い |
|---|---|
| 対象RobotのLink・Joint | 出力する |
| 対象Robotが参照する共通Material | 出力する |
| 対象Robotに属する未知要素・属性 | 復元する |
| 他のRobotに属する情報 | 出力しない |
| 所属先不明の未知情報 | 元SourceDocumentに保持し、診断する。自動複製・割当しない |
| 元の複数ツリー構造 | 復元しない |

完全Exportの判定は対象RobotDefinitionの復元範囲に対して行う。所属先不明の情報を復元済みとは報告しない。既存のConflict・lossy判定規則を維持する。

## 6. 編集操作別の扱い

| 編集操作 | Known Data | Preserved Data |
|---|---|---|
| mass / limit等の変更 | 新値で出力 | 所有要素へ再付与 |
| Link / Joint名変更 | 新名で出力 | domain IDで追従。未知断片内部の文字列は原則変更せず診断 |
| Link / Joint追加 | 新規要素として出力 | 元の未知情報は変更しない |
| 所有Link / Joint削除 | 対象を出力しない | 孤立データを保持し、完全ExportをConflictにする |
| 既知要素の並べ替え | 新しいcanonical順序を優先 | anchorが解決できない場合はConflict |
| 既知属性と未知属性の衝突 | 勝手に上書きしない | Conflictとして両値を報告 |

## 7. XML保持保証

初期必須: 未知要素の名前空間・属性値・子要素階層・text、未知属性の値、未知兄弟要素の相対順序、所有要素への対応。

初期非保証: 元ファイルと同一の空白、引用符の種類、属性順序、XML宣言の字面、entity referenceの字面、完全なbyte一致。

コメント・CDATA・処理命令は、XML解析ライブラリの保存能力をfixtureで評価し、保証するかどうかを決定する。CDATAを通常textへ正規化した場合、文字列値が同じでも表現が変わるため、その差を診断・仕様で扱う。

## 8. Fixtureとテスト対応

- `minimal.urdf`: 2 Link / 1 Joint、origin / axis / limit / inertial（RM-001〜RM-015）
- `unknown-extension.urdf`: Robot / Link / Jointの未知属性と未知子要素（RM-011, RM-025）
- `mixed-order.urdf`: 既知要素間に未知要素、namespace付き属性（RM-026, RM-027, RM-032）
- `renamed-owner.urdf`: 所有Link名変更と未知断片内の旧名参照（RM-028）
- `deleted-owner.urdf`: 所有Link削除後のConflict（RM-029）
- `conflicting-attribute.urdf`: 既知・未知属性衝突（RM-030）

fixtureは本リポジトリ独自作成とし、第三者Robotの再配布条件に依存させない。

## 9. 実装開始前の確認点

- XMLライブラリのnamespace・コメント・CDATA・未知属性保持能力
- URDF parserの既知要素判定と、XML構造解析のsource mappingの整合性
- SourceNodeKey / SourceDocumentIdの永続化表現
- ExportのConflict診断構造と、明示的lossy操作のAPI
- canonical JSONのversioned schemaとPreserved Data保存方式
- SourceDocumentの独立永続化と管理層による関連付けの具体的なschema・crate配置
- 分割Importの部分成功・単独Export・共有情報保持のテスト（資料20 RM-045〜RM-051）

これらをfixtureとテストで確かめるまで、完全な非破壊Export対応を宣言しない。

---

[目次](../目次.md) > 検討資料 > robot-model 非破壊Import Export 詳細設計
