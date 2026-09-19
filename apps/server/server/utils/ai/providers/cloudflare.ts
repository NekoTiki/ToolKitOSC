import { detectClusterHints } from '../clustering'
import { toStrictJsonSchema } from '../jsonSchema'
import type { PromptProfile } from '../profiles'
import { resolveModel } from '../profiles'
import { buildControlSuggestionSchema, buildSystemPrompt, buildUserPrompt } from '../prompt'
import { throwOpenAiCompatibleError, tryRecoverFailedGeneration } from '../providerError'
import type { AiParameterInput, AiProvider, AiSuggestionResult } from '../types'

// Workers AI's REST API is scoped under a Cloudflare account, not just an API token - the OpenAI-
// compatible endpoint needs both (see NUXT_CLOUDFLARE_ACCOUNT_ID in .env.example). isConfigured()
// requires both, so this just reports itself unavailable (same as a provider missing its API key
// entirely) rather than failing at request time, until an account id is added too.
export function createCloudflareProvider(
  apiKey: string,
  accountId: string,
  primaryModel: string
): AiProvider {
  return {
    id: 'cloudflare',
    label: 'Cloudflare Workers AI',
    isConfigured: () => !!apiKey && !!accountId,
    resolveModelForProfile: (profileId) => resolveModel('cloudflare', profileId, primaryModel),

    async suggestControlGroups(
      avatarName: string,
      parameters: AiParameterInput[],
      profile: PromptProfile
    ): Promise<AiSuggestionResult> {
      if (!apiKey || !accountId) {
        throw new Error(
          'Cloudflare Workers AI is not configured (missing NUXT_CLOUDFLARE_API_KEY or NUXT_CLOUDFLARE_ACCOUNT_ID)'
        )
      }

      const model = resolveModel('cloudflare', profile.id, primaryModel)
      const clusterHints = profile.useClusterHints ? detectClusterHints(parameters) : []
      const schema = toStrictJsonSchema(buildControlSuggestionSchema(profile))

      const response = await fetch(
        `https://api.cloudflare.com/client/v4/accounts/${accountId}/ai/v1/chat/completions`,
        {
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
        }
      )

      if (!response.ok) {
        const body = await response.text()
        const recovered = tryRecoverFailedGeneration(body)

        if (recovered) {
          console.warn('[ai] Cloudflare Workers AI rejected structured output as invalid, but a usable generation was recovered from it')

          return recovered
        }

        throwOpenAiCompatibleError('Cloudflare Workers AI', response.status, body)
      }

      const data = (await response.json()) as {
        choices?: { message?: { content?: string } }[]
      }
      const content = data.choices?.[0]?.message?.content

      if (!content) throw new Error('Cloudflare Workers AI returned no content')

      return JSON.parse(content) as AiSuggestionResult
    }
  }
}
