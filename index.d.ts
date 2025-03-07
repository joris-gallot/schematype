/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export interface SchemaTypeOptions {
  preferUnknownOverAny: boolean
  preferInterfaceOverType: boolean
}
export interface OpenApiOutput {
  paths: Array<OpenApiPath>
  components: Array<OpenApiComponent>
}
export interface OpenApiComponent {
  name: string
  tsType: string
}
export interface OpenApiPath {
  path: string
  method: string
  summary?: string
  description?: string
  queryTsType?: string
  pathTsType?: string
  requestBody?: string
  responses: Record<string, OpenApiResponse>
}
export interface OpenApiParameter {
  description?: string
  required: boolean
  tsType: string
}
export interface OpenApiResponse {
  description: string
  tsType: string
}
export declare function openApiToTypes(openApiInput: object): OpenApiOutput
export declare function schemaToType(name: string, schemaInput: object, options?: SchemaTypeOptions | undefined | null): string
