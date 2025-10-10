static INIT: std::sync::Once = std::sync::Once::new();

#[allow(dead_code)]
#[cfg(not(target_arch = "wasm32"))]
fn init_tracing_subscriber() {
    use std::env;
    use std::fs::File;
    use std::io;
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_env("AZURE_LOG").unwrap_or_else(|_| EnvFilter::from("off"));
    let b = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false);

    match env::var("AZURE_LOG_PATH") {
        Ok(p) => {
            let f = File::create(p).expect("failed to create the log file");
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
