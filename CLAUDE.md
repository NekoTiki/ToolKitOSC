# CLAUDE.md

## Avatar parameters

- Every avatar parameter the app itself reads or writes (as opposed to the user's own parameters that controls point at) is named `TKOSC/<Name>`, e.g. `TKOSC/Panic`. `TKOSC` stands for ToolKitOSC. Its OSC address is `/avatar/parameters/TKOSC/<Name>`.

## Releases

### Cutting a release

- Releases are cut from `origin/master` by pushing tags: `client-vX.Y.Z` and `server-vX.Y.Z` (annotated, message `client vX.Y.Z` / `server vX.Y.Z`). The two can share a version number.
- The version comes from the tag. `package.json`, `tauri.conf.json` and `Cargo.toml` are not bumped.
- `client-v*` runs `release-client.yml`. It creates a **draft** GitHub release named `ToolKitOSC vX.Y.Z` with auto-generated notes, builds Windows (nsis) and Linux (deb + AppImage), then publishes the draft. Publishing only flips `draft=false`, so you can rewrite the notes while the builds run (`gh release edit client-vX.Y.Z --notes-file ...`).
- `server-v*` runs `release-server.yml`. It publishes `ghcr.io/nekotiki/toolkitosc-server:X.Y.Z` and `:latest` (amd64 + arm64). There is no GitHub release for the server; server changes go in the client release notes.

### Release notes style

Write for the people who use the app (streamers and their viewers), not for developers. Look at the PRs and commits since the previous stable `client-v*` tag and describe what changes for the user. Leave out internals like refactors, CI, file layout, protocol details or function names.

Structure, in this order (leave out sections that would be empty):

```markdown
# <emoji> ToolKitOSC vX.Y.Z

<One or two sentences: the theme of the release, with **ToolKitOSC** and the headline topic in bold.>

## ✨ What's new

- <emoji> **<Feature name>:** <what the user can do now, in plain sentences.>

## 🐛 Fixes

- <emoji> **<Area>:** <what was wrong from the user's point of view, and that it now works.>

## ⚠️ Known limitations on <platform>

- <emoji> **<Feature>** <what doesn't work there yet, and briefly why if it helps.>

## 📦 Install

1. ⬇️ Download the file for your system below:
   - 🪟 **Windows:** `ToolKitOSC_X.Y.Z_x64-setup.exe`
   - 🐧 **Debian / Ubuntu:** `ToolKitOSC_X.Y.Z_amd64.deb`, then install it with `sudo apt install ./ToolKitOSC_X.Y.Z_amd64.deb`
   - 🐧 **Other distributions:** `ToolKitOSC_X.Y.Z_amd64.AppImage`, then make it executable with `chmod +x ToolKitOSC_X.Y.Z_amd64.AppImage`
2. ▶️ Run it and launch ToolKitOSC.
3. 🥽 Make sure OSC is enabled in VRChat.

Already on v<previous>? The app updates itself.

**Full Changelog**: https://github.com/NekoTiki/ToolKitOSC/compare/client-v<previous>...client-vX.Y.Z
```

Rules:

- The title emoji matches the release's theme (🎉 first stable, 🐧 Linux, and so on).
- Every bullet starts with one emoji that fits it, then a bold label ending in a colon (`**Label:**`), then plain sentences.
- Use the app's current UI labels in bold (e.g. **Launch with SteamVR**, **Launch with VRChat**, Settings > Startup). File names, commands, addresses and env vars go in backticks.
- Keep the sentences short and plain, with no marketing tone. Address the reader as "you" ("shows which avatar you're wearing").
- When a fix or feature is server-side, add a line before the Full Changelog for self-hosters: `🐳 Running your own server? ... pull ghcr.io/nekotiki/toolkitosc-server:X.Y.Z (or latest).`
- Always end with the **Full Changelog** compare link against the previous stable client tag (never a `server-v*` tag).
