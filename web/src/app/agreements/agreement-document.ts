// OpenAPI object fields retain extensions while structured editors replace one field.
export function object(value: unknown): Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? value as Record<string, unknown> : {}
}
export function operations(value: unknown): { key: string; label: string }[] {
  return Object.entries(object(object(value).paths)).flatMap(([path, item]) =>
    ['get', 'put', 'post', 'delete', 'options', 'head', 'patch', 'trace'].flatMap(method => {
      const op = object(object(item)[method]), key = op['x-shadows-operation-id']
      return typeof key === 'string' ? [{ key, label: `${method.toUpperCase()} ${path}` }] : []
    }))
}
