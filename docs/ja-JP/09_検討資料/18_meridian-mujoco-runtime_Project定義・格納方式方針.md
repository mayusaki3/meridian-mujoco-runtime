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

Projectの物理root構造はworkflow-ide-frameworkのProject仕様をそのまま使用する。Runtime独自のProject root構造は定義しない。

```text
<Project Root>/
├─ project.toml
├─ framework/
│  └─ framework_settings.toml
├─ resources/
└─ application/
```

保存済みProjectでは `project.toml` を必須とし、その他のdirectoryは必要時に作成する。

### project.toml

Visual StudioのProject fileに相当するProject Definitionとして、workflow-ide-frameworkの `project.toml` を使用する。Runtime独自の `meridian.toml` は設けない。

Frameworkは `project.toml`、`framework/`、Project lifecycleを所有する。RuntimeはFrameworkの公開Project契約を利用し、Framework所有dataの内部表現を重複管理しない。

Project directory内にfileが存在するだけではProject所属とはみなさず、FrameworkのResource Registryへ登録されたResourceをProject所属として扱う。

### Application と Project Format

Projectの永続化形式は、Framework所有領域とApplication所有領域で独立してversion管理する。

- `project.format_version`: Frameworkが所有・解釈するProject形式version
- `application.data_version`: Meridian Applicationが所有・解釈するApplication data形式version

Runtimeが組み込むFramework本体のversionと、Project永続化形式のversionは別に扱う。

Applicationが想定するFramework本体versionとの互換性確認はApplication起動時に行う。これはProject互換性判定ではなく、Applicationと組み込みFramework本体の組み合わせを確認するためのものとする。

Project Open時は、Frameworkが自身の管理する `project.format_version` をProjectごとに確認し、Framework管理dataについて必要なmigrationをFramework自身が担当する。Runtimeはこの判定・migrationへ直接関与しない。

続いてFrameworkは `application.data_version` をFramework Public API / Adapter契約を通してRuntimeへ渡す。RuntimeはRuntime管理dataを確認し、必要な互換性判断・migrationをRuntime自身が担当する。Runtimeが `project.toml` から値を直接取得することはしない。

Runtimeは `project.toml`、`framework/`、Resource Registryの永続化fileを直接read/writeせず、Framework Public API / Adapter契約だけを使用する。

Framework本体version、Runtime製品version、`project.format_version`、`application.data_version` を混同しない。永続化形式についてはFrameworkとRuntimeがそれぞれ自身の担当versionを1つ管理する。

Project作者が管理するProject自身のversion、author、license等を保持する必要性はあるが、現行Framework v0.1.0の `project.toml` 公開schemaにはこれらが含まれていない。格納先はFramework側との責務境界を確認して後続で決定し、現時点では独自fieldを追加しない。

現行Framework schemaでRuntimeが使用する部分の概念例:

```toml
[project]
format_version = 1
name = "KHR-3HV Walking Test"
description = "KHR-3HV walking simulation project"
language = "ja-JP"
save_id = "..."
saved_at = "..."

[application]
id = "meridian-mujoco-runtime"
name = "Meridian MuJoCo Runtime"
description = "..."
language = "ja-JP"
data_version = "1"
```

正式なApplication IDは後続仕様で決定する。

### framework/

Framework所有のProject stateを保存する領域とする。現行仕様では `framework/framework_settings.toml` を使用する。

Dock Layout等のFramework共通stateはFrameworkの責務であり、Runtime独自の `.meridian/` directoryは設けない。

### resources/

Project Scope Resourceの物理rootはFramework仕様に従い `<Project Root>/resources/` とする。

Robot、Actuator、Field、Scenario、Execution Profile、Meridian内部定義、URDF、MJCF、mesh、texture、Project固有入力data、保存対象result等、Projectに所属させるfileは用途に応じてここへ配置し、Resource Registryへ登録する。

Resourceの論理分類と物理directory構造は分離する。`resources/robots/`、`resources/assets/`、`resources/results/` 等の固定分類directoryをProject Format上の必須条件にはしない。

Meridian内部定義はURDF / MJCFより多くの情報を保持できるcanonical表現とし、URDF / MJCFは主として交換・外部利用向け表現として扱う方向とする。同一logical objectについて複数表現をProject内で併存させてよい。

