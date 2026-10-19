# MER-REG-007 Registry／Robot Catalog保存整合性

## 対象
Framework所有のResource Registry（プロジェクト保存で確定）と、Meridian所有の`application/robot-catalog.json`（現状は登録時に即時保存）の整合性。Project Panelのツリー展開はFramework側の別課題とする。

## 現状とリスク
- 登録処理はRegistryへIDを登録した後、Catalogをディスクへ書き込む。
- Registryを含むFrameworkプロジェクト保存前に変更を破棄すると、Catalogが存在しないRegistry IDを参照する可能性がある。
- Catalog保存失敗時、現行の要求キューは毎フレーム再試行する。Registry IDは再利用されても、失敗を無制限に繰り返すことは望ましくない。
- 同じURDFを明示的に複数回登録した場合、既存設計では異なるRobot IDを持つ別Robotとなる。再試行とユーザーによる新規登録を区別する必要がある。

## 決定事項
1. 不整合を黙って修復しない。Robot ID、Registry ID、Catalogファイルを勝手に再生成・削除しない。
2. 登録の成功条件は、RegistryとCatalogの両方が永続化され、再オープン後に相互参照を解決できること。現状の「Catalog書込み成功」は暫定成功である。
3. 保存失敗時は失敗を記録し、ユーザー操作または明示的な再試行によって再実行する。フレームごとの無制限再試行は避ける。
4. 既存のURDF/MJCFインポートではCatalogを暗黙登録しない。
5. FW所有のRegistryをMeridianから直接シリアライズしない。Frameworkの保存機構と連携する。
6. 未解決参照を検出した場合は診断情報（Robot ID、Registry ID）を残し、Catalogを変更しない。

## テスト仕様
| ID | 操作 | 期待結果 |
|---|---|---|
| MER-REG-PERSIST-TC-001 | 登録後にFramework保存せず終了・再オープン | 未解決Registry IDを検出し、Robot IDとCatalog内容を保持 |
| MER-REG-PERSIST-TC-002 | 登録後にFramework保存・再オープン | Registry IDとRobot IDが変化せず参照が解決する |
| MER-REG-PERSIST-TC-003 | Catalog保存失敗 | 重複Robotを生成せず、失敗を報告し、無制限再試行しない |
| MER-REG-PERSIST-TC-004 | 同一要求を明示的に再試行 | 同一操作の再試行では新しいRobot IDを作らない |
| MER-REG-PERSIST-TC-005 | RegistryにないIDを含むCatalogを読込 | Catalogを上書きせず、該当RobotとRepresentationを診断 |
| MER-REG-PERSIST-TC-006 | プロジェクト切替中の保留要求 | 他プロジェクトに適用しない |
| MER-REG-PERSIST-TC-007 | Save As | コピー先でもRobot ID／Registry IDを維持 |
| MER-REG-PERSIST-TC-008 | 既存URDFの通常インポート | Catalogへの暗黙登録なし |

## 実装順序
1. RegistryとCatalogの参照検査（読み取り専用）およびテスト。
2. 登録キューの再試行制御（失敗時停止、明示的再試行、要求ID）。
3. Framework保存との整合を取るコミット境界を設計・実装。
4. Windows GUIで保存・破棄・再オープン・Save Asを検証。

## 未確定事項
- Framework保存APIをGUIコマンドから安全に起動する公開手段。存在しない場合はFrameworkへAPI要望を出す。
- Catalog書込みとFramework保存の間にプロセス異常終了した場合のリカバリ方式。
- 旧バージョンが作成した未解決参照をユーザー操作で再接続するUI。
