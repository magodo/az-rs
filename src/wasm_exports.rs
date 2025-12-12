use crate::azidentityext::access_token_credential::AccessTokenCredential;
use crate::log::set_global_logger;
use crate::run;
use azure_core::credentials::TokenCredential;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn run_cli(args: Vec<String>, token: &str) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();
    set_global_logger();

    let cred = AccessTokenCredential::new(token.to_string());
    run(PathBuf::new(), args, Some(Arc::new(cred))).await.map_err(jsfy)
}

fn jsfy<E>(e: E) -> JsValue
where
    E: Debug,
{
    let es = format!("{e:#?}");
    JsValue::from_str(es.as_str())
}
