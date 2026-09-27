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
├─ project.toml
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

### project.toml

Visual Studio の Project file に相当するProject Definitionとして、workflow-ide-frameworkの `project.toml` を使用する。Runtime独自の `meridian.toml` は設けない。

Project DefinitionのFramework共通metadata、Application識別、Project lifecycle、Resource RegistryはFrameworkの公開契約を使用する。

Runtime固有のRobot、Actuator、Field、Scenario等のdomain dataはApplication所有領域へ保存し、Frameworkはその内容を解釈しない。

Project directory内にファイルが存在するだけではProject所属とはみなさず、FrameworkのResource Registryへ登録されたResourceをProject所属として扱う。

Project Definitionに個々のRobotやField等の詳細情報をすべて集約することは目的としない。

### Application と Project Format

`workflow-ide-framework`はMeridian Applicationへ一体化される内部Frameworkとして扱うため、Project DefinitionへFramework IDやFramework versionを記録しない。

Projectの永続化形式は、Framework所有領域とApplication所有領域で独立してversion管理する。

- `project.format_version`: Frameworkが所有・解釈するProject形式version
- `application.data_version`: Meridian Applicationが所有・解釈するApplication data形式version

Runtimeは、Projectを作成・保存したworkflow-ide-framework自体のversionを意識せず、Framework所有領域の互換性判断・migrationはFrameworkへ委ねる。これによりRuntimeが利用するFramework versionが更新されても、Runtime側へFramework Project形式の互換性処理を持ち込まない。

逆にFrameworkは`application.data_version`の意味を解釈しない。保存値をApplicationへ渡し、Applicationが実データを確認した上で互換性・変換可否・整合性を判断する。Application dataのmigrationはApplication側が所有する。

Framework versionとApplication製品versionは、これらの永続化形式versionとは分離する。各所有者は、自身が担当する永続化形式について1つのversionを管理する。

Project作者が管理するProject自身のversionは、Project形式やApplication data形式のversionとは別のProject metadataとして扱う。

概念例:

```toml
[project]
format_version = 1
id = "khr3hv-walking-test"
name = "KHR-3HV Walking Test"
version = "0.1.0"
description = "KHR-3HV walking simulation project"
authors = ["..."]
license = "..."

[application]
id = "meridian-mujoco-runtime"
data_version = "1"
```

具体的なschema、正式なApplication ID、Project Definitionのファイル名・拡張子はworkflow-ide-frameworkの確定仕様との整合を確認しながら後続仕様で決定する。

### resources

Projectを構成する意味のある定義を扱う。

Robot、Actuator、Field、Scenario、Execution Profile、URDF、MJCF等を想定する。

Resourceの論理分類と物理ディレクトリ構造は分離する。Resourceを特定の `resources/<分類>/` 配下へ置くことをProject Format上の必須条件にはしない。

Meridian内部定義は、URDF / MJCFより多くの情報を保持できる正本として扱う方向とする。

URDF / MJCFは主としてデータ交換、およびMuJoCo等の外部系へ渡す表現形式として位置付ける。ただし利用者から見ればProjectを構成するデータであるため、Meridian内部定義とURDF / MJCFをProject内で併存させてよい。

Project Resourceはworkflow-ide-frameworkのResource Registryへ登録されたものをProject所属として扱う。

Framework共通のRegistry entryはstable `resource_id` と `ResourceReference` を持ち、Project ScopeではFrameworkが管理するProject Resource Rootからの相対pathを使用する。

Robot / Field等の「論理的な役割」、Meridian内部形式 / URDF / MJCF等の「表現形式」、同一論理対象間の関係はRuntime固有metadataとしてApplication側が所有する。FrameworkのResourceEntryへMuJoCo固有schemaを追加することを前提としない。

### assets

mesh、texture等、Resourceから参照される補助ファイルを扱う。

Assetの論理分類と物理ディレクトリ構造は分離する。既存URDF / MJCF等の相対参照関係を維持できるよう、Assetを特定の `assets/` 配下へ移動することをProject Format上の必須条件にはしない。

