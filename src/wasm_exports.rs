use crate::azidentityext::access_token_credential::AccessTokenCredential;
use crate::client::Client;
use crate::run;
use std::fmt::Debug;
use std::{path::PathBuf, result::Result};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
    
    #[wasm_bindgen(js_namespace = console)]
    fn warn(s: &str);
    
    #[wasm_bindgen(js_namespace = console)]
    fn debug(s: &str);
}

// Initialize tracing for WASM/browser environment
fn init_tracing() {
    use tracing;
    use tracing_subscriber::prelude::*;
    use tracing_web::{MakeConsoleWriter, performance_layer};
    
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false) // No ANSI colors in browser console
        .with_writer(MakeConsoleWriter); // Route to browser console
    
    let perf_layer = performance_layer();
    
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(perf_layer)
        .init();
}

#[wasm_bindgen]
pub async fn run_cli(args: Vec<String>, token: &str) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();
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
