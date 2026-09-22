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

Visual Studio の Project file に相当する Project Definition とする。

Project Definition は、次を明示的に定義する。

- このProjectが対象とするMeridian Applicationを識別する情報
- Project ID
- Project名
- Project自身のversion
- 説明
- 作成者
- 権利・License情報
- Projectに所属するResource
- Projectに所属するAsset

Project directory内にファイルが存在するだけではProject所属とはみなさず、Project Definitionへ登録されたResource / AssetをProject所属として扱う。

Project Definitionに個々のRobotやField等の詳細情報をすべて集約することは目的としない。

### Application と Project Format

`workflow-ide-framework`はMeridian Applicationへ一体化される内部Frameworkとして扱うため、Project DefinitionへFramework IDやFramework versionを記録しない。

Project Definitionの汎用的な読み書き、Resource / Asset管理、相対path解決、Format migration等の機構はFramework側へ持たせることができるが、利用者およびRuntimeから見たProjectの互換性はMeridian Applicationとして扱う。

Project Definitionには、そのProjectを扱うMeridian Applicationを識別するための情報を持たせる方向とする。Application versionをProject Definitionへ記録するか、またそれをProject Formatの互換性判断にどう使用するかは、Application側のversion方針と合わせて後続仕様で決定する。

Project自身のversionはApplication versionとは別の情報であり、Project作者が管理する。

概念例:

```toml
[application]
id = "meridian-mujoco-runtime"

[project]
id = "khr3hv-walking-test"
name = "KHR-3HV Walking Test"
version = "0.1.0"
description = "KHR-3HV walking simulation project"
authors = ["..."]
license = "..."
```

具体的なschema、正式なApplication ID、Project Definitionのファイル名・拡張子は後続仕様で決定する。

### resources

Projectを構成する意味のある定義を扱う。

Robot、Actuator、Field、Scenario、Execution Profile、URDF、MJCF等を想定する。

Resourceの論理分類と物理ディレクトリ構造は分離する。Resourceを特定の `resources/<分類>/` 配下へ置くことをProject Format上の必須条件にはしない。

Meridian内部定義は、URDF / MJCFより多くの情報を保持できる正本として扱う方向とする。

URDF / MJCFは主としてデータ交換、およびMuJoCo等の外部系へ渡す表現形式として位置付ける。ただし利用者から見ればProjectを構成するデータであるため、Meridian内部定義とURDF / MJCFをProject内で併存させてよい。

Project Definitionへ明示的に登録されたResourceをProject所属として扱う。

Resource登録では、Robot / Field等の「論理的な役割」と、Meridian内部形式 / URDF / MJCF等の「表現形式」を区別して扱える構造とする。具体的なschemaは後続仕様で決定する。

### assets

mesh、texture等、Resourceから参照される補助ファイルを扱う。

Assetの論理分類と物理ディレクトリ構造は分離する。既存URDF / MJCF等の相対参照関係を維持できるよう、Assetを特定の `assets/` 配下へ移動することをProject Format上の必須条件にはしない。

Project Definitionへ明示的に登録されたAssetをProject所属として扱う。

URDF / MJCF等から参照されるmeshやtextureも、Projectを成立させるデータとしてProject Definitionへの登録対象とする。

Import時などに交換形式から参照されるAssetを検出した場合、利用者が一つずつ登録することを必須とせず、ApplicationがProjectへの取り込みと登録を自動化できる構造とする。

必要なAssetが存在しない場合は、エラーとして扱うだけでなく、用途に応じてダミーmesh等の代替Assetを生成してProjectを成立させる補助機能も検討する。生成した代替Assetを使用する場合もProject Definitionへ登録し、Projectから認識可能な状態とする。

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

初期段階では、これらをProject内へImportし、Meridian内部定義へ変換できることを目標とする。

Import元のURDF / MJCF自体も、利用者から見たProject構成データとしてProject内へ保持し、Resourceとして登録してよい。Meridian内部定義と交換形式を二重に保持することを許容する。

Import時にはURDF / MJCFから参照されるmesh等の関連Assetを検出し、Projectへ取り込んだものをProject Definitionへ登録することを基本とする。

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

## 9. Project Definition の Resource / Asset 登録案

Project Definitionでは、Projectに所属するResource / Assetを明示的に登録する。

初期schemaでは、論理的な対象とファイル表現を混同しないため、Resourceに少なくとも次の情報を持たせる方向とする。

- `id`: Project内で一意なResource識別子
- `role`: Robot、Field、Actuator、Scenario等の論理的役割
- `format`: Meridian内部形式、URDF、MJCF等の表現形式
- `path`: Project rootを基準とする物理ファイルへの相対パス
- `source`: 同一論理対象の正本Resourceを示す任意の参照

`source`は、URDF / MJCF等がMeridian内部定義から生成・変換された表現であることを表せるようにするための候補である。具体的な名称と必須性は後続仕様で確定する。

概念例:

```toml
[[resources]]
id = "main-robot"
role = "robot"
format = "meridian"
path = "robot/robot.toml"

[[resources]]
id = "main-robot-mjcf"
role = "robot"
format = "mjcf"
path = "robot/robot.xml"
source = "main-robot"

[[resources]]
id = "main-robot-urdf"
role = "robot"
format = "urdf"
path = "robot/robot.urdf"
source = "main-robot"
```

