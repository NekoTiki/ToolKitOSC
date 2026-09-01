// OSC wire types shared between the desktop client's Tauri backend/renderer and the server's
// relay. Previously duplicated: the client declared these in src/renderer/src/env.d.ts, the
// server's useOscMessages.ts redeclared just OSCArg locally.

export type OSCArg = number | string | boolean

export interface OSCMessage {
  address: string
  args: OSCArg[]
}

export interface OscCommand {
  address: string
  args: OSCArg[]
}

export type AvatarParameter = {
  name: string
  input?: {
    address: string
    type: 'Bool' | 'Float' | 'Int'
  }
  output?: {
    address: string
    type: 'Bool' | 'Float' | 'Int'
  }
}

export type AvatarDetails = {
  id: `avtr_${string}`
  name: string
  hash: number
  parameters: AvatarParameter[]
}
