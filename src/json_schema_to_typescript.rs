use napi_derive::napi;
use openapiv3::{
  BooleanType, IntegerType, NumberType, ReferenceOr, Schema, SchemaKind, StringType, Type,
};
use std::fmt;

#[derive(Debug, Default)]
#[napi(object)]
pub struct SchemaTypeOptions {
  pub prefer_unknown_over_any: bool,
  pub prefer_interface_over_type: bool,
}

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
  options: SchemaTypeOptions,
  expressions: Vec<Expression>,
}

#[derive(Debug, Clone)]
struct TypeObject {
  properties: Vec<ObjectProperty>,
  is_array: bool,
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
  is_array: bool,
}

#[derive(Debug, Clone)]
struct PrimitiveProperty {
  primitive_type: PrimitiveType,
  enumeration: Vec<String>,
  is_array: bool,
}

#[derive(Debug, Clone)]
struct ObjectProperty {
  name: String,
  expressions: Vec<Expression>,
  required: bool,
  description: Option<String>,
  deprecated: bool,
}

#[derive(Debug, Clone)]
struct Expression {
  types: Vec<ObjectOrPrimitiveOrRef>,
  link: Option<UnionOrIntersection>,
}

impl TypeInterface {
  fn get_separator(separator: &Option<UnionOrIntersection>) -> &'static str {
    match separator {
      Some(UnionOrIntersection::Union) => " | ",
      Some(UnionOrIntersection::Intersection) => " & ",
      None => " | ",
    }
  }

  fn reference_to_string(reference: &RefProperty, is_in_expression_array: bool) -> String {
    if is_in_expression_array {
      reference.reference.to_string()
    } else {
      reference
        .is_array
        .then_some(format!("{}[]", reference.reference))
        .unwrap_or(reference.reference.to_string())
    }
  }

  fn primitive_to_ts_string(
    primitive_type: &PrimitiveType,
    options: &SchemaTypeOptions,
  ) -> &'static str {
    match primitive_type {
      PrimitiveType::String => "string",
      PrimitiveType::Number => "number",
      PrimitiveType::Boolean => "boolean",
      PrimitiveType::Null => "null",
      PrimitiveType::Any => {
        if options.prefer_unknown_over_any {
          "unknown"
        } else {
          "any"
        }
      }
    }
  }

  fn primitive_to_string(
    primitive: &PrimitiveProperty,
    is_in_expression_array: bool,
    options: &SchemaTypeOptions,
  ) -> String {
    let primitive_str = TypeInterface::primitive_to_ts_string(&primitive.primitive_type, options);

    if primitive.enumeration.is_empty() {
      if is_in_expression_array {
        primitive_str.into()
      } else {
        primitive
          .is_array
          .then_some(format!("{}[]", primitive_str))
          .unwrap_or(primitive_str.into())
      }
    } else {
      let enum_string = primitive
        .enumeration
        .iter()
        .map(|s| {
          if matches!(primitive.primitive_type, PrimitiveType::String) {
            format!("\"{}\"", s)
          } else {
            s.to_string()
          }
        })
        .collect::<Vec<String>>()
        .join(TypeInterface::get_separator(&Some(
          UnionOrIntersection::Union,
        )));

      if is_in_expression_array {
        enum_string
      } else if primitive.is_array {
        if primitive.enumeration.len() > 1 {
          format!("({})[]", enum_string)
        } else {
          format!("{}[]", enum_string)
        }
      } else {
        enum_string
      }
    }
  }

  fn format_string_expression(exp_string: String, is_array: bool) -> String {
    format!(
      "{}{}{}{}",
      if is_array { "(" } else { "" },
      exp_string,
      if is_array { ")" } else { "" },
      if is_array { "[]" } else { "" }
    )
  }

  fn expression_is_array(expression: &Expression) -> bool {
    expression.types.len() > 1
      && expression.types.iter().all(|t| match t {
        ObjectOrPrimitiveOrRef::TypeObject(obj) => obj.is_array,
        ObjectOrPrimitiveOrRef::PrimitiveProperty(primitive) => primitive.is_array,
        ObjectOrPrimitiveOrRef::RefProperty(reference) => reference.is_array,
      })
  }

  fn type_object_to_string(
    object: &ObjectOrPrimitiveOrRef,
    depth: usize,
    expression_is_array: bool,
    options: &SchemaTypeOptions,
  ) -> String {
    match object {
      ObjectOrPrimitiveOrRef::TypeObject(type_object) => {
        if type_object.properties.is_empty() {
          return "{}".to_string();
        }

        let object_string = type_object
          .properties
          .iter()
          .map(|property| {
            let ts_types_string = property
              .expressions
              .iter()
              .map(|expression| {
                let expression_is_array = TypeInterface::expression_is_array(expression);

                let exp_string = expression
                  .types
                  .iter()
                  .map(|t| match t {
                    ObjectOrPrimitiveOrRef::TypeObject(obj) => {
                      TypeInterface::type_object_to_string(
                        &ObjectOrPrimitiveOrRef::TypeObject(obj.clone()),
                        depth + 1,
                        expression_is_array,
                        options,
                      )
                    }
                    ObjectOrPrimitiveOrRef::PrimitiveProperty(primitive) => {
                      TypeInterface::primitive_to_string(primitive, expression_is_array, options)
                    }
                    ObjectOrPrimitiveOrRef::RefProperty(reference) => {
                      TypeInterface::reference_to_string(reference, expression_is_array)
                    }
                  })
                  .collect::<Vec<String>>()
                  .join(TypeInterface::get_separator(&expression.link));

                TypeInterface::format_string_expression(exp_string, expression_is_array)
              })
              .collect::<Vec<String>>()
              .join(TypeInterface::get_separator(&Some(
                UnionOrIntersection::Union,
              )));

            let whitespace = "  ".repeat(depth);
            let comment = if let Some(description) = &property.description {
              format!(
                "{}/**\n{} * {}{}\n{} */\n",
                whitespace,
                whitespace,
                if property.deprecated {
                  "@deprecated "
                } else {
                  ""
                },
                description,
                whitespace
              )
            } else {
              "".to_string()
            };

            format!(
              "{}{}{}{}: {};",
              comment,
              whitespace,
              property.name,
              if property.required { "" } else { "?" },
              ts_types_string
            )
          })
          .collect::<Vec<String>>();

        format!(
          "{{\n{}\n{}}}{}",
          object_string.join("\n"),
          "  ".repeat(depth - 1),
          if type_object.is_array && !expression_is_array {
            "[]"
          } else {
            ""
          }
        )
      }
      ObjectOrPrimitiveOrRef::PrimitiveProperty(primitive) => {
        TypeInterface::primitive_to_string(primitive, expression_is_array, options)
      }
      ObjectOrPrimitiveOrRef::RefProperty(reference) => {
        TypeInterface::reference_to_string(reference, expression_is_array)
      }
    }
  }
}