mesh、texture等のAssetもFramework上ではResourceである。Assetであること、mesh / texture等の意味、生成物であることはRuntime固有metadataとして管理する。

URDF / MJCF等から参照される関連Assetも、Projectを成立させるdataとしてResource Registryへの登録対象とする。

Project固有の入力・計測dataやsimulation / sysid結果を保存対象とする場合も、独立したroot `data/` や `results/` をProject Formatとして要求せず、Project Resourceとして扱う。具体的なlogical roleと保存policyはRuntime側で定義する。

### application/

Runtime固有のProject dataを保存するApplication所有領域とする。Frameworkはこのdirectory内部を解釈しない。

RobotDefinition、ActuatorDefinition、FieldDefinition、ScenarioDefinition等のlogical object、Runtime固有Resource metadata、logical objectと `resource_id` の関係等を保存する。

RuntimeSessionの実行中動的状態はApplication Project dataとは分離する。保存対象となった結果だけをProject Resourceとして扱う。

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

## 11. Runtime Application data の永続化方針

Runtime所有dataは `application/` 以下へ保存する。初期段階では、domain objectごとに多数のfileへ分割せず、Project全体のRuntime固有定義を一つのmanifestへまとめる方式を第一候補とする。

```text
application/
└─ runtime_project.json
```

`runtime_project.json` はFrameworkのProject Definitionではなく、Runtimeが所有する `RuntimeProjectData` のserializationとする。Frameworkは内容を解釈しない。

JSONを初期候補とする理由は、Runtime domain dataがRobot / Actuator / Field / Scenario等の構造化dataを中心とし、開発中のschema確認、diff、test fixture作成を容易にするためである。将来、容量・性能・部分更新等の要件から形式やfile分割を変更する場合は `application.data_version` のmigration対象としてRuntimeが処理する。

初期構造の概念例:

```json
{
  "resource_metadata": {
    "res-main-robot": {
      "role": "robot",
      "format": "meridian",
      "logical_object_id": "robot-main",
      "representation": "canonical"
    }
  },
  "logical_objects": {
    "robots": [],
    "actuators": [],
    "fields": [],
    "scenarios": [],
    "execution_profiles": []
  },
  "relationships": []
}
```

### versionの扱い

`runtime_project.json` 内にFramework用versionやRuntime製品versionを重複保存しない。

Runtime Application dataの形式versionは `application.data_version` とし、その意味と互換性判断はRuntimeが所有する。永続化先はFrameworkが所有し、RuntimeはFramework Public API / Adapterから渡された値として扱う。Runtimeは `project.toml` を直接read/writeしない。Runtimeは実dataも検査し、stored `data_version` だけを根拠に互換性を決定しない。

## 10.5 SysID中心のActuator / Drive domainモデル

SysIDは共有形式からではなく、Project内での編集、Real / Sim双方への変換・実行、計測、同定、検証、履歴管理を中心に設計する。SysID固有の共有方式は現段階では定義せず、将来のProject Resource Export / Importで扱う。

SysIDの入口は次の2系統とする。

```text
Robot起点
  Robot -> 構成部品を選択 -> SysID Unitを編集 -> SysID Target

単体起点
  Motor / Servo等を登録 -> SysID Target
```

対象決定後の編集・Real実行・Sim実行・Identification・Validationは共通化する。

### Actuator Product

市販Motor / Servo等の製品としてのidentityと公称情報を表す。manufacturer、model、category、interface、nominal specification等を保持する候補とする。

ProductはRobotの特定実装位置を表さない。同一Productを複数Robot、複数箇所で使用できる。

### Actuator Instance

特定Robotまたは実験環境内に実装されたActuatorを表す。Productへの関係と、Robot内のslot / joint等との関係を持つ。

同一Productであっても個体差を扱えるよう、InstanceとProductを分離する。製造serial等の個体識別情報を必須にはしない。

### SysID Target / SysID Unit

SysID Targetは同定対象を一般化したlogical conceptとする。初期対象には少なくともMotor単体、Servo Motor単体、Robot内のMotor / Servo、Gear・Joint・Link等の機構を含む駆動装置を含める。

Robot起点では、Robotの構成部品境界をそのままSysID境界に固定せず、利用者が複数構成部品をSysID Unitとしてまとめられるようにする。これによりMotor単体、Servo assembly、Servo + reduction mechanism、Jointを含む駆動系等を異なる粒度で同定できる。

