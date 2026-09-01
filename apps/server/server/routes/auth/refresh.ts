// Hacky fix found here https://github.com/atinux/nuxt-auth-utils/issues/356#issuecomment-2773560411 to refresh session duration

export default defineEventHandler(async (event) => {
  const session = await getUserSession(event)

  Object.entries(event.context.sessions ?? {}).forEach(
    ([k, v]) => (event.context.sessions![k] = { ...v, createdAt: Date.now() })
  )

  await setUserSession(event, session)

  return { success: true }
})
