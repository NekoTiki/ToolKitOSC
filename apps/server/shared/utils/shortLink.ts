// Re-exported through Nuxt's own shared/ auto-import convention (mirrors shared/types/protocol.ts)
// so both app/ (Vue) and server/ (Nitro) code can import from '#shared/utils/shortLink' without
// reaching into the workspace package directly.
//
// Imports the package's dedicated `/utils` subpath, not its root - same reasoning as
// shared/types/protocol.ts importing `/types` instead of the root: this file is pulled into the
// Nitro/server typecheck project too, which has no DOM lib. The package root also exports Vue
// components and the DOM-touching useTheme composable; resolving those (even for an unrelated
// named re-export) would drag their implementations into that project's program and break it.
export { decodeShareCode, encodeShareCode } from '@toolkitosc/shared-ui/utils'