### SysID Definition

SysID DefinitionをSysIDの主たる編集状態とする。

候補要素は、Target / SysID Unit、動作pattern / excitation、input / output signal、Real実行設定、Sim実行設定、計測設定、既知の実装値、fixed / estimate parameter、初期値・制約、Identification設定、Validation設定、および現在採用している同定値とする。

DefinitionからReal側とSim側の実行表現へ変換する。Real用定義とSim用定義を独立して手編集することを基本形としない。

SysID Definition / SysID Unitは複数のInputと複数のOutputを保持できる構造とし、SISOだけを前提にしない。共通Runtimeは複数commandのReal / Sim双方への供給、複数measurementの収集、時系列対応、比較、履歴との関連付けを扱える境界を提供する。

一方、具体的なSISO / SIMO / MIMO同定algorithm、excitation pattern、評価関数、parameter推定方法、専用UIはSysID Panel / Calculation Model側の責務とする。共通基盤へ特定MIMO algorithmを固定しない。

### Measurement Dataset

実対象から取得した入力・出力・観測値等の計測結果はSysID Definitionへ埋め込まず、別file / Project Resourceとして保存する。

同じMeasurement Datasetに対して計算modelやIdentification設定を変更して再同定できるようにする。Datasetは取得時のSysID Definition、対象、実行条件を追跡できる関係を保持する。

### Calculation Model

Identificationで使用する計算modelを、Measurement Datasetや同定結果とは分離して識別できるようにする。

同一Datasetに複数Calculation Modelを適用した比較、または同一Calculation Modelを複数Datasetへ適用した比較を可能にする。Calculation Modelの具体的な表現形式とResource粒度は後続設計で確定する。

### SysID History / Result

同定結果は現在値だけを上書きして失わず、履歴として保持する。

各履歴entryは少なくとも、使用したMeasurement Dataset、Calculation Model、Identification設定、初期parameter、同定parameter、Simulation結果、Validation結果、および実行時条件を追跡できるようにする。

```text
SysID Definition
  └─ current result -> History entry

SysID History
  ├─ History A
  │   ├─ Measurement Dataset A
  │   ├─ Calculation Model A
  │   └─ Identified / Validation Result
  └─ History B
      ├─ Measurement Dataset A
      ├─ Calculation Model B
      └─ Identified / Validation Result
```

現在採用している同定値は、履歴を破壊して更新するのではなく、採用した履歴entryとの関係を保持する。これにより別modelで再同定した後でも以前の結果へ戻せるようにする。

Resultの適用範囲はProduct-level、Instance-level、Mechanism-level等を区別できるようにする。Product-level Resultは同一Actuator Productを使用するRobot内Instanceへ適用できることを基本ユースケースとするが、電源、制御mode、負荷、温度等の条件差を無視して無条件適用しない。

Mechanism-level ResultからMotor / Servo単体のProduct-level特性を自動的に取り出せるとはみなさない。機構由来の摩擦、backlash、compliance、load inertia等が結果へ含まれるためである。

### Resource化の基本方針

```text
Actuator Product Definition -> canonical Resource候補
SysID Definition            -> Resource候補
Measurement Dataset         -> Project Resource
Calculation Model           -> Resource候補
Simulation / Validation data-> Project Resource候補

Actuator Instance           -> Project composition側
SysID History relation      -> Runtime Application data候補
SysID Unit                  -> SysID Definitionから参照される構成
```

SysID Definition、Calculation Model、Historyをどのfile粒度で保存するかは後続schema設計で確定する。現段階では、計測実dataをDefinitionから分離し、同定履歴から使用DatasetとCalculation Modelを確実に追跡できることを優先する。

SysID固有のExport / Import packageや共有単位はここでは定義しない。必要な関連Resourceをまとめる方法は、Project ResourceのExport / Import設計で後続検討する。

### canonical内部表現とApplication dataの境界

Robot、Actuator、Field等のうち、独立したidentityを持ち、Project内で複数表現を持ち得るdomain定義のcanonical内部表現は、`runtime_project.json` へ本体を埋め込まず、Project Resourceとして分離する方針とする。

Framework Resource Registryへ登録されたcanonical Resourceを正規定義の実体とし、Runtime Application dataはstable `resource_id` を使ってlogical objectとの関係を保持する。同じlogical objectへMJCF、URDF等のexchange / derived表現を追加しても、logical object identityとcanonical定義を分離したまま管理できる。

