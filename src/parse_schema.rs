use openapiv3::{ReferenceOr, Schema, SchemaKind, Type};

#[derive(Debug)]
enum ObjectOrPrimitiveOrRef {
    TypeObject(TypeObject),
    PrimitiveProperty(PrimitiveProperty),
    RefProperty(RefProperty),
}

#[derive(Debug)]
pub struct TypeInterface {
    name: String,
    types: Vec<ObjectOrPrimitiveOrRef>,
}

#[derive(Debug)]
struct TypeObject {
    properties: Vec<ObjectProperty>,
    is_array: bool,
}

#[derive(Debug)]
enum PrimitiveType {
    String,
    Number,
    Boolean,
    Null,
    Any,
}

#[derive(Debug)]
struct RefProperty {
    reference: String,
    is_array: bool,
}

#[derive(Debug)]
struct PrimitiveProperty {
    primitive_type: PrimitiveType,
    enumeration: Vec<String>,
    is_array: bool,
}

#[derive(Debug)]
struct ObjectProperty {
    name: String,
    ts_types: Vec<ObjectOrPrimitiveOrRef>,
    required: bool,
}

impl TypeInterface {
    fn reference_to_string(reference: &RefProperty) -> String {
        reference
            .is_array
            .then(|| format!("{}[]", reference.reference))
            .unwrap_or(reference.reference.to_string())
    }

    fn primitive_to_string(primitive: &PrimitiveProperty) -> String {
        match primitive.primitive_type {
            PrimitiveType::String => {
                if primitive.enumeration.is_empty() {
                    primitive
                        .is_array
                        .then(|| "string[]".to_string())
                        .unwrap_or("string".to_string())
                } else {
                    format!(
                        "{}",
                        primitive
                            .enumeration
                            .iter()
                            .map(|s| format!("\"{}\"", s))
                            .collect::<Vec<String>>()
                            .join(" | ")
                    )
                }
            }
            PrimitiveType::Number => primitive
                .is_array
                .then(|| "number[]".to_string())
                .unwrap_or("number".to_string()),
            PrimitiveType::Boolean => primitive
                .is_array
                .then(|| "boolean[]".to_string())
                .unwrap_or("boolean".to_string()),
            PrimitiveType::Null => primitive
                .is_array
                .then(|| "null[]".to_string())
                .unwrap_or("null".to_string()),
            PrimitiveType::Any => primitive
                .is_array
                .then(|| "any[]".to_string())
                .unwrap_or("any".to_string()),
        }
    }

    fn type_object_to_string(object_or_string: &ObjectOrPrimitiveOrRef, depth: usize) -> String {
        match object_or_string {
            ObjectOrPrimitiveOrRef::TypeObject(object) => {
                let mut object_string = Vec::new();

                for property in &object.properties {
                    let ts_types_string = property
                        .ts_types
                        .iter()
                        .map(|ts_type| match ts_type {
                            ObjectOrPrimitiveOrRef::TypeObject(_) => {
                                TypeInterface::type_object_to_string(ts_type, depth + 1)
                            }
                            ObjectOrPrimitiveOrRef::PrimitiveProperty(primitive_property) => {
                                TypeInterface::primitive_to_string(primitive_property)
                            }
                            ObjectOrPrimitiveOrRef::RefProperty(ref_property) => {
                                TypeInterface::reference_to_string(ref_property)
                            }
                        })
                        .collect::<Vec<String>>()
                        .join(" | ");

                    object_string.push(format!(
                        "{}{}{}: {};",
                        "  ".repeat(depth),
                        property.name,
                        if property.required { "" } else { "?" },
                        ts_types_string,
                    ));
                }

                return format!(
                    "{{\n{}\n{}}}{}",
                    object_string.join("\n"),
                    "  ".repeat(depth - 1),
                    if object.is_array { "[]" } else { "" }
                );
            }
            ObjectOrPrimitiveOrRef::PrimitiveProperty(primitive_property) => {
                return TypeInterface::primitive_to_string(primitive_property);
            }
            ObjectOrPrimitiveOrRef::RefProperty(ref_property) => {
                return TypeInterface::reference_to_string(ref_property);
            }
        }
    }

    pub fn to_string(&self) -> String {
        if self.types.len() > 1 {
            let mut result = Vec::new();
            for current_type in &self.types {
                result.push(TypeInterface::type_object_to_string(current_type, 1));
            }

            return format!("type {} = {};", self.name, result.join(" | "));
        }

        if self.types.len() == 1 {
            let type_string = TypeInterface::type_object_to_string(&self.types[0], 1);
            return format!("interface {} {};", self.name, type_string);
        }

        return "".to_string();
    }
}

trait SchemaLike {
    fn as_schema(&self) -> &Schema;
}

impl SchemaLike for Schema {
    fn as_schema(&self) -> &Schema {
        self
    }
}

impl SchemaLike for Box<Schema> {
    fn as_schema(&self) -> &Schema {
        self.as_ref()
    }
}

