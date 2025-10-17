static INIT: std::sync::Once = std::sync::Once::new();

#[allow(dead_code)]
fn get_log_level() -> tracing_subscriber::filter::LevelFilter {
    use tracing_subscriber::filter::LevelFilter;

    match std::env::var("AZURE_LOG").as_deref() {
        Ok(level) => match level.to_lowercase().as_str() {
            "trace" => LevelFilter::TRACE,
            "debug" => LevelFilter::DEBUG,
            "info" => LevelFilter::INFO,
            "warn" => LevelFilter::WARN,
            "error" => LevelFilter::ERROR,
            "off" => LevelFilter::OFF,
            _ => LevelFilter::OFF, // default level
        },
        _ => LevelFilter::TRACE, // default level
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn init_tracing_subscriber() {
    use std::io;
    use std::{env, fs};
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_env("AZURE_LOG").unwrap_or_else(|_| EnvFilter::from("off"));
    let b = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false);

    match env::var("AZURE_LOG_PATH") {
        Ok(p) => {
            let f = fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(p)
                .expect("open log file");
            b.with_writer(f).init();
        }
        Err(_) => {
            b.with_writer(io::stderr).init();
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn init_tracing_subscriber() {
    use tracing_subscriber::prelude::*;
    use tracing_web::MakeWebConsoleWriter;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .without_time()
        .with_writer(MakeWebConsoleWriter::new());

    tracing_subscriber::registry().with(fmt_layer).init();
}

pub fn set_global_logger() {
    INIT.call_once(|| {
        init_tracing_subscriber();
        tracing::debug!("Logger initialized");
    });
}
