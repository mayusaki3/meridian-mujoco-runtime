# MER-CAT-EDIT-001 Robot Catalog編集・Project Panel連携設計

## 対象
Robot論理レコードの名前変更・複製・削除。Framework所有のResource Registry・実ファイルは変更しない。Meridian所有のRobot IDとFramework所有のRegistry Resource IDを区別する。HLDocS v0.6.1に基づく実装前の設計・テスト観点。

## 要件
- R01 名前変更：Robot IDをキーに名前のみ変更。空白名と存在しないIDは拒否。Robot ID、origin、representations、Registry IDを保持。
- R02 複製：新しいRobot IDを発行。originとrepresentationsをコピーし、Registry IDは共有。空白名・存在しないIDは拒否。
- R03 削除：指定Robotの論理レコードのみ除去。Registry登録・実ファイル・他Robotの参照を維持。存在しないIDは拒否。
- R04 編集は対象プロジェクトのCatalog Stageで実施。成功時のみStageを更新しProject Panel Refresh Handleへ通知。失敗時はStage不変。
- R05 保存はProject Adapterを通じて実施。未保存編集はディスクに反映しない。破棄・プロジェクト切替でStageを引き継がない。
- R06 名前変更時はAppPanelNode IDを維持。複製時は別ノードID。削除時は対象ノードのみ消す。
- R07 GUIでの削除はユーザー確認後に実行。入力・確認UIの接続方法はFramework公開API確認後に決定する。
- R08 Registryの孤立エントリを本機能で自動削除しない。削除とRegistry解除は別操作とする。

## テスト仕様
| ID | 操作 | 期待結果 |
|---|---|---|
| MER-CAT-EDIT-TC-001 | 名前変更 | IDと参照維持、表示名更新 |
| MER-CAT-EDIT-TC-002 | 空白名・不明IDで名前変更 | エラー、Catalog不変 |
| MER-CAT-EDIT-TC-003 | 複製 | 別Robot ID、Registry ID共有 |
| MER-CAT-EDIT-TC-004 | 空白名・不明IDで複製 | エラー、Catalog不変 |
| MER-CAT-EDIT-TC-005 | 削除 | 対象のみ除去、共有参照維持 |
| MER-CAT-EDIT-TC-006 | 不明IDで削除 | エラー、Catalog不変 |
| MER-CAT-EDIT-TC-007 | 名前変更後Panel再取得 | 同じノードID、変更後ラベル |
| MER-CAT-EDIT-TC-008 | 複製後Panel再取得 | 別ノードID、Registry ID共有 |
| MER-CAT-EDIT-TC-009 | 削除後Panel再取得 | 対象ノードのみ消失 |
| MER-CAT-EDIT-TC-010 | 保存せず破棄 | ディスクCatalog不変 |
| MER-CAT-EDIT-TC-011 | 保存・再オープン | IDと編集結果維持 |
| MER-CAT-EDIT-TC-012 | 別プロジェクトの編集要求 | 適用しない |
| MER-CAT-EDIT-TC-013 | 編集失敗 | Stage不変、更新通知なし |
| MER-CAT-EDIT-TC-014 | 削除後Registry照会 | Registryと実ファイル維持 |

## 実装順序
1. RobotCatalog::removeと単体テスト。既存rename/duplicateの回帰確認。
2. Catalog Stageを介するプロジェクトスコープ編集サービスとテスト。
3. Framework Project Panel公開Action APIを調査し、編集コマンドとGUIを接続。
4. Windows GUIで更新通知・保存・破棄・再オープンを検証し記録。

## 未確定事項
- Project Panelコンテキストメニュー・名前入力・削除確認の公開API。
- Catalog Stage編集時のFramework Dirty UI同期手段。
- 入力ダイアログと多言語化キー。
