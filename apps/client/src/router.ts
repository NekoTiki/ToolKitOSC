import { createRouter, createWebHashHistory } from 'vue-router'

// Hash history, not HTML5 history: the packaged app is served from Tauri's custom asset protocol,
// which has no SPA fallback - a reload on `/controls` would look for a real file there. Hash URLs
// always load index.html.
export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/controls' },
    {
      path: '/controls/:groupId?',
      name: 'controls',
      component: () => import('@renderer/pages/ControlsPage.vue')
    },
    {
      path: '/controls/:groupId/new',
      name: 'control-new',
      component: () => import('@renderer/pages/ControlEditorPage.vue')
    },
    {
      path: '/controls/:groupId/:controlId',
      name: 'control-edit',
      component: () => import('@renderer/pages/ControlEditorPage.vue')
    },
    { path: '/presets', name: 'presets', component: () => import('@renderer/pages/PresetsPage.vue') },
    { path: '/presets/new', name: 'preset-new', component: () => import('@renderer/pages/PresetsPage.vue') },
    { path: '/presets/:presetId', name: 'preset', component: () => import('@renderer/pages/PresetsPage.vue') },
    { path: '/parameters', name: 'parameters', component: () => import('@renderer/pages/ParametersPage.vue') },
    { path: '/parameters/log', name: 'parameter-log', component: () => import('@renderer/pages/ParameterLogPage.vue') },
    { path: '/profiles', name: 'profiles', component: () => import('@renderer/pages/ProfilesPage.vue') },
    { path: '/profiles/new', name: 'profile-new', component: () => import('@renderer/pages/ProfilesPage.vue') },
    { path: '/profiles/:profileId', name: 'profile', component: () => import('@renderer/pages/ProfilesPage.vue') },
    { path: '/viewers/:clientId?', name: 'viewers', component: () => import('@renderer/pages/ViewersPage.vue') },
    { path: '/activity', name: 'activity', component: () => import('@renderer/pages/ActivityPage.vue') },
    { path: '/ai', name: 'ai', component: () => import('@renderer/pages/AiPage.vue') },
    { path: '/settings/:section?', name: 'settings', component: () => import('@renderer/pages/SettingsPage.vue') },
    { path: '/:pathMatch(.*)*', redirect: '/controls' }
  ]
})
