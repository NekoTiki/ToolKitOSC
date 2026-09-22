// Reads Tailwind's own palette straight from its CSS variables (Tailwind v4 exposes every default
// color as --color-{name}-{shade}, see node_modules/tailwindcss/theme.css - imported via
// app/assets/css/main.css's `@import "tailwindcss"`) instead of hardcoding copies of them, so chart
// colors always match whatever Tailwind actually renders rather than a hand-picked approximation.
// Canvas fillStyle can't resolve var(...) itself the way a normal DOM element's style can, so the
// value has to be read once via getComputedStyle instead of passed through as a live reference.
export function tailwindColor(name: string): string {
  if (typeof document === 'undefined') return '#000'

  return getComputedStyle(document.documentElement).getPropertyValue(`--color-${name}`).trim()
}
