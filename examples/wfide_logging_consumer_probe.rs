use workflow_ide_framework as wfide;

fn main() {
    let mut config = wfide::logging::LoggingConfig::default();
    config.directory = "logs/meridian".into();
    config.file_prefix = "meridian".into();
    config.retention_days = 7;
    config.memory_lines = 64;

    let guard = wfide::logging::init("meridian-mujoco-runtime", &config)
        .expect("failed to initialize WFIDE logging");

    wfide::tracing::info!(
        target: "meridian::application",
        "Meridian application logging probe"
    );

    let lines = guard.snapshot();
    let consumer = lines.iter().any(|line| {
        line.contains("meridian::application")
            && line.contains("Meridian application logging probe")
    });

    println!("MERIDIAN_WFIDE_LOGGING consumer_in_memory={consumer}");
    assert!(consumer, "Meridian consumer log was not captured in WFIDE memory");
}
