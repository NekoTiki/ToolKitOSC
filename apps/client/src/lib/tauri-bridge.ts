// The webview's way into Rust: every Tauri command and event, split by Rust module under
// lib/tauri/ (one file each, next to its types). This file keeps them together as one `api`
// object, with the same method names the old Electron preload's `window.api` had, so the
// composables built on top of it barely had to change when the app moved to Tauri.
import { addonsApi } from './tauri/addons'
import { appApi } from './tauri/app'
import { automationsApi } from './tauri/automations'
import { boardsApi } from './tauri/boards'
import { controlsApi } from './tauri/controls'
import { hapticsApi } from './tauri/haptics'
import { openshockApi } from './tauri/openshock'
import { oscApi } from './tauri/osc'
import { serverApi } from './tauri/server'
import { toysApi } from './tauri/toys'

export type * from './tauri/addons'
export type * from './tauri/app'
export type * from './tauri/automations'
export type * from './tauri/boards'
export type * from './tauri/controls'
export type * from './tauri/haptics'
export type * from './tauri/openshock'
export type * from './tauri/osc'
export type * from './tauri/server'
export type * from './tauri/toys'

export const api = {
  ...appApi,
  ...addonsApi,
  ...oscApi,
  ...controlsApi,
  ...boardsApi,
  ...automationsApi,
  ...serverApi,
  ...openshockApi,
  ...toysApi,
  ...hapticsApi
}

export type TauriApiType = typeof api
