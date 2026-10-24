// Bijective base62 codec for a Discord snowflake id, used to build a short `/s/<code>` link for
// the share page (see apps/client's AppHeader.vue and apps/server's `server/routes/s/[code].get.ts`)
// without needing any lookup table/database on the server - the share page's room id has always
// been the raw Discord id verbatim (see server/routes/host.ts's `getRoomId`), so this is a pure,
// reversible re-encoding of that same id, not a random code that has to be stored somewhere. A
// snowflake is a 64-bit integer (~18-19 decimal digits); base62 needs roughly log62(2^64) ≈ 11
// characters to represent the same range, which is what actually makes the link shorter.
//
// Deliberately no imports and no DOM/Vue dependency - both `packages/shared-ui`'s package root and
// `apps/server/shared/utils/shortLink.ts` (its Nitro-server-side re-export) rely on that to stay
// safe to import from the server's DOM-lib-free typecheck project (see that file's own comment).
const BASE62_ALPHABET = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz'
const BASE = BigInt(BASE62_ALPHABET.length)
// `BigInt(0)` rather than a `0n` literal: Nitro bundles the server with esbuild targeting es2019,
// which predates bigint literal syntax and warns on every one (the runtime itself is fine).
const ZERO = BigInt(0)

export const encodeShareCode = (discordId: string): string => {
  let n = BigInt(discordId)

  if (n === ZERO) return BASE62_ALPHABET.charAt(0)

  let code = ''

  while (n > ZERO) {
    code = BASE62_ALPHABET.charAt(Number(n % BASE)) + code
    n /= BASE
  }

  return code
}

// Returns null for a code containing anything outside the base62 alphabet - a malformed/garbled
// short link should 404, not resolve to some unrelated numeric id.
export const decodeShareCode = (code: string): string | null => {
  let n = ZERO

  for (const char of code) {
    const digit = BASE62_ALPHABET.indexOf(char)

    if (digit === -1) return null

    n = n * BASE + BigInt(digit)
  }

  return n.toString()
}

// A share link's custom name (/s/<name>): 3 to 24 lowercase letters, digits and inner dashes.
// Checked by the server; the app uses it to explain a rejected name before sending it.
const SHARE_NAME_PATTERN = /^[a-z0-9][a-z0-9-]{1,22}[a-z0-9]$/
const RESERVED_SHARE_NAMES = ['admin', 'official', 'staff', 'support', 'tkosc', 'toolkitosc']

export const isValidShareName = (name: string): boolean => SHARE_NAME_PATTERN.test(name) && !RESERVED_SHARE_NAMES.includes(name)
