mod urdf_io;
mod runtime_project;
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
    .run()
    .expect("failed to start workflow IDE framework");
}