この分離により、canonical定義もProject所属ResourceとしてMissing / Locate / Resource Operation等のFramework機構を利用でき、将来の選択的な再利用・Import / Export対象として扱える。canonical Resourceの物理directoryは固定せず、Framework Resource Registryを所属・pathの正本とする。

一方、Project内の組み合わせや実行構成だけに意味を持つ小規模な定義は、必ずしも個別Resourceへ分割しない。Scenario、Execution Profile、logical object間relation等については、独立再利用性、外部交換、file単位操作の必要性が生じるまではRuntime Application dataへ保持できるものとする。

初期分類基準は次の通りとする。

- 独立identityを持ち、単体で再利用・交換・複数表現管理の対象となるdomain定義: canonical Project Resource
- 現在のProject内で複数Resource / logical objectを組み合わせる構成情報: Runtime Application data
- simulation / sysid等の保存実data: Project Resource
- RuntimeSession、cache、temporary build output等の実行時状態: Project永続domain定義には含めない

この分類は「domain typeごとに必ずfileを1つ作る」という意味ではない。どのdomain typeを独立Resourceとするか、およびcanonical内部表現のfile format / schema / extensionは後続仕様で確定する。

### canonical内部表現の初期file形式

canonical内部表現は、URDF / MJCFを正本として拡張するのではなく、Runtime固有の構造化dataとして保持する。Robotのactuator / sensor slot、Actuator Productのcapability / nominal specification、FieldのRuntime固有属性等、URDF / MJCFだけでは保持できない情報を欠落なく表現できることを優先する。SysID Setup、Measurement Dataset、SysID ResultはActuator Product定義へ埋め込まず、それぞれ再利用・共有可能な成果物として分離する。

初期file形式はJSONを第一候補とする。理由は、現在のdomain data案が階層化された構造dataを中心としていること、開発中にschema・diff・test fixtureを確認しやすいこと、URDF / MJCF等との変換処理で中間構造を扱いやすいことによる。binary形式は初期canonical形式には採用しない。

ただし、canonical Resource全体を一つの巨大fileへまとめる方式は採らない。独立Resourceと判断されたRobot、Actuator、Field等は、それぞれのcanonical定義を個別Resourceとして保存できる構造とする。

初期段階では独自拡張子を必須化せず、通常の `.json` を使用する方向とする。Meridian独自拡張子は、OS file association、誤編集防止、交換package識別等の具体的要件が生じた場合に別途検討する。拡張子だけを変更したJSONや独自packageを先に定義しない。

canonical JSON内にはFrameworkの `resource_id`、filesystem path、Project ID等のFramework所有identityを正本として埋め込まない。canonical定義自身のdomain identityとdomain内容を保持し、Project内でのResource membershipとlogical objectとの関連付けはFramework Resource RegistryおよびRuntime Application dataが担当する。

canonical JSONのschema versionを別途追加するかは後続仕様で決定する。Application全体のmigration入口は `application.data_version` がRuntime所有versionとして存在するため、domain Resourceごとに独立versionを増やす必要性を先に確認する。versionを追加する場合も、担当責務ごとに一つというversion方針を崩さない。

### Resource参照

Runtime Application dataからProject Resourceを参照する場合はstable `resource_id` を使用し、`resources/` からのfilesystem pathをRuntime側へ重複保存しない。

Resourceのpath、Scope、Missing等はFramework Resource Registryを正本とする。RuntimeはRegistryの永続化fileへ直接アクセスせず、Framework Public APIからResource情報を取得・操作する。

### 保存対象としないもの

実行中だけ存在するRuntimeSessionの動的状態、cache、一時build生成物は `runtime_project.json` の永続domain definitionへ含めない。

simulation / sysid結果等を利用者がProjectへ保存する場合、その実dataはProject Resourceとして登録し、Runtime Application dataには必要なlogical relation / metadataのみを保持する。

### Save / Open

Save / Save As / OpenはFrameworkのApplication Project Adapter境界を使用する。

RuntimeはApplication dataのserialization、compatibility、migration、consistency検査を担当する。Frameworkは `application/` 内部をblind copyまたは解釈しない。

`runtime_project.json` のatomic write方法、temporary file命名、障害回復の詳細は実装仕様で決定する。Project全体のsave順序はFrameworkのProject Save契約に従う。

