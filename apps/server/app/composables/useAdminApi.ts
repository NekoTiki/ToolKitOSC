// Thin $fetch wrapper shared by both /dashboard pages - the middleware/admin.ts route guard only
// checks "logged in", so the first real admin check for a given visitor happens here, against the
// live server response, not a client-side guess. A 403 (not an admin, or session expired) bounces
// them back to the homepage with a toast instead of leaving them staring at empty tables.
export function useAdminApi(): {
  adminFetch: <T>(request: string, opts?: Parameters<typeof $fetch>[1]) => Promise<T>
} {
  const toast = useToast()

  const adminFetch = async <T>(request: string, opts?: Parameters<typeof $fetch>[1]): Promise<T> => {
    try {
      return await $fetch<T>(request, opts)
    } catch (error) {
      const statusCode = (error as { statusCode?: number; response?: { status?: number } })?.statusCode ?? (error as { response?: { status?: number } })?.response?.status

      if (statusCode === 403) {
        toast.add({ title: 'Not authorized', color: 'error' })
        await navigateTo('/')
      }

      throw error
    }
  }

  return { adminFetch }
}
