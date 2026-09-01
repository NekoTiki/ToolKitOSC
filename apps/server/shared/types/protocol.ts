// Re-exported through Nuxt's own shared/ auto-import convention (mirrors shared/types/auth.d.ts)
// so both app/ (Vue) and server/ (Nitro) code can import from '#shared/types/protocol' /
// '~~/shared/types/protocol' without reaching into the workspace package directly.
//
// Deliberately imports the package's `/types` subpath, not its root — this file is included in
// the Nitro/server typecheck project too, which has no DOM lib. The package root also exports
// Vue components and the DOM-touching useTheme composable; resolving those (even for a type-only
// re-export) would drag their implementations into that project's program and break it.
export type * from '@vrc-osc-toolkit/shared-ui/types'