この例では3ファイルは同じRobotを表すが、`main-robot`を情報量の多いMeridian内部定義として扱い、MJCF / URDFはその交換・外部利用向け表現として関連付ける。

Assetには、少なくとも次の情報を持たせる方向とする。

- `id`: Project内で一意なAsset識別子
- `role`: mesh、texture等の論理的役割
- `format`: STL、OBJ、PNG等の表現形式
- `path`: Project rootを基準とする物理ファイルへの相対パス
- `generated`: Applicationによって生成された代替Asset等であることを示す任意情報

概念例:

```toml
[[assets]]
id = "body-mesh"
role = "mesh"
format = "stl"
path = "robot/meshes/body.stl"

[[assets]]
id = "missing-hand-dummy"
role = "mesh"
format = "stl"
path = ".meridian/generated/missing-hand.stl"
generated = true
```

Resource / Assetの登録情報はProjectを構成するファイルを識別するための情報であり、RobotやMeshそのものの詳細仕様をProject Definitionへ重複記述することを目的としない。

### 同一論理対象の複数表現

同じRobot / Field等について、Meridian内部定義、URDF、MJCFを同時にProjectへ登録できる。

```text
Logical Robot
  ├─ Meridian internal definition  ← canonical
  ├─ MJCF                          ← exchange / MuJoCo
  └─ URDF                          ← exchange
```

Project Definitionはこれらの所属と関係を管理するが、変換処理そのものはApplication側の責務とする。

### Asset依存関係

URDF / MJCF内部に記述されたmesh等への参照は、その交換形式自身が持つ情報として維持する。

Project Definitionへ同じ参照関係をすべて二重記述することは必須としない。

一方、参照先Asset自体はProject Definitionへ登録する。

これによりApplicationはProject DefinitionからProject所属Assetを把握でき、URDF / MJCF解析時に次の検査を行える。

```text
Resource内のAsset参照
        ↓
参照先を解決
        ↓
Project Assetとして登録済みか
        ↓
実ファイルが存在するか
        ↓
利用可能 / 不足 / 不整合を判定
```

不足時にダミーAssetを生成する場合は、その生成物をProjectへ追加し、Project Definitionにも登録する。

### Import時の登録

URDF / MJCF Importでは、概念的に次の処理を行う。

```text
URDF / MJCF選択
    ↓
参照ファイル解析
    ↓
Resource本体をProjectへ取り込み・登録
    ↓
mesh等の関連AssetをProjectへ取り込み・登録
    ↓
不足Assetを検出
    ↓
必要に応じて代替Assetを生成・登録
    ↓
Meridian内部定義へ変換
    ↓
Meridian内部ResourceをProjectへ登録
```

利用者からは一つのImport操作として扱えることを目標とし、関連AssetのProject Definition登録を手作業で要求しない。

## 10. Step 5との関係

workflow-ide-framework Step 5では、このProject構造の全機能を実装しない。

Step 5 Consumer実装で必要になる範囲として、少なくとも次を想定する。

- ProjectというApplication側の作業単位を認識できる
- Project内DocumentをConsumer Panelから扱える
- Text Editor等のFramework Standard PanelへProject内Documentを渡せる
- Panel実装が特定の絶対パスへ依存しない
- Project / Resourceの意味はMeridian側が所有し、FrameworkへMuJoCo固有概念を持ち込まない

File Explorer、完全なImport / Export UI、Resource Inspector等はFramework側APIの進捗に応じて後続Stepで扱う。

## 11. 今回確定する基本原則

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
11. workflow-ide-frameworkはMeridian Applicationへ一体化される内部Frameworkとして扱い、Project DefinitionへFramework ID / versionを記録しない。
12. Project Definitionには対象Meridian Applicationを識別する情報を持たせる。
13. Projectの互換性はFramework単体ではなくMeridian Applicationとして扱う。
14. Project Definitionの汎用的な読み書き・Resource / Asset管理・相対path解決・Format migration等の機構はFramework側へ共通化できる。
15. Application versionのProject Definitionへの記録方法とProject Format互換性への利用方法は、Application側のversion方針と合わせて後続仕様で決定する。
16. Project directory内に存在するだけでは所属とせず、Resource / AssetはProject Definitionへ明示的に登録する。
17. Resource / Assetの論理分類と物理ディレクトリ構造を分離する。
18. Solution相当の上位Project集合概念は今回導入しない。
19. Meridian内部定義を情報量の多い正本とし、URDF / MJCFは主として交換・外部利用向け表現として扱う。
20. Meridian内部定義とURDF / MJCF等の交換形式はProject内で併存してよい。
21. Resourceの論理的な役割と表現形式を分離して管理できる構造とする。
22. URDF / MJCF等から参照されるmesh・texture等もProject Assetとして登録対象とする。
23. Import等によるResource / Asset登録はApplicationが自動化できる構造とし、利用者による全ファイルの手動登録を要求しない。
24. 不足Assetに対して代替Assetを生成する場合も、生成物をProject Definitionへ登録してProjectから認識可能にする。
25. Resource登録では、論理的役割・表現形式・物理パスを別の情報として扱う。
26. 同一論理対象についてMeridian内部定義・MJCF・URDF等の複数表現を関連付けられる構造とする。
27. Assetの参照関係をProject Definitionへ完全に二重記述することは必須とせず、URDF / MJCF等が持つ参照情報を利用できるようにする。
28. Asset自体のProject所属はProject Definitionへの登録によって管理する。