AssetもFramework上ではProject ResourceとしてResource Registryへ登録し、Assetであることやmesh / texture等の意味はRuntime側で管理する。

URDF / MJCF等から参照されるmeshやtextureも、Projectを成立させるデータとしてResource Registryへの登録対象とする。

Import時などに交換形式から参照されるAssetを検出した場合、利用者が一つずつ登録することを必須とせず、ApplicationがProjectへの取り込みと登録を自動化できる構造とする。

必要なAssetが存在しない場合は、エラーとして扱うだけでなく、用途に応じてダミーmesh等の代替Assetを生成してProjectを成立させる補助機能も検討する。生成した代替Assetを使用する場合もResource Registryへ登録し、Projectから認識可能な状態とする。

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

Import時にはURDF / MJCFから参照されるmesh等の関連Assetを検出し、Projectへ取り込んだものをResource Registryへ登録することを基本とする。

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

既存要件の「MuJoCoデータセット」は、MJCF、URDF、mesh、Runtimeデータ等をひとまとまりの流通・選択単位として扱う初期構想である。

現在のProject / Resourceモデルでは、この概念を単一の物理folderや独自packageとしてProjectの基礎単位にはしない。役割を次のように分解する。

```text
Meridian Project
  ├─ Framework Resource Registry
  │   ├─ MJCF Resource
  │   ├─ URDF Resource
  │   ├─ mesh / texture Resource
  │   └─ その他のProject Resource
  │
  └─ RuntimeProjectData
      ├─ Robot / Field / Actuator等のlogical object
      ├─ ResourceのRuntime固有metadata
      └─ logical objectとResourceの関係
```

従来「MuJoCoデータセット」が担っていた機能は、次の責務へ読み替える方向とする。

- Project内で使用するfileの所属・path・存在状態: Framework Resource Registry
- Robot / Field / Actuator等としての意味: RuntimeProjectData
- MJCF / URDF / mesh等の表現形式: Runtime側Resource metadata
- 複数Resourceの組み合わせ: Scenario等のRuntime domain definition
- MuJoCo実行用MJCF / Runtime dataの生成: Runtimeのbuild処理
- 外部との交換: URDF / MJCFおよび必要な関連fileのImport / Export

したがって、既存要件の「カテゴリ毎にMuJoCoデータセットを選択し、複数データセットからRuntime用データを構築する」という利用目的は維持できるが、その選択対象はfolder/packageそのものではなく、Project内のlogical object / Resourceへ再定義する方向とする。

既存要件にある `Identification.json` は、Project所属管理やResource識別の正本としては使用しない。Project所属とResource identityはFramework Resource Registry、domain上の識別・説明・関係はRuntimeProjectDataが担当する。交換元fileとして既存データに含まれる場合の扱いはImport仕様で別途決定する。

また、既存要件の「folderごとzip圧縮し拡張子をrenameする独自Export / Import形式」は現在の方針とは一致しない。今回の初期Export / Importでは独自packageを定義せず、URDF / MJCFと必要な関連fileを対象とする。

この整理を要件定義へ反映する際は、「MuJoCoデータセット」という用語を単純置換するのではなく、利用者が必要とする選択・組み合わせ・Runtime build・交換能力をProject / Resource / RuntimeProjectDataへ分解して要件を書き直す。

## 9. Framework Resource Registry と Runtime domain metadata

Project所属ファイルの登録はworkflow-ide-frameworkのResource Registryを使用し、Runtime独自のResource / Asset registryを重複して設けない。

Framework共通entryは概念的に次の情報を持つ。

```text
ResourceEntry
├─ resource_id
└─ ResourceReference
   ├─ scope
   └─ path
```

Project内のRobot、MJCF、URDF、mesh等はいずれもFrameworkから見ればResourceである。

一方、次の情報はMuJoCo / Runtime domainの意味であるためApplication側が所有する。

