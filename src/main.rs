fn main() -> eframe_result::Result<()> {
    workflow_ide_framework::Application::new(
        "meridian-mujoco-runtime",
        "Meridian MuJoCo Runtime",
    )
    .run()
}

mod eframe_result {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}
