// Re-exported through Nuxt's own shared/ auto-import convention (mirrors shared/types/auth.d.ts)
// so both app/ (Vue) and server/ (Nitro) code can import from '#shared/types/protocol' /
// '~~/shared/types/protocol' without reaching into the workspace package directly.
//
// Deliberately imports the package's `/types` subpath, not its root — this file is included in
// the Nitro/server typecheck project too, which has no DOM lib. The package root also exports
// Vue components and the DOM-touching useTheme composable; resolving those (even for a type-only
// re-export) would drag their implementations into that project's program and break it.
export type * from '@toolkitosc/shared-ui/types'

// Named (non type-only) re-export: PROTOCOL_VERSION / MIN_SUPPORTED_PROTOCOL_VERSION are plain
// numeric constants declared in protocol.ts itself (no DOM-touching imports), so pulling in their
// runtime values doesn't risk dragging the package's Vue components into this project's typecheck
// the way `export * from` off the root would.
export { MIN_SUPPORTED_PROTOCOL_VERSION, PROTOCOL_VERSION } from '@toolkitosc/shared-ui/types'

// Same reasoning: KNOWN_CONTROL_TYPES is derived in controls.ts (also import-free) from a Record
// literal, not from anything DOM-touching.
export { KNOWN_CONTROL_TYPES } from '@toolkitosc/shared-ui/types'
