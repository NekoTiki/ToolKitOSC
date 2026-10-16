// The app itself: opening links, the tray, and the SteamVR and VRCX auto-launch integrations -
// src-tauri/src/app.rs, steamvr/ and vrcx/.
import { invoke } from '@tauri-apps/api/core'

// Whether an optional integration (SteamVR, VRCX) can be offered here - mirrors `Availability` in
// src-tauri/src/app.rs. 'unsupported' means it can't work on this OS at all, 'not-installed'
// that it could but the app it hooks into wasn't found.
export type IntegrationAvailability = 'available' | 'not-installed' | 'unsupported'

export const appApi = {
  openUrl: (url: string): void => {
    void invoke('open_url', { url })
  },
  steamVrAvailable: (): Promise<IntegrationAvailability> => invoke('steamvr_available'),
  steamVrSetAutoLaunch: (enable: boolean): Promise<void> =>
    invoke('steamvr_set_auto_launch', { enable }),
  steamVrGetAutoLaunch: (): Promise<boolean> => invoke('steamvr_get_auto_launch'),
  vrcxAvailable: (): Promise<IntegrationAvailability> => invoke('vrcx_available'),
  vrcxSetAutoLaunch: (enable: boolean): Promise<void> => invoke('vrcx_set_auto_launch', { enable }),
  vrcxGetAutoLaunch: (): Promise<boolean> => invoke('vrcx_get_auto_launch'),
  setMinimizeToTray: (enabled: boolean): void => {
    void invoke('set_minimize_to_tray', { enabled })
  }
}
