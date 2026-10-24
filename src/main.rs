mod catalog_edit;
mod catalog_registration;
mod catalog_stage;
mod localized_errors;
mod mjcf_project;
mod robot_catalog;
mod robot_project_panel;
mod robot_registry_integration;
mod runtime_project;
#[cfg(test)]
mod urdf_fixture_tests;
mod urdf_io;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use workflow_ide_framework::{
    LayoutConfig, PanelDefinition, PanelKind,
    command::{CommandDefinition, MenuItemDefinition},
    logging,
    project::ProjectContext,
    project_event::ProjectEventKind,
};

fn data_menu_import_item() -> MenuItemDefinition {
    MenuItemDefinition::custom_key("data", "meridian.menu.data", "runtime.urdf.import")
        .menu_after("menu.file")
        .access_key('D')
        .requires_stored_project()
}

#[cfg(test)]
mod menu_access_tests {
    use super::*;
    use workflow_ide_framework::command::MenuPlacement;

    #[test]
    fn data_menu_has_d_access_key_and_follows_file() {
        let item = data_menu_import_item();
        assert_eq!(item.access_key, Some('D'));
        assert!(item.requires_stored_project);
        assert_eq!(
            item.top_level_placement,
            MenuPlacement::After("menu.file".into())
        );
    }

    #[test]
    fn data_commands_require_stored_project() {
        let source = include_str!("main.rs");
        for key in [
            "meridian.menu.urdf.import",
            "meridian.menu.urdf.export",
            "meridian.menu.mjcf.import",
            "meridian.menu.mjcf.reimport",
        ] {
            let expected = format!("}}).label_key(\"{key}\").requires_stored_project())");
            assert!(
                source.contains(&expected),
                "{key} must require a stored project"
            );
        }
    }

    #[test]
    fn data_menu_translations_do_not_duplicate_access_key() {
        for (source, expected) in [
            (
                include_str!("../resources/meridian-locales/en-US.toml"),
                "Data",
            ),
            (
                include_str!("../resources/meridian-locales/ja-JP.toml"),
                "データ",
            ),
        ] {
            assert!(
                source
                    .lines()
                    .any(|line| line.trim() == format!("\"meridian.menu.data\" = \"{expected}\""))
            );
        }
    }
}

