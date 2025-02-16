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
    required: bool,
}

impl TypeInterface {
    fn type_object_to_string(object_or_string: &TypeObjectOrString) -> String {
        match object_or_string {
            TypeObjectOrString::TypeObject(object) => {
                let mut result = Vec::new();
                for property in &object.properties {
                    result.push(format!(
                        "  {}{}: {};",
                        property.name,
                        if property.required { "" } else { "?" },
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

fn get_ts_types_from_schema(schema: &ReferenceOr<Box<Schema>>) -> Vec<String> {
    match schema {
        ReferenceOr::Item(schema) => match &schema.schema_kind {
            SchemaKind::Type(Type::String(_)) => vec!["string".to_string()],
            SchemaKind::Type(Type::Number(_)) => vec!["number".to_string()],
            SchemaKind::Type(Type::Boolean(_)) => vec!["boolean".to_string()],
            SchemaKind::Type(Type::Array(v)) => {
                let ts_type = match &v.items {
                    Some(item) => get_ts_types_from_schema(item).join(" | "),
                    None => "any".to_string(),
                };

                return vec![format!("{}[]", ts_type)];
            }
            SchemaKind::Type(Type::Object(_)) => vec!["object".to_string()],
            _ => {
                panic!("unknown schema kind for {:?}", schema);
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
                let properties: Vec<ObjectProperty> = object
                    .properties
                    .iter()
                    .map(|(key, value)| {
                        return ObjectProperty {
                            name: key.to_string(),
                            ts_types: get_ts_types_from_schema(value),
                            required: object.required.contains(key),
                        };
                    })
                    .collect();

                type_object_or_strings
                    .push(TypeObjectOrString::TypeObject(TypeObject { properties }));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_without_required_properties() {
        let schema_json = r#"
        {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "title": { "type": "string" },
                "author": { "type": "string" },
                "genres": { "type": "array", "items": { "type": "string" } },
                "publishedDate": { "type": "string", "format": "date" },
                "rating": { "type": "number", "format": "float" }
            }
        }
        "#;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("Book", &ReferenceOr::Item(schema));

        let expected = r##"interface Book {
  id?: string;
  title?: string;
  author?: string;
  genres?: string[];
  publishedDate?: string;
  rating?: number;
};"##;
        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_object_with_required_properties() {
        let schema_json = r#"
        {
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "author": { "type": "string" },
                "genres": { "type": "array", "items": { "type": "string" } },
                "publishedDate": { "type": "string", "format": "date" },
                "rating": { "type": "number", "format": "float" }
            },
            "required": ["title", "author"]
        }
        "#;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("NewBook", &ReferenceOr::Item(schema));

        let expected = r##"interface NewBook {
  title: string;
  author: string;
  genres?: string[];
  publishedDate?: string;
  rating?: number;
};"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_object_with_oneof() {
        let schema_json = r##"
        {
            "oneOf": [
                { "$ref": "#/components/schemas/Book" },
                {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "genres": { "type": "array", "items": { "type": "string" } },
                        "rating": { "type": "number", "format": "float" }
                    }
                }
            ]
        }
        "##;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface =
            get_interface_from_schema("SearchCriteria", &ReferenceOr::Item(schema));

        let expected = r##"type SearchCriteria = Book | {
  query?: string;
  genres?: string[];
  rating?: number;
};"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }
}
