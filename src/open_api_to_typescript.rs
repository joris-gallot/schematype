use crate::json_schema_to_typescript::TypeInterface;
use std::collections::HashMap;

use napi_derive::napi;
use openapiv3::{
  OpenAPI, Operation, Parameter, ParameterData, ParameterSchemaOrContent, ReferenceOr, Schema,
};
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
  pub summary: Option<String>,
  pub description: Option<String>,
  pub query_parameters: HashMap<String, OpenApiParameter>,
  pub path_parameters: HashMap<String, OpenApiParameter>,
  pub request_body: Option<String>,
  pub responses: HashMap<String, OpenApiResponse>,
}

#[derive(Debug)]
#[napi(object)]
pub struct OpenApiParameter {
  pub description: Option<String>,
  pub required: bool,
  pub ts_type: String,
}

#[derive(Debug)]
#[napi(object)]
pub struct OpenApiResponse {
  pub description: String,
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

  fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
      Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
      None => String::new(),
    }
  }

  fn get_interface_name(path: &str, method: &OpenApiMethod) -> String {
    format!(
      "{}{}",
      OpenApiOutput::capitalize_first(OpenApiOutput::open_api_method_to_string(method)),
      path
        .split("/")
        .filter(|s| !s.is_empty())
        .map(|s| s.replace("{", "").replace("}", ""))
        .map(|s| s[0..1].to_uppercase() + &s[1..])
        .collect::<String>()
    )
  }
}

fn get_open_api_parameter(parameter_data: &ParameterData) -> (String, OpenApiParameter) {
  let name = &parameter_data.name;

  (
    name.clone(),
    OpenApiParameter {
      description: parameter_data.description.clone(),
      required: parameter_data.required,
      ts_type: crate::json_schema_to_typescript::schema_to_typescript(
        OpenApiOutput::capitalize_first(name),
        match &parameter_data.format {
          ParameterSchemaOrContent::Schema(schema) => schema.clone(),
          ParameterSchemaOrContent::Content(_) => panic!("Content not implemented"),
        },
        None,
      )
      .to_string(),
    },
  )
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
    crate::json_schema_to_typescript::schema_to_typescript(
      format!("{}Body", OpenApiOutput::get_interface_name(path, &method)),
      request_body.clone(),
      None,
    )
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

  let query_parameters: HashMap<String, OpenApiParameter> = parameters
    .iter()
    .filter_map(|parameter| {
      if let Parameter::Query { parameter_data, .. } = parameter {
        Some(get_open_api_parameter(parameter_data))
      } else {
        None
      }
    })
    .collect();

  let path_parameters: HashMap<String, OpenApiParameter> = parameters
    .iter()
    .filter_map(|parameter| {
      if let Parameter::Path { parameter_data, .. } = parameter {
        Some(get_open_api_parameter(parameter_data))
      } else {
        None
      }
    })
    .collect();

  let responses: HashMap<String, OpenApiResponse> = operation
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

      let res_schema_interface = crate::json_schema_to_typescript::schema_to_typescript(
        format!(
          "{}Response",
          OpenApiOutput::get_interface_name(path, &method)
        ),
        res_schema.clone(),
        None,
      );

      Some((
        status_code.to_string(),
        OpenApiResponse {
          description: res.description.clone(),
          ts_type: res_schema_interface.to_string(),
        },
      ))
    })
    .collect();

  OpenApiPath {
    path: path.to_string(),
    method: OpenApiOutput::open_api_method_to_string(&method).to_string(),
    summary: operation.summary.clone(),
    description: operation.description.clone(),
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
      ts_type: crate::json_schema_to_typescript::schema_to_typescript(
        name.clone(),
        schema.clone(),
        None,
      )
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
      .filter_map(move |(method, operation)| {
        operation
          .as_ref()
          .map(|op| get_open_api_path(path, method, op))
      })
    })
    .collect();

  OpenApiOutput { paths, components }
}
