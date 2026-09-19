import { buildUserPrompt, CONTROL_SUGGESTION_SCHEMA, SYSTEM_PROMPT } from '../prompt'
import type { AiParameterInput, AiProvider, AiSuggestionResult } from '../types'

const GEMINI_URL = 'https://generativelanguage.googleapis.com/v1beta/models'

export function createGeminiProvider(apiKey: string, model: string): AiProvider {
  return {
    id: 'gemini',
    label: 'Google Gemini',
    isConfigured: () => !!apiKey,

    async suggestControlGroups(
      avatarName: string,
      parameters: AiParameterInput[]
    ): Promise<AiSuggestionResult> {
      if (!apiKey) throw new Error('Gemini is not configured (missing NUXT_GEMINI_API_KEY)')

      const response = await fetch(`${GEMINI_URL}/${model}:generateContent?key=${apiKey}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          systemInstruction: { parts: [{ text: SYSTEM_PROMPT }] },
          contents: [{ role: 'user', parts: [{ text: buildUserPrompt(avatarName, parameters) }] }],
          generationConfig: {
            responseMimeType: 'application/json',
            responseSchema: CONTROL_SUGGESTION_SCHEMA,
            // See providers/groq.ts's identical comment - a real parameter list needs headroom
            // well past whatever a short default budget would give it, or the response comes back
            // silently truncated (schema-valid but missing most of the data) instead of erroring.
            maxOutputTokens: 8000
          }
        })
      })

      if (!response.ok) {
        const body = await response.text()
        throw new Error(`Gemini request failed (${response.status}): ${body}`)
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
