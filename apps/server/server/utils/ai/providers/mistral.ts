import { detectClusterHints } from '../clustering'
import { toStrictJsonSchema } from '../jsonSchema'
import type { PromptProfile } from '../profiles'
import { resolveModel } from '../profiles'
import { buildControlSuggestionSchema, buildSystemPrompt, buildUserPrompt } from '../prompt'
import { throwOpenAiCompatibleError, tryRecoverFailedGeneration } from '../providerError'
import type { AiParameterInput, AiProvider, AiSuggestionResult } from '../types'

const MISTRAL_URL = 'https://api.mistral.ai/v1/chat/completions'

// OpenAI-compatible, same request/response shape as Groq (see providers/groq.ts).
export function createMistralProvider(apiKey: string, primaryModel: string): AiProvider {
  return {
    id: 'mistral',
    label: 'Mistral',
    isConfigured: () => !!apiKey,
    resolveModelForProfile: (profileId) => resolveModel('mistral', profileId, primaryModel),

    async suggestControlGroups(
      avatarName: string,
      parameters: AiParameterInput[],
      profile: PromptProfile
    ): Promise<AiSuggestionResult> {
      if (!apiKey) throw new Error('Mistral is not configured (missing NUXT_MISTRAL_API_KEY)')

      const model = resolveModel('mistral', profile.id, primaryModel)
      const clusterHints = profile.useClusterHints ? detectClusterHints(parameters) : []
      const schema = toStrictJsonSchema(buildControlSuggestionSchema(profile))

      const response = await fetch(MISTRAL_URL, {
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
        const recovered = tryRecoverFailedGeneration(body)

        if (recovered) {
          console.warn('[ai] Mistral rejected structured output as invalid, but a usable generation was recovered from it')

          return recovered
        }

        throwOpenAiCompatibleError('Mistral', response.status, body)
      }

      const data = (await response.json()) as {
        choices?: { message?: { content?: string } }[]
      }
      const content = data.choices?.[0]?.message?.content

      if (!content) throw new Error('Mistral returned no content')

      return JSON.parse(content) as AiSuggestionResult
    }
  }
}