### 未決事項

次は実装前に別途確定する。

- `runtime_project.json` というfile名の正式採用
- JSON schemaの正式なfield名・必須/optional
- logical object IDの生成規則
- relationship表現の具体schema
- Project作者のversion / authors / license metadataの所有先
- canonical JSONの正式schema、必須/optional field、domain identity規則
- canonical Resource個別のschema versionが必要かどうか
- 保存対象result / datasetのmetadata schema

## 11.1 内部library / domainの命名と責務分離

MeridianはProject / Application / 規格を表す名称として使用し、内部domain modelや汎用libraryの名称には原則として対象・責務を表す一般名を使用する。内部構造をMeridian固有名称へ不必要に結合しない。

初期の責務分離候補は次の通りとする。crate名は責務を示す仮称であり、Rust workspace実装時に正式決定する。

```text
robot-model
  RobotDefinition
  Link / Joint / Component
  ActuatorDefinition / SensorDefinition
  Connection

robot-model-import
  URDF -> RobotDefinition
  MJCF -> Robot / Actuator等の内部Definition

system-identification
  SysID Definition / Target / Unit
  Input / Output
  Measurement Dataset
  Calculation Model interface
  Identification / Validation
  History / Result

mujoco-adapter
  Robot / SysID等のcanonical内部Definition
    -> MuJoCo実行表現

runtime
  Real execution
  Simulation execution
  orchestration
```

`robot-model` はURDF / MJCF / MuJoCo / Meridianのいずれかをそのままdomain modelとせず、Robot構造・構成部品・接続を表現する中立なmodelとする。URDF / MJCFはImport元およびExport先となる外部表現であり、Import後は内部Definitionを編集上の正本とする。

この境界は将来Open MeriForma等のRobotを扱う場合にも維持する。Open MeriForma固有のController、Forma Unit、通信経路、Actuator、Sensor等を表現・関連付ける必要が生じても、URDF構造そのものを拡張して対応することを前提としない。必要な一般domain概念を `robot-model` 側へ追加し、特定Robot / 規格からの変換はImporter / Adapter側で扱う。

SysIDもUIから分離する。`system-identification` はSysID Definition、Measurement、Calculation Modelとの境界、Identification / Validation、Result / History等を扱うUI非依存libraryとし、workflow-ide-frameworkのWorkspace / Panel型へ依存させない。SysID PanelはApplication側からこのlibraryのPublic APIを利用する。

同様にMuJoCoへの展開は `mujoco-adapter` の責務とし、canonical内部Definitionや同定済みparameterをMuJoCo実行表現へ変換する。SysID Result自体をMJCFそのものとして正本管理しない。

依存方向は、基礎domainがApplication / UI / Frameworkへ依存しない方向を基本とする。

```text
external formats / Robot-specific definitions
             |
             v
      importer / adapter
             |
             v
         robot-model
             ^
             |
   system-identification
             |
             v
       mujoco-adapter
             |
             v
           runtime
             ^
             |
    Meridian Application
             ^
             |
   workflow-ide-framework
   (Application integration)
```

具体的なcrate分割、Rust trait境界、Python Calculation Modelとの接続方式、serialization schemaは実装仕様で確定する。JSON形式をRust structの単純なserialization結果として先に固定せず、domain modelと責務境界を確定してから永続化schemaを定義する。

## 12. Step 5との関係

workflow-ide-framework Step 5では、このProject構造の全機能を実装しない。

Step 5 Consumer実装で必要になる範囲として、少なくとも次を想定する。

- ProjectというApplication側の作業単位を認識できる
- Project内DocumentをConsumer Panelから扱える
- Text Editor等のFramework Standard PanelへProject内Documentを渡せる
- Panel実装が特定の絶対パスへ依存しない
- Project / Resourceの意味はMeridian側が所有し、FrameworkへMuJoCo固有概念を持ち込まない

File Explorer、完全なImport / Export UI、Resource Inspector等はFramework側APIの進捗に応じて後続Stepで扱う。

