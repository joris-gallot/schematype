# Snapshot report for `node/tests/openapi_to_types.test.mjs`

The actual snapshot is saved in `openapi_to_types.test.mjs.snap`.

Generated by [AVA](https://avajs.dev).

## basic openapi input

> Snapshot 1

    {
      components: [
        {
          name: 'User',
          tsType: `{␊
            age?: number;␊
            id?: string;␊
            name?: string;␊
          }`,
        },
      ],
      paths: [
        {
          method: 'get',
          path: '/users',
          responses: {
            200: `{␊
              age?: number;␊
              id?: string;␊
              name?: string;␊
            }`,
          },
        },
        {
          method: 'post',
          path: '/users',
          requestBody: 'User',
          responses: {},
        },
      ],
    }
