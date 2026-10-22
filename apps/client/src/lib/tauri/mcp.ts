// AI assistants - src-tauri/src/mcp/. Mirrors mcp/model.rs (what mcp.json holds) and the
// assistants connected now (mcp/mod.rs `Client`).
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export type McpPermission = 'off' | 'ask' | 'on'

export interface McpLevels {
  read: McpPermission
  play: McpPermission
  edit: McpPermission
  shock: McpPermission
}

export interface McpSettings {
  port: number
  levels: McpLevels
  rawParameters: boolean
  switchProfiles: boolean
}

export interface McpClient {
  id: number
  // From its MCP handshake, e.g. "claude-code".
  name: string
  // Unix ms.
  since: number
  calls: number
  lastTool: string | null
}

// Rust asking the webview (see mcp/bridge.rs): a read of data that lives here, or `approve` for
// the approval pop-up. Answer with mcpRespond.
export interface McpRequest {
  id: number
  kind: string
  // The assistant asking.
  client: string
  args: Record<string, unknown>
}

export const mcpApi = {
  mcpSettings: (): Promise<McpSettings> => invoke('mcp_settings'),
  mcpSave: (settings: McpSettings): Promise<void> => invoke('mcp_save', { settings }),
  // The key, only when you ask to see or copy it.
  mcpKey: (): Promise<string> => invoke('mcp_key'),
  // A new key cuts off every assistant set up with the old one.
  mcpNewKey: (): Promise<string> => invoke('mcp_new_key'),
  mcpClients: (): Promise<McpClient[]> => invoke('mcp_clients'),
  mcpRespond: (id: number, answer: { result: unknown } | { error: string }): Promise<void> => invoke('mcp_respond', { id, ...answer }),
  onMcpRequest: (callback: (request: McpRequest) => void): void => {
    void listen<McpRequest>('mcp-request', (event) => callback(event.payload))
  },
  onMcpClients: (callback: (clients: McpClient[]) => void): void => {
    void listen<McpClient[]>('mcp-clients', (event) => callback(event.payload))
  }
}
