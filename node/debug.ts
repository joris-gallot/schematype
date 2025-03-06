/* eslint-disable no-console */
import { schemaToType } from "../index.js";

const schema = {
  type: "object",
  properties: {
    id: { type: "string" },
    age: { type: "number" }
  },
  required: ["id"]
};

const options = {
  preferInterfaceOverType: true,
  preferUnknownOverAny: false
}

const tsType = schemaToType("User", schema, options);
console.log(tsType);
