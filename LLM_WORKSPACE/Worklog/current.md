# Worklog

更新日: 2026-11-25 (JST)
対象ブランチ: develop

## 現在工程
URDF意味検証・非破壊Import/Exportの設計更新。資料21・22更新済み、資料20は未更新。

## 決定事項
- 初版のため既存API・データ構造との互換性維持を制約としない。
- XML構文エラーは通常Import失敗とする。
- URDF意味検証エラー時はSource Documentと診断結果を保持し、RobotDefinitionを生成しない。シミュレーション不可とする。
- 警告のみの場合はRobotDefinitionを生成できる。
- Source Document修正後に構文・意味検証を再実行し、意味検証エラーが解消した場合にRobotDefinitionを生成する。
- URDF意味検証とSimulator固有検証を分離する。
- URDF編集方法は現段階では限定しない。

- SourceDocumentとRobotDefinitionは独立して保持し、管理層で関連付ける（A案、2026-11-25合意）。Source Mappingとdomain IDの対応は意味検証成功後のRobotDefinition生成時に確立する。

## 最新正本・更新対象
- [資料20](../../docs/ja-JP/09_検討資料/20_robot-model_初期Vertical_Slice_テスト仕様.md)
- [資料21](../../docs/ja-JP/09_検討資料/21_robot-model_実装前型・変換境界設計.md)
- [資料22](../../docs/ja-JP/09_検討資料/22_robot-model_非破壊Import_Export_詳細設計.md)

## 実施状況・保留事項
- 上記方針は利用者合意済み。資料21への反映完了（commit 46d8b189b8eb54f4997904e5076d35f97e9bdfae、再取得一致確認PASS）。資料22への反映完了（commit 48c279b29dcd8dc897b18ab089c343e2839ed610、再取得一致確認PASS）。資料20への反映は未完了。
- Robot Catalog・Project Panel連携の作業は保留。
- 今回の意味検証変更に関する実装・テスト実行は未実施。
- 過去の作業記録全体の復元は未実施。本ファイルは今回の合意事項を起点とする。

## 最新正本との照合結果
- 資料20 RM-012はXML構文エラーとURDF意味検証エラーを区別していない。
- 資料21 第6章はXML構文解析と意味検証の分離、SourceDocument保持、Simulator検証分離へ更新済み。管理層の具体的crate配置、診断コード体系は未確定。
- 資料22 第2・3章はSourceDocumentの独立保持と管理層の関連付けに更新済み。関連付けの多重度・具体的schemaは未確定。
- 資料22 第4章は意味検証をcanonical変換前に行い、未生成状態から再検証する手順に更新済み。既存RobotDefinitionの再Import更新規則は保留。
- 資料22 第5章は正常なRobotDefinitionを通常モデルExportの前提とし、出力の再解析・意味検証・復元確認後に完全Export成功とするよう更新済み。

## 次工程
- 資料20の最新全文を取得し、更新済み資料21・22と整合するテスト仕様へ更新する。
- 設計・テスト仕様の差分を確定し、反映する。RM-012および関連既存テストの期待結果を見直し、追加テスト候補RM-033〜RM-039を整理する。
- 設計・テスト仕様の確定後にテスト作成、実装へ進む。
- 工程進捗、決定事項、検証結果に応じて本Worklogを更新する。