fn get_types_from_schema<T: SchemaLike>(
    schema: &ReferenceOr<T>,
    is_array: bool,
) -> Vec<ObjectOrPrimitiveOrRef> {
    match schema {
        ReferenceOr::Item(schema) => {
            let mut types: Vec<ObjectOrPrimitiveOrRef> = Vec::new();
            let schema = schema.as_schema();

            match &schema.schema_kind {
                SchemaKind::Type(Type::String(string_type)) => {
                    let enumeration = string_type
                        .enumeration
                        .iter()
                        .filter(|s| s.is_some())
                        .map(|s| s.as_ref().unwrap().to_string())
                        .collect::<Vec<String>>();

                    types.push(ObjectOrPrimitiveOrRef::PrimitiveProperty(
                        PrimitiveProperty {
                            primitive_type: PrimitiveType::String,
                            is_array: is_array,
                            enumeration: enumeration,
                        },
                    ));
                }
                SchemaKind::Type(Type::Number(_)) => {
                    types.push(ObjectOrPrimitiveOrRef::PrimitiveProperty(
                        PrimitiveProperty {
                            primitive_type: PrimitiveType::Number,
                            is_array: is_array,
                            enumeration: vec![],
                        },
                    ));
                }
                SchemaKind::Type(Type::Boolean(_)) => {
                    types.push(ObjectOrPrimitiveOrRef::PrimitiveProperty(
                        PrimitiveProperty {
                            primitive_type: PrimitiveType::Boolean,
                            is_array: is_array,
                            enumeration: vec![],
                        },
                    ));
                }
                SchemaKind::Type(Type::Array(v)) => {
                    let ts_type: Vec<ObjectOrPrimitiveOrRef> = match &v.items {
                        Some(item) => get_types_from_schema(item, true),
                        None => vec![ObjectOrPrimitiveOrRef::PrimitiveProperty(
                            PrimitiveProperty {
                                primitive_type: PrimitiveType::Any,
                                is_array: true,
                                enumeration: vec![],
                            },
                        )],
                    };

                    types.extend(ts_type);
                }
                SchemaKind::Type(Type::Object(object)) => {
                    let properties: Vec<ObjectProperty> = object
                        .properties
                        .iter()
                        .map(|(key, value)| ObjectProperty {
                            name: key.to_string(),
                            ts_types: get_types_from_schema(value, false),
                            required: object.required.contains(key),
                        })
                        .collect();

                    types.push(ObjectOrPrimitiveOrRef::TypeObject(TypeObject {
                        properties,
                        is_array: is_array,
                    }));
                }
                SchemaKind::OneOf { one_of } => {
                    for one_of_item in one_of {
                        types.extend(get_types_from_schema(one_of_item, is_array));
                    }
                }
                _ => {
                    println!("unknown schema kind for {:?}", schema);
                }
            }

            if schema.schema_data.nullable {
                types.push(ObjectOrPrimitiveOrRef::PrimitiveProperty(
                    PrimitiveProperty {
                        primitive_type: PrimitiveType::Null,
                        is_array: is_array,
                        enumeration: vec![],
                    },
                ));
            }

            return types;
        }
        ReferenceOr::Reference { reference } => {
            let reference_name = reference.split('/').last().unwrap_or_default().to_string();
            return vec![ObjectOrPrimitiveOrRef::RefProperty(RefProperty {
                reference: reference_name,
                is_array: is_array,
            })];
        }
    }
}

