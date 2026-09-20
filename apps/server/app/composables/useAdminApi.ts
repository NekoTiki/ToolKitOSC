// Thin $fetch wrapper shared by both /dashboard pages, for imperative calls (mutations, manual
// reloads) - the middleware/admin.ts route guard only checks "logged in", so the first real admin
// check for a given visitor happens here, against the live server response, not a client-side
// guess. A 403 (not an admin, or session expired) bounces them back to the homepage with a toast
// instead of leaving them staring at empty tables/failed actions.
//
// The pages' own initial data loads use useFetch directly instead (for real SSR - see
// stats.vue/users.vue), and call redirectIfUnauthorized themselves on its `error` ref, since
// useFetch reports failures there rather than by throwing the way $fetch does.
export function useAdminApi(): {
  adminFetch: <T>(request: string, opts?: Parameters<typeof $fetch>[1]) => Promise<T>
  redirectIfUnauthorized: (error: unknown) => Promise<void>
} {
  const toast = useToast()

  const statusCodeOf = (error: unknown): number | undefined =>
    (error as { statusCode?: number; response?: { status?: number } })?.statusCode ?? (error as { response?: { status?: number } })?.response?.status

  const redirectIfUnauthorized = async (error: unknown): Promise<void> => {
    if (statusCodeOf(error) !== 403) return

    // Toasting server-side would be a no-op anyway (nothing's rendered a toast host yet during
    // SSR), and there's no point risking whatever internal state useToast expects mid-render -
    // the redirect below is what actually matters there, client or server.
    if (import.meta.client) toast.add({ title: 'Not authorized', color: 'error' })

    await navigateTo('/')
  }

  const adminFetch = async <T>(request: string, opts?: Parameters<typeof $fetch>[1]): Promise<T> => {
    try {
      return await $fetch<T>(request, opts)
    } catch (error) {
      await redirectIfUnauthorized(error)

      throw error
    }
  }

  return { adminFetch, redirectIfUnauthorized }
}
