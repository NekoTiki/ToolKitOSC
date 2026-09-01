import { ref } from 'vue'

const ipStatistics = ref<Record<string, number>>()
const discordIdStatistics = ref<Record<string, number>>()

export function useCommandAnalytics(): {
  getIpStatistics: (ip: string) => number
  getDiscordIdStatistics: (discordId: string) => number
  increaseIpCount: (ip: string) => void
  increaseDiscordIdCount: (discordId: string) => void
} {
  const getIpStatistics = (ip: string): number => {
    return ipStatistics.value ? ipStatistics.value[ip] || 0 : 0
  }

  const getDiscordIdStatistics = (discordId: string): number => {
    return discordIdStatistics.value ? discordIdStatistics.value[discordId] || 0 : 0
  }

  const increaseIpCount = (ip: string): void => {
    if (!ipStatistics.value) ipStatistics.value = {}
    if (!ipStatistics.value[ip]) ipStatistics.value[ip] = 0

    ipStatistics.value[ip] += 1
  }

  const increaseDiscordIdCount = (discordId: string): void => {
    if (!discordIdStatistics.value) discordIdStatistics.value = {}
    if (!discordIdStatistics.value[discordId]) discordIdStatistics.value[discordId] = 0

    discordIdStatistics.value[discordId] += 1
  }

  return {
    getIpStatistics,
    getDiscordIdStatistics,
    increaseIpCount,
    increaseDiscordIdCount
  }
}
