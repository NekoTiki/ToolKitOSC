// Text width from a TrueType font's own metrics (cmap format 4 + hmtx), for fitting text into the
// link-preview image (server/utils/ogImage.ts) - resvg can measure rendered text (getBBox), but at
// ~200ms of synchronous work per call, on the same thread as the WS server. Ignores kerning, which
// only moves a line by a few pixels at the sizes used there.

export interface TextMetrics {
  // A string's advance width, in ems.
  width: (text: string) => number
  // Whether the font has a glyph for this character (resvg draws a .notdef box otherwise).
  hasGlyph: (char: string) => boolean
}

export function createTextMetrics(font: Buffer): TextMetrics {
  const tables = new Map<string, number>()
  const numTables = font.readUInt16BE(4)

  for (let i = 0; i < numTables; i++) {
    const record = 12 + i * 16
    tables.set(font.toString('latin1', record, record + 4), font.readUInt32BE(record + 8))
  }

  const head = tables.get('head')
  const hhea = tables.get('hhea')
  const hmtx = tables.get('hmtx')
  const cmap = tables.get('cmap')

  if (head === undefined || hhea === undefined || hmtx === undefined || cmap === undefined) {
    throw new Error('Font is missing a head/hhea/hmtx/cmap table')
  }

  const metrics = hmtx
  const unitsPerEm = font.readUInt16BE(head + 18)
  const numberOfHMetrics = font.readUInt16BE(hhea + 34)

  // The Windows Unicode BMP subtable (platform 3, encoding 1), always format 4.
  let subtable: number | null = null

  for (let i = 0; i < font.readUInt16BE(cmap + 2); i++) {
    const record = cmap + 4 + i * 8

    if (font.readUInt16BE(record) === 3 && font.readUInt16BE(record + 2) === 1) {
      subtable = cmap + font.readUInt32BE(record + 4)
    }
  }

  if (subtable === null || font.readUInt16BE(subtable) !== 4) {
    throw new Error('Font has no format 4 Unicode cmap subtable')
  }

  const segCount = font.readUInt16BE(subtable + 6) / 2
  const endCodes = subtable + 14
  const startCodes = endCodes + segCount * 2 + 2
  const idDeltas = startCodes + segCount * 2
  const idRangeOffsets = idDeltas + segCount * 2

  function glyphId(codePoint: number): number {
    for (let i = 0; i < segCount; i++) {
      if (font.readUInt16BE(endCodes + i * 2) < codePoint) continue

      const start = font.readUInt16BE(startCodes + i * 2)
      if (start > codePoint) return 0

      const delta = font.readUInt16BE(idDeltas + i * 2)
      const rangeOffset = font.readUInt16BE(idRangeOffsets + i * 2)
      if (rangeOffset === 0) return (codePoint + delta) & 0xffff

      const glyph = font.readUInt16BE(idRangeOffsets + i * 2 + rangeOffset + (codePoint - start) * 2)
      return glyph === 0 ? 0 : (glyph + delta) & 0xffff
    }

    return 0
  }

  const advances = new Map<number, number>()

  function advance(codePoint: number): number {
    let width = advances.get(codePoint)

    if (width === undefined) {
      // Past numberOfHMetrics, every glyph shares the last listed advance (monospaced tail).
      const glyph = codePoint > 0xffff ? 0 : Math.min(glyphId(codePoint), numberOfHMetrics - 1)
      width = font.readUInt16BE(metrics + glyph * 4) / unitsPerEm
      advances.set(codePoint, width)
    }

    return width
  }

  return {
    width: (text) => [...text].reduce((sum, char) => sum + advance(char.codePointAt(0)!), 0),
    hasGlyph: (char) => {
      const codePoint = char.codePointAt(0)!
      return codePoint <= 0xffff && glyphId(codePoint) !== 0
    }
  }
}
