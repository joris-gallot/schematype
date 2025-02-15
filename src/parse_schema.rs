use openapiv3::{ReferenceOr, Schema, SchemaKind, Type};

#[derive(Debug)]
enum TypeObjectOrString {
    TypeObject(TypeObject),
    String(String),
}

#[derive(Debug)]
pub struct TypeInterface {
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

    pub fn to_string(&self) -> String {
        if self.types.len() > 1 {
            let mut result = Vec::new();
            for current_type in &self.types {
                result.push(TypeInterface::type_object_to_string(current_type));
            }

            return format!("type {} = {};", self.name, result.join(" | "));
        }

        if self.types.len() == 1 {
            let type_string = TypeInterface::type_object_to_string(&self.types[0]);
            return format!("interface {} {};", self.name, type_string);
        }

        return "".to_string();
    }
}

fn get_object_property_from_schema(
    name: &str,
    schema: &ReferenceOr<Box<Schema>>,
) -> ObjectProperty {
    match schema {
        ReferenceOr::Item(schema) => match &schema.schema_kind {
            SchemaKind::Type(Type::String(_)) => {
                return ObjectProperty {
                    name: name.to_string(),
                    ts_types: vec!["string".to_string()],
                };
            }
            SchemaKind::Type(Type::Number(_)) => {
                return ObjectProperty {
                    name: name.to_string(),
                    ts_types: vec!["number".to_string()],
                };
            }
            SchemaKind::Type(Type::Boolean(_)) => {
                return ObjectProperty {
                    name: name.to_string(),
                    ts_types: vec!["boolean".to_string()],
                };
            }
            SchemaKind::Type(Type::Array(v)) => {
                let ts_type = match &v.items {
                    Some(item) => {
                        let ts_type = get_object_property_from_schema(name, item);
                        ts_type.ts_types.join(" | ")
                    }
                    None => "any".to_string(),
                };

                return ObjectProperty {
                    name: name.to_string(),
                    ts_types: vec![format!("{}[]", ts_type)],
                };
            }
            SchemaKind::Type(Type::Object(_)) => {
                return ObjectProperty {
                    name: name.to_string(),
                    ts_types: vec!["object".to_string()],
                };
            }
            _ => {
                panic!("unknown schema kind for {:?}", name);
            }
        },
        ReferenceOr::Reference { reference } => {
            panic!("not implemented reference: {}", reference);
        }
    }
}

fn get_types_from_schema(schema: &ReferenceOr<Schema>) -> Vec<TypeObjectOrString> {
    let mut type_object_or_strings = Vec::new();

    match schema {
        ReferenceOr::Item(schema) => match &schema.schema_kind {
            SchemaKind::Type(Type::Object(object)) => {
                type_object_or_strings.push(TypeObjectOrString::TypeObject(TypeObject {
                    properties: object
                        .properties
                        .iter()
                        .map(|(key, value)| get_object_property_from_schema(key, value))
                        .collect(),
                }));
            }
            SchemaKind::OneOf { one_of } => {
                for one_of_item in one_of {
                    type_object_or_strings.extend(get_types_from_schema(one_of_item));
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

pub fn get_interface_from_schema(name: &str, schema: &ReferenceOr<Schema>) -> TypeInterface {
    return TypeInterface {
        name: name.to_string(),
        types: get_types_from_schema(schema),
    };
}
