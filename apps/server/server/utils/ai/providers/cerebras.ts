import { detectClusterHints } from '../clustering'
import { toStrictJsonSchema } from '../jsonSchema'
import type { PromptProfile } from '../profiles'
import { resolveModel } from '../profiles'
import { buildControlSuggestionSchema, buildSystemPrompt, buildUserPrompt } from '../prompt'
import type { AiParameterInput, AiProvider, AiSuggestionResult } from '../types'

const CEREBRAS_URL = 'https://api.cerebras.ai/v1/chat/completions'

// OpenAI-compatible, same request shape as Groq (see providers/groq.ts) - Cerebras' main draw is
// inference speed, not model variety, so the default model is deliberately a plain, broadly-
// capable one rather than anything exotic.
export function createCerebrasProvider(apiKey: string, primaryModel: string): AiProvider {
  return {
    id: 'cerebras',
    label: 'Cerebras',
    isConfigured: () => !!apiKey,

    async suggestControlGroups(
      avatarName: string,
      parameters: AiParameterInput[],
      profile: PromptProfile
    ): Promise<AiSuggestionResult> {
      if (!apiKey) throw new Error('Cerebras is not configured (missing NUXT_CEREBRAS_API_KEY)')

      const model = resolveModel('cerebras', profile.id, primaryModel)
      const clusterHints = profile.useClusterHints ? detectClusterHints(parameters) : []
      const schema = toStrictJsonSchema(buildControlSuggestionSchema(profile))

      const response = await fetch(CEREBRAS_URL, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${apiKey}`
        },
        body: JSON.stringify({
          model,
          messages: [
            { role: 'system', content: buildSystemPrompt(profile, clusterHints) },
            { role: 'user', content: buildUserPrompt(avatarName, parameters) }
          ],
          max_tokens: profile.maxCompletionTokens,
          response_format: {
            type: 'json_schema',
            json_schema: { name: 'control_groups', strict: true, schema }
          }
        })
      })

      if (!response.ok) {
        const body = await response.text()
        throw new Error(`Cerebras request failed (${response.status}): ${body}`)
      }

      const data = (await response.json()) as {
        choices?: { message?: { content?: string } }[]
      }
      const content = data.choices?.[0]?.message?.content

      if (!content) throw new Error('Cerebras returned no content')

      return JSON.parse(content) as AiSuggestionResult
    }
  }
}
