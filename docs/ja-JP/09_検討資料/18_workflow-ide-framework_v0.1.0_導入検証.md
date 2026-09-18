<!--
HLDocS:LLM-MANAGED
lang: ja-JP
canonical_title: workflow-ide-framework v0.1.0 導入検証
document_type: note
canonical_document: true
-->

# workflow-ide-framework v0.1.0 導入検証

## 1. 目的

meridian-mujoco-runtime を workflow-ide-framework v0.1.0 の最初の実利用アプリとして扱い、Framework 利用アプリを実装する手順から Framework の設定・公開境界を検証する。

Step 1〜4 は Framework 側で大枠を定義済みであり、本書では Meridian 側から実際に成立するかを確認する。

Step 5 以降は Panel 宣言だけでは完結せず Rust API / callback / ownership 境界が関係するため、Step 1〜4 の実装検証結果を API 設計へフィードバックしてから詰める。

## 2. 責務境界

```text
workflow-ide-framework
  Window / UI event loop / Dock / Panel lifecycle
  Surface infrastructure / OS input

meridian-mujoco-runtime
  application state / application actions
  MuJoCo Runtime / simulation loop / RuntimeSession
  Robot / Actuator / Field / Scenario
  sysid / SIL / HIL
```

Framework は MuJoCo domain object や RuntimeSession を所有しない。
UI frame lifetime と simulation / HIL control lifetime を同一にしない。

## 3. Step 1: Framework 導入

### 仮実装

- Cargo dependency として workflow-ide-framework を参照する。
- Application を Framework の起動境界へ渡す。
- egui / eframe / egui_dock / wgpu / CEF の個別初期化は Meridian 側に要求しない。

### 検証

- 最小 Application から IDE Window が起動できるか。
- Framework 終了と Meridian Runtime 終了を独立して扱える設計か。
- backend 固有型が Meridian の起動コードへ漏れないか。

## 4. Step 2: Application 基本設定

### 仮設定

必須候補:

- Application ID
- 表示名

Meridian では仮に次を使用する。

```text
id   = meridian-mujoco-runtime
name = Meridian MuJoCo Runtime
```

任意候補:

- Window title / size / minimum size
- Custom Title Bar
- Embedded Font
- UI scale
- icon

未指定項目は Framework 既定値を使用する。

### 検証

- ID / 表示名だけで起動できるか。
- Meridian 固有 Runtime 設定を ApplicationConfig に混在させずに済むか。

## 5. Step 3: Panel 宣言

初期検証用 Panel:

```text
simulation       : Simulation View : GPU Viewport
runtime-control  : Runtime Control : Standard UI
runtime-status   : Runtime Status  : Standard UI
log              : Log             : Standard UI
help             : Help            : Browser (optional)
```

この段階では MuJoCo 描画、Runtime 操作、Log 表示等を実装しない。

### 検証

- Panel ID / 表示名 / kind / 初期表示状態だけで宣言できるか。
- GPU / Browser backend 型を宣言へ要求せずに済むか。
- 未実装 Panel を宣言した状態で Layout を構築できるか。

## 6. Step 4: 初期 Layout

初期案:

```text
+------------------+--------------------------------+
| Runtime Control  |                                |
|                  |       Simulation View          |
+------------------+                                |
| Runtime Status   |                                |
+------------------+--------------------------------+
| Log                                               |
+---------------------------------------------------+
```

Layout は Panel ID のみを参照し、Dock backend 固有型を Meridian 側へ露出させない。

### 検証

- 水平 / 垂直分割。
- 分割比率。
- 同一領域への複数 Panel。
- 初期選択 Panel。
- Layout 外の宣言済み Panel。
- 未宣言 Panel ID の検出。
- Layout / Visibility 変更が simulation lifetime を直接変更しないこと。

## 7. Step 4 完了時の Framework 外部検証

Step 4 までの Framework 基盤が成立した時点で、Framework 側の Probe Panel を含む verification build を利用し、利用可能な外部環境で基盤検証を行う。

Meridian はこの時点では Step 5 の Panel 内部実装を完成させる必要はない。Application 起動、Panel 宣言、Layout、および Framework maintenance Probe が同一の Framework 利用形で成立することを確認する。

Probe Panel は既存 standalone Probe の置換ではなく統合フロントエンドとする。Framework が起動しない場合の切り分け用に standalone Probe も維持する。

外部環境の協力者からは `○ / ✕ / ？`、環境情報、log、必要な手動確認結果を Export して受領できる構成を想定する。

この検証を Step 5 以降の Rust API / callback / ownership 詳細設計へ進む前の中間マイルストーンとする。

## 8. Step 1〜4 の検証成果物

Meridian 側で仮実装を進める際、Framework API がまだ存在しない箇所は完成 API を推測して固定せず、必要な設定・型・責務を「要求」として記録する。

検証結果は少なくとも次に分類する。

- Framework 側に必要な設定
- Framework 公開 API に必要な境界
- Meridian 側だけに必要な実装
- backend 内部へ隠すべき処理
- Step 5 以降で決める事項
- 未検証事項

## 9. Step 5 以降への入口

Step 1〜4 の検証後、次を順に具体化する。

1. Standard UI Panel の Rust 実装境界。
2. GPU Viewport と MuJoCo rendering の接続境界。
3. Browser Panel の実装境界。
4. Panel と Runtime state の接続方法。
5. Rust Action の登録・実行・状態通知。
6. Framework 全体からの Build / Run / Debug / Test。

特に GPU Viewport では simulation loop を UI repaint rate に従属させないことを必須条件とする。