impl fmt::Display for TypeInterface {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if self.expressions.is_empty() {
      return write!(f, "{}", String::new());
    }

    let types = self
      .expressions
      .iter()
      .map(|expression| {
        let expression_is_array = TypeInterface::expression_is_array(expression);
        let exp_string = expression
          .types
          .iter()
          .map(|t| TypeInterface::type_object_to_string(t, 1, expression_is_array, &self.options))
          .collect::<Vec<String>>()
          .join(TypeInterface::get_separator(&expression.link));

        TypeInterface::format_string_expression(exp_string, expression_is_array)
      })
      .collect::<Vec<String>>();

    let is_single_type_object = self.expressions.len() == 1
      && matches!(
        self.expressions[0].types[0],
        ObjectOrPrimitiveOrRef::TypeObject(_)
      );

    let export_type = if self.options.prefer_interface_over_type && is_single_type_object {
      format!("export interface {}", self.name)
    } else {
      format!("export type {} =", self.name)
    };

    write!(
      f,
      "{} {};",
      export_type,
      types.join(TypeInterface::get_separator(&Some(
        UnionOrIntersection::Union
      )))
    )
  }
}

trait HasEnumeration {
  type ReturnType;
  fn get_enumeration(&self) -> &Vec<Option<Self::ReturnType>>;
  fn to_string(&self, value: &Self::ReturnType) -> String;
}

impl HasEnumeration for NumberType {
  type ReturnType = f64;
  fn get_enumeration(&self) -> &Vec<Option<Self::ReturnType>> {
    &self.enumeration
  }
  fn to_string(&self, value: &Self::ReturnType) -> String {
    value.to_string()
  }
}

impl HasEnumeration for IntegerType {
  type ReturnType = i64;
  fn get_enumeration(&self) -> &Vec<Option<Self::ReturnType>> {
    &self.enumeration
  }
  fn to_string(&self, value: &Self::ReturnType) -> String {
    value.to_string()
  }
}

impl HasEnumeration for StringType {
  type ReturnType = String;
  fn get_enumeration(&self) -> &Vec<Option<Self::ReturnType>> {
    &self.enumeration
  }
  fn to_string(&self, value: &Self::ReturnType) -> String {
    value.to_string()
  }
}

