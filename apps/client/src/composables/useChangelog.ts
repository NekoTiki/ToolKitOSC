import { getVersion } from '@tauri-apps/api/app'
import MarkdownIt from 'markdown-it'
import type { Ref } from 'vue'
import { ref } from 'vue'

// Release notes for the client, straight from the GitHub releases (the same ones the updater's
// latest.json points at). Shown in Settings › About, and reached from the "Update available" toast
// so you can read what changed before restarting.
const RELEASES_URL = 'https://api.github.com/repos/NekoTiki/ToolKitOSC/releases?per_page=30'
const TAG_PREFIX = 'client-v'

export interface ChangelogRelease {
  version: string
  publishedAt: string
  // Rendered notes, safe to v-html: markdown-it escapes any raw HTML in the release body.
  html: string
  url: string
  // Newer than the installed version, i.e. part of the update you'd get by restarting.
  isNewer: boolean
}

interface GithubRelease {
  tag_name: string
  draft: boolean
  prerelease: boolean
  published_at: string | null
  body: string | null
  html_url: string
}

const markdown = new MarkdownIt({ html: false, linkify: true, breaks: false })

// Links in the notes open in the system browser - the container's click handler in AboutSettings
// picks them up by this attribute instead of letting the webview navigate away from the app.
const defaultLinkOpen = markdown.renderer.rules.link_open ?? ((tokens, idx, options, _env, self) => self.renderToken(tokens, idx, options))
markdown.renderer.rules.link_open = (tokens, idx, options, env, self) => {
  tokens[idx]!.attrSet('data-external', '')
  return defaultLinkOpen(tokens, idx, options, env, self)
}

// The card already shows the version, so the notes' own "# ⚡ ToolKitOSC vX.Y.Z" title goes, and
// so does "📦 Install" - which file to download is no use inside the app that's updating itself.
function notesToHtml(body: string): string {
  const withoutTitle = body.replace(/^\s*#\s[^\n]*\n/, '')
  const withoutInstall = withoutTitle.replace(/^##\s*📦[^\n]*\n[\s\S]*?(?=^##\s|(?![\s\S]))/m, '')
  return markdown.render(withoutInstall.trim())
}

// [1, 3, 0] plus an optional prerelease suffix ("beta.1"); a prerelease sorts before its release.
function parseVersion(version: string): { parts: number[]; pre: string } {
  const [core = '', pre = ''] = version.split('-', 2)
  return { parts: core.split('.').map((n) => Number.parseInt(n, 10) || 0), pre }
}

export function compareVersions(a: string, b: string): number {
  const va = parseVersion(a)
  const vb = parseVersion(b)
  for (let i = 0; i < Math.max(va.parts.length, vb.parts.length); i++) {
    const diff = (va.parts[i] ?? 0) - (vb.parts[i] ?? 0)
    if (diff !== 0) return diff
  }
  if (va.pre === vb.pre) return 0
  if (!va.pre) return 1
  if (!vb.pre) return -1
  return va.pre.localeCompare(vb.pre, undefined, { numeric: true })
}

// Shared app-wide and fetched once per session: GitHub allows 60 unauthenticated requests an hour.
const releases = ref<ChangelogRelease[]>([])
const currentVersion = ref<string>()
const loading = ref(false)
const error = ref(false)
let loaded = false

async function load(options: { force?: boolean } = {}): Promise<void> {
  if (loading.value || (loaded && !options.force)) return
  loading.value = true
  error.value = false

  try {
    const [version, response] = await Promise.all([
      getVersion(),
      fetch(RELEASES_URL, { headers: { Accept: 'application/vnd.github+json' } })
    ])
    if (!response.ok) throw new Error(`GitHub answered ${response.status}`)
    const data = (await response.json()) as GithubRelease[]

    // Prereleases only show up for people already running one.
    const onPrerelease = parseVersion(version).pre !== ''

    currentVersion.value = version
    releases.value = data
      .filter((r) => r.tag_name.startsWith(TAG_PREFIX) && !r.draft && (onPrerelease || !r.prerelease))
      .map((r) => {
        const releaseVersion = r.tag_name.slice(TAG_PREFIX.length)
        return {
          version: releaseVersion,
          publishedAt: r.published_at ?? '',
          html: notesToHtml(r.body ?? ''),
          url: r.html_url,
          isNewer: compareVersions(releaseVersion, version) > 0
        }
      })
      .sort((a, b) => compareVersions(b.version, a.version))
    loaded = true
  } catch (err) {
    console.error('Failed to load the changelog', err)
    error.value = true
  } finally {
    loading.value = false
  }
}

export function useChangelog(): {
  releases: Ref<ChangelogRelease[]>
  currentVersion: Ref<string | undefined>
  loading: Ref<boolean>
  error: Ref<boolean>
  load: (options?: { force?: boolean }) => Promise<void>
} {
  return { releases, currentVersion, loading, error, load }
}
