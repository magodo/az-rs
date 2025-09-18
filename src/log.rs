static INIT: std::sync::Once = std::sync::Once::new();

#[cfg(not(target_arch = "wasm32"))]
fn build_fmt_layer() -> impl tracing_subscriber::layer::Layer<tracing_subscriber::Registry> + Send + Sync {
    use std::io;
    tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(io::stderr)
}

#[cfg(target_arch = "wasm32")]
fn build_fmt_layer() -> impl tracing_subscriber::layer::Layer<tracing_subscriber::Registry> + Send + Sync {
    use tracing_web::MakeWebConsoleWriter;
    tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .without_time()
        .with_writer(MakeWebConsoleWriter::new())
}

fn _set_global_logger() {
    use tracing_subscriber::prelude::*;
    
    let fmt_layer = build_fmt_layer();
    
    tracing_subscriber::registry()
        .with(fmt_layer)
        .init();
}

pub fn set_global_logger() {
    INIT.call_once(|| {
        _set_global_logger();
        tracing::debug!("Global logger initialized");
    });
}
