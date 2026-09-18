use workflow_ide_framework::{Application, ApplicationConfig};

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
        .run()
        .expect("failed to start workflow IDE framework");
}
