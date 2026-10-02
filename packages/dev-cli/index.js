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
  title: 'ToolKitOSC — Dev',
  fullUnicode: true,
  autoPadding: true
})

const header = blessed.box({
  top: 0,
  left: 0,
  width: '100%',
  height: 1,
  content: ' ToolKitOSC — Dev Launcher',
  tags: true,
  style: { fg: 'black', bg: 'white', bold: true }
})

// 'lr' places the panes side by side (focus switches with ←/→), 'ud' stacks
// them (focus switches with ↑/↓). Toggled at runtime with the 's' key.
let splitMode = 'lr'

// blessed's own keys/vi scroll bindings can't be reconfigured after the widget
// is built, and bundle the plain up/down arrows together with j/k, g/G, etc.
// We implement scrolling ourselves instead so the plain up/down arrows can be
// released to switch focus once the panes are stacked (splitMode === 'ud').
function bindScrollKeys(box) {
  box.on('keypress', (ch, key) => {
    if ((key.name === 'up' || key.name === 'down') && splitMode === 'ud') return
    if (key.name === 'up' || key.name === 'k') return void (box.scroll(-1), screen.render())
    if (key.name === 'down' || key.name === 'j') return void (box.scroll(1), screen.render())
    if (key.ctrl && key.name === 'u') {
      return void (box.scroll(-((box.height / 2) | 0) || -1), screen.render())
    }
    if (key.ctrl && key.name === 'd') {
      return void (box.scroll((box.height / 2) | 0 || 1), screen.render())
    }
    if (key.ctrl && key.name === 'b') return void (box.scroll(-box.height || -1), screen.render())
    if (key.ctrl && key.name === 'f') return void (box.scroll(box.height || 1), screen.render())
    if (key.name === 'g' && !key.shift) return void (box.scrollTo(0), screen.render())
    if (key.name === 'g' && key.shift) {
      return void (box.scrollTo(box.getScrollHeight()), screen.render())
    }
  })

  // The literal PageUp/PageDown keys aren't key names blessed's keypress
  // events use directly above, so bind them separately to a full-page scroll.
  box.key(['pageup'], () => {
    box.scroll(-(box.height || 1))
    screen.render()
  })
  box.key(['pagedown'], () => {
    box.scroll(box.height || 1)
    screen.render()
  })
}

function makeLogBox() {
  const box = blessed.log({
    label: ' ... ',
    tags: true,
    border: { type: 'line' },
    scrollback: 5000,
    scrollbar: { ch: ' ', track: { bg: 'grey' }, style: { inverse: true } },
    mouse: true,
    style: { border: { fg: 'grey' } }
  })

  bindScrollKeys(box)

  return box
}

const boxes = {
  server: makeLogBox(),
  client: makeLogBox()
}

function setBoxLayout(box, { top, left, width, height }) {
  box.top = top
  box.left = left
  box.width = width
  box.height = height
}

function layoutBoxes() {
  if (splitMode === 'lr') {
    setBoxLayout(boxes.server, { top: 1, left: 0, width: '50%', height: '100%-4' })
    setBoxLayout(boxes.client, { top: 1, left: '50%', width: '50%', height: '100%-4' })
  } else {
    setBoxLayout(boxes.server, { top: 1, left: 0, width: '100%', height: '50%-2' })
    setBoxLayout(boxes.client, { top: '50%-1', left: 0, width: '100%', height: '50%-2' })
  }
  screen.render()
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

  const focusArrows = splitMode === 'lr' ? '←→' : '↑↓'
  const scrollKeys = splitMode === 'lr' ? '↑↓ / PgUp PgDn' : 'PgUp PgDn / j k'
  const splitLabel = splitMode === 'lr' ? 'left/right' : 'up/down'

  footer.setContent(
    ` {bold}[1]{/bold} restart server   {bold}[2]{/bold} restart client   {bold}[a]{/bold} restart all   ` +
      `{bold}[${focusArrows}]{/bold} switch focus   {bold}[${scrollKeys}]{/bold} scroll   ` +
      `{bold}[s]{/bold} split: ${splitLabel}   {bold}[q]{/bold} quit\n` +
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

// The keys used to switch focus between panes follow the current split
// orientation: ←→ when the panes sit side by side, ↑↓ when they're stacked.
const FOCUS_KEYS = { lr: ['left', 'right'], ud: ['up', 'down'] }
const switchFocus = () => focus(focused === 'server' ? 'client' : 'server')

function bindFocusKeys() {
  screen.key(FOCUS_KEYS[splitMode], switchFocus)
}

function toggleSplit() {
  screen.unkey(FOCUS_KEYS[splitMode], switchFocus)
  splitMode = splitMode === 'lr' ? 'ud' : 'lr'
  bindFocusKeys()
  layoutBoxes()
  renderFooter()
}

screen.key(['s'], () => toggleSplit())
screen.key(['q', 'C-c'], () => quit())

process.on('SIGINT', quit)
process.on('SIGTERM', quit)

layoutBoxes()
bindFocusKeys()
updateLabel('server')
updateLabel('client')
renderFooter()
focus('server')

startService('server')
startService('client')
