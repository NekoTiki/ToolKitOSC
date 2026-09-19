import { detectClusterHints } from '../clustering'
import type { PromptProfile } from '../profiles'
import { resolveModel } from '../profiles'
import { buildControlSuggestionSchema, buildSystemPrompt, buildUserPrompt } from '../prompt'
import { throwGeminiError } from '../providerError'
import type { AiParameterInput, AiProvider, AiSuggestionResult } from '../types'

const GEMINI_URL = 'https://generativelanguage.googleapis.com/v1beta/models'

export function createGeminiProvider(apiKey: string, primaryModel: string): AiProvider {
  return {
    id: 'gemini',
    label: 'Google Gemini',
    isConfigured: () => !!apiKey,
    resolveModelForProfile: (profileId) => resolveModel('gemini', profileId, primaryModel),

    async suggestControlGroups(
      avatarName: string,
      parameters: AiParameterInput[],
      profile: PromptProfile
    ): Promise<AiSuggestionResult> {
      if (!apiKey) throw new Error('Gemini is not configured (missing NUXT_GEMINI_API_KEY)')

      const model = resolveModel('gemini', profile.id, primaryModel)
      const clusterHints = profile.useClusterHints ? detectClusterHints(parameters) : []

      const response = await fetch(`${GEMINI_URL}/${model}:generateContent?key=${apiKey}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          systemInstruction: { parts: [{ text: buildSystemPrompt(profile, clusterHints) }] },
          contents: [{ role: 'user', parts: [{ text: buildUserPrompt(avatarName, parameters) }] }],
          generationConfig: {
            responseMimeType: 'application/json',
            responseSchema: buildControlSuggestionSchema(profile),
            // See providers/groq.ts's identical comment - a real parameter list needs headroom
            // well past whatever a short default budget would give it, or the response comes back
            // silently truncated (schema-valid but missing most of the data) instead of erroring.
            maxOutputTokens: profile.maxCompletionTokens
          }
        })
      })

      if (!response.ok) {
        const body = await response.text()
        throwGeminiError(response.status, body)
      }

      const data = (await response.json()) as {
        candidates?: { content?: { parts?: { text?: string }[] } }[]
      }
      const content = data.candidates?.[0]?.content?.parts?.[0]?.text

      if (!content) throw new Error('Gemini returned no content')

      return JSON.parse(content) as AiSuggestionResult
    }
  }
}
