use workflow_ide_framework::{
    Application, ApplicationConfig, PanelDefinition, PanelKind,
};

fn main() {
    let mut config = ApplicationConfig::new(
        "meridian-mujoco-runtime",
        "Meridian MuJoCo Runtime",
    );

    config.window.title = Some("Meridian MuJoCo Runtime".into());
    config.window.initial_size = Some([1280.0, 800.0]);
    config.window.min_size = Some([900.0, 600.0]);
    config.appearance.ui_scale = Some(1.0);

    Application::with_config(config)
        .panel(PanelDefinition::new(
            "simulation-view",
            "Simulation View",
            PanelKind::GpuViewport,
        ))
        .panel(PanelDefinition::new(
            "runtime-control",
            "Runtime Control",
            PanelKind::StandardUi,
        ))
        .panel(PanelDefinition::new(
            "runtime-status",
            "Runtime Status",
            PanelKind::StandardUi,
        ))
        .panel(PanelDefinition::new(
            "log",
            "Log",
            PanelKind::StandardUi,
        ))
        .panel(
            PanelDefinition::new(
                "help",
                "Help",
                PanelKind::Browser,
            )
            .initially_visible(false),
        )
        .run()
        .expect("failed to start workflow IDE framework");
}
