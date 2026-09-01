export default defineEventHandler(async (event) => {
  const { user } = await requireUserSession(event)
  const { username } = await readBody(event)

  await setUserSession(event, {
    user: {
      ...user,
      userDefinedDisplayName: username
    }
  })

  return { success: true }
})
