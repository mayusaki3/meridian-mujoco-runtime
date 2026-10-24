# MER-REG-007 Registry／Robot Catalog保存整合性

## 対象
Framework所有のResource Registry（プロジェクト保存で確定）と、Meridian所有の`application/robot-catalog.json`（現在はCatalog Stageに保持し、保存時に永続化）の整合性。Project PanelはMeridianのCatalog Stageを参照し、登録直後の更新通知で再構築する。

## 現状とリスク
- 登録処理はRegistryへIDを登録した後、Catalog Stageを更新する。Catalogの永続化はProject Adapterの保存処理で行う。
- Registryを含むFrameworkプロジェクト保存前に変更を破棄すると、Catalogが存在しないRegistry IDを参照する可能性がある。
- 登録要求は一度消費する。保存時の部分失敗や異常終了時には、RegistryとCatalogの不整合が残る可能性がある。
- 同じURDFを明示的に複数回登録した場合、既存設計では異なるRobot IDを持つ別Robotとなる。再試行とユーザーによる新規登録を区別する必要がある。

## 決定事項
1. 不整合を黙って修復しない。Robot ID、Registry ID、Catalogファイルを勝手に再生成・削除しない。
2. 登録の成功条件は、RegistryとCatalogの両方が永続化され、再オープン後に相互参照を解決できること。登録直後の成功ログはメモリ上の登録成功を示す。永続化の成功は保存処理と再オープンで別途確認する。
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

## Framework develop確認結果（2026-10-19）

Framework開発側から以下の回答を受領した。

- `ApplicationProjectAdapter::save_project_data()`／`save_project_data_as()`はFrameworkのプロジェクト保存処理に参加する正式なAPI。
- 保存順序はアプリデータ→Framework設定（Registry）→Projectメタデータ。各ファイルは個別にatomic_writeされるが、複数ファイル全体の原子性・ロールバックは未対応。
- `RegistryAccess`は正式なRegistry操作API。`on_registry_access()`の呼出頻度はAPI契約として保証されないため、登録要求をキューに積んで一度だけ処理する。
- 保存失敗時はDirty状態とメモリ上のRegistry変更を維持するが、ディスク上の部分保存は戻されない。
- プロジェクトオープン成功後、Registryが参照可能になった段階で`find_by_id()`により整合性を検査する。
- 保存失敗時に`Saved`／`SavedAs`イベントは発行されない。保存前処理はイベントではなくAdapterを使用する。

### Meridian側で実施する変更

1. アプリが参照するCatalog共有状態をProject AdapterとRegistry登録ハンドラーで共有し、対象プロジェクトルートを厳密に照合する。
2. 登録時にCatalogを即時保存せず、メモリ上の変更のみを保持する。
3. 通常保存・Save As時にAdapterがCatalogを保存する。Save Asでは保存元のメモリ上の変更を優先し、Robot ID／Registry IDを維持する。
4. 保存破棄・プロジェクト切替では保留中のCatalogを破棄する。保存失敗時はメモリ上の変更を保持する。
5. 登録失敗はログに残し、フレームごとの無制限再試行をしない。
6. 開いたプロジェクトの未解決Registry参照を検出・報告する。自動修復は行わない。

### Framework側の未実装依存

- Registry・アプリデータ・Projectメタデータの原子的保存、部分保存時の復旧
- コマンドからの明示的なRegistry操作API
- Registryとアプリデータの統合整合性検査・UI通知

これらの実装が完了するまで、Meridianの保存成功は「現行Frameworkの段階的保存が成功した」ことを意味し、複数ファイルを跨ぐクラッシュ耐性までは保証しない。

## Windows GUI検証記録（2026-10-24）

- 対象：`D:\\WORKPLACE\\MELTEST-MJCF01`、Meridian `develop` コミット `52a4822`。
- 既存CatalogにはRobotが2件あり、同一Registry Resource IDを共有する構成。
- メニューから既存URDFを新規Robotとして登録した直後、`Project Panel refresh requested after Catalog registration` を確認。
- 続けて `Project Panel snapshot requested` に `source="stage" robots=3` を確認。保存操作なしでProject Panelに3件表示。**即時更新PASS**。
- 以前の「保存するまで表示されない」現象は、ローカル `develop` がリモートより5コミット古く、更新通知処理を含まないバイナリで実行していたことが原因。
- `cargo run` は成功。初回の `cargo test --workspace` はWindowsのページングファイル不足（OS error 1455）でコンパイル停止。その後 `CARGO_BUILD_JOBS=1`、`CARGO_INCREMENTAL=0`、`cargo test --workspace -j 1` で **94/94 PASS**。`cargo fmt --all` 後の `cargo fmt --all -- --check` も **PASS**。整形差分3ファイルはローカル未コミット。
- ローカルの未コミット変更3ファイルはstashへ退避済み。差分確認まで `stash pop/drop` は実施しない。
- Framework側の変更は不要だった。
