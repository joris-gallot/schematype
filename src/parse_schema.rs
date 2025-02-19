use openapiv3::{ReferenceOr, Schema, SchemaKind, Type};

#[derive(Debug, Clone)]
enum ObjectOrPrimitiveOrRef {
    TypeObject(TypeObject),
    PrimitiveProperty(PrimitiveProperty),
    RefProperty(RefProperty),
}

#[derive(Debug, Clone)]
enum UnionOrIntersection {
    Union,
    Intersection,
}

#[derive(Debug)]
pub struct TypeInterface {
    name: String,
    expressions: Vec<Expression>,
}

#[derive(Debug, Clone)]
struct TypeObject {
    properties: Vec<ObjectProperty>,
}

#[derive(Debug, Clone)]
enum PrimitiveType {
    String,
    Number,
    Boolean,
    Null,
    Any,
}

#[derive(Debug, Clone)]
struct RefProperty {
    reference: String,
}

#[derive(Debug, Clone)]
struct PrimitiveProperty {
    primitive_type: PrimitiveType,
    enumeration: Vec<String>,
}

#[derive(Debug, Clone)]
struct ObjectProperty {
    name: String,
    expressions: Vec<Expression>,
    required: bool,
}

#[derive(Debug, Clone)]
struct Expression {
    types: Vec<ObjectOrPrimitiveOrRef>,
    link: Option<UnionOrIntersection>,
    is_array: bool,
}