pub fn get_interface_from_schema(name: &str, schema: &ReferenceOr<Schema>) -> TypeInterface {
    TypeInterface {
        name: name.to_string(),
        types: get_types_from_schema(schema, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_object() {
        let schema_json = r#"
        {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "title": { "type": "string" },
                "author": { "type": "string" },
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
  publishedDate?: string;
  rating?: number;
};"##;
        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_object_with_array() {
        let schema_json = r#"
        {
            "type": "object", 
            "properties": {
                "id": { "type": "string" },
                "genres": { "type": "array", "items": { "type": "string" } },
                "tags": { "type": "array", "items": { "type": "string" } }
            }
        }
        "#;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("BookMetadata", &ReferenceOr::Item(schema));

        let expected = r##"interface BookMetadata {
  id?: string;
  genres?: string[];
  tags?: string[];
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
    fn test_object_with_nullable_properties() {
        let schema_json = r#"
        {
            "type": "object",
            "properties": {
                "reviewer": { 
                    "type": "string",
                    "description": "Name of the reviewer"
                },
                "comment": {
                    "type": "string",
                    "nullable": true,
                    "description": "Review comment"
                },
                "rating": {
                    "type": "number",
                    "format": "float", 
                    "nullable": true,
                    "description": "Rating given by the reviewer"
                },
                "date": {
                    "type": "string",
                    "format": "date-time",
                    "nullable": true,
                    "description": "Date of the review"
                }
            }
        }
        "#;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("Review", &ReferenceOr::Item(schema));

        let expected = r##"interface Review {
  reviewer?: string;
  comment?: string | null;
  rating?: number | null;
  date?: string | null;
};"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_object_with_enum() {
        let schema_json = r#"
        {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "status": { 
                    "type": "string",
                    "enum": ["draft", "published", "archived"]
                },
                "visibility": {
                    "type": "string", 
                    "enum": ["public", "private"],
                    "nullable": true
                }
            },
            "required": ["id", "status"]
        }
        "#;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("Post", &ReferenceOr::Item(schema));

        let expected = r##"interface Post {
  id: string;
  status: "draft" | "published" | "archived";
  visibility?: "public" | "private" | null;
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

    #[test]
    fn test_array_with_oneof() {
        let schema_json = r##"
        {
            "type": "array",
            "items": {
                "oneOf": [
                    { "type": "string" },
                    { "type": "number" },
                    { 
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "value": { "type": "number" }
                        },
                        "required": ["name", "value"]
                    }
                ]
            }
        }
        "##;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("MixedArray", &ReferenceOr::Item(schema));

        let expected = r##"type MixedArray = string[] | number[] | {
  name: string;
  value: number;
}[];"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_object_with_nested_objects() {
        let schema_json = r#"
        {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "name": { "type": "string" },
                "address": {
                    "type": "object",
                    "properties": {
                        "street": { "type": "string" },
                        "city": { "type": "string" },
                        "country": { "type": "string" },
                        "coordinates": {
                            "type": "object",
                            "properties": {
                                "latitude": { "type": "number" },
                                "longitude": { "type": "number" }
                            },
                            "required": ["latitude", "longitude"]
                        }
                    },
                    "required": ["street", "city"]
                }
            },
            "required": ["id", "name", "address"]
        }
        "#;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("Location", &ReferenceOr::Item(schema));

        let expected = r##"interface Location {
  id: string;
  name: string;
  address: {
    street: string;
    city: string;
    country?: string;
    coordinates?: {
      latitude: number;
      longitude: number;
    };
  };
};"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_object_with_nested_arrays() {
        let schema_json = r#"
        {
            "type": "object", 
            "properties": {
                "id": { "type": "string" },
                "categories": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "subcategories": {
                                "type": "array",
                                "items": { "type": "string" }
                            }
                        },
                        "required": ["name"]
                    }
                }
            },
            "required": ["id"]
        }
        "#;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("Product", &ReferenceOr::Item(schema));

        let expected = r##"interface Product {
  id: string;
  categories?: {
    name: string;
    subcategories?: string[];
  }[];
};"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_object_with_nested_oneof() {
        let schema_json = r#"
        {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "content": {
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["text"] },
                                "text": { "type": "string" }
                            },
                            "required": ["type", "text"]
                        },
                        {
                            "type": "object", 
                            "properties": {
                                "type": { "type": "string", "enum": ["image"] },
                                "url": { "type": "string" },
                                "dimensions": {
                                    "type": "object",
                                    "properties": {
                                        "width": { "type": "number" },
                                        "height": { "type": "number" }
                                    }
                                }
                            },
                            "required": ["type", "url"]
                        }
                    ]
                }
            },
            "required": ["id", "content"]
        }
        "#;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("Message", &ReferenceOr::Item(schema));

        let expected = r##"interface Message {
  id: string;
  content: {
    type: "text";
    text: string;
  } | {
    type: "image";
    url: string;
    dimensions?: {
      width?: number;
      height?: number;
    };
  };
};"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_nested_object_with_array_oneof() {
        let schema_json = r#"
          {
              "type": "object",
              "properties": {
                  "id": { "type": "string" },
                  "metadata": {
                      "type": "object",
                      "properties": {
                          "title": { "type": "string" },
                          "tags": {
                              "type": "array",
                              "items": {
                                  "oneOf": [
                                      { "type": "string" },
                                      {
                                          "type": "object",
                                          "properties": {
                                              "name": { "type": "string" },
                                              "value": { "type": "number" },
                                              "metadata": {
                                                  "type": "object",
                                                  "properties": {
                                                      "description": { "type": "string" },
                                                      "priority": { "type": "number" }
                                                  },
                                                  "required": ["description"]
                                              }
                                          },
                                          "required": ["name", "value"]
                                      }
                                  ]
                              }
                          }
                      },
                      "required": ["title", "tags"]
                  }
              },
              "required": ["id", "metadata"]
          }
          "#;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("DeepArray", &ReferenceOr::Item(schema));

        let expected = r##"interface DeepArray {
  id: string;
  metadata: {
    title: string;
    tags: string[] | {
      name: string;
      value: number;
      metadata?: {
        description: string;
        priority?: number;
      };
    }[];
  };
};"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_object_with_deep_array_refs() {
        let schema_json = r##"
        {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "data": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "references": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ExternalRef" }
                        }
                    },
                    "required": ["name", "references"]
                }
            },
            "required": ["id", "data"]
        }
        "##;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("DeepRefArray", &ReferenceOr::Item(schema));

        let expected = r##"interface DeepRefArray {
  id: string;
  data: {
    name: string;
    references: ExternalRef[];
  };
};"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }
}
