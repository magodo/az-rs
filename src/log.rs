static INIT: std::sync::Once = std::sync::Once::new();

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
            _ => LevelFilter::INFO, // default level
        },
        _ => LevelFilter::INFO, // default level
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn init_tracing_subscriber() {
    use std::io;
    use tracing_subscriber::prelude::*;
    
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(io::stderr);
    
    let level_filter = get_log_level();
    
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(level_filter)
        .init();
}

#[cfg(target_arch = "wasm32")]
fn init_tracing_subscriber() {
    use tracing_subscriber::prelude::*;
    use tracing_web::MakeWebConsoleWriter;
    
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .without_time()
        .with_writer(MakeWebConsoleWriter::new());
    
    tracing_subscriber::registry()
        .with(fmt_layer)
        .init();
}

pub fn set_global_logger() {
    INIT.call_once(|| {
        init_tracing_subscriber();
        tracing::debug!("Global logger initialized");
    });
}