- Robot、Field、Actuator、Scenario、Asset等の論理的役割
- Meridian内部形式、URDF、MJCF、STL、OBJ等の表現形式
- 同一論理対象のcanonical / derived / exchange表現の関係
- Application生成Assetであること
- Resource間のdomain依存関係

Runtime側はFrameworkのstable `resource_id` を参照してこれらのdomain metadataを関連付ける。具体的なApplication data schemaは後続仕様で決定する。

この分離により、FrameworkのResource RegistryをProject所属とfilesystem整合性の正本とし、Runtime側はResourceの意味だけを管理する。

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

一方、参照先Asset自体はResource Registryへ登録する。

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

不足時にダミーAssetを生成する場合は、その生成物をProjectへ追加し、Resource Registryにも登録する。

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

利用者からは一つのImport操作として扱えることを目標とし、関連AssetのResource Registry登録を手作業で要求しない。

## 10. Runtime Application data の最小構造

workflow-ide-frameworkがProject lifecycleとResource Registryを所有するため、Runtime側のApplication dataはProject filesystemの再管理ではなく、Runtime domainの意味とResource間関係を保持する。

初期の最小構造は次を候補とする。

```text
RuntimeProjectData
├─ resource_metadata
│  └─ resource_id -> RuntimeResourceMetadata
├─ logical_objects
│  ├─ RobotDefinition
│  ├─ ActuatorDefinition
│  ├─ FieldDefinition
│  ├─ ScenarioDefinition
│  └─ ExecutionProfile
└─ relationships
   └─ logical object / resource_id 間の関係
```

`RuntimeProjectData` はFrameworkのProjectそのものを表す別Project objectではなく、Frameworkの `application/` 領域に保存するRuntime固有dataのrootとする。

### Resource metadata

Runtime固有metadataはFrameworkのstable `resource_id` をkeyとしてResource Registry entryへ関連付ける。

概念例:

```json
{
  "resource_metadata": {
    "res-main-robot": {
      "role": "robot",
      "format": "meridian",
      "logical_object_id": "robot-main",
      "representation": "canonical"
    },
    "res-main-robot-mjcf": {
      "role": "robot",
      "format": "mjcf",
      "logical_object_id": "robot-main",
      "representation": "exchange"
    },
    "res-body-mesh": {
      "role": "asset",
      "asset_type": "mesh",
      "format": "stl",
      "generated": false
    }
  }
}
```

Runtime側metadataにはfilesystem pathを重複保存しない。path、Scope、Missing等のfilesystem上の状態はFrameworkのResource Registry / ResourceReferenceを参照する。

### Logical object

RobotDefinition等のdomain objectはResource fileそのものと同一視しない。一つのLogical RobotにMeridian内部表現、MJCF、URDF等の複数Resourceを関連付けられる。

```text
RobotDefinition: robot-main
  ├─ res-main-robot       : canonical / meridian
  ├─ res-main-robot-mjcf  : exchange / mjcf
  └─ res-main-robot-urdf  : exchange / urdf
```

これにより交換形式を追加・再生成してもRobot identityを維持できる。

### 既存データ構造案との関係

検討資料08の `RuntimeProject` は、Framework導入前のProject root object案であるため、そのままProject所有者として採用しない。

一方、`RobotDefinition`、`ActuatorDefinition`、`FieldDefinition`、`ScenarioDefinition`、`ExecutionProfile`等のdomain分離方針は継続候補とする。

検討資料08にある `model_uri` 等のfilesystem path直接保持は、Framework Resource Registryとの重複を避けるため、原則としてstable `resource_id` 参照へ置き換える方向とする。

`RuntimeSession` は実行時状態であり、RuntimeProjectDataの永続domain definitionとは分離する。保存対象となる結果・記録はProjectの結果dataとして別途扱う。

### Application data_version

このApplication data構造の互換性はRuntimeが `application.data_version` で管理する。

