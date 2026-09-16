// One-off generator for public/og-image.png, the static Open Graph/Twitter card banner used by
// the share page (see app/pages/share/[shareId].vue). Not part of the build - `sharp` (SVG
// rasterization) is a devDependency only used by this script, run manually whenever the banner
// design changes: `node scripts/generate-og-image.mjs` from apps/server.
//
// Reuses the same background/accent gradients as apps/client/src/assets/logo.svg (the app's own
// icon) so the banner reads as the same brand, just laid out wide instead of square.
import { mkdir } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import sharp from 'sharp'

const outDir = join(dirname(fileURLToPath(import.meta.url)), '../public')
const outFile = join(outDir, 'og-image.png')

const WIDTH = 1200
const HEIGHT = 630

// The marionette-control-bar icon from logo.svg, unchanged apart from its own background rect
// (dropped here - the banner draws one shared background behind everything instead).
const icon = `
  <g transform="translate(120, 145) scale(0.68)">
    <rect x="146" y="90" width="220" height="26" rx="13" fill="url(#fg)"/>
    <circle cx="146" cy="103" r="18" fill="url(#fg)"/>
    <circle cx="366" cy="103" r="18" fill="url(#fg)"/>
    <g stroke="#6ee7b7" stroke-width="9" fill="none" stroke-linecap="round">
      <path d="M 192,103 L 196,254"/>
      <path d="M 256,103 L 256,212"/>
      <path d="M 320,103 L 316,268"/>
    </g>
    <g fill="#454b5c">
      <rect x="180" y="254" width="32" height="170" rx="16"/>
      <rect x="240" y="212" width="32" height="212" rx="16"/>
      <rect x="300" y="268" width="32" height="156" rx="16"/>
    </g>
    <g fill="url(#fg)">
      <circle cx="196" cy="254" r="23"/>
      <circle cx="256" cy="212" r="23"/>
      <circle cx="316" cy="268" r="23"/>
    </g>
  </g>
`

const svg = `
<svg xmlns="http://www.w3.org/2000/svg" width="${WIDTH}" height="${HEIGHT}" viewBox="0 0 ${WIDTH} ${HEIGHT}">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#20232e"/>
      <stop offset="1" stop-color="#14161d"/>
    </linearGradient>
    <linearGradient id="fg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#34d399"/>
      <stop offset="1" stop-color="#059669"/>
    </linearGradient>
  </defs>

  <rect width="${WIDTH}" height="${HEIGHT}" fill="url(#bg)"/>

  ${icon}

  <text x="560" y="270" font-family="Arial, Helvetica, sans-serif" font-size="76" font-weight="700" fill="#f4f4f5">VRC OSC</text>
  <text x="560" y="356" font-family="Arial, Helvetica, sans-serif" font-size="76" font-weight="700" fill="url(#fg)">Toolkit</text>
  <text x="560" y="410" font-family="Arial, Helvetica, sans-serif" font-size="27" fill="#9ca3af">Remote control panel for your VRChat avatar</text>
</svg>
`

await mkdir(outDir, { recursive: true })
await sharp(Buffer.from(svg)).png().toFile(outFile)

console.log(`Wrote ${outFile}`)
