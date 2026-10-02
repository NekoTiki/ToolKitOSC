const relative = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' })

const STEPS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['second', 60],
  ['minute', 60],
  ['hour', 24],
  ['day', 7],
  ['week', 4.35],
  ['month', 12],
  ['year', Infinity]
]

// "3 minutes ago", "yesterday"... in the user's own locale. `now` is for callers that re-render on
// a clock (useNow), so the text keeps up as time passes.
export function timeAgo(date: string | number | Date, now: number | Date = Date.now()): string {
  let value = (new Date(date).getTime() - new Date(now).getTime()) / 1000

  for (const [unit, size] of STEPS) {
    if (Math.abs(value) < size) return relative.format(Math.round(value), unit)
    value /= size
  }

  return relative.format(Math.round(value), 'year')
}
