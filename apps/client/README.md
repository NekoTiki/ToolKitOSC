# @toolkitosc/client

The ToolKitOSC desktop app — a Tauri + Vue application that bridges VRChat's OSC protocol
to the rest of the toolkit (the companion `@toolkitosc/server` website). Control rendering
components are shared with the server via `@toolkitosc/shared-ui`.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Volar](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Project Setup

Run `npm install` from the monorepo root first (this app is an npm workspace, not a standalone
package). Requires the Rust toolchain (`rustup`) in addition to Node — see
[tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/).

### Development

```bash
npm run dev              # tauri dev, points at a local server (SERVER_WS_URL defaults to localhost:3000)
npm run dev:prod         # tauri dev, points at the deployed server instead
```

### Build

```bash
npm run build             # tauri build — produces a platform installer under src-tauri/target/release/bundle
```

### Rust-only checks

```bash
cd src-tauri
cargo check
cargo build
```