## 13. 今回確定する基本原則

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
13. Applicationが想定するFramework本体versionとの互換性確認はApplication起動時に行い、Project互換性判定とは分離する。RuntimeはProject Open時のFramework本体version判定を行わない。
14. Project Open時、Frameworkは `project.format_version` を確認してFramework管理dataの互換性判断・migrationを担当し、その後Application所有data形式は `application.data_version` でApplicationだけが解釈・互換性判断・migrationを担当する。
15. Frameworkは `application.data_version` の意味を解釈せず、Applicationへ渡す。
16. Framework version、Application製品version、永続化形式versionを分離し、各所有者は担当する永続化形式について1つのversionを管理する。
17. Project Definitionには対象Meridian Applicationを識別するstable Application IDを持たせる。
18. Project作者が管理するProject自身のversionは、Project形式・Application data形式のversionとは別のmetadataとする。
19. Project directory内に存在するだけでは所属とせず、Project所属fileはFramework Resource Registryへ明示的に登録する。
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
39. Project rootの物理構造はFramework仕様の `project.toml / framework / resources / application` を使用し、Runtime独自の `assets / data / results / .meridian` rootを要求しない。
40. Project固有入力dataや保存対象resultも、Project所属fileである場合はFramework Resource Registryへ登録し、用途上の意味はRuntime側で管理する。
41. Runtime Application dataは `application/` 以下へ保存し、初期永続化は単一JSON manifestを第一候補とする。
42. Application data形式の互換性versionは `application.data_version` のみを使用し、Runtime製品versionやFramework versionを重複してdata format判定へ使用しない。
43. Runtime Application dataからProject Resourceを参照するときはstable `resource_id` を使用し、filesystem pathを重複保存しない。
43a. 独立identityを持ち、再利用・交換・複数表現管理の対象となるcanonical domain定義はProject Resourceとして分離し、Runtime Application dataへ定義本体を重複保存しない。
43b. Scenario、Execution Profile、relation等のProject内構成情報は、独立Resourceとして扱う必要が生じるまではRuntime Application dataへ保持できる。
43c. canonical内部表現の初期file形式は通常のJSONを第一候補とし、独自拡張子・binary形式・独自packageを初期要件としない。
43d. canonical ResourceにはFramework所有のresource_id、filesystem path、Project IDを正本として重複保存しない。
43e. SysIDはRobot起点とMotor / Servo等の単体登録起点の2系統を持ち、対象決定後の編集・Real / Sim実行・Identification・Validationを共通化する。
43f. Actuator ProductとRobot内Actuator Instanceを分離し、Product-level SysID Resultを同一ProductのInstanceへ再利用可能にする。
43g. SysID Definitionを主たる編集状態とし、Real / Sim双方の実行表現はDefinitionから変換する。
43h. Measurement DatasetはDefinitionとは別file / Project Resourceとして保存する。
43i. SysID結果は履歴として保持し、各履歴から使用Measurement Dataset、Calculation Model、Identification設定、Simulation / Validation結果を追跡可能にする。
43j. SysID ResultはProduct-level、Instance-level、Mechanism-level等の適用範囲と適用条件を保持し、Product identity一致だけで無条件適用しない。
43k. SysID固有の共有packageは現段階では定義せず、将来のProject Resource Export / Importで扱う。
43l. SysID共通基盤はSysID Unitごとに複数Input / Outputを扱える実行・計測・比較・履歴の枠組みを提供し、SISOだけを前提にしない。
43m. 具体的なSISO / SIMO / MIMO同定algorithm、excitation、評価・parameter推定、専用UIはSysID Panel / Calculation Model側で定義可能とし、共通基盤へ特定MIMO algorithmを固定しない。
43n. Meridian名称はProject / Application / 規格レベルで使用し、内部domain model / libraryは対象・責務を表す汎用名称を基本とする。
43o. Robotのcanonical内部modelはURDF / MJCF / MuJoCo / Meridian固有構造をそのままdomain modelとせず、将来Open MeriForma等を扱える中立なRobot domainとして設計する。
43p. SysID domain libraryはworkflow-ide-frameworkのWorkspace / Panel等へ依存させず、Application側UIからPublic APIを利用する。
43q. MuJoCoへの展開はadapter境界で行い、canonical内部Definitionおよび同定済みparameterからMuJoCo実行表現を生成する。
44. RuntimeはFramework所有の `project.toml`、`framework/`、Resource Registry永続化fileへ直接アクセスせず、Framework Public API / Adapter契約だけを使用する。
45. `application.data_version` はRuntimeが意味を所有するが、Framework所有Project fileから直接取得・更新せず、FrameworkとのAPI境界を通して受け渡す。
