# Project Panel Public API案 — Meridian側レビュー回答

状態: **Meridian側レビュー済み・FW側API確定待ち**  
対象: workflow-ide-framework `develop` の `ProjectPanel_Public_API_Meridian確認案.md`（commit `60b3ba0`）

## 結論

`ApplicationProjectPanelProvider`、`AppPanelNode`、`AppPanelNodeKind`、`AppPanelAction`、`AppPanelActionResult` の基本構成に賛成する。以下の補足を反映してFW側でAPIを確定してほしい。

現在のMeridianではURDF原本（`application/robot-source.urdf`）とMJCFスナップショットの保存・再読み込みを実装済み。一方、Robot / SysIDを独立した論理リソースとして永続管理する機能は未実装であり、既存の固定Project Panelツリーを将来置き換える必要がある。

## FW確認事項への回答

| # | 確認事項 | Meridian回答 |
|---|---|---|
| 1 | 論理リソースID | Robot / SysIDに表示名から独立したプロジェクト内で一意の永続IDを導入する。表示名変更やファイル移動では変えない。現状は未実装。 |
| 2 | 子ファイル | Registryへのファイル単位登録を基本とする。1つの論理リソースから複数ファイルを参照できること。MJCFのentry/include等を想定する。 |
| 3 | ツリー階層 | Category / LogicalResource / RegisteredResourceの再帰構造に合意。Robot / SysID以外のカテゴリと入れ子も許容する。 |
| 4 | 操作 | Selected / Activated / ContextActionを使用する。複数選択は必要。操作ごとの対応可否はMeridian側で判断する。 |
| 5 | 更新 | 初期版は全量スナップショット再取得に合意。Meridianは変更通知を発行し、FWが再取得する。安定ノードIDにより選択・展開状態を可能な範囲で維持する。 |
| 6 | 削除 | 論理リソース削除と実ファイル削除を分離する。論理リソース削除時に関連ファイルを暗黙に削除しない。 |
| 7 | Save/Open | ApplicationProjectAdapterを正とする。Open後のアプリデータ読込・整合性確認完了時にツリー再構築。Save As / Closeでも対象を更新する。 |
| 8 | 未登録ファイル | 初期版ではツリーへの直接表示は不要。既存の未登録ファイル検索を利用する。 |
| 9 | Project Settings | FW管理の独立ルートでよい。アプリのProviderには含めない。 |

## API確定に向けた補足

### 1. IDの分離

- `AppPanelNode.id`: ツリー上の出現位置を識別するstable ID。同一ファイルの複数箇所表示を許容する。
- `LogicalResource.resource_id`: Meridian管理のRobot / SysID実体を識別する永続ID。
- `RegisteredResource.registry_resource_id`: FW ResourceRegistryのファイルリソースID。

これらは同一視しない。Registryが管理するpath/scope/stateをノードへ重複保存しない。

### 2. Registryとの境界

Meridianのアプリ内部保存ファイルを、Registry未登録のまま`RegisteredResource`として表示しない。Registry登録と参照関係の管理はMeridian側の今後の実装事項とする。既存のFWファイル操作・Journal・Recoveryの責務を維持する。

### 3. 操作とフォールバック

`NotHandled`の場合はFWでサポート済みの既定操作へフォールバックできること。操作を拒否する場合は`Rejected`を返し、関連ファイルの削除等を暗黙に行わないこと。

### 4. 変更通知Public API

Providerの`project_panel_roots`とは別に、アプリが論理リソース追加・削除・改名・参照変更をFWへ通知できる公開IFを明確にすること。FW側で再取得のタイミングを制御し、必要に応じて通知を集約する。Project Open/Close/Save Asによる切替も対象とする。

### 5. 後方互換性

既存の`ProjectPanelModel` / `TreeModel`利用者を破壊しない追加APIとする。

## Meridian側の今後の実装事項

1. Robot / SysID論理リソースと永続IDのモデル・保存仕様を定義する。
2. 複数関連ファイルとRegistry登録・参照関係を定義する。
3. FWの確定APIに対してProviderと操作ハンドラを実装する。
4. 変更通知をインポート・編集・削除・プロジェクト切替に接続する。
5. 実装前テストを作成し、Save/Open・Save As・選択維持・削除非連動等を検証する。

## FWへの依頼

上記を反映してAPIを確定し、`PP-HYB-001`〜`PP-HYB-010`の実装前テストから着手してほしい。Meridian側のコード変更はFWのAPI確定後に別途実施する。
