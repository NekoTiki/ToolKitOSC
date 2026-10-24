import { describe, expect, it } from 'vitest'

import { connectionRefusal } from '../server/utils/connectionCaps'

const limits = { perIp: 3, perRoom: 2 }

describe('connectionRefusal', () => {
  it('lets a connection in under both limits', () => {
    expect(connectionRefusal({ socketsFromIp: 2, viewersInRoom: 1, alreadyInRoom: false }, limits)).toBeNull()
  })

  it('refuses one IP past its socket limit, even for a viewer already in the room', () => {
    expect(connectionRefusal({ socketsFromIp: 3, viewersInRoom: 0, alreadyInRoom: true }, limits)).toBe('too-many-connections')
  })

  it('refuses a new viewer in a full room', () => {
    expect(connectionRefusal({ socketsFromIp: 0, viewersInRoom: 2, alreadyInRoom: false }, limits)).toBe('room-full')
  })

  it('lets another tab of a viewer into a full room', () => {
    expect(connectionRefusal({ socketsFromIp: 1, viewersInRoom: 2, alreadyInRoom: true }, limits)).toBeNull()
  })
})
