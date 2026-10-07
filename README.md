<div align="center">

<img src="apps/client/src/assets/logo.svg" alt="ToolKitOSC logo" width="120" />

# ToolKitOSC

**Turn your VRChat avatar's parameters into a live control panel — and share it.**

A desktop companion app that talks to VRChat over OSC, plus a website where the people you
invite can drive your avatar, toys and shockers in real time.

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
![Platforms: Windows | Linux](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux-0078d6)
![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24c8db)
![Nuxt 4](https://img.shields.io/badge/server-Nuxt%204-00dc82)

</div>

---

## ✨ Features

### 🎛️ Cockpit controls

- Build a board of **controls** mapped to your avatar's OSC parameters — toggles, sliders,
  enums, step enums, boolean groups and presets.
- Arrange them in **groups**, resize tiles, drag and drop, and edit everything from right-click menus.
- Large, touch-friendly tiles and linear sliders designed to stay usable from inside VR.

### 🌐 Share with viewers

- Every host gets a **share page** that viewers open in the browser — no install needed.
- Sign-in with Discord, or join anonymously with a random _Adjective Animal_ name.
- See who's connected and what they did on the **Viewers** and **Activity** pages, and ban anyone who misbehaves.
- **Lock profiles** decide which controls viewers are allowed to touch.
- **Stop everything** from the top bar, a global hotkey or the `TKOSC/Panic` avatar parameter: every toy and
  shocker stops, and viewers see a paused page until you resume.
- Optionally post viewer actions to your **VRChat chatbox**, so the people in your instance see who's controlling you.

### 🔌 Integrations

- **VRChat OSC** — automatic avatar detection, live parameter values and a parameter change log.
- **Intiface / Buttplug** — control connected toys and play vibration patterns.
- **OpenShock** — shocker controls, including the gamble slot-machine reels.
- **SteamVR & VRCX** — optionally launch the toolkit alongside your VR session.

### 🤖 AI suggestions

- Generate control layouts from your avatar's parameters with your choice of AI provider
  (Gemini, Groq, OpenRouter, Mistral or Cloudflare Workers AI).
- Runs keep going in the background and you get a notification when one is done.

### 🎨 Aurora theme

- A two-color theme you pick yourself, shared by the desktop app and the website.

---

## 🧱 Repository layout

This is an npm workspaces monorepo:

| Path                 | What it is                                                                                |
| -------------------- | ----------------------------------------------------------------------------------------- |
| `apps/client`        | Desktop app — **Tauri 2** (Rust) + **Vue 3** + Nuxt UI. Talks OSC to VRChat.              |
| `apps/server`        | Website, share page, admin dashboard and WebSocket relay — **Nuxt 4** + SQLite (Drizzle). |
| `packages/shared-ui` | Control tiles, theme and types used by both the client and the server.                    |
| `packages/dev-cli`   | Interactive launcher that runs server + client side by side with split logs.              |

```text
 VRChat ◄──OSC──► Desktop client ◄──WebSocket──► Server ◄──WebSocket──► Viewers (browser)
                     │
                     ├── Intiface Central (toys)
                     └── OpenShock (shockers)
```

---

## 🚀 Getting started

### Prerequisites

- [Node.js](https://nodejs.org/) 22+
- [Rust toolchain](https://rustup.rs/) and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) (client only)
- Windows or Linux to build the desktop client

### Install

```bash
git clone git@github.com:NekoTiki/ToolKitOSC.git
cd ToolKitOSC
npm install
```

### Configure the server

```bash
cp apps/server/.env.example apps/server/.env
```

At minimum, set `NUXT_SESSION_PASSWORD` and the Discord OAuth credentials
(`NUXT_OAUTH_DISCORD_CLIENT_ID` / `NUXT_OAUTH_DISCORD_CLIENT_SECRET`). AI providers are all
optional — any provider without an API key is simply reported as "not configured". See
[`apps/server/.env.example`](apps/server/.env.example) for every option.

### Run in development

```bash
npm run dev
```

This opens the dev launcher with the server (`http://localhost:3000`) and the client in split
panes — press `1` / `2` to restart either one. You can also run them on their own:

```bash
npm run dev -w apps/server      # Nuxt dev server
npm run dev -w apps/client      # Tauri app pointed at the local server
npm run dev:prod -w apps/client # Tauri app pointed at the deployed server
```

---

## 📦 Building

```bash
npm run build:client             # Installer for your OS (nsis on Windows, deb + AppImage on Linux) under apps/client/src-tauri/target/release/bundle
npm run build -w apps/server     # Nuxt production build in apps/server/.output
```

The server also ships as a Docker image on GitHub Container Registry:

```bash
docker pull ghcr.io/nekotiki/toolkitosc-server:latest
docker run --rm --env-file apps/server/.env -p 3000:3000 ghcr.io/nekotiki/toolkitosc-server:latest
```

To build it yourself, run `docker build -f apps/server/Dockerfile -t toolkitosc-server .` **from the repo root** so the workspace packages resolve.

See [`apps/server/docker-compose.yml`](apps/server/docker-compose.yml) for a setup with a persistent database volume.

### Releases

Releases are cut by pushing a tag:

- `client-vX.Y.Z` — creates a draft GitHub release, builds the Windows installer and the Linux `.deb` and `.AppImage` into it, then publishes it once every build succeeded. A suffixed tag (`client-vX.Y.Z-beta.1`) is published as a prerelease, which the in-app updater ignores.
- `server-vX.Y.Z` — builds the multi-arch server image and pushes it to GitHub Container Registry.

The client release needs these set in the repository's Actions settings:

| Name                                 | Kind     | Purpose                                                       |
| ------------------------------------ | -------- | ------------------------------------------------------------- |
| `VITE_SERVER_WS_URL`                 | Variable | Default server the app connects to (e.g. `wss://vot.nek.ovh`) |
| `TAURI_SIGNING_PRIVATE_KEY`          | Secret   | Signs the installers for the in-app updater                   |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Secret   | Password of that key, if it has one                           |

---

## 🛠️ Development scripts

| Command             | Description                             |
| ------------------- | --------------------------------------- |
| `npm run dev`       | Launch server + client with the dev CLI |
| `npm run lint`      | ESLint across the whole repo            |
| `npm run format`    | Prettier across the whole repo          |
| `npm run typecheck` | Type-check every workspace              |

---

## 🤝 Contributing

Issues and pull requests are welcome. Before opening a PR, please run `npm run lint` and
`npm run typecheck`, and follow the [Conventional Commits](https://www.conventionalcommits.org/)
style used in the history (`feat(client): …`, `fix(server): …`).

---

## 📄 License

ToolKitOSC is free software: you can redistribute it and/or modify it under the terms of
the **GNU General Public License v3.0 or later**. See [`LICENSE`](LICENSE) for the full text.

This project is not affiliated with or endorsed by VRChat Inc.
