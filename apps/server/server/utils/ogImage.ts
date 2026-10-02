// Link-preview (Open Graph / Twitter card) image, drawn per request instead of shipped as a static
// PNG, so a shared room link shows the host's name, live state and theme colors - and the logo
// never goes stale again (the old static banner still had the pre-rename marionette icon).
//
// SVG -> PNG via resvg rather than sharp: the production image is node:22-alpine with no system
// fonts at all, so text only renders because the Outfit TTFs (the app's own UI font, see
// shared-ui's styles/aurora.css) ship as Nitro server assets (server/assets/fonts) and are handed
// to resvg explicitly. Characters Outfit lacks (emoji, CJK...) are dropped from a host's name.
import { mkdir, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { renderAsync } from '@resvg/resvg-js'

import type { TextMetrics } from '~~/server/utils/fontMetrics'
import { createTextMetrics } from '~~/server/utils/fontMetrics'
import type { ShareInfo } from '~~/server/utils/shareInfo'

export const OG_IMAGE_WIDTH = 1200
export const OG_IMAGE_HEIGHT = 630

// Tailwind's 300/400 shades as sRGB hex: the theme itself is defined in oklch (shared-ui's
// styles/theme-colors.css), which resvg can't parse.
const PALETTE: Record<string, { light: string; base: string }> = {
  red: { light: '#fca5a5', base: '#f87171' },
  orange: { light: '#fdba74', base: '#fb923c' },
  amber: { light: '#fcd34d', base: '#fbbf24' },
  yellow: { light: '#fde047', base: '#facc15' },
  lime: { light: '#bef264', base: '#a3e635' },
  green: { light: '#86efac', base: '#4ade80' },
  emerald: { light: '#6ee7b7', base: '#34d399' },
  teal: { light: '#5eead4', base: '#2dd4bf' },
  cyan: { light: '#67e8f9', base: '#22d3ee' },
  sky: { light: '#7dd3fc', base: '#38bdf8' },
  blue: { light: '#93c5fd', base: '#60a5fa' },
  indigo: { light: '#a5b4fc', base: '#818cf8' },
  violet: { light: '#c4b5fd', base: '#a78bfa' },
  purple: { light: '#d8b4fe', base: '#c084fc' },
  fuchsia: { light: '#f0abfc', base: '#e879f9' },
  pink: { light: '#f9a8d4', base: '#f472b6' },
  rose: { light: '#fda4af', base: '#fb7185' },
  slate: { light: '#cbd5e1', base: '#94a3b8' },
  gray: { light: '#d1d5db', base: '#9ca3af' },
  zinc: { light: '#d4d4d8', base: '#a1a1aa' },
  neutral: { light: '#d4d4d4', base: '#a3a3a3' }
}

// The logo's own colors (public/favicon.svg), used when the host never sent a theme.
const DEFAULT_PRIMARY = 'teal'
const DEFAULT_SECONDARY = 'orange'

export interface OgCard {
  // null draws the generic site card (no room, or an offline room whose host name is unknown).
  share: ShareInfo | null
  primary?: string | null
  secondary?: string | null
}

interface Fonts {
  files: string[]
  // Outfit Bold, the title's weight.
  bold: TextMetrics
}

let fonts: Promise<Fonts> | null = null

// Stable resvg-js (2.6.x) only loads fonts from disk (`fontFiles`) - its `fontBuffers` option is
// alpha-only and silently ignored, falling back to system fonts - while Nitro bundles server
// assets into the JS output rather than shipping them as files. So they're written out to the OS
// temp dir once per process.
async function readFonts(): Promise<Fonts> {
  const storage = useStorage('assets:server')
  const names = ['Outfit-Medium.ttf', 'Outfit-Bold.ttf']
  const dir = join(tmpdir(), 'toolkitosc-fonts')

  await mkdir(dir, { recursive: true })

  const buffers = await Promise.all(
    names.map(async (name) => {
      const buffer = await storage.getItemRaw<Buffer>(`fonts/${name}`)
      if (!buffer) throw new Error(`Missing server asset fonts/${name}`)

      await writeFile(join(dir, name), buffer)
      return Buffer.from(buffer)
    })
  )

  return { files: names.map((name) => join(dir, name)), bold: createTextMetrics(buffers[1]!) }
}

function loadFonts(): Promise<Fonts> {
  fonts ??= readFonts()
  return fonts
}

function escapeXml(text: string): string {
  return text.replace(/[<>&'"]/g, (char) => `&#${char.charCodeAt(0)};`)
}

// Long names shrink down to MIN_TITLE_SIZE first, then get truncated with an ellipsis. Advance
// widths (fontMetrics.ts) land within ~5% of the rendered ink either way, hence the slack.
const TEXT_X = 560
const TEXT_WIDTH = OG_IMAGE_WIDTH - TEXT_X - 56
const MAX_TITLE_SIZE = 84
const MIN_TITLE_SIZE = 46
const MEASURE_SLACK = 1.06

function fitTitle(text: string, metrics: TextMetrics): { text: string; size: number } {
  const measure = (value: string): number => metrics.width(value) * MEASURE_SLACK
  const size = Math.floor(Math.min(MAX_TITLE_SIZE, TEXT_WIDTH / measure(text)))

  if (size >= MIN_TITLE_SIZE) return { text, size }

  const chars = [...text]
  let truncated = text

  while (chars.length > 1 && measure(truncated) * MIN_TITLE_SIZE > TEXT_WIDTH) {
    chars.pop()
    truncated = `${chars.join('').trimEnd()}…`
  }

  return { text: truncated, size: MIN_TITLE_SIZE }
}

// public/favicon.svg's dial (its 512-unit viewBox), without the rounded tile behind it - the card
// draws one shared background instead. Recolored from the host's theme.
function dial(primary: { light: string; base: string }, secondary: { base: string }): string {
  return `
    <defs>
      <linearGradient id="arc" gradientUnits="userSpaceOnUse" x1="120" y1="380" x2="400" y2="140">
        <stop offset="0" stop-color="${primary.base}"/>
        <stop offset="1" stop-color="${secondary.base}"/>
      </linearGradient>
    </defs>
    <g transform="translate(85, 110) scale(0.8)">
      <path d="M151.3 360.7 A148 148 0 1 1 360.7 360.7" stroke="#000000" stroke-opacity="0.38" stroke-width="58" stroke-linecap="round" fill="none"/>
      <path d="M151.3 360.7 A148 148 0 1 1 360.7 360.7" stroke="#ffffff" stroke-opacity="0.1" stroke-width="58" stroke-linecap="round" fill="none"/>
      <path d="M151.3 360.7 A148 148 0 1 1 381.5 177.6" stroke="url(#arc)" stroke-width="58" stroke-linecap="round" fill="none"/>
      <circle cx="381.5" cy="182.6" r="44" fill="#000000" fill-opacity="0.35"/>
      <circle cx="381.5" cy="177.6" r="44" fill="#ffffff"/>
      <circle cx="256" cy="256" r="34" fill="${primary.light}"/>
    </g>
  `
}

function shareText(share: ShareInfo, fonts: Fonts, primary: { base: string }, secondary: { base: string }): string {
  // Characters Outfit has no glyph for would draw as boxes - dropped, falling back to the generic
  // title if that leaves nothing (e.g. a name written entirely in emoji or kana).
  const name = [...(share.hostName ?? '')].filter((char) => fonts.bold.hasGlyph(char)).join('').replace(/\s+/g, ' ').trim()
  const title = fitTitle(name || 'Shared Controls', fonts.bold)

  let status: string
  let dotColor: string
  let footer: string

  if (!share.online) {
    status = 'Offline'
    dotColor = '#6b7280'
    footer = 'Check back once they’re online.'
  } else if (share.controlCount === 0) {
    status = 'Live • no controls shared yet'
    dotColor = secondary.base
    footer = 'Open the link to join the room.'
  } else {
    status = `Live • ${share.controlCount} ${share.controlCount === 1 ? 'control' : 'controls'}`
    dotColor = secondary.base
    footer = 'Open the link to view and use them.'
  }

  return `
    <text x="${TEXT_X}" y="200" font-size="34" font-weight="500" fill="#e4e4e7">ToolKit<tspan fill="${primary.base}">OSC</tspan></text>
    <text x="${TEXT_X - 4}" y="300" font-size="${title.size}" font-weight="700" fill="#fafafa">${escapeXml(title.text)}</text>
    <circle cx="${TEXT_X + 12}" cy="${368}" r="11" fill="${dotColor}"/>
    <text x="${TEXT_X + 38}" y="380" font-size="34" font-weight="500" fill="#e4e4e7">${escapeXml(status)}</text>
    <text x="${TEXT_X}" y="450" font-size="28" font-weight="500" fill="#a1a1aa">${escapeXml(footer)}</text>
  `
}

function siteText(primary: { base: string }): string {
  return `
    <text x="${TEXT_X - 4}" y="300" font-size="96" font-weight="700" fill="#fafafa">ToolKit<tspan fill="${primary.base}">OSC</tspan></text>
    <text x="${TEXT_X}" y="370" font-size="32" font-weight="500" fill="#a1a1aa">Remote control panel</text>
    <text x="${TEXT_X}" y="414" font-size="32" font-weight="500" fill="#a1a1aa">for your VRChat avatar</text>
  `
}

export async function renderOgImage({ share, primary, secondary }: OgCard): Promise<Buffer> {
  const primaryColor = PALETTE[primary ?? ''] ?? PALETTE[DEFAULT_PRIMARY]!
  const secondaryColor = PALETTE[secondary ?? ''] ?? PALETTE[DEFAULT_SECONDARY]!
  const fonts = await loadFonts()

  // Same two glows as the Aurora page background (shared-ui's styles/aurora.css): primary from
  // the top-left, secondary from the bottom-right, over a near-black base tinted by the primary.
  const svg = `
    <svg xmlns="http://www.w3.org/2000/svg" width="${OG_IMAGE_WIDTH}" height="${OG_IMAGE_HEIGHT}" viewBox="0 0 ${OG_IMAGE_WIDTH} ${OG_IMAGE_HEIGHT}" font-family="Outfit">
      <defs>
        <radialGradient id="glow1" gradientUnits="userSpaceOnUse" cx="70" cy="0" r="900">
          <stop offset="0" stop-color="${primaryColor.base}" stop-opacity="0.32"/>
          <stop offset="1" stop-color="${primaryColor.base}" stop-opacity="0"/>
        </radialGradient>
        <radialGradient id="glow2" gradientUnits="userSpaceOnUse" cx="${OG_IMAGE_WIDTH}" cy="${OG_IMAGE_HEIGHT}" r="820">
          <stop offset="0" stop-color="${secondaryColor.base}" stop-opacity="0.22"/>
          <stop offset="1" stop-color="${secondaryColor.base}" stop-opacity="0"/>
        </radialGradient>
      </defs>
      <rect width="100%" height="100%" fill="#03070a"/>
      <rect width="100%" height="100%" fill="${primaryColor.base}" fill-opacity="0.05"/>
      <rect width="100%" height="100%" fill="url(#glow1)"/>
      <rect width="100%" height="100%" fill="url(#glow2)"/>
      ${dial(primaryColor, secondaryColor)}
      ${share ? shareText(share, fonts, primaryColor, secondaryColor) : siteText(primaryColor)}
    </svg>
  `

  const image = await renderAsync(svg, {
    font: { fontFiles: fonts.files, loadSystemFonts: false, defaultFontFamily: 'Outfit' }
  })

  return image.asPng()
}
