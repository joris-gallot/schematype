import antfu from '@antfu/eslint-config'

export default antfu({
  typescript: true,
  ignores: ['**/__snapshots__/**', 'index.js'],
  rules: {
    'test/consistent-test-it': 'off',
  },
})