impl HasEnumeration for BooleanType {
  type ReturnType = bool;
  fn get_enumeration(&self) -> &Vec<Option<Self::ReturnType>> {
    &self.enumeration
  }
  fn to_string(&self, value: &Self::ReturnType) -> String {
    value.to_string()
  }
}

fn get_primitive_expression<T>(
  type_with_enum: &T,
  primitive_type: PrimitiveType,
  is_array: bool,
) -> Expression
where
  T: HasEnumeration,
{
  let enumeration = type_with_enum
    .get_enumeration()
    .iter()
    .filter(|s| s.is_some())
    .map(|s| T::to_string(type_with_enum, s.as_ref().unwrap()))
    .collect::<Vec<String>>();

  Expression {
    types: vec![ObjectOrPrimitiveOrRef::PrimitiveProperty(
      PrimitiveProperty {
        primitive_type,
        enumeration,
        is_array,
      },
    )],
    link: None,
  }
}

fn schema_to_typescript_any_one_all_of_types(
  schema: &[ReferenceOr<Schema>],
  is_array: bool,
  separator: Option<UnionOrIntersection>,
) -> Vec<ObjectOrPrimitiveOrRef> {
  schema
    .iter()
    .flat_map(|any_of_item| {
      schema_to_typescript_expressions(any_of_item, is_array, separator.clone())
    })
    .flat_map(|expression| expression.types)
    .collect()
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

fn schema_to_typescript_expressions<T: SchemaLike>(
  schema: &ReferenceOr<T>,
  is_array: bool,
  separator: Option<UnionOrIntersection>,
) -> Vec<Expression> {
  match schema {
    ReferenceOr::Item(schema) => {
      let schema = schema.as_schema();

      let base_expressions = match &schema.schema_kind {
        SchemaKind::Type(Type::String(string_type)) => {
          vec![get_primitive_expression(
            string_type,
            PrimitiveType::String,
            is_array,
          )]
        }
        SchemaKind::Type(Type::Number(number_type)) => {
          vec![get_primitive_expression(
            number_type,
            PrimitiveType::Number,
            is_array,
          )]
        }
        SchemaKind::Type(Type::Integer(integer_type)) => {
          vec![get_primitive_expression(
            integer_type,
            PrimitiveType::Number,
            is_array,
          )]
        }
        SchemaKind::Type(Type::Boolean(boolean_type)) => {
          vec![get_primitive_expression(
            boolean_type,
            PrimitiveType::Boolean,
            is_array,
          )]
        }
        SchemaKind::Type(Type::Array(v)) => match &v.items {
          Some(item) => schema_to_typescript_expressions(item, true, separator.clone()),
          None => vec![Expression {
            types: vec![ObjectOrPrimitiveOrRef::PrimitiveProperty(
              PrimitiveProperty {
                primitive_type: PrimitiveType::Any,
                enumeration: vec![],
                is_array: true,
              },
            )],
            link: None,
          }],
        },
        SchemaKind::Type(Type::Object(object)) => {
          let properties: Vec<ObjectProperty> = object
            .properties
            .iter()
            .map(|(key, value)| {
              let description = match value {
                ReferenceOr::Item(schema) => schema.as_schema().schema_data.description.clone(),
                ReferenceOr::Reference { .. } => None,
              };

              let deprecated = match value {
                ReferenceOr::Item(schema) => schema.as_schema().schema_data.deprecated,
                ReferenceOr::Reference { .. } => false,
              };

              ObjectProperty {
                name: key.to_string(),
                expressions: schema_to_typescript_expressions(value, false, None),
                required: object.required.contains(key),
                description,
                deprecated,
              }
            })
            .collect();

          vec![Expression {
            types: vec![ObjectOrPrimitiveOrRef::TypeObject(TypeObject {
              properties,
              is_array,
            })],
            link: None,
          }]
        }
        SchemaKind::AnyOf { any_of } => vec![Expression {
          types: schema_to_typescript_any_one_all_of_types(any_of, is_array, None),
          link: Some(UnionOrIntersection::Union),
        }],
        SchemaKind::OneOf { one_of } => vec![Expression {
          types: schema_to_typescript_any_one_all_of_types(one_of, is_array, None),
          link: Some(UnionOrIntersection::Union),
        }],
        SchemaKind::AllOf { all_of } => vec![Expression {
          types: schema_to_typescript_any_one_all_of_types(all_of, is_array, None),
          link: Some(UnionOrIntersection::Intersection),
        }],
        _ => {
          println!(
            "schema type not recognized, defaulting to any type\n{:?}",
            schema.schema_kind
          );
          vec![Expression {
            types: vec![ObjectOrPrimitiveOrRef::PrimitiveProperty(
              PrimitiveProperty {
                primitive_type: PrimitiveType::Any,
                enumeration: vec![],
                is_array,
              },
            )],
            link: None,
          }]
        }
      };

      if schema.schema_data.nullable {
        base_expressions
          .into_iter()
          .map(|mut expression| {
            expression
              .types
              .push(ObjectOrPrimitiveOrRef::PrimitiveProperty(
                PrimitiveProperty {
                  primitive_type: PrimitiveType::Null,
                  enumeration: vec![],
                  is_array,
                },
              ));
            expression
          })
          .collect()
      } else {
        base_expressions
      }
    }
    ReferenceOr::Reference { reference } => {
      let reference_name = reference.split('/').last().unwrap_or_default().to_string();
      vec![Expression {
        types: vec![ObjectOrPrimitiveOrRef::RefProperty(RefProperty {
          reference: reference_name,
          is_array,
        })],
        link: separator,
      }]
    }
  }
}

pub fn schema_to_typescript(
  name: String,
  schema: ReferenceOr<Schema>,
  options: Option<SchemaTypeOptions>,
) -> TypeInterface {
  TypeInterface {
    name,
    options: options.unwrap_or_default(),
    expressions: schema_to_typescript_expressions(&schema, false, None),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_empty_object() {
    let schema_json = r#"
        {
            "type": "object",
            "properties": {
                "metadata": {
                    "type": "object",
                    "properties": {}
                }
            }
        }
        "#;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("EmptyObject".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type EmptyObject = {
  metadata?: {};
};"##;
    assert_eq!(type_interface.to_string(), expected.to_string());
  }

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
                "rating": { "type": "number", "format": "float" },
                "age": { "type": "integer" }
            }
        }
        "#;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface = schema_to_typescript("Book".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type Book = {
  id?: string;
  title?: string;
  author?: string;
  publishedDate?: string;
  rating?: number;
  age?: number;
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("BookMetadata".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type BookMetadata = {
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("NewBook".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type NewBook = {
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
                    "type": "string"
                },
                "comment": {
                    "type": "string",
                    "nullable": true
                },
                "rating": {
                    "type": "number",
                    "format": "float",
                    "nullable": true
                },
                "date": {
                    "type": "string",
                    "format": "date-time",
                    "nullable": true
                }
            }
        }
        "#;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("Review".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type Review = {
  reviewer?: string;
  comment?: string | null;
  rating?: number | null;
  date?: string | null;
};"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_object_with_string_enum() {
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
                },
                "size": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["small"],
                        "nullable": true
                    }
                },
                "options": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["medium"]
                    },
                    "nullable": true
                },
                "tags": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["featured", "trending", "new"]
                    },
                    "nullable": true
                },
                "categories": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["tech", "science"],
                        "nullable": true
                    }
                },
                "isActive": {
                    "type": "boolean",
                    "enum": [true, false]
                }
            },
            "required": ["id", "status"]
        }
        "#;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface = schema_to_typescript("Post".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type Post = {
  id: string;
  status: "draft" | "published" | "archived";
  visibility?: "public" | "private" | null;
  size?: ("small" | null)[];
  options?: "medium"[] | null;
  tags?: ("featured" | "trending" | "new")[] | null;
  categories?: ("tech" | "science" | null)[];
  isActive?: true | false;
};"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_object_with_number_enum() {
    let schema_json = r#"
        {
            "type": "object", 
            "properties": {
                "id": { "type": "string" },
                "priority": {
                    "type": "number",
                    "enum": [1.5, 2.5, 3.5]
                },
                "score": {
                    "type": "number",
                    "enum": [0.5, 1.0],
                    "nullable": true
                },
                "size": {
                    "type": "array",
                    "items": {
                        "type": "number",
                        "enum": [1]
                    },
                    "nullable": true
                },
                "options": {
                    "type": "array",
                    "items": {
                        "type": "number",
                        "enum": [1]
                    },
                    "nullable": true
                },
                "weights": {
                    "type": "array",
                    "items": {
                        "type": "number",
                        "enum": [0.1, 0.2, 0.3]
                    },
                    "nullable": true
                },
                "metrics": {
                    "type": "array",
                    "items": {
                        "type": "number",
                        "enum": [1.1, 2.2],
                        "nullable": true
                    }
                },
                "isEnabled": {
                    "type": "boolean",
                    "enum": [true]
                }
            },
            "required": ["id", "priority"]
        }
        "#;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface = schema_to_typescript("Task".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type Task = {
  id: string;
  priority: 1.5 | 2.5 | 3.5;
  score?: 0.5 | 1 | null;
  size?: 1[] | null;
  options?: 1[] | null;
  weights?: (0.1 | 0.2 | 0.3)[] | null;
  metrics?: (1.1 | 2.2 | null)[];
  isEnabled?: true;
};"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_object_with_integer_enum() {
    let schema_json = r#"
        {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "level": {
                    "type": "integer",
                    "enum": [1, 2, 3]
                },
                "grade": {
                    "type": "integer", 
                    "enum": [0, 1],
                    "nullable": true
                },
                "size": {
                    "type": "array",
                    "items": {
                        "type": "number",
                        "enum": [2]
                    },
                    "nullable": true
                },
                "options": {
                    "type": "array",
                    "items": {
                        "type": "number",
                        "enum": [2]
                    },
                    "nullable": true
                },
                "stages": {
                    "type": "array",
                    "items": {
                        "type": "integer",
                        "enum": [10, 20, 30]
                    },
                    "nullable": true
                },
                "points": {
                    "type": "array",
                    "items": {
                        "type": "integer",
                        "enum": [100, 200],
                        "nullable": true
                    }
                },
                "isPublic": {
                    "type": "boolean",
                    "enum": [false]
                }
            },
            "required": ["id", "level"]
        }
        "#;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface = schema_to_typescript("Grade".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type Grade = {
  id: string;
  level: 1 | 2 | 3;
  grade?: 0 | 1 | null;
  size?: 2[] | null;
  options?: 2[] | null;
  stages?: (10 | 20 | 30)[] | null;
  points?: (100 | 200 | null)[];
  isPublic?: false;
};"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_object_with_boolean_enum() {
    let schema_json = r#"
        {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "isActive": {
                    "type": "boolean",
                    "enum": [true]
                },
                "isPublic": {
                    "type": "boolean",
                    "enum": [false],
                    "nullable": true
                },
                "flags": {
                    "type": "array",
                    "items": {
                        "type": "boolean",
                        "enum": [true]
                    },
                    "nullable": true
                },
                "settings": {
                    "type": "array",
                    "items": {
                        "type": "boolean",
                        "enum": [false],
                        "nullable": true
                    }
                }
            },
            "required": ["id", "isActive"]
        }
        "#;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("Config".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type Config = {
  id: string;
  isActive: true;
  isPublic?: false | null;
  flags?: true[] | null;
  settings?: (false | null)[];
};"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_object_with_invalid_property() {
    let schema_json = r#"
        {
            "type": "object",
            "properties": {
                "invalid_property": {
                    "required": ["invalid_property"],
                    "anyOf": [
                        {
                            "type": "string"
                        },
                        {
                            "type": "number"
                        }
                    ]
                }
            }
        }
        "#;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("InvalidObject".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type InvalidObject = {
  invalid_property?: any;
};"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_object_with_mixed_enums_oneof() {
    let schema_json = r##"
        {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "value": {
                    "oneOf": [
                        {
                            "type": "string",
                            "enum": ["low", "medium", "high"]
                        },
                        {
                            "type": "integer",
                            "enum": [0, 1, 2]
                        },
                        {
                            "type": "number",
                            "enum": [0.5, 1.5, 2.5]
                        },
                        {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "enum": ["A", "B", "C"]
                            }
                        },
                        {
                            "type": "boolean",
                            "enum": [true, false]
                        },
                        {
                            "$ref": "#/components/schemas/ExternalRef"
                        }
                    ]
                },
                "mixedArray": {
                    "type": "array",
                    "items": {
                        "oneOf": [
                            {
                                "type": "string",
                                "enum": ["red", "green", "blue"]
                            },
                            {
                                "type": "number",
                                "enum": [1, 2, 3]
                            },
                            {
                                "type": "boolean",
                                "enum": [true]
                            },
                            {
                                "type": "string",
                                "enum": ["small", "medium", "large"]
                            },
                            {
                                "$ref": "#/components/schemas/ExternalRef"
                            }
                        ]
                    }
                }
            },
            "required": ["id", "value", "mixedArray"]
        }
        "##;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("MixedEnum".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type MixedEnum = {
  id: string;
  value: "low" | "medium" | "high" | 0 | 1 | 2 | 0.5 | 1.5 | 2.5 | ("A" | "B" | "C")[] | true | false | ExternalRef;
  mixedArray: ("red" | "green" | "blue" | 1 | 2 | 3 | true | "small" | "medium" | "large" | ExternalRef)[];
};"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_object_with_mixed_enums_anyof() {
    let schema_json = r##"
        {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "value": {
                    "anyOf": [
                        {
                            "type": "string",
                            "enum": ["low", "medium", "high"]
                        },
                        {
                            "type": "integer",
                            "enum": [0, 1, 2]
                        },
                        {
                            "type": "number",
                            "enum": [0.5, 1.5, 2.5]
                        },
                        {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "enum": ["A", "B", "C"]
                            }
                        },
                        {
                            "type": "boolean",
                            "enum": [true, false]
                        },
                        {
                            "$ref": "#/components/schemas/ExternalRef"
                        }
                    ]
                },
                "mixedArray": {
                    "type": "array",
                    "items": {
                        "anyOf": [
                            {
                                "type": "string",
                                "enum": ["red", "green", "blue"]
                            },
                            {
                                "type": "number",
                                "enum": [1, 2, 3]
                            },
                            {
                                "type": "boolean",
                                "enum": [true]
                            },
                            {
                                "type": "string",
                                "enum": ["small", "medium", "large"]
                            },
                            {
                                "$ref": "#/components/schemas/ExternalRef"
                            }
                        ]
                    }
                }
            },
            "required": ["id", "value", "mixedArray"]
        }
        "##;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("MixedEnum".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type MixedEnum = {
  id: string;
  value: "low" | "medium" | "high" | 0 | 1 | 2 | 0.5 | 1.5 | 2.5 | ("A" | "B" | "C")[] | true | false | ExternalRef;
  mixedArray: ("red" | "green" | "blue" | 1 | 2 | 3 | true | "small" | "medium" | "large" | ExternalRef)[];
};"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_object_with_mixed_enums_allof() {
    let schema_json = r##"
        {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "value": {
                    "allOf": [
                        {
                            "type": "object",
                            "properties": {
                                "status": {
                                    "type": "string",
                                    "enum": ["active", "inactive"]
                                }
                            }
                        },
                        {
                            "type": "object",
                            "properties": {
                                "priority": {
                                    "type": "number",
                                    "enum": [1, 2, 3]
                                }
                            }
                        },
                        {
                            "type": "object",
                            "properties": {
                                "isEnabled": {
                                    "type": "boolean",
                                    "enum": [true]
                                }
                            }
                        },
                        {
                            "$ref": "#/components/schemas/ExternalRef"
                        }
                    ]
                }
            },
            "required": ["id", "value"]
        }
        "##;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface = schema_to_typescript(
      "MixedEnumAllOf".to_string(),
      ReferenceOr::Item(schema),
      None,
    );

    let expected = r##"export type MixedEnumAllOf = {
  id: string;
  value: {
    status?: "active" | "inactive";
  } & {
    priority?: 1 | 2 | 3;
  } & {
    isEnabled?: true;
  } & ExternalRef;
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface = schema_to_typescript(
      "SearchCriteria".to_string(),
      ReferenceOr::Item(schema),
      None,
    );

    let expected = r##"export type SearchCriteria = Book | {
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface = schema_to_typescript(
      "BookWithMetadata".to_string(),
      ReferenceOr::Item(schema),
      None,
    );

    let expected = r##"export type BookWithMetadata = Book & {
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("UserInfo".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type UserInfo = {
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("MixedArray".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type MixedArray = (string | number | {
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("CombinedArray".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type CombinedArray = ({
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("MixedAnyArray".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type MixedAnyArray = (string | number | {
  name: string;
  value: number;
})[];"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_object_with_anyof_array_primitive_object() {
    let schema_json = r##"
        {
            "type": "object", 
            "properties": {
                "key": {
                    "type": "string"
                },
                "value": {
                    "anyOf": [
                        {
                            "type": "array",
                            "items": {
                                "type": "number"
                            }
                        },
                        {
                            "type": "string"
                        },
                        {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string" },
                                    "count": { "type": "number" }
                                },
                                "required": ["name", "count"]
                            }
                        },
                        {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/SomeType"
                            }
                        }
                    ]
                }
            },
            "required": [
                "key",
                "value"
            ]
        }
        "##;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("MixedValue".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type MixedValue = {
  key: string;
  value: number[] | string | {
    name: string;
    count: number;
  }[] | SomeType[];
};"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_object_with_oneof_array_primitive_object() {
    let schema_json = r##"
        {
            "type": "object", 
            "properties": {
                "key": {
                    "type": "string"
                },
                "value": {
                    "oneOf": [
                        {
                            "type": "array",
                            "items": {
                                "type": "number"
                            }
                        },
                        {
                            "type": "string"
                        },
                        {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string" },
                                    "count": { "type": "number" }
                                },
                                "required": ["name", "count"]
                            }
                        },
                        {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/SomeType"
                            }
                        }
                    ]
                }
            },
            "required": [
                "key",
                "value"
            ]
        }
        "##;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("MixedValue".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type MixedValue = {
  key: string;
  value: number[] | string | {
    name: string;
    count: number;
  }[] | SomeType[];
};"##;

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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("Location".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type Location = {
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("Product".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type Product = {
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("Organization".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type Organization = {
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("DeepArray".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type DeepArray = {
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface = schema_to_typescript(
      "DeepArrayAllOf".to_string(),
      ReferenceOr::Item(schema),
      None,
    );

    let expected = r##"export type DeepArrayAllOf = {
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("DeepArrayAny".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type DeepArrayAny = {
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

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("DeepRefArray".to_string(), ReferenceOr::Item(schema), None);

    let expected = r##"export type DeepRefArray = {
  id: string;
  data: {
    name: string;
    references: ExternalRef[];
  };
};"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_object_with_property_descriptions() {
    let schema_json = r##"
        {
            "type": "object",
            "description": "Root object description",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Unique identifier"
                },
                "numbers": {
                    "type": "array",
                    "description": "List of important numbers",
                    "items": {
                        "type": "number",
                        "description": "A numeric value"
                    }
                },
                "flags": {
                    "type": "object", 
                    "description": "Configuration flags",
                    "properties": {
                        "isEnabled": {
                            "type": "boolean",
                            "description": "Whether feature is enabled"
                        },
                        "priority": {
                            "type": "integer",
                            "description": "Priority level 1-5",
                            "minimum": 1,
                            "maximum": 5
                        }
                    },
                    "required": ["isEnabled"]
                },
                "metadata": {
                    "type": "array",
                    "description": "List of metadata objects",
                    "items": {
                        "type": "object",
                        "description": "Metadata entry",
                        "properties": {
                            "key": {
                                "type": "string",
                                "description": "Metadata key"
                            },
                            "value": {
                                "type": "string",
                                "description": "Metadata value"
                            }
                        },
                        "required": ["key", "value"]
                    }
                }
            },
            "required": ["id", "flags", "metadata"]
        }
        "##;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface =
      schema_to_typescript("ComplexObject".to_string(), ReferenceOr::Item(schema), None);
    let expected = r##"export type ComplexObject = {
  /**
   * Unique identifier
   */
  id: string;
  /**
   * List of important numbers
   */
  numbers?: number[];
  /**
   * Configuration flags
   */
  flags: {
    /**
     * Whether feature is enabled
     */
    isEnabled: boolean;
    /**
     * Priority level 1-5
     */
    priority?: number;
  };
  /**
   * List of metadata objects
   */
  metadata: {
    /**
     * Metadata key
     */
    key: string;
    /**
     * Metadata value
     */
    value: string;
  }[];
};"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_object_with_deprecated_properties() {
    let schema_json = r##"
        {
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Unique identifier"
                },
                "oldName": {
                    "type": "string",
                    "description": "Use name instead",
                    "deprecated": true
                },
                "name": {
                    "type": "string",
                    "description": "The current name field"
                },
                "oldTags": {
                    "type": "array",
                    "description": "Use categories instead",
                    "deprecated": true,
                    "items": {
                        "type": "string"
                    }
                },
                "categories": {
                    "type": "array",
                    "description": "Current categories field",
                    "items": {
                        "type": "string"
                    }
                },
                "config": {
                    "type": "object",
                    "properties": {
                        "oldSetting": {
                            "type": "boolean",
                            "description": "Deprecated setting - use newSetting",
                            "deprecated": true
                        },
                        "newSetting": {
                            "type": "boolean",
                            "description": "Current setting to use"
                        },
                        "oldOptions": {
                            "type": "array",
                            "description": "Deprecated options array - use newOptions",
                            "deprecated": true,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "key": { "type": "string" },
                                    "value": { "type": "string" }
                                }
                            }
                        },
                        "newOptions": {
                            "type": "array",
                            "description": "Current options to use",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "key": { "type": "string" },
                                    "value": { "type": "string" }
                                }
                            }
                        }
                    }
                }
            },
            "required": ["id", "name"]
        }
        "##;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let type_interface = schema_to_typescript(
      "ObjectWithDeprecated".to_string(),
      ReferenceOr::Item(schema),
      None,
    );

    let expected = r##"export type ObjectWithDeprecated = {
  /**
   * Unique identifier
   */
  id: string;
  /**
   * @deprecated Use name instead
   */
  oldName?: string;
  /**
   * The current name field
   */
  name: string;
  /**
   * @deprecated Use categories instead
   */
  oldTags?: string[];
  /**
   * Current categories field
   */
  categories?: string[];
  config?: {
    /**
     * @deprecated Deprecated setting - use newSetting
     */
    oldSetting?: boolean;
    /**
     * Current setting to use
     */
    newSetting?: boolean;
    /**
     * @deprecated Deprecated options array - use newOptions
     */
    oldOptions?: {
      key?: string;
      value?: string;
    }[];
    /**
     * Current options to use
     */
    newOptions?: {
      key?: string;
      value?: string;
    }[];
  };
};"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_schema_with_any_types() {
    let schema_json = r##"
        {
            "type": "object",
            "properties": {
                "id": {
                    "type": "string"
                },
                "dynamicValue": {},
                "arrayOfAny": {
                    "type": "array",
                    "items": {}
                },
                "objectWithAny": {
                    "type": "object",
                    "properties": {
                        "anyProp": {}
                    }
                }
            },
            "required": ["id"]
        }
        "##;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    // Test with default options (prefer_unknown_over_any = false)
    let type_interface = schema_to_typescript(
      "SchemaWithAny".to_string(),
      ReferenceOr::Item(schema.clone()),
      None,
    );

    let expected = r##"export type SchemaWithAny = {
  id: string;
  dynamicValue?: any;
  arrayOfAny?: any[];
  objectWithAny?: {
    anyProp?: any;
  };
};"##;

    assert_eq!(type_interface.to_string(), expected.to_string());
  }
  #[test]
  fn test_schema_with_unknown_types() {
    let schema_json = r##"
        {
            "type": "object",
            "properties": {
                "id": {
                    "type": "string"
                },
                "dynamicValue": {},
                "arrayOfAny": {
                    "type": "array",
                    "items": {}
                },
                "objectWithAny": {
                    "type": "object",
                    "properties": {
                        "anyProp": {}
                    }
                }
            },
            "required": ["id"]
        }
        "##;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");

    let options = Some(SchemaTypeOptions {
      prefer_unknown_over_any: true,
      prefer_interface_over_type: false,
    });

    let type_interface_unknown = schema_to_typescript(
      "SchemaWithUnknown".to_string(),
      ReferenceOr::Item(schema.clone()),
      options,
    );

    let expected_unknown = r##"export type SchemaWithUnknown = {
  id: string;
  dynamicValue?: unknown;
  arrayOfAny?: unknown[];
  objectWithAny?: {
    anyProp?: unknown;
  };
};"##;

    assert_eq!(
      type_interface_unknown.to_string(),
      expected_unknown.to_string()
    );
  }

  #[test]
  fn test_prefer_interface_simple_object() {
    let schema_json = r##"
        {
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            }
        }
        "##;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");
    let options = Some(SchemaTypeOptions {
      prefer_unknown_over_any: false,
      prefer_interface_over_type: true,
    });

    let interface = schema_to_typescript("Person".to_string(), ReferenceOr::Item(schema), options);

    let expected = r##"export interface Person {
  name?: string;
  age?: number;
};"##;

    assert_eq!(interface.to_string(), expected.to_string());
  }

  #[test]
  fn test_prefer_interface_union_type() {
    let schema_json = r##"
        {
            "oneOf": [
                {"type": "string"},
                {"type": "number"}
            ]
        }
        "##;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");
    let options = Some(SchemaTypeOptions {
      prefer_unknown_over_any: false,
      prefer_interface_over_type: true,
    });

    let type_def =
      schema_to_typescript("UnionType".to_string(), ReferenceOr::Item(schema), options);

    // Should still use type for unions even with prefer_interface_over_type
    let expected = r##"export type UnionType = string | number;"##;

    assert_eq!(type_def.to_string(), expected.to_string());
  }

  #[test]
  fn test_prefer_interface_nested_object() {
    let schema_json = r##"
        {
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "settings": {
                            "type": "object",
                            "properties": {
                                "theme": {"type": "string"},
                                "notifications": {"type": "boolean"}
                            }
                        }
                    }
                }
            }
        }
        "##;

    let schema: Schema = serde_json::from_str(schema_json).expect("Could not deserialize schema");
    let options = Some(SchemaTypeOptions {
      prefer_unknown_over_any: false,
      prefer_interface_over_type: true,
    });

    let interface =
      schema_to_typescript("UserConfig".to_string(), ReferenceOr::Item(schema), options);

    let expected = r##"export interface UserConfig {
  id?: string;
  user?: {
    name?: string;
    settings?: {
      theme?: string;
      notifications?: boolean;
    };
  };
};"##;

    assert_eq!(interface.to_string(), expected.to_string());
  }
}
