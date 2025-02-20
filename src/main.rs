use openapiv3::OpenAPI;
use serde_json;

mod json_schema_to_typescript;
mod open_api_to_typescript;

fn main() {
    let data = include_str!("../openapi_example.json");
    let openapi: OpenAPI = serde_json::from_str(data).expect("Could not deserialize input");

    open_api_to_typescript::open_api_to_typescript(openapi);
}