impl TypeInterface {
    fn get_separator(separator: &Option<UnionOrIntersection>) -> &'static str {
        match separator {
            Some(UnionOrIntersection::Union) => " | ",
            Some(UnionOrIntersection::Intersection) => " & ",
            None => " | ",
        }
    }

    fn reference_to_string(reference: &RefProperty) -> String {
        reference.reference.to_string()
    }

    fn primitive_to_string(primitive: &PrimitiveProperty) -> String {
        match primitive.primitive_type {
            PrimitiveType::String => {
                if primitive.enumeration.is_empty() {
                    "string".to_string()
                } else {
                    format!(
                        "{}",
                        primitive
                            .enumeration
                            .iter()
                            .map(|s| format!("\"{}\"", s))
                            .collect::<Vec<String>>()
                            .join(TypeInterface::get_separator(&Some(
                                UnionOrIntersection::Union
                            )))
                    )
                }
            }
            PrimitiveType::Number => "number".to_string(),
            PrimitiveType::Boolean => "boolean".to_string(),
            PrimitiveType::Null => "null".to_string(),
            PrimitiveType::Any => "any".to_string(),
        }
    }

    fn type_object_to_string(object: &ObjectOrPrimitiveOrRef, depth: usize) -> String {
        match object {
            ObjectOrPrimitiveOrRef::TypeObject(type_object) => {
                let mut object_string = Vec::new();

                for property in &type_object.properties {
                    let ts_types_string = property
                        .expressions
                        .iter()
                        .map(|expression| {
                            let exp_string = expression
                                .types
                                .iter()
                                .map(|t| match t {
                                    ObjectOrPrimitiveOrRef::TypeObject(obj) => {
                                        TypeInterface::type_object_to_string(
                                            &ObjectOrPrimitiveOrRef::TypeObject(obj.clone()),
                                            depth + 1,
                                        )
                                    }
                                    ObjectOrPrimitiveOrRef::PrimitiveProperty(primitive) => {
                                        TypeInterface::primitive_to_string(primitive)
                                    }
                                    ObjectOrPrimitiveOrRef::RefProperty(reference) => {
                                        TypeInterface::reference_to_string(reference)
                                    }
                                })
                                .collect::<Vec<String>>()
                                .join(TypeInterface::get_separator(&expression.link));

                            let need_parentheses = expression.link.is_some()
                                && (expression.is_array || expression.types.len() > 1);

                            format!(
                                "{}{}{}{}",
                                if need_parentheses { "(" } else { "" },
                                exp_string,
                                if need_parentheses { ")" } else { "" },
                                if expression.is_array { "[]" } else { "" }
                            )
                        })
                        .collect::<Vec<String>>()
                        .join(TypeInterface::get_separator(&Some(
                            UnionOrIntersection::Union,
                        )));

                    object_string.push(format!(
                        "{}{}{}: {};",
                        "  ".repeat(depth),
                        property.name,
                        if property.required { "" } else { "?" },
                        ts_types_string,
                    ));
                }

                format!(
                    "{{\n{}\n{}}}",
                    object_string.join("\n"),
                    "  ".repeat(depth - 1),
                )
            }
            ObjectOrPrimitiveOrRef::PrimitiveProperty(primitive) => {
                TypeInterface::primitive_to_string(primitive)
            }
            ObjectOrPrimitiveOrRef::RefProperty(reference) => {
                TypeInterface::reference_to_string(reference)
            }
        }
    }

    pub fn to_string(&self) -> String {
        if self.expressions.is_empty() {
            return String::new();
        }

        let types = self
            .expressions
            .iter()
            .map(|expression| {
                let exp_string = expression
                    .types
                    .iter()
                    .map(|t| TypeInterface::type_object_to_string(t, 1))
                    .collect::<Vec<String>>()
                    .join(TypeInterface::get_separator(&expression.link));

                let need_parentheses = (expression.link.is_some() && expression.is_array)
                    || (expression.link.is_some() && self.expressions.len() > 1);

                format!(
                    "{}{}{}{}",
                    if need_parentheses { "(" } else { "" },
                    exp_string,
                    if need_parentheses { ")" } else { "" },
                    if expression.is_array { "[]" } else { "" }
                )
            })
            .collect::<Vec<String>>();

        format!(
            "type {} = {};",
            self.name,
            types.join(TypeInterface::get_separator(&Some(
                UnionOrIntersection::Union
            )))
        )
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

fn get_any_one_all_of_types(
    schema: &Vec<ReferenceOr<Schema>>,
    is_array: bool,
    separator: Option<UnionOrIntersection>,
) -> Vec<ObjectOrPrimitiveOrRef> {
    schema
        .iter()
        .map(|any_of_item| get_expressions_from_schema(any_of_item, is_array, separator.clone()))
        .flatten()
        .map(|expression| expression.types)
        .flatten()
        .collect()
}

fn get_expressions_from_schema<T: SchemaLike>(
    schema: &ReferenceOr<T>,
    is_array: bool,
    separator: Option<UnionOrIntersection>,
) -> Vec<Expression> {
    match schema {
        ReferenceOr::Item(schema) => {
            let mut expressions: Vec<Expression> = Vec::new();
            let schema = schema.as_schema();

            match &schema.schema_kind {
                SchemaKind::Type(Type::String(string_type)) => {
                    let enumeration = string_type
                        .enumeration
                        .iter()
                        .filter(|s| s.is_some())
                        .map(|s| s.as_ref().unwrap().to_string())
                        .collect::<Vec<String>>();

                    expressions.push(Expression {
                        types: vec![ObjectOrPrimitiveOrRef::PrimitiveProperty(
                            PrimitiveProperty {
                                primitive_type: PrimitiveType::String,
                                enumeration: enumeration,
                            },
                        )],
                        is_array: is_array,
                        link: None,
                    });
                }
                SchemaKind::Type(Type::Number(_)) => {
                    expressions.push(Expression {
                        types: vec![ObjectOrPrimitiveOrRef::PrimitiveProperty(
                            PrimitiveProperty {
                                primitive_type: PrimitiveType::Number,
                                enumeration: vec![],
                            },
                        )],
                        is_array: is_array,
                        link: None,
                    });
                }
                SchemaKind::Type(Type::Boolean(_)) => {
                    expressions.push(Expression {
                        types: vec![ObjectOrPrimitiveOrRef::PrimitiveProperty(
                            PrimitiveProperty {
                                primitive_type: PrimitiveType::Boolean,
                                enumeration: vec![],
                            },
                        )],
                        is_array: is_array,
                        link: None,
                    });
                }
                SchemaKind::Type(Type::Array(v)) => {
                    let array_expressions: Vec<Expression> = match &v.items {
                        Some(item) => get_expressions_from_schema(item, true, separator.clone()),
                        None => vec![Expression {
                            types: vec![ObjectOrPrimitiveOrRef::PrimitiveProperty(
                                PrimitiveProperty {
                                    primitive_type: PrimitiveType::Any,
                                    enumeration: vec![],
                                },
                            )],
                            is_array: true,
                            link: None,
                        }],
                    };

                    expressions.extend(array_expressions);
                }
                SchemaKind::Type(Type::Object(object)) => {
                    let properties: Vec<ObjectProperty> = object
                        .properties
                        .iter()
                        .map(|(key, value)| ObjectProperty {
                            name: key.to_string(),
                            expressions: get_expressions_from_schema(value, false, None),
                            required: object.required.contains(key),
                        })
                        .collect();

                    expressions.push(Expression {
                        types: vec![ObjectOrPrimitiveOrRef::TypeObject(TypeObject {
                            properties,
                        })],
                        is_array: is_array,
                        link: None,
                    });
                }
                SchemaKind::AnyOf { any_of } => {
                    expressions.push(Expression {
                        types: get_any_one_all_of_types(any_of, is_array, None),
                        is_array: is_array,
                        link: Some(UnionOrIntersection::Union),
                    });
                }
                SchemaKind::OneOf { one_of } => {
                    expressions.push(Expression {
                        types: get_any_one_all_of_types(one_of, is_array, None),
                        is_array: is_array,
                        link: Some(UnionOrIntersection::Union),
                    });
                }
                SchemaKind::AllOf { all_of } => {
                    expressions.push(Expression {
                        types: get_any_one_all_of_types(all_of, is_array, None),
                        is_array: is_array,
                        link: Some(UnionOrIntersection::Intersection),
                    });
                }
                _ => {
                    println!("unknown schema kind for {:?}", schema);
                }
            }

            if schema.schema_data.nullable {
                expressions.push(Expression {
                    types: vec![ObjectOrPrimitiveOrRef::PrimitiveProperty(
                        PrimitiveProperty {
                            primitive_type: PrimitiveType::Null,
                            enumeration: vec![],
                        },
                    )],
                    is_array: is_array,
                    link: None,
                });
            }

            return expressions;
        }
        ReferenceOr::Reference { reference } => {
            let reference_name = reference.split('/').last().unwrap_or_default().to_string();
            return vec![Expression {
                types: vec![ObjectOrPrimitiveOrRef::RefProperty(RefProperty {
                    reference: reference_name,
                })],
                is_array: is_array,
                link: separator,
            }];
        }
    }
}

pub fn get_interface_from_schema(name: &str, schema: &ReferenceOr<Schema>) -> TypeInterface {
    TypeInterface {
        name: name.to_string(),
        expressions: get_expressions_from_schema(schema, false, None),
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

        let expected = r##"type Book = {
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

        let expected = r##"type BookMetadata = {
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

        let expected = r##"type NewBook = {
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

        let expected = r##"type Review = {
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

        let expected = r##"type Post = {
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
    fn test_object_with_allof() {
        let schema_json = r##"
        {
            "allOf": [
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
            get_interface_from_schema("BookWithMetadata", &ReferenceOr::Item(schema));

        let expected = r##"type BookWithMetadata = Book & {
  query?: string;
  genres?: string[];
  rating?: number;
};"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_object_with_anyof() {
        let schema_json = r##"
        {
            "anyOf": [
                {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "age": { "type": "number" }
                    },
                    "required": ["name"]
                },
                {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "role": {
                            "type": "string",
                            "enum": ["admin", "user"]
                        }
                    },
                    "required": ["id", "role"]
                }
            ]
        }
        "##;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("UserInfo", &ReferenceOr::Item(schema));

        let expected = r##"type UserInfo = {
  name: string;
  age?: number;
} | {
  id: string;
  role: "admin" | "user";
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

        let expected = r##"type MixedArray = (string | number | {
  name: string;
  value: number;
})[];"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_array_with_allof() {
        let schema_json = r##"
        {
            "type": "array",
            "items": {
                "allOf": [
                    { "type": "object",
                      "properties": {
                          "id": { "type": "string" },
                          "name": { "type": "string" }
                      },
                      "required": ["id"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "metadata": {
                                "type": "object",
                                "properties": {
                                    "created": { "type": "string" },
                                    "modified": { "type": "string" }
                                }
                            }
                        }
                    }
                ]
            }
        }
        "##;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("CombinedArray", &ReferenceOr::Item(schema));

        let expected = r##"type CombinedArray = ({
  id: string;
  name?: string;
} & {
  metadata?: {
    created?: string;
    modified?: string;
  };
})[];"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_array_with_anyof() {
        let schema_json = r##"
        {
            "type": "array",
            "items": {
                "anyOf": [
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

        let type_interface = get_interface_from_schema("MixedAnyArray", &ReferenceOr::Item(schema));

        let expected = r##"type MixedAnyArray = (string | number | {
  name: string;
  value: number;
})[];"##;

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

        let expected = r##"type Location = {
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

        let expected = r##"type Product = {
  id: string;
  categories?: {
    name: string;
    subcategories?: string[];
  }[];
};"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_object_with_complex_nested_arrays() {
        let schema_json = r#"
        {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "departments": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "teams": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "teamName": { "type": "string" },
                                        "members": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "properties": {
                                                    "id": { "type": "string" },
                                                    "name": { "type": "string" },
                                                    "skills": {
                                                        "type": "array",
                                                        "items": {
                                                            "type": "object",
                                                            "properties": {
                                                                "name": { "type": "string" },
                                                                "level": { "type": "number" },
                                                                "certifications": {
                                                                    "type": "array",
                                                                    "items": { "type": "string" }
                                                                }
                                                            },
                                                            "required": ["name", "level"]
                                                        }
                                                    }
                                                },
                                                "required": ["id", "name"]
                                            }
                                        },
                                        "projects": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        }
                                    },
                                    "required": ["teamName", "members"]
                                }
                            }
                        },
                        "required": ["name", "teams"]
                    }
                }
            },
            "required": ["id", "departments"]
        }
        "#;

        let schema: Schema =
            serde_json::from_str(schema_json).expect("Could not deserialize schema");

        let type_interface = get_interface_from_schema("Organization", &ReferenceOr::Item(schema));

        let expected = r##"type Organization = {
  id: string;
  departments: {
    name: string;
    teams: {
      teamName: string;
      members: {
        id: string;
        name: string;
        skills?: {
          name: string;
          level: number;
          certifications?: string[];
        }[];
      }[];
      projects?: string[];
    }[];
  }[];
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

        let expected = r##"type DeepArray = {
  id: string;
  metadata: {
    title: string;
    tags: (string | {
      name: string;
      value: number;
      metadata?: {
        description: string;
        priority?: number;
      };
    })[];
  };
};"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_nested_object_with_array_allof() {
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
                                  "allOf": [
                                      {
                                          "type": "object",
                                          "properties": {
                                              "id": { "type": "string" },
                                              "type": { "type": "string" }
                                          },
                                          "required": ["id"]
                                      },
                                      {
                                          "type": "object",
                                          "properties": {
                                              "metadata": {
                                                  "type": "object",
                                                  "properties": {
                                                      "description": { "type": "string" },
                                                      "created": { "type": "string" }
                                                  }
                                              }
                                          }
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

        let type_interface =
            get_interface_from_schema("DeepArrayAllOf", &ReferenceOr::Item(schema));

        let expected = r##"type DeepArrayAllOf = {
  id: string;
  metadata: {
    title: string;
    tags: ({
      id: string;
      type?: string;
    } & {
      metadata?: {
        description?: string;
        created?: string;
      };
    })[];
  };
};"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }

    #[test]
    fn test_nested_object_with_array_anyof() {
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
                                  "anyOf": [
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

        let type_interface = get_interface_from_schema("DeepArrayAny", &ReferenceOr::Item(schema));

        let expected = r##"type DeepArrayAny = {
  id: string;
  metadata: {
    title: string;
    tags: (string | {
      name: string;
      value: number;
      metadata?: {
        description: string;
        priority?: number;
      };
    })[];
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

        let expected = r##"type DeepRefArray = {
  id: string;
  data: {
    name: string;
    references: ExternalRef[];
  };
};"##;

        assert_eq!(type_interface.to_string(), expected.to_string());
    }
}
