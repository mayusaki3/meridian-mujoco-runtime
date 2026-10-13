mod urdf_io;
mod runtime_project;
mod mjcf_project;
mod localized_errors;
#[cfg(test)]
mod urdf_fixture_tests;

use std::{path::PathBuf, sync::{Arc, Mutex}};
use workflow_ide_framework::{
    command::{CommandDefinition, MenuItemDefinition},
    project::ProjectContext,
    project_event::ProjectEventKind,
    project_panel::{ProjectPanelModel, ProjectPanelNode},
    resource_registry::ResourceRegistry,
    PanelDefinition, PanelKind, LayoutConfig,
};

fn main() {
    let project_tree = ProjectPanelModel {
        roots: vec![ProjectPanelNode::logical("project", "Meridian")],
        framework_resources: vec![],
        application_resources: vec![],
    };
    let project_tree = project_tree.tree(&ResourceRegistry::default());
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
    .localization_resources("resources/locales")
    .project_adapter(runtime_project::RuntimeProjectAdapter)
    .panel(PanelDefinition::new("project", "meridian.panel.project", PanelKind::StandardUi))
    .panel(PanelDefinition::new("logs", "meridian.panel.logs", PanelKind::StandardUi))
    .layout(LayoutConfig::new(["project"]).split_below("project", 0.70, ["logs"]).selected("project"))
    .tree_viewer_panel("project", project_tree)
    .log_viewer_panel("logs")
    .language_settings_panel()
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
            workflow_ide_framework::tracing::warn!(target: "meridian::urdf", "URDFインポート: プロジェクトを開いてください");
            return;
        };
        let Some(path) = rfd::FileDialog::new().add_filter("URDF", &["urdf"]).pick_file() else {
            return;
        };
        let result = std::fs::read_to_string(&path).and_then(|source| {
            runtime_project::RuntimeProjectAdapter::import_source(&ProjectContext::new(root), &source)
        });
        match result {
            Ok(()) => workflow_ide_framework::tracing::info!(target: "meridian::urdf", file = %path.display(), "URDFインポート完了"),
            Err(error) => workflow_ide_framework::tracing::error!(target: "meridian::urdf", file = %path.display(), error = %error, "URDFインポート失敗"),
        }
    }))
    .menu_item(MenuItemDefinition::custom("data", "データ", "runtime.urdf.import"))
    .command(CommandDefinition::new("runtime.urdf.export", "URDFエクスポート...", move || {
        let root = export_project.lock().ok().and_then(|active| active.clone());
        let Some(root) = root else {
            workflow_ide_framework::tracing::warn!(target: "meridian::urdf", "URDFエクスポート: プロジェクトを開いてください");
            return;
        };
        let imported = match runtime_project::RuntimeProjectAdapter::load_source(&ProjectContext::new(root)) {
            Ok(Some(model)) => model,
            Ok(None) => {
                workflow_ide_framework::tracing::warn!(target: "meridian::urdf", "URDFエクスポート: インポート済みURDFがありません");
                return;
            }
            Err(error) => {
                workflow_ide_framework::tracing::error!(target: "meridian::urdf", error = %error, "URDFエクスポート失敗");
                return;
            }
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("URDF", &["urdf"])
            .set_file_name("robot.urdf")
            .save_file() else { return; };
        match urdf_io::export_unchanged_urdf(&imported, &path) {
            Ok(()) => workflow_ide_framework::tracing::info!(target: "meridian::urdf", file = %path.display(), "URDFエクスポート完了"),
            Err(error) => workflow_ide_framework::tracing::error!(target: "meridian::urdf", error = %error, "URDFエクスポート失敗"),
        }
    }))
    .menu_item(MenuItemDefinition::custom("data", "データ", "runtime.urdf.export"))
    .command(CommandDefinition::new("runtime.mjcf.import", "MJCFインポート...", move || {
        let root = mjcf_import_project.lock().ok().and_then(|active| active.clone());
        let Some(root) = root else {
            workflow_ide_framework::tracing::warn!(target: "meridian::mjcf", "MJCFインポート: プロジェクトを開いてください");
            return;
        };
        let context = ProjectContext::new(root);
        match mjcf_project::load_snapshot(&context) {
            Ok(Some(_)) => {
                workflow_ide_framework::tracing::warn!(target: "meridian::mjcf", "MJCFインポート: 既存MJCFがあります。MJCF再インポートを使用してください");
                return;
            }
            Ok(None) => {}
            Err(error) => {
                workflow_ide_framework::tracing::error!(target: "meridian::mjcf", error = %error, "MJCFインポート: 保存済みデータを確認できません");
                return;
            }
        }
        let Some(path) = rfd::FileDialog::new().add_filter("MJCF", &["xml", "mjcf"]).pick_file() else {
            return;
        };
        let Some(source_root) = path.parent() else {
            workflow_ide_framework::tracing::error!(target: "meridian::mjcf", file = %path.display(), "MJCFインポート: ソースルートを特定できません");
            return;
        };
        match mjcf_project::import_snapshot(&context, &path, source_root) {
            Ok(snapshot) => workflow_ide_framework::tracing::info!(target: "meridian::mjcf", file = %path.display(), body = snapshot.bundle.model.bodies.len(), joint = snapshot.bundle.model.joints.len(), actuator = snapshot.bundle.model.actuators.len(), "MJCFインポート完了"),
            Err(error) => workflow_ide_framework::tracing::error!(target: "meridian::mjcf", entry_file = %path.display(), error = %error, "MJCFインポート失敗"),
        }
    }))
    .menu_item(MenuItemDefinition::custom("data", "データ", "runtime.mjcf.import"))
    .command(CommandDefinition::new("runtime.mjcf.reimport", "MJCF再インポート...", move || {
        let root = mjcf_reimport_project.lock().ok().and_then(|active| active.clone());
        let Some(root) = root else {
            workflow_ide_framework::tracing::warn!(target: "meridian::mjcf", "MJCF再インポート: プロジェクトを開いてください");
            return;
        };
        let context = ProjectContext::new(root);
        let previous = match mjcf_project::load_snapshot(&context) {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => {
                workflow_ide_framework::tracing::warn!(target: "meridian::mjcf", "MJCF再インポート: インポート済みMJCFがありません");
                return;
            }
            Err(error) => {
                workflow_ide_framework::tracing::error!(target: "meridian::mjcf", error = %error, "MJCF再インポート: 保存済みデータを読み込めません");
                return;
            }
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("MJCF", &["xml", "mjcf"])
            .set_file_name(previous.bundle.entry_path.file_name().unwrap_or_default().to_string_lossy().as_ref())
            .pick_file() else { return; };
        let Some(source_root) = path.parent() else {
            workflow_ide_framework::tracing::error!(target: "meridian::mjcf", file = %path.display(), "MJCF再インポート: ソースルートを特定できません");
            return;
        };
        match mjcf_project::reimport_snapshot(&context, &path, source_root) {
            Ok(result) => workflow_ide_framework::tracing::info!(target: "meridian::mjcf", file = %path.display(), uuid_reused = result.plan.reuse.len(), unresolved = result.plan.unresolved.len(), "MJCF再インポート完了"),
            Err(error) => workflow_ide_framework::tracing::error!(target: "meridian::mjcf", entry_file = %path.display(), error = %error, "MJCF再インポート失敗"),
        }
    }))
    .menu_item(MenuItemDefinition::custom("data", "データ", "runtime.mjcf.reimport"))
    .run()
    .expect("failed to start workflow IDE framework");
}
