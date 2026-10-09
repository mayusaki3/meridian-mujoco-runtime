mod urdf_io;
mod runtime_project;

fn main() {
    workflow_ide_framework::Application::new(
        "meridian-mujoco-runtime",
        "Meridian MuJoCo Runtime",
    )
    .project_adapter(runtime_project::RuntimeProjectAdapter)
    .run()
    .expect("failed to start workflow IDE framework");
}
