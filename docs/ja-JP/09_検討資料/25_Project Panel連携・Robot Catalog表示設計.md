# MER-RESOURCE-001 Project Panel連携・Robot Catalog表示設計

## 1. 適用範囲・依存

- Meridian側の設計と検証対象。FWリポジトリは変更しない。
- 新API確認元: workflow-ide-framework `f0534f6`（FW側の実画面最終検証前）。
- Meridianの現在の依存: `4b5f578`。新APIへの切替はFW検証後、依存更新とWindowsビルド検証を必要とする。
- 既存 `application/robot-catalog.json` のschema v1、既存URDF/MJCF保存、プロジェクト互換性は維持。

## 2. IDの独立性

| 種別 | 所有者 | 用途 |
| --- | --- | --- |
| Robot ID | Meridian RobotCatalog | 永続的な論理ロボット識別子（UUID v4） |
| Registry Resource ID | FW ResourceRegistry | プロジェクト内のファイル参照識別子 |
| Project Panel Node ID | Meridianの表示アダプタ | 同じリソースを複数位置に表示するためのノード識別子 |

- Node IDは表示対象と親ノードを含めて一意に生成し、Robot IDを置換しない。
- Robotの名称変更時にRobot ID / Registry Resource IDは変更しない。
- 同じRegistry Resource IDの複数ノード参照は許容する。
- Registryに存在しない参照をパスや名前から推測して修復しない。

## 3. 初期表示

```text
Robots                                 [Category]
  <robot name>                         [LogicalResource; robot_id]
    Representations                    [Category]
      <format/role>                    [RegisteredResource; registry_resource_id]
```

- RobotCatalogに登録されたロボットのみを表示する。既存の `application/robot-source.urdf` および `application/mjcf-snapshot.json` の存在だけでRobotを生成・関連付けしない。
- 各表示ノードに専用IDを割り当てる。`RegisteredResource`にはRegistry IDのみを渡す。
- 新規プロジェクトやlegacy catalogなしの場合は空のRobotsカテゴリを返す。
- ファイル実体の有無とRegistry登録の有無は別概念。 `Directory`表示は今後、ロボットごとの保存フォルダを定義した後に追加。

## 4. APIと検証境界

- `ApplicationProjectPanelProvider::project_panel_roots` は `ProjectContext` のCatalogを読み取ってスナップショットを生成する。
- `handle_project_panel_action` は選択・アクティブ化を受け取るが、未対応の操作は `NotHandled` とし、勝手なファイル操作はしない。
- `Application::application_project_panel(panel_id, provider, registry_snapshot)` でFWに登録する。
- `ProjectPanelRefreshHandle::invalidate_application_project_panel` をImport/Export・名称変更・プロジェクト切替で利用する。
- Registry取得はFWが所有する保存済みRegistryの取得APIを先に確認する。空Registryを常時返す仮実装は行わない（RegisteredResourceが解決できないため）。
- 未登録Registry IDはFWの `UnknownRegisteredResource` で検出される。保存データの一貫性判定とUIエラー表示の扱いを分けて決定する。
- 物理ファイル列挙はFWの `Directory` APIに委任し、プロジェクトルート外への参照禁止を維持する。
- FW `f0534f6` の実フォルダUI接続は未検証。Meridian実画面で別途確認する。

## 5. 実装前テスト（次の工程）

| ID | テスト内容 |
| --- | --- |
| MER-PP-TC-001 | Catalogなし・空Catalog → Robotsカテゴリのみ |
| MER-PP-TC-002 | 2ロボットのNode ID一意性とRobot ID保存 |
| MER-PP-TC-003 | ロボット改名でもRobot IDおよびNode ID不変 |
| MER-PP-TC-004 | RepresentationがRegistry Resource IDを参照 |
| MER-PP-TC-005 | 同一Registry IDの複数表示ノードはIDが重複しない |
| MER-PP-TC-006 | 未登録Registry参照 → 明示的なエラー、勝手な登録・補修なし |
| MER-PP-TC-007 | Save As後もRobot ID / Registry IDを保持 |
| MER-PP-TC-008 | プロジェクト切替後は前のツリーを持ち越さない |
| MER-PP-TC-009 | 選択・アクティブ化とNotHandled動作 |
| MER-PP-TC-010 | Import/Export成功時のみ適切な再描画要求 |
| MER-PP-TC-011 | Directoryのプロジェクト相対パス・拡張子フィルタ |
| MER-PP-TC-012 | ファイル表示がRegistry自動登録を生まない |

## 6. 段階的な実装

1. この設計の確定と実装前テスト作成。
2. Catalog→AppPanelNodeの純粋な変換処理（Registryを変更しない）を実装。
3. FWのRegistryスナップショット取得・永続登録APIを確認。
4. FW依存先を検証済みコミットに更新し、MeridianへのProvider登録・更新通知を組み込む。
5. Windowsでfmt/test/clippy、Project Panel表示、Save As、選択・更新動作を検証。

## 7. 保留事項

- URDF/MJCF ImportからのRobot新規登録とRegistry登録をどのように不可分・復旧可能にするか。
- `application/` 直下のlegacyファイルをどの時点でロボット専用フォルダへ移行するか。
- Registry参照欠損をプロジェクト整合性不良として扱う場合の復旧UI。
- 同一ファイルの重複表示とDirectory表示の双方がある場合のUX。
