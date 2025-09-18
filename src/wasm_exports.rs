use crate::azidentityext::access_token_credential::AccessTokenCredential;
use crate::client::Client;
use crate::run;
use crate::log::set_global_logger;
use std::fmt::Debug;
use std::path::PathBuf;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn run_cli(args: Vec<String>, token: &str) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();
    set_global_logger();

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
