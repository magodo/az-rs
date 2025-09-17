use crate::azidentityext::access_token_credential::AccessTokenCredential;
use crate::client::Client;
use crate::run;
use std::fmt::Debug;
use std::path::PathBuf;
use wasm_bindgen::prelude::*;

// Initialize tracing for WASM/browser environment
#[wasm_bindgen]
pub fn init_tracing() {
    use tracing_web::{MakeWebConsoleWriter, performance_layer};
    use tracing_subscriber::fmt::format::Pretty;
    use tracing_subscriber::prelude::*;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false) // Only partially supported across browsers
        .without_time()   // std::time is not available in browsers
        .with_writer(MakeWebConsoleWriter::new()); // write events to the console
    let perf_layer = performance_layer().with_details_from_fields(Pretty::default());

    let init_result = tracing_subscriber::registry()
        .with(fmt_layer)
        .with(perf_layer)
        .try_init();
    match init_result {
        Ok(_) => tracing::info!("Tracing initialized successfully"),
        Err(e) => tracing::error!("Failed to initialize tracing: {}", e),
    }
}

#[wasm_bindgen]
pub async fn run_cli(args: Vec<String>, token: &str) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();

    tracing::debug!("Running CLI with input: {:?}", args);
    let credential = AccessTokenCredential::new(token.to_string()).map_err(jsfy)?;
    let client = Client::new(
        "https://management.azure.com",
        vec!["https://management.azure.com/.default"],
        credential,
        None,
    )
    .map_err(jsfy)?;
    run(PathBuf::new(), &client, args).await.map_err(jsfy)
}

fn jsfy<E>(e: E) -> JsValue
where
    E: Debug,
{
    let es = format!("{e:#?}");
    JsValue::from_str(es.as_str())
}
