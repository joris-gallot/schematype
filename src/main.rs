use openapiv3::ObjectType;
use openapiv3::OpenAPI;
use openapiv3::ReferenceOr;
use openapiv3::Schema;
use openapiv3::SchemaKind;
use openapiv3::Type;
use serde_json;

#[derive(Debug)]
enum TypeObjectOrString {
    TypeObject(TypeObject),
    String(String),
}

#[derive(Debug)]
struct TypeInterface {
    name: String,
    types: Vec<TypeObjectOrString>,
}

#[derive(Debug)]
struct TypeObject {
    properties: Vec<ObjectProperty>,
}

#[derive(Debug)]
struct ObjectProperty {
    name: String,
    ts_types: Vec<String>,
}

impl TypeInterface {
    fn type_object_to_string(object_or_string: &TypeObjectOrString) -> String {
        match object_or_string {
            TypeObjectOrString::TypeObject(object) => {
                let mut result = Vec::new();
                for property in &object.properties {
                    result.push(format!(
                        "  {}: {};",
                        property.name,
                        property.ts_types.join(" | ")
                    ));
                }
                return format!("{{\n{}\n}}", result.join("\n"));
            }
            TypeObjectOrString::String(string) => {
                return string.clone();
            }
        }
    }

    fn to_string(&self) -> String {
        if self.types.len() > 1 {
            let mut result = Vec::new();
            for current_type in &self.types {
                result.push(TypeInterface::type_object_to_string(current_type));
            }

            return format!("type {} = {};\n", self.name, result.join(" | "));
        }

        if self.types.len() == 1 {
            let type_string = TypeInterface::type_object_to_string(&self.types[0]);
            return format!("interface {} {};\n", self.name, type_string);
        }

        return "".to_string();
    }
}

fn get_properties_from_object(schema_object: &ObjectType) -> Vec<ObjectProperty> {
    let mut properties = Vec::new();

    for (key, value) in schema_object.properties.iter() {
        match value {
            ReferenceOr::Item(schema) => match &schema.schema_kind {
                SchemaKind::Type(Type::String(_)) => {
                    properties.push(ObjectProperty {
                        name: key.to_string(),
                        ts_types: vec!["string".to_string()],
                    });
                }
                SchemaKind::Type(Type::Number(_)) => {
                    properties.push(ObjectProperty {
                        name: key.to_string(),
                        ts_types: vec!["number".to_string()],
                    });
                }
                SchemaKind::Type(Type::Boolean(_)) => {
                    properties.push(ObjectProperty {
                        name: key.to_string(),
                        ts_types: vec!["boolean".to_string()],
                    });
                }
                SchemaKind::Type(Type::Array(_)) => {
                    properties.push(ObjectProperty {
                        name: key.to_string(),
                        ts_types: vec!["array".to_string()],
                    });
                }
                SchemaKind::Type(Type::Object(_)) => {
                    properties.push(ObjectProperty {
                        name: key.to_string(),
                        ts_types: vec!["object".to_string()],
                    });
                }
                _ => {
                    println!("unknown schema kind for {:?}", key);
                }
            },
            ReferenceOr::Reference { reference } => {
                println!("reference: {}", reference);
            }
        }
    }

    return properties;
}

fn get_types_from_schema(schema: &ReferenceOr<Schema>) -> Vec<TypeObjectOrString> {
    let mut type_object_or_strings = Vec::new();

    match schema {
        ReferenceOr::Item(schema) => match &schema.schema_kind {
            SchemaKind::Type(Type::Object(object)) => {
                type_object_or_strings.push(TypeObjectOrString::TypeObject(TypeObject {
                    properties: get_properties_from_object(object),
                }));
            }
            SchemaKind::OneOf { one_of } => {
                for item in one_of {
                    type_object_or_strings.extend(get_types_from_schema(item));
                }
            }
            _ => {
                println!("unknown schema kind for {:?}", schema);
            }
        },
        ReferenceOr::Reference { reference } => {
            let reference_name = reference.split('/').last().unwrap_or_default().to_string();
            type_object_or_strings.push(TypeObjectOrString::String(reference_name.clone()));
        }
    }

    return type_object_or_strings;
}

fn main() {
    let data = include_str!("../openapi_example.json");
    let openapi: OpenAPI = serde_json::from_str(data).expect("Could not deserialize input");

    for (name, schema) in openapi.components.unwrap().schemas.iter() {
        let type_interface = TypeInterface {
            name: name.to_string(),
            types: get_types_from_schema(schema),
        };

        println!("{}", type_interface.to_string());
    }
}
