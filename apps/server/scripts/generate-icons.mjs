// One-off generator for the installable web app icons referenced by public/manifest.webmanifest
// and nuxt.config.ts's head links. Not part of the build - `sharp` (SVG rasterization) is a
// devDependency only used by this script, run manually whenever the logo changes:
// `node scripts/generate-icons.mjs` from apps/server.
//
// Rasterizes public/favicon.svg (the app logo) as-is for the regular icons, plus a full-bleed
// variant for "maskable" (Android crops it into its own shape) and apple-touch-icon (iOS rounds it
// itself, and fills transparent corners with black). The dial already fits inside the maskable
// safe zone (a centered circle of radius 40%), so that variant only drops the rounded corners and
// the edge highlight.
import { mkdir, readFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import sharp from 'sharp'

const publicDir = join(dirname(fileURLToPath(import.meta.url)), '../public')
const iconsDir = join(publicDir, 'icons')

const logo = await readFile(join(publicDir, 'favicon.svg'), 'utf8')
const fullBleed = logo.replaceAll(' rx="112"', '').replace(/\s*<rect x="2" y="2"[^>]*\/>/, '')

const outputs = [
  { svg: logo, size: 192, file: join(iconsDir, 'icon-192.png') },
  { svg: logo, size: 512, file: join(iconsDir, 'icon-512.png') },
  { svg: fullBleed, size: 512, file: join(iconsDir, 'icon-maskable-512.png') },
  // Also served at the root path iOS probes on its own when a page has no apple-touch-icon link.
  { svg: fullBleed, size: 180, file: join(publicDir, 'apple-touch-icon.png') }
]

await mkdir(iconsDir, { recursive: true })

for (const { svg, size, file } of outputs) {
  // Rasterized at 4x (288dpi over the 512-unit viewBox) and downscaled, for clean edges at every size.
  await sharp(Buffer.from(svg), { density: 288 })
    .resize(size, size)
    .png()
    .toFile(file)
  console.log(`Wrote ${file}`)
}
