use openapiv3::OpenAPI;
use openapiv3::ReferenceOr;
use openapiv3::SchemaKind;
use openapiv3::Type;
use serde_json;

#[derive(Debug)]
struct TsInterface {
    name: String,
    properties: Vec<TsProperty>,
}

impl TsInterface {
    fn to_string(&self) -> String {
        let mut result = format!("interface {} {{\n", self.name);
        for property in &self.properties {
            result.push_str(&format!("  {}: {};\n", property.name, property.ts_type));
        }
        result.push_str("}");
        return result;
    }
}

#[derive(Debug)]
struct TsProperty {
    name: String,
    ts_type: String,
}

fn main() {
    let data = include_str!("../openapi_example.json");
    let openapi: OpenAPI = serde_json::from_str(data).expect("Could not deserialize input");

    for (name, schema) in openapi.components.unwrap().schemas.iter() {
        // handle only Book for now
        if name != "Book" {
            continue;
        }

        let mut ts_interface = TsInterface {
            name: name.to_string(),
            properties: Vec::new(),
        };

        match schema {
            ReferenceOr::Item(schema) => match &schema.schema_kind {
                SchemaKind::Type(Type::Object(object)) => {
                    for (key, value) in object.properties.iter() {
                        match value {
                            ReferenceOr::Item(schema) => match &schema.schema_kind {
                                SchemaKind::Type(Type::String(_)) => {
                                    ts_interface.properties.push(TsProperty {
                                        name: key.to_string(),
                                        ts_type: "string".to_string(),
                                    });
                                }
                                SchemaKind::Type(Type::Number(_)) => {
                                    ts_interface.properties.push(TsProperty {
                                        name: key.to_string(),
                                        ts_type: "number".to_string(),
                                    });
                                }
                                SchemaKind::Type(Type::Boolean(_)) => {
                                    ts_interface.properties.push(TsProperty {
                                        name: key.to_string(),
                                        ts_type: "boolean".to_string(),
                                    });
                                }
                                SchemaKind::Type(Type::Array(_)) => {
                                    ts_interface.properties.push(TsProperty {
                                        name: key.to_string(),
                                        ts_type: "array".to_string(),
                                    });
                                }
                                SchemaKind::Type(Type::Object(_)) => {
                                    ts_interface.properties.push(TsProperty {
                                        name: key.to_string(),
                                        ts_type: "object".to_string(),
                                    });
                                }
                                _ => {
                                    println!("unknown schema kind");
                                }
                            },
                            ReferenceOr::Reference { reference } => {
                                println!("reference: {}", reference);
                            }
                        }
                    }
                }
                _ => {
                    println!("unknown schema kind");
                }
            },
            ReferenceOr::Reference { reference } => {
                println!("reference: {}", reference);
            }
        }

        println!("{}", ts_interface.to_string());
    }
}
