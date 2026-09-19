import { detectClusterHints } from '../clustering'
import { toStrictJsonSchema } from '../jsonSchema'
import type { PromptProfile } from '../profiles'
import { resolveModel } from '../profiles'
import { buildControlSuggestionSchema, buildSystemPrompt, buildUserPrompt } from '../prompt'
import { throwOpenAiCompatibleError, tryRecoverFailedGeneration } from '../providerError'
import type { AiParameterInput, AiProvider, AiSuggestionResult } from '../types'

const OPENROUTER_URL = 'https://openrouter.ai/api/v1/chat/completions'

// OpenAI-compatible, same request shape as Groq (see providers/groq.ts) - OpenRouter proxies many
// underlying models, so json_schema strict-mode support isn't guaranteed for every model it
// offers, only for the one actually configured here (NUXT_OPENROUTER_MODEL).
export function createOpenRouterProvider(apiKey: string, primaryModel: string): AiProvider {
  return {
    id: 'openrouter',
    label: 'OpenRouter',
    isConfigured: () => !!apiKey,
    resolveModelForProfile: (profileId) => resolveModel('openrouter', profileId, primaryModel),

    async suggestControlGroups(
      avatarName: string,
      parameters: AiParameterInput[],
      profile: PromptProfile
    ): Promise<AiSuggestionResult> {
      if (!apiKey) throw new Error('OpenRouter is not configured (missing NUXT_OPENROUTER_API_KEY)')

      const model = resolveModel('openrouter', profile.id, primaryModel)
      const clusterHints = profile.useClusterHints ? detectClusterHints(parameters) : []
      const schema = toStrictJsonSchema(buildControlSuggestionSchema(profile))

      const response = await fetch(OPENROUTER_URL, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${apiKey}`,
          // OpenRouter asks for these on every request for their own analytics/attribution - not
          // load-bearing for the API call itself, just good citizenship.
          'HTTP-Referer': 'https://github.com/NekoTiki/VRC-OSC-Toolkit',
          'X-Title': 'VRC OSC Toolkit'
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
          console.warn('[ai] OpenRouter rejected structured output as invalid, but a usable generation was recovered from it')

          return recovered
        }

        throwOpenAiCompatibleError('OpenRouter', response.status, body)
      }

      const data = (await response.json()) as {
        choices?: { message?: { content?: string } }[]
      }
      const content = data.choices?.[0]?.message?.content

      if (!content) throw new Error('OpenRouter returned no content')

      return JSON.parse(content) as AiSuggestionResult
    }
  }
}
