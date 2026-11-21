// "2 h 15 min" since a host's app connected, ticking once a minute (the Live page, a user's page).
export function useOnlineFor(): (since: number) => string {
  const now = useNow({ interval: 60_000 })

  return (since) => {
    const minutes = Math.max(0, Math.floor((now.value.getTime() - since) / 60_000))
    const hours = Math.floor(minutes / 60)

    if (!hours) return minutes ? `${minutes} min` : 'less than a minute'

    return minutes % 60 ? `${hours} h ${minutes % 60} min` : `${hours} h`
  }
}
