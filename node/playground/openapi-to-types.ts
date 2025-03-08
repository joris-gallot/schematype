/* eslint-disable no-console */
import { openApiToTypes } from "../../index.js";
import openApiExample from "../../openapi_example.json";

const openApi = openApiToTypes(openApiExample);

console.log(JSON.stringify(openApi, null, 2));