fn main() {
    let current_project = Arc::new(Mutex::new(None::<PathBuf>));
    let catalog_stage = catalog_stage::SharedCatalogStage::default();
    let adapter_stage = Arc::clone(&catalog_stage);
    let panel_stage = Arc::clone(&catalog_stage);
    let selected_robot = Arc::new(Mutex::new(None::<String>));
    let panel_selection = Arc::clone(&selected_robot);
    let event_selection = Arc::clone(&selected_robot);
    let callback_stage = Arc::clone(&catalog_stage);
    let event_stage = Arc::clone(&catalog_stage);
    let pending_registration = catalog_registration::SharedPending::default();
    let pending_diagnostic = Arc::new(Mutex::new(None::<PathBuf>));
    let callback_diagnostic = Arc::clone(&pending_diagnostic);
    let event_diagnostic = Arc::clone(&pending_diagnostic);
    let callback_pending = Arc::clone(&pending_registration);
    let callback_project = Arc::clone(&current_project);
    let command_pending = Arc::clone(&pending_registration);
    let command_project = Arc::clone(&current_project);
    let event_pending = Arc::clone(&pending_registration);
    let import_project = Arc::clone(&current_project);
    let export_project = Arc::clone(&current_project);
    let event_project = Arc::clone(&current_project);
    let mjcf_import_project = Arc::clone(&current_project);
    let mjcf_reimport_project = Arc::clone(&current_project);

    let application = workflow_ide_framework::Application::new(
        "meridian-mujoco-runtime",
        "Meridian MuJoCo Runtime",
    );
    let panel_refresh = application.project_panel_refresh_handle();
    application
    .localization_resources("resources/meridian-locales")
    .project_adapter(runtime_project::StagedRuntimeProjectAdapter { stage: adapter_stage })
    .panel(PanelDefinition::new("project", "meridian.panel.project", PanelKind::StandardUi))
    .panel(PanelDefinition::new("logs", "meridian.panel.logs", PanelKind::StandardUi))
    .layout(LayoutConfig::new(["project"]).split_below("project", 0.70, ["logs"]).selected("project"))
    .application_project_panel("project", robot_project_panel::StagedRobotProjectPanelProvider { stage: panel_stage, selection: panel_selection })
    .log_viewer_panel("logs")
    .language_settings_panel()
    .on_registry_access(move |registry| {
        catalog_registration::diagnose_opened_project(&callback_diagnostic, &callback_project, registry);
        catalog_registration::process_pending(&callback_pending, &callback_project, registry, &callback_stage, &panel_refresh);
    })
    .on_project_event(move |event| {
        if matches!(event.kind, ProjectEventKind::Created | ProjectEventKind::Opened | ProjectEventKind::SavedAs | ProjectEventKind::Closed) {
            if let Ok(mut selection) = event_selection.lock() { *selection = None; }
        }
        if let Ok(mut diagnostic) = event_diagnostic.lock() {
            match event.kind {
                ProjectEventKind::Opened => *diagnostic = event.root.clone(),
                ProjectEventKind::Created | ProjectEventKind::SavedAs | ProjectEventKind::Closed => *diagnostic = None,
                _ => {}
            }
        }
        if matches!(event.kind, ProjectEventKind::Created | ProjectEventKind::Opened | ProjectEventKind::SavedAs | ProjectEventKind::Closed) {
            if let Ok(mut pending) = event_pending.lock() {
                pending.request = None;
            }
        }
        if let Ok(mut stage) = event_stage.lock() {
            match event.kind {
                ProjectEventKind::Created | ProjectEventKind::Opened | ProjectEventKind::Closed => stage.discard(),
                ProjectEventKind::Saved => {
                    if let Some(root) = &event.root { stage.mark_saved(root); }
                }
                ProjectEventKind::SavedAs => stage.discard(),
                ProjectEventKind::CloseRequested => {}
            }
        }
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
        let capture = logging::dialog_on_key("meridian.urdf.import.result.title");
        let _operation = capture.span().enter();
        let root = import_project.lock().ok().and_then(|active| active.clone());
        let Some(root) = root else {
            workflow_ide_framework::tracing::warn!(target: "meridian::urdf", message = %workflow_ide_framework::localization::text("meridian.urdf.import.no_project"));
            return;
        };
        let Some(path) = rfd::FileDialog::new().add_filter("URDF", &["urdf"]).pick_file() else {
            return;
        };
        let result = std::fs::read_to_string(&path).and_then(|source| {
            runtime_project::RuntimeProjectAdapter::import_source(&ProjectContext::new(root), &source)
        });
        match result {
            Ok(()) => workflow_ide_framework::tracing::info!(target: "meridian::urdf", file = %path.display(), message = %workflow_ide_framework::localization::text("meridian.urdf.import.ok")),
            Err(error) => workflow_ide_framework::tracing::error!(target: "meridian::urdf", file = %path.display(), error = %error, message = %workflow_ide_framework::localization::text("meridian.urdf.import.failed")),
        }
    }).label_key("meridian.menu.urdf.import").requires_stored_project())
    .menu_item(data_menu_import_item())
    .command(CommandDefinition::new("runtime.catalog.register_urdf", "既存URDFをRobot Catalogへ登録", move || {
        let root = command_project.lock().ok().and_then(|active| active.clone());
        let Some(root) = root else { return; };
        let context = ProjectContext::new(root.clone());
        match runtime_project::RuntimeProjectAdapter::load_source(&context) {
            Ok(Some(_imported)) => {
                let name = "Imported URDF Robot".to_owned();
                if let Ok(mut pending) = command_pending.lock() {
                    pending.request = Some(catalog_registration::Request {
                        root,
                        name,
                        path: "application/robot-source.urdf",
                        format: "URDF",
                    });
                }
            }
            Ok(None) => workflow_ide_framework::tracing::warn!(target: "meridian::catalog", "No imported URDF source to register"),
            Err(error) => workflow_ide_framework::tracing::error!(target: "meridian::catalog", error=%error, "Cannot load imported URDF source"),
        }
    }).requires_stored_project())
    .menu_item(MenuItemDefinition::custom_key("data", "meridian.menu.data", "runtime.catalog.register_urdf"))
    .command(CommandDefinition::new("runtime.urdf.export", "URDFエクスポート...", move || {
        let capture = logging::dialog_on_key("meridian.urdf.export.result.title");
        let _operation = capture.span().enter();
        let root = export_project.lock().ok().and_then(|active| active.clone());
        let Some(root) = root else {
            workflow_ide_framework::tracing::warn!(target: "meridian::urdf", message = %workflow_ide_framework::localization::text("meridian.urdf.export.no_project"));
            return;
        };
        let imported = match runtime_project::RuntimeProjectAdapter::load_source(&ProjectContext::new(root)) {
            Ok(Some(model)) => model,
            Ok(None) => {
                workflow_ide_framework::tracing::warn!(target: "meridian::urdf", message = %workflow_ide_framework::localization::text("meridian.urdf.export.no_source"));
                return;
            }
            Err(error) => {
                workflow_ide_framework::tracing::error!(target: "meridian::urdf", error = %error, message = %workflow_ide_framework::localization::text("meridian.urdf.export.failed"));
                return;
            }
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("URDF", &["urdf"])
            .set_file_name("robot.urdf")
            .save_file() else { return; };
        match urdf_io::export_unchanged_urdf(&imported, &path) {
            Ok(()) => workflow_ide_framework::tracing::info!(target: "meridian::urdf", file = %path.display(), message = %workflow_ide_framework::localization::text("meridian.urdf.export.ok")),
            Err(error) => workflow_ide_framework::tracing::error!(target: "meridian::urdf", error = %error, message = %workflow_ide_framework::localization::text("meridian.urdf.export.failed")),
        }
    }).label_key("meridian.menu.urdf.export").requires_stored_project())
    .menu_item(MenuItemDefinition::custom_key("data", "meridian.menu.data", "runtime.urdf.export"))
    .command(CommandDefinition::new("runtime.mjcf.import", "MJCFインポート...", move || {
        let capture = logging::dialog_on_key("meridian.mjcf.import.result.title");
        let _operation = capture.span().enter();
        let root = mjcf_import_project.lock().ok().and_then(|active| active.clone());
        let Some(root) = root else {
            workflow_ide_framework::tracing::warn!(target: "meridian::mjcf", message = %workflow_ide_framework::localization::text("meridian.mjcf.import.no_project"));
            return;
        };
        let context = ProjectContext::new(root);
        match mjcf_project::load_snapshot(&context) {
            Ok(Some(_)) => {
                workflow_ide_framework::tracing::warn!(target: "meridian::mjcf", message = %workflow_ide_framework::localization::text("meridian.mjcf.import.existing"));
                return;
            }
            Ok(None) => {}
            Err(error) => {
                workflow_ide_framework::tracing::error!(target: "meridian::mjcf", error = %error, message = %workflow_ide_framework::localization::text("meridian.mjcf.import.read_failed"));
                return;
            }
        }
        let Some(path) = rfd::FileDialog::new().add_filter("MJCF", &["xml", "mjcf"]).pick_file() else {
            return;
        };
        let Some(source_root) = path.parent() else {
            workflow_ide_framework::tracing::error!(target: "meridian::mjcf", file = %path.display(), message = %workflow_ide_framework::localization::text("meridian.mjcf.import.no_root"));
            return;
        };
        match mjcf_project::import_snapshot(&context, &path, source_root) {
            Ok(snapshot) => workflow_ide_framework::tracing::info!(target: "meridian::mjcf", file = %path.display(), body = snapshot.bundle.model.bodies.len(), joint = snapshot.bundle.model.joints.len(), actuator = snapshot.bundle.model.actuators.len(), message = %workflow_ide_framework::localization::text("meridian.mjcf.import.ok")),
            Err(error) => workflow_ide_framework::tracing::error!(target: "meridian::mjcf", entry_file = %path.display(), error = %error, message = %workflow_ide_framework::localization::text("meridian.mjcf.import.failed")),
        }
    }).label_key("meridian.menu.mjcf.import").requires_stored_project())
    .menu_item(MenuItemDefinition::custom_key("data", "meridian.menu.data", "runtime.mjcf.import"))
    .command(CommandDefinition::new("runtime.mjcf.reimport", "MJCF再インポート...", move || {
        let capture = logging::dialog_on_key("meridian.mjcf.reimport.result.title");
        let _operation = capture.span().enter();
        let root = mjcf_reimport_project.lock().ok().and_then(|active| active.clone());
        let Some(root) = root else {
            workflow_ide_framework::tracing::warn!(target: "meridian::mjcf", message = %workflow_ide_framework::localization::text("meridian.mjcf.reimport.no_project"));
            return;
        };
        let context = ProjectContext::new(root);
        let previous = match mjcf_project::load_snapshot(&context) {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => {
                workflow_ide_framework::tracing::warn!(target: "meridian::mjcf", message = %workflow_ide_framework::localization::text("meridian.mjcf.reimport.no_source"));
                return;
            }
            Err(error) => {
                workflow_ide_framework::tracing::error!(target: "meridian::mjcf", error = %error, message = %workflow_ide_framework::localization::text("meridian.mjcf.reimport.read_failed"));
                return;
            }
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("MJCF", &["xml", "mjcf"])
            .set_file_name(previous.bundle.entry_path.file_name().unwrap_or_default().to_string_lossy().as_ref())
            .pick_file() else { return; };
        let Some(source_root) = path.parent() else {
            workflow_ide_framework::tracing::error!(target: "meridian::mjcf", file = %path.display(), message = %workflow_ide_framework::localization::text("meridian.mjcf.reimport.no_root"));
            return;
        };
        match mjcf_project::reimport_snapshot(&context, &path, source_root) {
            Ok(result) => workflow_ide_framework::tracing::info!(target: "meridian::mjcf", file = %path.display(), uuid_reused = result.plan.reuse.len(), unresolved = result.plan.unresolved.len(), message = %workflow_ide_framework::localization::text("meridian.mjcf.reimport.ok")),
            Err(error) => workflow_ide_framework::tracing::error!(target: "meridian::mjcf", entry_file = %path.display(), error = %error, message = %workflow_ide_framework::localization::text("meridian.mjcf.reimport.failed")),
        }
    }).label_key("meridian.menu.mjcf.reimport").requires_stored_project())
    .menu_item(MenuItemDefinition::custom_key("data", "meridian.menu.data", "runtime.mjcf.reimport"))
    .run()
    .expect("failed to start workflow IDE framework");
}
