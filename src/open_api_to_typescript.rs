use crate::json_schema_to_typescript::TypeInterface;
use openapiv3::{OpenAPI, Operation, ReferenceOr, Schema, StatusCode};
use std::fmt;

#[derive(Debug)]
enum OpenApiMethod {
  Get,
  Post,
  Put,
  Delete,
  Patch,
  Options,
}

#[derive(Debug)]
struct OpenApiClient {
  paths: Vec<OpenApiPath>,
}

impl OpenApiClient {
  fn open_api_method_to_string(method: &OpenApiMethod) -> String {
    match method {
      OpenApiMethod::Get => "Get".to_string(),
      OpenApiMethod::Post => "Post".to_string(),
      OpenApiMethod::Put => "Put".to_string(),
      OpenApiMethod::Delete => "Delete".to_string(),
      OpenApiMethod::Patch => "Patch".to_string(),
      OpenApiMethod::Options => "Options".to_string(),
    }
  }

  fn get_interface_name(path: &str, method: &OpenApiMethod) -> String {
    format!(
      "{}{}",
      OpenApiClient::open_api_method_to_string(method),
      path
        .split("/")
        .filter(|s| !s.is_empty())
        .map(|s| s.replace("{", "").replace("}", ""))
        .map(|s| s[0..1].to_uppercase() + &s[1..])
        .collect::<String>()
    )
  }
}

impl fmt::Display for OpenApiClient {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let result = self
      .paths
      .iter()
      .flat_map(|path| {
        let request_body_interface: Option<TypeInterface> = match &path.request_body {
          Some(request_body) => {
            let request_interface_name = format!(
              "{}Body",
              OpenApiClient::get_interface_name(&path.path, &path.method)
            );

            Some(crate::json_schema_to_typescript::schema_to_typescript(
              request_interface_name,
              request_body.clone(),
              None,
            ))
          }
          None => None,
        };

        let responses_interaces = path
          .responses
          .iter()
          .map(|response| {
            let interface_name = format!(
              "{}Response",
              OpenApiClient::get_interface_name(&path.path, &path.method)
            );
            let interface = crate::json_schema_to_typescript::schema_to_typescript(
              interface_name,
              response.schema.clone(),
              None,
            );

            interface.to_string()
          })
          .collect::<Vec<String>>()
          .join("\n");

        Some(format!(
          "{} {}\nPayload: {}\nResponses: {}\n",
          OpenApiClient::open_api_method_to_string(&path.method),
          path.path,
          request_body_interface
            .map_or_else(|| "None".to_string(), |interface| interface.to_string()),
          if responses_interaces.is_empty() {
            "None".to_string()
          } else {
            responses_interaces
          }
        ))
      })
      .collect::<Vec<String>>()
      .join("\n");

    write!(f, "{}", result)
  }
}

#[derive(Debug)]
struct OpenApiPath {
  path: String,
  method: OpenApiMethod,
  #[allow(dead_code)] // TODO: remove this
  summary: Option<String>,
  #[allow(dead_code)] // TODO: remove this
  description: Option<String>,
  // parameters: Vec<OpenApiParameter>,
  request_body: Option<ReferenceOr<Schema>>,
  responses: Vec<OpenApiResponse>,
}

#[derive(Debug)]
struct OpenApiResponse {
  #[allow(dead_code)] // TODO: remove this
  status_code: StatusCode,
  #[allow(dead_code)] // TODO: remove this
  description: String,
  schema: ReferenceOr<Schema>,
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

  let responses: Vec<OpenApiResponse> = operation
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

      Some(OpenApiResponse {
        status_code: status_code.clone(),
        description: res.description.clone(),
        schema: res_schema.clone(),
      })
    })
    .collect();

  OpenApiPath {
    path: path.to_string(),
    method,
    summary: operation.summary.clone(),
    description: operation.description.clone(),
    request_body,
    responses,
  }
}

pub fn open_api_to_typescript(open_api: OpenAPI) {
  let mut client = OpenApiClient { paths: vec![] };

  for (path, path_item_ref) in open_api.paths.iter() {
    let path_item = match path_item_ref {
      ReferenceOr::Item(path_item) => path_item,
      ReferenceOr::Reference { reference } => {
        panic!("Reference not implemented for path: {}", reference);
      }
    };

    if let Some(path_item) = &path_item.get {
      client
        .paths
        .push(get_open_api_path(path, OpenApiMethod::Get, path_item));
    }

    if let Some(path_item) = &path_item.put {
      client
        .paths
        .push(get_open_api_path(path, OpenApiMethod::Put, path_item));
    }

    if let Some(path_item) = &path_item.post {
      client
        .paths
        .push(get_open_api_path(path, OpenApiMethod::Post, path_item));
    }

    if let Some(path_item) = &path_item.delete {
      client
        .paths
        .push(get_open_api_path(path, OpenApiMethod::Delete, path_item));
    }

    if let Some(path_item) = &path_item.patch {
      client
        .paths
        .push(get_open_api_path(path, OpenApiMethod::Patch, path_item));
    }

    if let Some(path_item) = &path_item.options {
      client
        .paths
        .push(get_open_api_path(path, OpenApiMethod::Options, path_item));
    }
  }

  println!("{}", client);
}
