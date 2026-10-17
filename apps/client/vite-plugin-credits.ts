import { execFileSync } from 'node:child_process'
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { dirname, join } from 'node:path'

import type { Plugin } from 'vite'

import type { CreditPackage, Credits } from './src/lib/credits.ts'

// Settings > About & updates > Credits lists every library the client ships. This writes
// `credits.json` next to the built page:
// - Rust: the crates `cargo metadata` resolves for the target being built, following only normal
//   dependencies from the app's crate (build and dev dependencies never reach the binary).
// - JS: the packages whose modules ended up in Vite's build, so build tools aren't listed.
// The dev server answers `/credits.json` itself, with the client's direct JS dependencies only.
const FILE_NAME = 'credits.json'
const LICENSE_FILE = /^(licen[cs]e|copying|notice)/i

interface CargoPackage {
  id: string
  name: string
  version: string
  license: string | null
  description: string | null
  authors: string[]
  repository: string | null
  homepage: string | null
  manifest_path: string
}

interface CargoMetadata {
  packages: CargoPackage[]
  resolve: {
    root: string
    nodes: { id: string; deps: { pkg: string; dep_kinds: { kind: string | null }[] }[] }[]
  }
}

type Person = string | { name?: string }

interface PackageJson {
  name?: string
  version?: string
  license?: string | { type?: string }
  licenses?: { type?: string }[]
  description?: string
  author?: Person
  contributors?: Person[]
  repository?: string | { url?: string }
  homepage?: string
}

type Draft = Omit<CreditPackage, 'licenseTexts'> & { dir: string }

// "Jane Doe <jane@example.com> (https://jane.dev)" -> "Jane Doe"
const personName = (person: Person): string =>
  (typeof person === 'string' ? person : (person.name ?? ''))
    .replace(/<[^>]*>|\([^)]*\)/g, '')
    .trim()

// git+https://github.com/a/b.git, git://github.com/a/b, git@github.com:a/b, github:a/b and a/b
// all become https://github.com/a/b.
const repositoryUrl = (repository: PackageJson['repository']): string | undefined => {
  let url = (typeof repository === 'string' ? repository : repository?.url)?.trim()
  if (!url) return undefined
  if (/^[\w.-]+\/[\w.-]+$/.test(url)) url = `github:${url}`
  url = url
    .replace(
      /^(github|gitlab|bitbucket):/,
      (_, host: string) => `https://${host}.${host === 'bitbucket' ? 'org' : 'com'}/`
    )
    .replace(/^git\+/, '')
    .replace(/^git@([^:]+):/, 'https://$1/')
    .replace(/^(git|ssh):\/\/(git@)?/, 'https://')
    .replace(/\.git$/, '')
    .replace(/#.*$/, '')
  return /^https?:\/\//.test(url) ? url : undefined
}

const licenseFiles = (dir: string): string[] => {
  try {
    return readdirSync(dir)
      .filter((file) => LICENSE_FILE.test(file) && statSync(join(dir, file)).isFile())
      .sort()
      .map((file) => readFileSync(join(dir, file), 'utf8').replace(/\r\n?/g, '\n').trim())
      .filter(Boolean)
  } catch {
    return []
  }
}

const targetTriple = (): string =>
  process.env.TAURI_ENV_TARGET_TRIPLE ||
  /^host: (.+)$/m.exec(execFileSync('rustc', ['-vV'], { encoding: 'utf8' }))?.[1]?.trim() ||
  ''

const rustPackages = (manifestPath: string): Draft[] => {
  const triple = targetTriple()
  const metadata = JSON.parse(
    execFileSync(
      'cargo',
      [
        'metadata',
        '--format-version',
        '1',
        '--locked',
        '--manifest-path',
        manifestPath,
        ...(triple ? ['--filter-platform', triple] : [])
      ],
      { encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 }
    )
  ) as CargoMetadata

  const packages = new Map(metadata.packages.map((p) => [p.id, p]))
  const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]))
  const seen = new Set([metadata.resolve.root])
  const queue = [metadata.resolve.root]
  for (let id = queue.shift(); id !== undefined; id = queue.shift()) {
    for (const dep of nodes.get(id)?.deps ?? []) {
      if (seen.has(dep.pkg) || !dep.dep_kinds.some((k) => k.kind === null)) continue
      seen.add(dep.pkg)
      queue.push(dep.pkg)
    }
  }
  seen.delete(metadata.resolve.root)

  return [...seen].flatMap((id) => {
    const p = packages.get(id)
    if (!p) return []
    return [
      {
        name: p.name,
        version: p.version,
        license: p.license ?? undefined,
        description: p.description?.trim() || undefined,
        authors: p.authors.map(personName).filter(Boolean),
        repository: p.repository ?? undefined,
        homepage: p.homepage ?? undefined,
        dir: dirname(p.manifest_path)
      }
    ]
  })
}