`data_version` はRuntime製品versionではなく、Runtime所有Application dataの形式versionである。Frameworkは値を保存・受け渡しするが意味を解釈しない。

## 11. Step 5との関係

workflow-ide-framework Step 5では、このProject構造の全機能を実装しない。

Step 5 Consumer実装で必要になる範囲として、少なくとも次を想定する。

- ProjectというApplication側の作業単位を認識できる
- Project内DocumentをConsumer Panelから扱える
- Text Editor等のFramework Standard PanelへProject内Documentを渡せる
- Panel実装が特定の絶対パスへ依存しない
- Project / Resourceの意味はMeridian側が所有し、FrameworkへMuJoCo固有概念を持ち込まない

File Explorer、完全なImport / Export UI、Resource Inspector等はFramework側APIの進捗に応じて後続Stepで扱う。

## 12. 今回確定する基本原則

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
12. Framework所有Project形式は `project.format_version` で管理し、Frameworkだけが解釈・互換性判断・migrationを担当する。
13. Runtimeは利用しているFramework versionをProject互換性のために意識しない。
14. Application所有data形式は `application.data_version` で管理し、Applicationだけが解釈・互換性判断・migrationを担当する。
15. Frameworkは `application.data_version` の意味を解釈せず、Applicationへ渡す。
16. Framework version、Application製品version、永続化形式versionを分離し、各所有者は担当する永続化形式について1つのversionを管理する。
17. Project Definitionには対象Meridian Applicationを識別するstable Application IDを持たせる。
18. Project作者が管理するProject自身のversionは、Project形式・Application data形式のversionとは別のmetadataとする。
19. Project directory内に存在するだけでは所属とせず、Resource / AssetはProject Definitionへ明示的に登録する。
20. Resource / Assetの論理分類と物理ディレクトリ構造を分離する。
21. Solution相当の上位Project集合概念は今回導入しない。
22. Meridian内部定義を情報量の多い正本とし、URDF / MJCFは主として交換・外部利用向け表現として扱う。
23. Meridian内部定義とURDF / MJCF等の交換形式はProject内で併存してよい。
24. Resourceの論理的な役割と表現形式を分離して管理できる構造とする。
25. URDF / MJCF等から参照されるmesh・texture等もProject Assetとして登録対象とする。
26. Import等によるResource / Asset登録はApplicationが自動化できる構造とし、利用者による全ファイルの手動登録を要求しない。
27. 不足Assetに対して代替Assetを生成する場合も、生成物をResource Registryへ登録してProjectから認識可能にする。
28. Resource登録では、論理的役割・表現形式・物理パスを別の情報として扱う。
29. 同一論理対象についてMeridian内部定義・MJCF・URDF等の複数表現を関連付けられる構造とする。
30. Assetの参照関係をProject Definitionへ完全に二重記述することは必須とせず、URDF / MJCF等が持つ参照情報を利用できるようにする。
31. Asset自体のProject所属はResource Registryへの登録によって管理する。
32. Runtime Application dataはFramework Projectとは別のProject objectを持たず、Frameworkの `application/` 領域にRuntime固有domain dataを保持する。
33. Runtime固有Resource metadataはFrameworkのstable `resource_id` に関連付け、filesystem pathを重複して正本管理しない。
34. Robot等のlogical object identityと、そのMeridian内部形式・MJCF・URDF等のResource表現を分離する。
35. 検討資料08のdomain分離方針は継続候補とするが、`RuntimeProject` とpath直接保持はFramework Project / Resource Registryへ合わせて再構成する。
36. 既存の「MuJoCoデータセット」は単一folder/packageをProject基礎単位とせず、所属管理・domain意味・Resource表現・組み合わせ・Runtime buildへ責務を分解する。
37. `Identification.json` をProject所属やResource identityの正本にはせず、Framework Resource RegistryとRuntimeProjectDataを正本とする。
38. 既存要件の独自zip派生packageは初期Export / Import方式として採用せず、URDF / MJCFと必要な関連fileを対象とする。
