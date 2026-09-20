# meridian-mujoco-runtime Project 定義・格納方式 方針

## 目的

workflow-ide-framework Step 5 の Consumer 実装に先立ち、`meridian-mujoco-runtime` が扱う Project の基本単位と格納方式を整理する。

本資料では Project の自己完結性を優先し、再利用については Export / Import 方針のみを記録する。

今回の Export / Import の具体的な対象は、既存形式である URDF、MJCF（MuJoCo XML）およびそれらから参照される必要なファイルの範囲に限定する。

## 1. Project の定義

Meridian Project は、1つのシミュレーション・検証作業を再現するために必要な定義をまとめる作業単位とする。

Project は原則として自己完結させる。

Project を別の保存場所や別PCへ移動した際に、Project外のMeridian固有共有ディレクトリを必須としない。

Project内では、少なくとも次の概念を扱える構造を想定する。

- Robot
- Actuator
- Field
- Scenario / Task
- Execution Profile
- Runtime Configuration
- Project固有の入力データ
- 実行結果

これらの詳細なデータモデルは既存の要件・検討資料で別途定義する。

## 2. Project と RuntimeSession の分離

Project は保存可能な定義・入力・結果を扱う。

実行中だけ存在する RuntimeSession の動的状態は Project 定義そのものには含めない。

必要な実行結果や記録のみを Project の結果データとして保存できるようにする。

## 3. 基本格納構造

初期案は次の構造とする。

```text
<project>/
├─ meridian.toml
├─ resources/
│  ├─ robots/
│  ├─ actuators/
│  ├─ fields/
│  ├─ scenarios/
│  └─ profiles/
├─ assets/
├─ data/
├─ results/
└─ .meridian/
```

### meridian.toml

Project の入口となる manifest。

Project の識別情報と、Project内で使用する主要Resourceへの参照を持つ。

個々のRobotやField等の詳細情報をすべて1ファイルへ集約することは目的としない。

manifestの具体的なschemaは後続仕様で決定する。

### resources

Projectを構成する意味のある定義を格納する。

Robot、Actuator、Field、Scenario、Execution Profile等を想定する。

### assets

mesh、texture等、Resourceから参照される補助ファイルを格納する。

Resource側の既存ファイル構成を維持した方が適切な場合は、Resource配下にassetsを持つことも許容する方向で検討する。

### data

Project固有の入力・計測データ等を格納する領域。

### results

simulation、sysid等によって生成され、保存対象とした結果を格納する。

cacheや一時ファイルとは分離する。

### .meridian

Application / IDE の作業状態を格納する領域。

Projectのsimulation上の意味を決定する情報とは分離する。

将来的にはDock Layout、開いているDocument、選択状態等を格納する可能性がある。

具体的な内容とGit管理方針は後続仕様で決定する。

## 4. パスの基本方針

Project内Resource間の参照は、Project内で解決可能な相対参照を基本とする。

通常のProject実行に、特定PCの絶対パスやMeridian共通共有ディレクトリを必須としない。

OS固有パス処理はConsumer側へ直接実装せず、必要な場合はFrameworkまたは共通層との責務境界を別途定義する。

## 5. 再利用の基本方針

Resourceの再利用は、Project外Resourceへの常時参照を基本方式とはせず、対象部分の Export / Import によって行う方針とする。

概念:

```text
Project A
  └─ Resource
       │
       │ Export
       ▼
   Exchange data
       │
       │ Import
       ▼
Project B
  └─ Resource copy
```

Import後のResourceはProject内へ取り込み、そのProjectだけで利用可能な状態を基本とする。

これにより、元Projectや共有保存場所が存在しなくてもProjectを再現できる構造を目指す。

## 6. Meridian計画内でのExport / Import共通化方針

将来的にはExport / Importの考え方および交換形式を、`meridian-mujoco-runtime`だけでなく他のMeridian計画Applicationでも利用可能な共通方式とすることを検討する。

ただし、各Application固有のProject形式そのものを共通化することは今回の決定事項としない。

共通化対象は、Application間で交換可能なResourceとその依存データを受け渡す仕組みを中心に検討する。

## 7. 今回扱うExport / Import範囲

今回のProject構造検討および初期実装では、独自のMeridian Exchange Package仕様は作成しない。

具体的なExport / Import対象は、既に一般的なファイル形式として存在する以下の範囲に限定する。

- URDF
- MJCF（MuJoCo XML）
- 上記ファイルから参照され、対象を成立させるために必要なmesh等の関連ファイル

初期段階では、これらをProject内へImportし、Project内のRobot / Field等のResourceとして扱えることを目標とする。

Projectから外部利用する場合も、まずはURDF / MJCF等の既存形式としてExport可能な範囲を対象とする。

次のものは今回のExport / Import仕様の対象外とする。

- Meridian独自Resource Package
- Meridian Application間の完全なProject交換
- sysid結果の共通Exchange Package
- RuntimeSession
- IDE Workspace状態
- Log
- Result全般の汎用交換形式
- Package repository / package manager
- 外部Resourceの常時参照・自動同期

これらを将来実装できないという意味ではなく、今回のProject定義では先行して仕様化しない。

## 8. Projectと既存MuJoCoデータセット概念の関係

既存要件では、MJCF、URDF、mesh、Runtimeデータ等を含む「MuJoCoデータセット」を選択・統合してRuntime用データを構築する考え方が定義されている。

Projectは、そのような入力データを実際の作業・実行単位として保持し、組み合わせる上位の単位として扱う方向とする。

既存の「MuJoCoデータセット」という用語・要件を直ちに置き換えず、Project / Resource / MuJoCoデータセットの最終的な用語整理は別途行う。

特に既存要件にある独自圧縮Export / Import形式については、今回の方針と合わせて後続で再検討する。

## 9. Step 5との関係

workflow-ide-framework Step 5では、このProject構造の全機能を実装しない。

Step 5 Consumer実装で必要になる範囲として、少なくとも次を想定する。

- ProjectというApplication側の作業単位を認識できる
- Project内DocumentをConsumer Panelから扱える
- Text Editor等のFramework Standard PanelへProject内Documentを渡せる
- Panel実装が特定の絶対パスへ依存しない
- Project / Resourceの意味はMeridian側が所有し、FrameworkへMuJoCo固有概念を持ち込まない

File Explorer、完全なImport / Export UI、Resource Inspector等はFramework側APIの進捗に応じて後続Stepで扱う。

## 10. 今回確定する基本原則

1. Meridian Projectは作業・実行の単位とする。
2. Projectは原則自己完結とする。
3. Project内Resource間の参照はProject内で解決可能にする。
4. Project外Resourceへの常時参照を再利用の基本方式としない。
5. 再利用はExport / Importを基本とする。
6. Export / Import方式は将来Meridian計画Application間で共通化する方向とする。
7. 今回のExport / Import対象はURDF、MJCFおよび必要な関連ファイルに限定する。
8. Meridian独自Exchange Packageは今回定義しない。
9. Project定義、RuntimeSession、IDE Workspace、生成結果を区別する。
10. FrameworkはProjectのMuJoCo固有意味を所有しない。
