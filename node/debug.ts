/* eslint-disable no-console */
import { schemaToTypescript } from '../index.js'

const schema = {
  type: 'object',
  properties: {
    id: { type: 'string' },
    metadata: {
      type: 'object',
      properties: {
        title: { type: 'string' },
        tags: {
          type: 'array',
          items: {
            anyOf: [
              { type: 'string' },
              {
                type: 'object',
                properties: {
                  name: { type: 'string' },
                  value: { type: 'number' },
                  metadata: {
                    type: 'object',
                    properties: {
                      description: { type: 'string' },
                      priority: { type: 'number' },
                    },
                    required: ['description'],
                  },
                },
                required: ['name', 'value'],
              },
            ],
          },
        },
      },
      required: ['title', 'tags'],
    },
  },
  required: ['id', 'metadata'],
}

const result = schemaToTypescript('MySchema', schema)

console.log(result)
