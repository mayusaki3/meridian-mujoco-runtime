mod urdf_io;
mod runtime_project;
mod mjcf_project;
#[cfg(test)]
mod urdf_fixture_tests;

use std::{path::PathBuf, sync::{Arc, Mutex}};
use workflow_ide_framework::{
    command::{CommandDefinition, MenuItemDefinition},
    project::ProjectContext,
    project_event::ProjectEventKind,
};

fn main() {
    let current_project = Arc::new(Mutex::new(None::<PathBuf>));
    let import_project = Arc::clone(&current_project);
    let export_project = Arc::clone(&current_project);
    let event_project = Arc::clone(&current_project);
    let mjcf_import_project = Arc::clone(&current_project);
    let mjcf_reimport_project = Arc::clone(&current_project);

    workflow_ide_framework::Application::new(
        "meridian-mujoco-runtime",
        "Meridian MuJoCo Runtime",
    )
    .project_adapter(runtime_project::RuntimeProjectAdapter)
    .on_project_event(move |event| {
        if let Ok(mut active) = event_project.lock() {
            match event.kind {
                ProjectEventKind::Created | ProjectEventKind::Opened | ProjectEventKind::Saved | ProjectEventKind::SavedAs => {
                    if let Some(root) = &event.root {
                        *active = Some(root.clone());
                    }
                }
                ProjectEventKind::Closed => *active = None,
                ProjectEventKind::CloseRequested => {}
            }
        }
    })
    .command(CommandDefinition::new("runtime.urdf.import", "URDFインポート...", move || {
        let root = import_project.lock().ok().and_then(|active| active.clone());
        let Some(root) = root else {
            eprintln!("URDFインポート: プロジェクトを開いてください");
            return;
        };
        let Some(path) = rfd::FileDialog::new().add_filter("URDF", &["urdf"]).pick_file() else {
            return;
        };
        let result = std::fs::read_to_string(&path).and_then(|source| {
            runtime_project::RuntimeProjectAdapter::import_source(&ProjectContext::new(root), &source)
        });
        match result {
            Ok(()) => eprintln!("URDFインポート完了: {}", path.display()),
            Err(error) => eprintln!("URDFインポート失敗: {error}"),
        }
    }))
    .menu_item(MenuItemDefinition::custom("data", "データ", "runtime.urdf.import"))
    .command(CommandDefinition::new("runtime.urdf.export", "URDFエクスポート...", move || {
        let root = export_project.lock().ok().and_then(|active| active.clone());
        let Some(root) = root else {
            eprintln!("URDFエクスポート: プロジェクトを開いてください");
            return;
        };
        let imported = match runtime_project::RuntimeProjectAdapter::load_source(&ProjectContext::new(root)) {
            Ok(Some(model)) => model,
            Ok(None) => {
                eprintln!("URDFエクスポート: インポート済みURDFがありません");
                return;
            }
            Err(error) => {
                eprintln!("URDFエクスポート失敗: {error}");
                return;
            }
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("URDF", &["urdf"])
            .set_file_name("robot.urdf")
            .save_file() else { return; };
        match urdf_io::export_unchanged_urdf(&imported, &path) {
            Ok(()) => eprintln!("URDFエクスポート完了: {}", path.display()),
            Err(error) => eprintln!("URDFエクスポート失敗: {error}"),
        }
    }))
    .menu_item(MenuItemDefinition::custom("data", "データ", "runtime.urdf.export"))
    .command(CommandDefinition::new("runtime.mjcf.import", "MJCFインポート...", move || {
        let root = mjcf_import_project.lock().ok().and_then(|active| active.clone());
        let Some(root) = root else {
            eprintln!("MJCFインポート: プロジェクトを開いてください");
            return;
        };
        let context = ProjectContext::new(root);
        match mjcf_project::load_snapshot(&context) {
            Ok(Some(_)) => {
                eprintln!("MJCFインポート: 既存MJCFがあります。MJCF再インポートを使用してください");
                return;
            }
            Ok(None) => {}
            Err(error) => {
                eprintln!("MJCFインポート: 保存済みデータを確認できません: {error}");
                return;
            }
        }
        let Some(path) = rfd::FileDialog::new().add_filter("MJCF", &["xml", "mjcf"]).pick_file() else {
            return;
        };
        let Some(source_root) = path.parent() else {
            eprintln!("MJCFインポート: ソースルートを特定できません");
            return;
        };
        match mjcf_project::import_snapshot(&context, &path, source_root) {
            Ok(snapshot) => eprintln!(
                "MJCFインポート完了: {} (body={}, joint={}, actuator={})",
                path.display(),
                snapshot.bundle.model.bodies.len(),
                snapshot.bundle.model.joints.len(),
                snapshot.bundle.model.actuators.len()
            ),
            Err(error) => eprintln!("MJCFインポート失敗: {error}"),
        }
    }))
    .menu_item(MenuItemDefinition::custom("data", "データ", "runtime.mjcf.import"))
    .command(CommandDefinition::new("runtime.mjcf.reimport", "MJCF再インポート...", move || {
        let root = mjcf_reimport_project.lock().ok().and_then(|active| active.clone());
        let Some(root) = root else {
            eprintln!("MJCF再インポート: プロジェクトを開いてください");
            return;
        };
        let context = ProjectContext::new(root);
        let previous = match mjcf_project::load_snapshot(&context) {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => {
                eprintln!("MJCF再インポート: インポート済みMJCFがありません");
                return;
            }
            Err(error) => {
                eprintln!("MJCF再インポート: 保存済みデータを読み込めません: {error}");
                return;
            }
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("MJCF", &["xml", "mjcf"])
            .set_file_name(previous.bundle.entry_path.file_name().unwrap_or_default().to_string_lossy().as_ref())
            .pick_file() else { return; };
        let Some(source_root) = path.parent() else {
            eprintln!("MJCF再インポート: ソースルートを特定できません");
            return;
        };
        match mjcf_project::reimport_snapshot(&context, &path, source_root) {
            Ok(result) => eprintln!(
                "MJCF再インポート完了: {} (UUID継承={}, 新規・未照合={})",
                path.display(), result.plan.reuse.len(), result.plan.unresolved.len()
            ),
            Err(error) => eprintln!("MJCF再インポート失敗: {error}"),
        }
    }))
    .menu_item(MenuItemDefinition::custom("data", "データ", "runtime.mjcf.reimport"))
    .run()
    .expect("failed to start workflow IDE framework");
}
