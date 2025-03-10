use crate::json_schema_to_typescript::TypeInterface;

use napi_derive::napi;
use openapiv3::{
  OpenAPI, Operation, Parameter, ParameterData, ParameterSchemaOrContent, ReferenceOr, Schema,
};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug)]
pub enum OpenApiMethod {
  Get,
  Post,
  Put,
  Delete,
  Patch,
  Options,
}

#[derive(Debug)]
#[napi(object)]
pub struct OpenApiOutput {
  pub paths: Vec<OpenApiPath>,
  pub components: Vec<OpenApiComponent>,
}

#[derive(Debug)]
#[napi(object)]
pub struct OpenApiComponent {
  pub name: String,
  pub ts_type: String,
}

#[derive(Debug)]
#[napi(object)]
pub struct OpenApiPath {
  pub path: String,
  pub method: String,
  pub query_parameters: Option<String>,
  pub path_parameters: Option<String>,
  pub request_body: Option<String>,
  pub responses: HashMap<String, String>,
}

#[derive(Debug)]
#[napi(object)]
pub struct OpenApiParameter {
  pub description: Option<String>,
  pub required: bool,
  pub ts_type: String,
}

impl OpenApiOutput {
  fn open_api_method_to_string(method: &OpenApiMethod) -> &'static str {
    match method {
      OpenApiMethod::Get => "get",
      OpenApiMethod::Post => "post",
      OpenApiMethod::Put => "put",
      OpenApiMethod::Delete => "delete",
      OpenApiMethod::Patch => "patch",
      OpenApiMethod::Options => "options",
    }
  }
}

fn extract_parameters(
  parameters: &[&Parameter],
  parameter_type: fn(&Parameter) -> Option<&ParameterData>,
) -> (serde_json::Map<String, serde_json::Value>, Vec<String>) {
  let props = parameters
    .iter()
    .filter_map(|parameter| {
      parameter_type(parameter).and_then(|parameter_data| {
        if let ParameterSchemaOrContent::Schema(schema) = &parameter_data.format {
          serde_json::to_value(schema)
            .ok()
            .map(|schema_json| (parameter_data.name.clone(), schema_json))
        } else {
          None
        }
      })
    })
    .collect();

  let required = parameters
    .iter()
    .filter_map(|parameter| {
      parameter_type(parameter).and_then(|parameter_data| {
        if parameter_data.required {
          Some(parameter_data.name.clone())
        } else {
          None
        }
      })
    })
    .collect();

  (props, required)
}

fn generate_parameters_ts_type(
  parameters: &[&Parameter],
  parameter_type: fn(&Parameter) -> Option<&ParameterData>,
) -> Option<String> {
  let has_parameters = parameters.iter().any(|p| parameter_type(p).is_some());

  if !has_parameters {
    return None;
  }

  let (props, required) = extract_parameters(parameters, parameter_type);

  let schema_json = json!({
      "type": "object",
      "properties": props,
      "required": required
  });

  if let Ok(schema) = serde_json::from_value(schema_json) {
    Some(
      crate::json_schema_to_typescript::schema_to_typescript(ReferenceOr::Item(schema), None)
        .to_string(),
    )
  } else {
    None
  }
}

fn get_open_api_path(path: &str, method: OpenApiMethod, operation: &Operation) -> OpenApiPath {
  let request_body: Option<ReferenceOr<Schema>> = match &operation.request_body {
    Some(request_body) => match request_body {
      ReferenceOr::Item(request_body) => match request_body.content.get("application/json") {
        Some(content) => content.schema.clone(),
        None => None,
      },
      ReferenceOr::Reference { reference } => {
        panic!("Reference not implemented for path: {}", reference);
      }
    },
    None => None,
  };

  let request_body_type: Option<TypeInterface> = request_body.map(|request_body| {
    crate::json_schema_to_typescript::schema_to_typescript(request_body.clone(), None)
  });

  let parameters: Vec<&Parameter> = operation
    .parameters
    .iter()
    .filter_map(|parameter| match parameter {
      ReferenceOr::Item(parameter) => Some(parameter),
      ReferenceOr::Reference { reference } => {
        eprintln!("Warning: Reference not implemented for path: {}", reference);
        None
      }
    })
    .collect();

  let query_parameters = generate_parameters_ts_type(&parameters, |p| match p {
    Parameter::Query { parameter_data, .. } => Some(parameter_data),
    _ => None,
  });

  let path_parameters = generate_parameters_ts_type(&parameters, |p| match p {
    Parameter::Path { parameter_data, .. } => Some(parameter_data),
    _ => None,
  });

  let responses: HashMap<String, String> = operation
    .responses
    .responses
    .iter()
    .filter_map(|(status_code, response)| {
      let res = match response {
        ReferenceOr::Item(response) => response,
        ReferenceOr::Reference { reference } => {
          panic!("Reference not implemented for path: {}", reference);
        }
      };

      let res_schema = match res.content.get("application/json") {
        Some(content) => match &content.schema {
          Some(schema) => schema,
          None => return None,
        },
        None => return None,
      };

      let res_schema_interface =
        crate::json_schema_to_typescript::schema_to_typescript(res_schema.clone(), None);

      Some((status_code.to_string(), res_schema_interface.to_string()))
    })
    .collect();

  OpenApiPath {
    path: path.to_string(),
    method: OpenApiOutput::open_api_method_to_string(&method).to_string(),
    query_parameters,
    path_parameters,
    request_body: request_body_type.map(|request_body_type| request_body_type.to_string()),
    responses,
  }
}

