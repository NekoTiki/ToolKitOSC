import { buildUserPrompt, SYSTEM_PROMPT } from '../prompt'
import type { AiParameterInput, AiProvider, AiSuggestionResult } from '../types'

const GROQ_URL = 'https://api.groq.com/openai/v1/chat/completions'

// Groq's strict json_schema mode requires every property listed under `required` and
// `additionalProperties: false` at every object level (unlike Gemini's responseSchema) - adapted
// from the shared base schema here rather than baking Groq-specific quirks into it.
const strictSchema = {
  type: 'object',
  properties: {
    groups: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          name: { type: 'string' },
          controls: {
            type: 'array',
            items: {
              type: 'object',
              properties: {
                name: { type: 'string' },
                address: { type: 'string' },
                type: { type: 'string', enum: ['boolean', 'slider', 'step-enum'] },
                options: {
                  type: 'array',
                  items: {
                    type: 'object',
                    properties: { name: { type: 'string' }, value: { type: 'number' } },
                    required: ['name', 'value'],
                    additionalProperties: false
                  }
                }
              },
              required: ['name', 'address', 'type', 'options'],
              additionalProperties: false
            }
          }
        },
        required: ['name', 'controls'],
        additionalProperties: false
      }
    }
  },
  required: ['groups'],
  additionalProperties: false
} as const

export function createGroqProvider(apiKey: string, model: string): AiProvider {
  return {
    id: 'groq',
    label: 'Groq',
    isConfigured: () => !!apiKey,

    async suggestControlGroups(
      avatarName: string,
      parameters: AiParameterInput[]
    ): Promise<AiSuggestionResult> {
      if (!apiKey) throw new Error('Groq is not configured (missing NUXT_GROQ_API_KEY)')

      const response = await fetch(GROQ_URL, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${apiKey}`
        },
        body: JSON.stringify({
          model,
          messages: [
            { role: 'system', content: SYSTEM_PROMPT },
            { role: 'user', content: buildUserPrompt(avatarName, parameters) }
          ],
          // Left at the API default (much too low for a few dozen grouped controls plus this
          // model's reasoning tokens), a real parameter list gets silently truncated mid-response -
          // strict-schema decoding still closes it out into syntactically valid but near-empty
          // JSON, so there's no error to catch, just a suspiciously small result. Verified against
          // a real 79-parameter avatar: default budget -> 1 group/19 controls (finish_reason
          // 'length'), this budget -> 9 groups/65 controls (finish_reason 'stop').
          max_completion_tokens: 8000,
          response_format: {
            type: 'json_schema',
            json_schema: { name: 'control_groups', strict: true, schema: strictSchema }
          }
        })
      })

      if (!response.ok) {
        const body = await response.text()
        throw new Error(`Groq request failed (${response.status}): ${body}`)
      }

      const data = (await response.json()) as {
        choices?: { message?: { content?: string } }[]
      }
      const content = data.choices?.[0]?.message?.content

      if (!content) throw new Error('Groq returned no content')

      return JSON.parse(content) as AiSuggestionResult
    }
  }
}
