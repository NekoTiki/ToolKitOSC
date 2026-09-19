// Lets the browser app decide whether to show a "Dashboard" link (see app/app.vue) without
// guessing client-side - reuses the same requireAdmin gate every other /api/admin/* route uses, so
// "has access" is always the real, current, server-side answer, not a stale flag baked into the
// session at login time.
export default defineEventHandler(async (event): Promise<{ isAdmin: true }> => {
  await requireAdmin(event)

  return { isAdmin: true }
})