pub fn open_api_to_typescript(open_api: OpenAPI) -> OpenApiOutput {
  let components: Vec<OpenApiComponent> = open_api
    .components
    .unwrap_or_default()
    .schemas
    .iter()
    .map(|(name, schema)| OpenApiComponent {
      name: name.clone(),
      ts_type: crate::json_schema_to_typescript::schema_to_typescript(schema.clone(), None)
        .to_string(),
    })
    .collect();

  let paths: Vec<OpenApiPath> = open_api
    .paths
    .iter()
    .flat_map(|(path, path_item_ref)| {
      let path_item = match path_item_ref {
        ReferenceOr::Item(path_item) => path_item,
        ReferenceOr::Reference { reference } => {
          panic!("Reference not implemented for path: {}", reference);
        }
      };

      vec![
        (OpenApiMethod::Get, &path_item.get),
        (OpenApiMethod::Put, &path_item.put),
        (OpenApiMethod::Post, &path_item.post),
        (OpenApiMethod::Delete, &path_item.delete),
        (OpenApiMethod::Patch, &path_item.patch),
        (OpenApiMethod::Options, &path_item.options),
      ]
      .into_iter()
      .filter_map(|(method, operation)| {
        operation
          .as_ref()
          .map(|op| get_open_api_path(path, method, op))
      })
    })
    .collect();

  OpenApiOutput { paths, components }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn create_openapi_json(paths_json: &str) -> OpenAPI {
    let base = json!({
      "openapi": "3.0.0",
      "info": {
        "title": "Test API",
        "version": "1.0.0"
      }
    });

    let paths: serde_json::Value = serde_json::from_str(paths_json).unwrap();

    let complete_json = json!({
      "openapi": base["openapi"],
      "info": base["info"],
      "paths": paths
    });

    serde_json::from_value(complete_json).unwrap()
  }

  #[test]
  fn test_basic_get() {
    let openapi = create_openapi_json(
      r#"{
      "/test": {
        "get": {
          "responses": {
            "200": {
              "description": "Success response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "message": {
                        "type": "string"
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }"#,
    );
    let result = open_api_to_typescript(openapi);

    assert_eq!(result.paths.len(), 1);
    assert_eq!(result.paths[0].method, "get");
    assert_eq!(result.paths[0].path, "/test");
    assert!(result.paths[0].responses.contains_key("200"));
  }

  #[test]
  fn test_with_query_params() {
    let openapi = create_openapi_json(
      r#"{
      "/search": {
        "get": {
          "parameters": [
            {
              "in": "query",
              "name": "search",
              "required": true,
              "schema": {
                "type": "string"
              }
            },
            {
              "in": "query",
              "name": "limit",
              "required": false,
              "schema": {
                "type": "integer"
              }
            }
          ],
          "responses": {
            "200": {
              "description": "Success response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object"
                  }
                }
              }
            }
          }
        }
      }
    }"#,
    );
    let result = open_api_to_typescript(openapi);

    assert_eq!(
      result.paths[0].query_parameters,
      Some("{\n  limit?: number;\n  search: string;\n}".to_string())
    );
    assert_eq!(result.paths[0].method, "get");
    assert_eq!(result.paths[0].path, "/search");
    assert!(result.paths[0].responses.contains_key("200"));
  }

  #[test]
  fn test_with_path_params() {
    let openapi = create_openapi_json(
      r#"{
      "/users/{id}": {
        "delete": {
          "parameters": [
            {
              "in": "path",
              "name": "id",
              "required": true,
              "schema": {
                "type": "integer"
              }
            }
          ],
          "responses": {
            "204": {
              "description": "User deleted successfully"
            }
          }
        }
      }
    }"#,
    );
    let result = open_api_to_typescript(openapi);

    assert_eq!(
      result.paths[0].path_parameters,
      Some("{\n  id: number;\n}".to_string())
    );
    assert_eq!(result.paths[0].method, "delete");
    assert_eq!(result.paths[0].path, "/users/{id}");
  }

  #[test]
  fn test_with_request_body() {
    let openapi = create_openapi_json(
      r#"{
      "/users": {
        "post": {
          "requestBody": {
            "required": true,
            "content": {
              "application/json": {
                "schema": {
                  "type": "object",
                  "properties": {
                    "name": {
                      "type": "string"
                    },
                    "email": {
                      "type": "string"
                    }
                  },
                  "required": ["name", "email"]
                }
              }
            }
          },
          "responses": {
            "201": {
              "description": "User created successfully",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "id": {
                        "type": "integer"
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }"#,
    );
    let result = open_api_to_typescript(openapi);

    assert!(result.paths[0].request_body.is_some());
    assert_eq!(result.paths[0].method, "post");
    assert_eq!(result.paths[0].path, "/users");
    assert!(result.paths[0].responses.contains_key("201"));
  }

  #[test]
  fn test_multiple_responses() {
    let openapi = create_openapi_json(
      r#"{
      "/users/{id}": {
        "patch": {
          "parameters": [
            {
              "in": "path",
              "name": "id",
              "required": true,
              "schema": {
                "type": "integer"
              }
            }
          ],
          "requestBody": {
            "required": true,
            "content": {
              "application/json": {
                "schema": {
                  "type": "object",
                  "properties": {
                    "name": {
                      "type": "string"
                    }
                  }
                }
              }
            }
          },
          "responses": {
            "200": {
              "description": "Success response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "id": {
                        "type": "integer"
                      },
                      "name": {
                        "type": "string"
                      }
                    }
                  }
                }
              }
            },
            "400": {
              "description": "Bad request",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "error": {
                        "type": "string"
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }"#,
    );
    let result = open_api_to_typescript(openapi);

    assert_eq!(result.paths[0].responses.len(), 2);
    assert!(result.paths[0].responses.contains_key("200"));
    assert!(result.paths[0].responses.contains_key("400"));
    assert_eq!(result.paths[0].method, "patch");
  }
}
