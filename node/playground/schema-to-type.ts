/* eslint-disable no-console */
import { schemaToType } from "../../index.js";

const schema = {
  type: "object",
  properties: {
    id: { type: "string" },
    age: { type: "number" }
  },
  required: ["id"]
};

const tsType = schemaToType(schema, { name: "User" });
console.log(tsType);