const jsPackage = (dir: string): Draft | null => {
  let pkg: PackageJson
  try {
    pkg = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8')) as PackageJson
  } catch {
    return null
  }
  if (!pkg.name || !pkg.version) return null
  const license =
    typeof pkg.license === 'string'
      ? pkg.license
      : (pkg.license?.type ??
        pkg.licenses
          ?.map((l) => l.type)
          .filter(Boolean)
          .join(' OR '))
  const people = pkg.author ? [pkg.author] : (pkg.contributors ?? [])
  const repository = repositoryUrl(pkg.repository)
  return {
    name: pkg.name,
    version: pkg.version,
    license: license || undefined,
    description: pkg.description?.trim() || undefined,
    authors: people.map(personName).filter(Boolean),
    repository,
    homepage:
      pkg.homepage && pkg.homepage.replace(/#readme$/, '') !== repository
        ? pkg.homepage
        : undefined,
    dir
  }
}

// The package a module belongs to: what follows its last `node_modules/`, one segment or two for a
// scoped name. Workspace packages (shared-ui) live outside node_modules and are ours anyway.
const packageDir = (moduleId: string): string | null => {
  const path = moduleId.split('?')[0]!.replace(/\\/g, '/')
  const at = path.lastIndexOf('/node_modules/')
  if (at < 0 || path.startsWith('\0')) return null
  const rest = path.slice(at + '/node_modules/'.length).split('/')
  const segments = rest[0]?.startsWith('@') ? 2 : 1
  return rest.length > segments
    ? path.slice(0, at) + '/node_modules/' + rest.slice(0, segments).join('/')
    : null
}

// Where Node would find `name` from `from`: its own node_modules, then each parent's (hoisted).
const resolvePackageDir = (name: string, from: string): string | null => {
  for (let dir = from; ; dir = dirname(dir)) {
    const candidate = join(dir, 'node_modules', name)
    if (existsSync(join(candidate, 'package.json'))) return candidate
    if (dirname(dir) === dir) return null
  }
}

const toCredits = (rust: Draft[], js: Draft[], partialJs = false): Credits => {
  const licenseTexts: string[] = []
  const textIndex = new Map<string, number>()
  const finish = (drafts: Draft[]): CreditPackage[] => {
    const byKey = new Map(drafts.map((d) => [`${d.name}@${d.version}`, d]))
    return [...byKey.values()]
      .sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version))
      .map(({ dir, ...pkg }) => ({
        ...pkg,
        licenseTexts: licenseFiles(dir).map((text) => {
          let index = textIndex.get(text)
          if (index === undefined) {
            index = licenseTexts.push(text) - 1
            textIndex.set(text, index)
          }
          return index
        })
      }))
  }
  return { rust: finish(rust), js: finish(js), licenseTexts, ...(partialJs ? { partialJs } : {}) }
}

export default function credits(): Plugin {
  let root = ''
  const manifestPath = (): string => join(root, 'src-tauri', 'Cargo.toml')

  return {
    name: 'toolkitosc:credits',
    configResolved(config) {
      root = config.root
    },
    configureServer(server) {
      let cached: string | undefined
      server.middlewares.use(`/${FILE_NAME}`, (_req, res) => {
        if (!cached) {
          let rust: Draft[] = []
          try {
            rust = rustPackages(manifestPath())
          } catch (error) {
            server.config.logger.warn(
              `[credits] cargo metadata failed, Rust crates won't be listed: ${String(error)}`
            )
          }
          const { dependencies = {} } = JSON.parse(
            readFileSync(join(root, 'package.json'), 'utf8')
          ) as {
            dependencies?: Record<string, string>
          }
          const js = Object.keys(dependencies).flatMap((name) => {
            const dir = resolvePackageDir(name, root)
            const pkg = dir && jsPackage(dir)
            return pkg ? [pkg] : []
          })
          cached = JSON.stringify(toCredits(rust, js, true))
        }
        res.setHeader('Content-Type', 'application/json')
        res.end(cached)
      })
    },
    generateBundle() {
      // A release must not ship without its Rust crates, so a cargo failure fails the build.
      const rust = rustPackages(manifestPath())
      const dirs = new Set([...this.getModuleIds()].map(packageDir).filter((dir) => dir !== null))
      const js = [...dirs].flatMap((dir) => {
        const pkg = jsPackage(dir)
        return pkg ? [pkg] : []
      })
      this.emitFile({
        type: 'asset',
        fileName: FILE_NAME,
        source: JSON.stringify(toCredits(rust, js))
      })
    }
  }
}
