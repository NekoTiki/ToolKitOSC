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
        candidates?: { content?: { parts?: { text?: string }[] }; finishReason?: string }[]
      }
      const candidate = data.candidates?.[0]
      const content = candidate?.content?.parts?.[0]?.text
      const finishReason = candidate?.finishReason

      if (!content) throw new Error(`Gemini returned no content${finishReason ? ` (finish reason: ${finishReason})` : ''}`)

      // A reply cut off at maxOutputTokens is unparseable JSON - name that cause instead of
      // surfacing a bare JSON.parse position error.
      if (finishReason === 'MAX_TOKENS') {
        throw new Error(`Gemini's reply was cut off at the ${profile.maxCompletionTokens}-token output limit (${content.length} characters)`)
      }

      try {
        return JSON.parse(content) as AiSuggestionResult
      } catch (error) {
        throw new Error(`Gemini returned invalid JSON (finish reason: ${finishReason ?? 'unknown'}): ${error instanceof Error ? error.message : String(error)}`, { cause: error })
      }
    }
  }
}
