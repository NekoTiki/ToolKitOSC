export default defineEventHandler(async (event) => {
  const { user } = await getUserSession(event)
  const { username } = await readBody(event)

  await setUserSession(event, {
    user: {
      ...user,
      userDefinedDisplayName: username
    }
  })

  return { success: true }
})
