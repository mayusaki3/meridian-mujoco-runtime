use workflow_ide_framework as wfide;

fn main() {
    let mut config = wfide::ApplicationConfig::new(
        "meridian-mujoco-runtime",
        "Meridian MuJoCo Runtime",
    );

    config.window.title = Some("Meridian MuJoCo Runtime".into());
    config.window.initial_size = Some([1280.0, 800.0]);
    config.window.min_size = Some([900.0, 600.0]);
    config.appearance.ui_scale = Some(1.0);

    config.logging.directory = "logs/meridian".into();
    config.logging.file_prefix = "meridian".into();
    config.logging.retention_days = 7;

    let layout = wfide::LayoutConfig::new(["simulation-view"])
        .split_left("simulation-view", 0.25, ["runtime-control"])
        .split_below("runtime-control", 0.50, ["runtime-status"])
        .split_below("simulation-view", 0.75, ["log"])
        .selected("simulation-view");


    wfide::Application::with_config(config)
        .panel(wfide::PanelDefinition::new(
            "simulation-view",
            "Simulation View",
            wfide::PanelKind::GpuViewport,
        ))
        .panel(wfide::PanelDefinition::new(
            "runtime-control",
            "Runtime Control",
            wfide::PanelKind::StandardUi,
        ))
        .panel(wfide::PanelDefinition::new(
            "runtime-status",
            "Runtime Status",
            wfide::PanelKind::StandardUi,
        ))
        .panel(wfide::PanelDefinition::new(
            "log",
            "Log",
            wfide::PanelKind::StandardUi,
        ))
        .panel(
            wfide::PanelDefinition::new(
                "help",
                "Help",
                wfide::PanelKind::Browser,
            )
            .initially_visible(false),
        )
        .layout(layout)
        .run()
        .expect("failed to start workflow IDE framework");
}
