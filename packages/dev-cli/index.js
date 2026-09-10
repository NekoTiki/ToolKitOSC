#!/usr/bin/env node
// Interactive dev launcher for the server and client: split log panes + keyboard
// shortcuts to restart either one, in the spirit of `docker compose up`.

import { spawn } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import blessed from 'blessed'
import treeKill from 'tree-kill'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const ROOT = path.resolve(__dirname, '..', '..')
const isWin = process.platform === 'win32'

const SERVICES = {
  server: {
    key: 'server',
    label: 'Server',
    cwd: path.join(ROOT, 'apps', 'server'),
    command: 'npm run dev',
    restartKey: '1'
  },
  client: {
    key: 'client',
    label: 'Client',
    cwd: path.join(ROOT, 'apps', 'client'),
    command: 'npm run dev',
    restartKey: '2'
  }
}

// Runtime state per service, kept separate from the static config above.
for (const svc of Object.values(SERVICES)) {
  svc.proc = null
  svc.status = 'stopped' // 'stopped' | 'running' | 'restarting' | 'stopping'
  svc.buf = { stdout: '', stderr: '' }
}

const screen = blessed.screen({
  smartCSR: true,
  title: 'VRC OSC Toolkit — Dev',
  fullUnicode: true,
  autoPadding: true
})

const header = blessed.box({
  top: 0,
  left: 0,
  width: '100%',
  height: 1,
  content: ' VRC OSC Toolkit — Dev Launcher',
  tags: true,
  style: { fg: 'black', bg: 'white', bold: true }
})

function makeLogBox(left) {
  const box = blessed.log({
    top: 1,
    left,
    width: '50%',
    height: '100%-4',
    label: ' ... ',
    tags: true,
    border: { type: 'line' },
    scrollback: 5000,
    scrollbar: { ch: ' ', track: { bg: 'grey' }, style: { inverse: true } },
    mouse: true,
    keys: true,
    vi: true,
    style: { border: { fg: 'grey' } }
  })

  // blessed's scrollable widgets only bind the plain arrow keys (and, with vi:true,
  // ctrl+b/f, ctrl+u/d, g/G) — the literal PageUp/PageDown keys aren't wired up by
  // default, so we bind them ourselves to a full-page scroll.
  box.key(['pageup'], () => {
    box.scroll(-(box.height || 1))
    screen.render()
  })
  box.key(['pagedown'], () => {
    box.scroll(box.height || 1)
    screen.render()
  })

  return box
}

const boxes = {
  server: makeLogBox(0),
  client: makeLogBox('50%')
}

const footer = blessed.box({
  bottom: 0,
  left: 0,
  width: '100%',
  height: 3,
  tags: true,
  style: { fg: 'white', bg: 'blue' }
})

screen.append(header)
screen.append(boxes.server)
screen.append(boxes.client)
screen.append(footer)

const STATUS_COLOR = {
  running: 'green',
  stopped: 'red',
  restarting: 'yellow',
  stopping: 'yellow'
}

function updateLabel(key) {
  const svc = SERVICES[key]
  const color = STATUS_COLOR[svc.status] ?? 'white'
  boxes[key].setLabel(
    `{${color}-fg}{bold} ${svc.label.toUpperCase()} · ${svc.status} {/bold}{/${color}-fg} `
  )
}

function renderFooter() {
  const statusLine = Object.values(SERVICES)
    .map((svc) => {
      const color = STATUS_COLOR[svc.status] ?? 'white'
      return `${svc.label}: {${color}-fg}${svc.status}{/${color}-fg}`
    })
    .join('   ')

  footer.setContent(
    ` {bold}[1]{/bold} restart server   {bold}[2]{/bold} restart client   {bold}[a]{/bold} restart all   ` +
      `{bold}[←→]{/bold} switch focus   {bold}[↑↓ / PgUp PgDn]{/bold} scroll   {bold}[q]{/bold} quit\n` +
      ` ${statusLine}`
  )
  screen.render()
}

function appendLine(key, line, kind = 'stdout') {
  let text = blessed.escape(line)
  if (kind === 'meta') {
    text = `{grey-fg}${text}{/grey-fg}`
  } else if (/error/i.test(line)) {
    text = `{red-fg}${text}{/red-fg}`
  } else if (/warn/i.test(line)) {
    text = `{yellow-fg}${text}{/yellow-fg}`
  }
  boxes[key].log(text)
}

function pump(key, chunk, streamName) {
  const svc = SERVICES[key]
  svc.buf[streamName] += chunk.toString()
  const lines = svc.buf[streamName].split(/\r?\n/)
  svc.buf[streamName] = lines.pop()
  for (const line of lines) {
    if (line.length) appendLine(key, line, streamName)
  }
}

function startService(key) {
  const svc = SERVICES[key]
  if (svc.proc) return

  svc.status = 'running'
  updateLabel(key)
  renderFooter()
  appendLine(key, `$ ${svc.command}`, 'meta')

  const proc = spawn(svc.command, {
    cwd: svc.cwd,
    shell: true,
    env: process.env,
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
    detached: !isWin
  })

  svc.proc = proc
  proc.stdout.on('data', (d) => pump(key, d, 'stdout'))
  proc.stderr.on('data', (d) => pump(key, d, 'stderr'))

  proc.on('exit', (code, signal) => {
    svc.proc = null
    const wasRestarting = svc.status === 'restarting'
    svc.status = 'stopped'
    updateLabel(key)
    appendLine(
      key,
      `--- exited (code ${code ?? 'null'}${signal ? `, signal ${signal}` : ''}) ---`,
      'meta'
    )
    renderFooter()
    if (wasRestarting) startService(key)
  })

  proc.on('error', (err) => {
    appendLine(key, `--- failed to start: ${err.message} ---`, 'meta')
  })
}

function stopService(key, done) {
  const svc = SERVICES[key]
  if (!svc.proc) {
    done?.()
    return
  }
  const pid = svc.proc.pid
  treeKill(pid, 'SIGTERM', () => done?.())
}

function restartService(key) {
  const svc = SERVICES[key]
  if (!svc.proc) {
    startService(key)
    return
  }
  svc.status = 'restarting'
  updateLabel(key)
  renderFooter()
  appendLine(key, '--- restarting ---', 'meta')
  stopService(key)
  // Actual restart happens from the 'exit' handler above once the old
  // process (and its children) have actually gone away.
}

let focused = 'server'
function focus(key) {
  focused = key
  boxes[key].focus()
  for (const k of Object.keys(boxes)) {
    boxes[k].style.border.fg = k === key ? 'cyan' : 'grey'
  }
  screen.render()
}

let quitting = false
function quit() {
  if (quitting) return
  quitting = true
  const pending = Object.keys(SERVICES).map(
    (key) => new Promise((resolve) => stopService(key, resolve))
  )
  const timeout = new Promise((resolve) => setTimeout(resolve, 3000))
  Promise.race([Promise.all(pending), timeout]).finally(() => {
    screen.destroy()
    process.exit(0)
  })
}

screen.key(['1'], () => restartService('server'))
screen.key(['2'], () => restartService('client'))
screen.key(['a'], () => {
  restartService('server')
  restartService('client')
})
screen.key(['left', 'right'], () => focus(focused === 'server' ? 'client' : 'server'))
screen.key(['q', 'C-c'], () => quit())

process.on('SIGINT', quit)
process.on('SIGTERM', quit)

updateLabel('server')
updateLabel('client')
renderFooter()
focus('server')

startService('server')
startService('client')
