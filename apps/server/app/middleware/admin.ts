// UX-only gate on the /dashboard route tree (see pages/dashboard.vue's definePageMeta) - the real
// authorization boundary is server-side (requireAdmin on every /api/admin/* route, see
// server/utils/requireAdmin.ts). This just redirects an obviously-not-logged-in visitor before
// they see a dashboard shell that's about to fail every request.
export default defineNuxtRouteMiddleware(() => {
  const { loggedIn } = useUserSession()

  if (!loggedIn.value) return navigateTo('/')
})
