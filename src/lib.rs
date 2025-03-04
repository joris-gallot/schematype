use napi::bindgen_prelude::*;
use napi::{Env, JsObject, JsString};
use napi_derive::napi;
use openapiv3::Schema;
mod json_schema_to_typescript;
use serde_json::Value;

#[napi]
pub fn schema_to_typescript(env: Env, name: String, schema_input: JsObject) -> Result<String> {
  let schema_json = js_object_to_serde_value(env, schema_input)?;

  let schema: Schema = serde_json::from_value(schema_json)
    .map_err(|e| napi::Error::new(napi::Status::InvalidArg, format!("Invalid schema: {}", e)))?;

  let interface = json_schema_to_typescript::schema_to_typescript(
    name,
    openapiv3::ReferenceOr::Item(schema),
    None,
  );

  Ok(interface.to_string())
}

fn js_object_to_serde_value(env: Env, obj: JsObject) -> Result<Value> {
  let global = env.get_global()?;
  let json = global.get_named_property::<JsObject>("JSON")?;

  let stringify = json.get_named_property::<JsFunction>("stringify")?;

  let json_string: JsString = stringify
    .call(None, &[obj.into_unknown()])?
    .coerce_to_string()?;

  let json_rust_str = json_string.into_utf8()?.into_owned()?;

  serde_json::from_str(&json_rust_str).map_err(|e| {
    napi::Error::new(
      napi::Status::GenericFailure,
      format!("JSON parse error: {}", e),
    )
  })
}
