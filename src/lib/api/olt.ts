import { getTokenOrThrow, safeInvoke } from './core';

export interface Olt {
  id: string;
  tenant_id: string;
  name: string;
  description?: string | null;
  olt_type: string;
  host: string;
  port: number;
  username: string;
  is_online?: boolean;
  last_seen_at?: string | null;
  last_error?: string | null;
  created_at?: string;
  updated_at?: string;
}

export interface OltStats {
  olt_id: string;
  olt_name: string;
  is_online: boolean;
  olt_type: string;
  system_info?: {
    uptime?: string | null;
    firmware_version?: string | null;
    serial_number?: string | null;
    model?: string | null;
    [key: string]: any;
  } | null;
  pon_ports?: PonPort[];
  onu_summary?: {
    total: number;
    online: number;
    offline: number;
    low_signal: number;
  };
  [key: string]: any;
}

export interface PonPort {
  port_id: string;
  port_name: string;
  status?: string | null;
  onu_count?: number;
  online_count?: number;
  offline_count?: number;
  [key: string]: any;
}

export interface OltDetails {
  olt_id: string;
  olt_name: string;
  is_online: boolean;
  stats: OltStats;
  onus: OnuDetail[];
}

export interface OnuDetail {
  onu_id: string;
  onu_name?: string | null;
  serial_number?: string | null;
  status?: string | null;
  pon_port?: string | null;
  rx_power?: number | null;
  tx_power?: number | null;
  distance_m?: number | null;
  uptime_seconds?: number | null;
  last_down_reason?: string | null;
  model?: string | null;
  [key: string]: any;
}

export interface OltTestResult {
  ok: boolean;
  message?: string;
  latency_ms?: number;
  firmware_version?: string;
  [key: string]: any;
}

export interface OltOnuHistoryEntry {
  id: string;
  olt_id: string;
  onu_id: string;
  onu_name?: string | null;
  event_type: string;
  message?: string | null;
  details?: any;
  created_at: string;
}

export interface OltPublicToken {
  id: string;
  olt_id: string;
  token: string;
  description?: string | null;
  enabled: boolean;
  expires_at?: string | null;
  created_at: string;
}

export const olt = {
  list: (): Promise<Olt[]> =>
    safeInvoke('list_olts', { token: getTokenOrThrow() }),

  create: (data: {
    name: string;
    description?: string | null;
    olt_type: string;
    host: string;
    port: number;
    username: string;
    password: string;
  }): Promise<Olt> =>
    safeInvoke('create_olt', {
      token: getTokenOrThrow(),
      name: data.name,
      description: data.description ?? null,
      olt_type: data.olt_type,
      oltType: data.olt_type,
      host: data.host,
      port: data.port,
      username: data.username,
      password: data.password,
    }),

  test: (data: {
    host: string;
    port: number;
    username: string;
    password: string;
    olt_type: string;
  }): Promise<OltTestResult> =>
    safeInvoke('test_olt_connection', {
      token: getTokenOrThrow(),
      host: data.host,
      port: data.port,
      username: data.username,
      password: data.password,
      olt_type: data.olt_type,
      oltType: data.olt_type,
    }),

  get: (id: string): Promise<Olt> =>
    safeInvoke('get_olt', { token: getTokenOrThrow(), id }),

  update: (
    id: string,
    data: {
      name?: string;
      description?: string | null;
      host?: string;
      port?: number;
      username?: string;
      password?: string;
    },
  ): Promise<Olt> =>
    safeInvoke('update_olt', {
      token: getTokenOrThrow(),
      id,
      ...data,
    }),

  delete: (id: string): Promise<void> =>
    safeInvoke('delete_olt', { token: getTokenOrThrow(), id }),

  stats: (id: string, forceRefresh?: boolean): Promise<OltStats> =>
    safeInvoke('get_olt_stats', {
      token: getTokenOrThrow(),
      id,
      force_refresh: forceRefresh ?? false,
      forceRefresh: forceRefresh ?? false,
    }),

  details: (id: string): Promise<OltDetails> =>
    safeInvoke('get_olt_details', { token: getTokenOrThrow(), id }),

  rebootOnu: (id: string, onuId: string, onuName: string): Promise<any> =>
    safeInvoke('reboot_olt_onu', {
      token: getTokenOrThrow(),
      id,
      olt_id: id,
      oltId: id,
      onu_id: onuId,
      onuId,
      onu_name: onuName,
      onuName,
    }),

  onuHistory: (id: string, limit?: number): Promise<OltOnuHistoryEntry[]> =>
    safeInvoke('list_olt_onu_history', {
      token: getTokenOrThrow(),
      id,
      limit: limit ?? 200,
    }),

  listPublicTokens: (id: string): Promise<OltPublicToken[]> =>
    safeInvoke('list_olt_public_tokens', { token: getTokenOrThrow(), id }),

  createPublicToken: (
    id: string,
    data: {
      description?: string | null;
      enabled: boolean;
      expires_at?: string | null;
    },
  ): Promise<OltPublicToken> =>
    safeInvoke('create_olt_public_token', {
      token: getTokenOrThrow(),
      id,
      description: data.description ?? null,
      enabled: data.enabled,
      expires_at: data.expires_at ?? null,
      expiresAt: data.expires_at ?? null,
    }),

  deletePublicToken: (id: string, tokenId: string): Promise<void> =>
    safeInvoke('delete_olt_public_token', {
      token: getTokenOrThrow(),
      id,
      token_id: tokenId,
      tokenId,
    }),
};
