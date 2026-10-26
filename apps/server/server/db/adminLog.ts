import { desc, eq } from 'drizzle-orm'
import { alias } from 'drizzle-orm/sqlite-core'

import { getDb } from './index'
import { adminLog, users } from './schema'

export async function logAdminAction(adminId: string, action: string, targetId: string | null, detail: unknown): Promise<void> {
  await getDb().insert(adminLog).values({ adminId, action, targetId, detail, createdAt: new Date() })
}

const admins = alias(users, 'admins')
const targets = alias(users, 'targets')

// Newest first, with names, for one user (their page) or everyone (the Log page).
export async function listAdminLog(targetId?: string, limit = 100) {
  return getDb()
    .select({
      id: adminLog.id,
      action: adminLog.action,
      detail: adminLog.detail,
      createdAt: adminLog.createdAt,
      adminId: adminLog.adminId,
      adminName: admins.displayName,
      targetId: adminLog.targetId,
      targetName: targets.displayName
    })
    .from(adminLog)
    .leftJoin(admins, eq(admins.discordId, adminLog.adminId))
    .leftJoin(targets, eq(targets.discordId, adminLog.targetId))
    .where(targetId ? eq(adminLog.targetId, targetId) : undefined)
    .orderBy(desc(adminLog.id))
    .limit(limit)
}

export type AdminLogEntry = Awaited<ReturnType<typeof listAdminLog>>[number]
