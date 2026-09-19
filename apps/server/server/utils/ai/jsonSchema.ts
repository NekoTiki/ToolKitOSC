// OpenAI-compatible strict json_schema mode (Groq, OpenRouter) requires every property listed
// under `required` and `additionalProperties: false` at every object level, unlike Gemini's
// responseSchema which accepts the schema as authored. Rather than hand-maintaining a second,
// stricter copy of buildControlSuggestionSchema (which is exactly how the two schemas drifted out
// of sync once before), this mechanically derives the strict version from the shared base schema.
type JsonSchemaNode = { type?: string; properties?: Record<string, JsonSchemaNode>; items?: JsonSchemaNode } & Record<
  string,
  unknown
>

export function toStrictJsonSchema<T extends JsonSchemaNode>(schema: T): T {
  if (schema.type === 'object' && schema.properties) {
    const properties: Record<string, JsonSchemaNode> = {}

    for (const [key, value] of Object.entries(schema.properties)) {
      properties[key] = toStrictJsonSchema(value)
    }

    return {
      ...schema,
      properties,
      required: Object.keys(schema.properties),
      additionalProperties: false
    }
  }

  if (schema.type === 'array' && schema.items) {
    return { ...schema, items: toStrictJsonSchema(schema.items) }
  }

  return schema
}
