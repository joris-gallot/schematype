use openapiv3::OpenAPI;
use serde_json;

mod parse_schema;

fn main() {
    let data = include_str!("../openapi_example.json");
    let openapi: OpenAPI = serde_json::from_str(data).expect("Could not deserialize input");

    for (name, schema) in openapi.components.unwrap().schemas.iter() {
        let type_interface = parse_schema::get_interface_from_schema(name, schema);

        println!("{}", type_interface.to_string());
    }
}
