import { detectClusterHints } from '../clustering'
import { toStrictJsonSchema } from '../jsonSchema'
import type { PromptProfile } from '../profiles'
import { resolveModel } from '../profiles'
import { buildControlSuggestionSchema, buildSystemPrompt, buildUserPrompt } from '../prompt'
import { throwOpenAiCompatibleError, tryRecoverFailedGeneration } from '../providerError'
import type { AiParameterInput, AiProvider, AiSuggestionResult } from '../types'

const GROQ_URL = 'https://api.groq.com/openai/v1/chat/completions'

export function createGroqProvider(apiKey: string, primaryModel: string): AiProvider {
  return {
    id: 'groq',
    label: 'Groq',
    isConfigured: () => !!apiKey,
    resolveModelForProfile: (profileId) => resolveModel('groq', profileId, primaryModel),

    async suggestControlGroups(
      avatarName: string,
      parameters: AiParameterInput[],
      profile: PromptProfile
    ): Promise<AiSuggestionResult> {
      if (!apiKey) throw new Error('Groq is not configured (missing NUXT_GROQ_API_KEY)')

      const model = resolveModel('groq', profile.id, primaryModel)
      const clusterHints = profile.useClusterHints ? detectClusterHints(parameters) : []
      const schema = toStrictJsonSchema(buildControlSuggestionSchema(profile))

      const response = await fetch(GROQ_URL, {
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
          // Left at the API default (much too low for a few dozen grouped controls plus this
          // model's reasoning tokens), a real parameter list gets silently truncated mid-response -
          // strict-schema decoding still closes it out into syntactically valid but near-empty
          // JSON, so there's no error to catch, just a suspiciously small result. Verified against
          // a real 79-parameter avatar. Budget is now per-profile (see profiles.ts) since the
          // richer schema needs more headroom the more control types a profile allows.
          max_completion_tokens: profile.maxCompletionTokens,
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
          console.warn('[ai] Groq rejected structured output as invalid, but a usable generation was recovered from it')

          return recovered
        }

        throwOpenAiCompatibleError('Groq', response.status, body)
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
