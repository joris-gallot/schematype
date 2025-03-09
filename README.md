# schematype

Convert **JSON Schema** and **OpenAPI v3.0** schemas into **TypeScript type declarations** at **Rust speed**.

This library is powered by Rust and leverages [`openapiv3`](https://crates.io/crates/openapiv3) for schema parsing. **Currently, only OpenAPI v3.0 is supported**.

## Installation

```sh
npm install @schematype/core
```

## Usage

This package supports **CommonJS** and **ES Modules**.

### **Example**

```typescript
import { schemaToType } from "@schematype/core";

const schema = {
  type: "object",
  properties: {
    id: { type: "string" },
    age: { type: "number" }
  },
  required: ["id"]
};

const tsType = schemaToType("User", schema);
console.log(tsType);
```

**Output:**
```ts
export type User = {
  id: string;
  age?: number;
};
```

### Options

```ts
{
  preferUnknownOverAny: boolean    // default to false
  preferInterfaceOverType: boolean // default to false
}
```


## OpenAPI to Typescript types

You can also convert OpenAPI v3.0 schemas to types:

```typescript
import { openApiToTypes } from "@schematype/core";

const openapi = {
  openapi: "3.0.0",
  info: {
    title: "Test API",
    version: "1.0.0"
  },
  paths: {
    "/users/{id}": {
      get: {
        parameters: [
          {
            name: "id",
            in: "path",
            required: true,
            schema: {
              type: "string"
            }
          }
        ],
        responses: {
          "200": {
            description: "Success response",
            content: {
              "application/json": {
                schema: {
                  type: "object",
                  properties: {
                    id: { type: "string" },
                    name: { type: "string" }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
};

const result = openApiToTypes(openapi);
console.log(result);
```

**Output:**
```ts
{
  "paths": [
    {
      "path": "/users/{id}",
      "method": "get",
      "pathParameters": "export type GetUsersIdPath = {\n  id: string;\n};",
      "responses": {
        "200": "export type GetUsersIdResponse = {\n  id?: string;\n  name?: string;\n};"
      }
    }
  ],
  "components": []
}
```

The `openApiToTypes` function returns an object containing:
- `paths`: Array of path objects containing TypeScript types for:
  - Query parameters
  - Path parameters
  - Request body
  - Responses
- `components`: Array of reusable schema components converted to TypeScript types



## Supported Features for JSON Schema

### Basic Types
- `string`
- `number`
- `integer` (converted to TypeScript `number`)
- `boolean`
- `null`
- `array`
- `object`

### Composition
- `anyOf` - Converted to TypeScript union types (`|`)
- `oneOf` - Converted to TypeScript union types (`|`)
- `allOf` - Converted to TypeScript intersection types (`&`)

### Object Properties
- Required properties
- Optional properties
- Nested objects
- Property descriptions (as JSDoc comments)
- Deprecated properties (marked with `@deprecated` in JSDoc)

### Arrays
- Simple arrays of primitive types
- Arrays of objects
- Arrays with `anyOf`/`oneOf`/`allOf`
- Nested arrays
- Multi-dimensional arrays

### Enums
- String enums
- Numeric enums (both integer and number)
- Boolean enums
- Mixed type enums via `anyOf`/`oneOf`

### References
- Schema references (`$ref`)
- Nested references in arrays and objects

## Contributing
PRs are welcome! Feel free to contribute to improve schema parsing and support for OpenAPI v3.1.

## License
MIT

