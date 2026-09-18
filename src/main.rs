fn main() {
    workflow_ide_framework::Application::new(
        "meridian-mujoco-runtime",
        "Meridian MuJoCo Runtime",
    )
    .run()
    .expect("failed to start workflow IDE framework");
}
