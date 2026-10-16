# Robot論理リソースID・Import競合・永続化設計（MER-RESOURCE-001）

状態: 設計案・実装前テスト定義。実装未着手。

## 基本方針

Robotのプロジェクト内永続IDと配布元の由来を分離する。FWがProject lifecycleとResource Registryを所有し、MeridianはRobotのdomain情報を所有する。既存のProject格納方針（検討資料18）とProject Panel APIレビュー（検討資料23）を前提とする。

| 識別子 | 役割 |
|---|---|
| `robot_id` | Project内で一意なRobot論理実体ID。Save/Open/Renameで維持。UUID v4相当を新規発行し、重複を検査 |
| `source_namespace` + `source_id` | 任意の配布元識別。由来を記録するが、同一性や内容一致の保証には使用しない |
| `registry_resource_id` | FW Resource RegistryのファイルID。URDF/MJCF/Asset等の表現に対応 |
| `AppPanelNode.id` | FW Project Panelの表示ノードID。Robot IDとは区別 |

コピーされたUUIDは重複し得る。配布元IDが一致しても自動上書き・自動マージしない。同じ配布元を持つ複数Robotを許容する。

## 保存形式案

Meridian所有の `application/robot-catalog.json`（仮）にRobot論理情報を保存する。正式ファイル名とschemaは実装前に確定する。FW Registryが管理するpath/scope/stateは重複保存しない。

```json
{
  "schema_version": 1,
  "robots": [
    {
      "robot_id": "2d246b45-77b2-47d7-b28c-9c8d1773a93d",
      "name": "Test Robot",
      "origin": { "source_namespace": "example.vendor/robots", "source_id": "test-robot" },
      "representations": [
        { "registry_resource_id": "res-urdf", "format": "urdf", "role": "exchange" },
        { "registry_resource_id": "res-mjcf", "format": "mjcf", "role": "exchange" }
      ]
    }
  ]
}
```

Robotの表現ファイルが不足しても論理情報を保持し、Registry参照の不整合は診断する。

## 操作規則

| 操作 | robot_id | 由来 |
|---|---|---|
| Save/Open、名前変更 | 維持 | 維持 |
| Project全体コピー、Save As | 維持 | 維持 |
| 同一Project内Robot複製 | 新規発行 | 継承可 |
| 外部URDF/MJCFを新規Import | 新規発行 | 元データに識別情報があれば保持 |
| Robot配布データをImport | 新規発行が既定 | 維持可 |
| 明示的な既存Robot更新 | 既存IDを維持 | 利用者が更新対象を指定。初期版対象外 |
| Project間移動 | 移動先で衝突検査 | 維持可 |

独自Robot配布Packageは現段階では実装しない。交換対象は既存方針どおりURDF/MJCFと必要な関連Assetに限定する。

## 既存Projectとの共存

現行の `application/robot-source.urdf` と `application/mjcf-snapshot.json` は維持する。Catalogがない旧Projectも読み込み可能とする。ファイルが存在するだけではFW Registry登録済みとみなさない。URDFとMJCFの共存だけで同一Robotとは断定しない。移行不能な参照を勝手に削除・上書きしない。Application data_versionの更新はFW Adapter契約に従い、FW所有のproject.tomlを直接変更しない。

## 実装前テスト

| ID | 検証 | 期待結果 |
|---|---|---|
| MER-RES-TC-001 | Robotを2件作成 | 異なるrobot_idで保存 |
| MER-RES-TC-002 | Save/Open、Rename | robot_id維持 |
| MER-RES-TC-003 | Robot複製 | 新規robot_id、元Robot不変 |
| MER-RES-TC-004 | 同一source_namespace/source_idの2件 | 両方保持、自動マージなし |
| MER-RES-TC-005 | Project内robot_id重複 | 拒否、既存データ非破壊 |
| MER-RES-TC-006 | 別Projectに同一robot_id | それぞれ独立してロード可能 |
| MER-RES-TC-007 | 外部URDF/MJCF新規Import | 新規robot_id、既存Robot不変 |
| MER-RES-TC-008 | Registry ID参照 | FW経由で解決、path二重保存なし |
| MER-RES-TC-009 | 不足・不正Registry参照 | 診断し論理情報保持 |
| MER-RES-TC-010 | Catalogのない旧Project | 既存URDF/MJCF動作を維持 |
| MER-RES-TC-011 | URDFとMJCFが共存 | 同一Robotと無条件に推定しない |
| MER-RES-TC-012 | Catalog保存途中で失敗 | 既存の有効な保存データを保持 |

## 実装順序

1. SchemaとID型を確定し、実装前テストを追加する。
2. Robot Catalogモデル・検証・保存・読込をFW UI非依存で実装する。
3. 旧Projectとの共存・移行を実装する。
4. FW Resource Registry公開APIに接続する。
5. FW Project Panelの確定APIに接続する。
6. GUIで追加・複製・名称変更を検証する。

未決定: Catalog正式ファイル名、originの外部表現、旧データのRobot同一性判定、明示更新UI、Registry登録操作のトランザクション境界。他domain objectのID規則は別途検討する。
