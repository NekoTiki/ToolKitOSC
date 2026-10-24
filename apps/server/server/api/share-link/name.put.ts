import { z } from 'zod'

import { isValidShareName } from '#shared/utils/shortLink'
import { setCustomShareName } from '~~/server/db/shareLinks'

const bodySchema = z.object({ name: z.string().nullable() })

// Sets the host's custom link name, or clears it with null.
export default defineEventHandler(async (event) => {
  const discordId = await shareLinkOwner(event)
  const { name } = await readValidatedBody(event, bodySchema.parse)
  const normalized = name?.trim().toLowerCase() || null

  if (normalized && !isValidShareName(normalized)) {
    throw createError({ statusCode: 400, statusMessage: 'Use 3 to 24 lowercase letters, digits or dashes' })
  }

  if ((await setCustomShareName(discordId, normalized)) === 'taken') {
    throw createError({ statusCode: 409, statusMessage: 'This name is taken' })
  }

  return { custom: normalized }
})
