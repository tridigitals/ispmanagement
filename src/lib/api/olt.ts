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
  last_polled_at?: string | null;
  last_updated?: string | null;
  last_error?: string | null;
  last_stats?: any;
  created_at?: string;
  updated_at?: string;
}

export interface PonPort {
  name: string;
  total: number;
  online: number;
  offline: number;
  [key: string]: any;
}

export interface OltStats {
  name?: string;
  ip?: string;
  pon_ports?: PonPort[];
  total_onus: number;
  online_onus: number;
  offline_onus: number;
  low_onus: number;
  risk_onus: number;
  [key: string]: any;
}

export interface OltSystemInfo {
  name: string;
  model: string;
  version: string;
  address: string;
}

export interface OltStatsResponse {
  status: string;
  data: OltStats;
  info?: OltSystemInfo | null;
  cached: boolean;
  is_online: boolean;
  updated_at?: string | null;
}

export interface OltDetails {
  status: string;
  info: OltSystemInfo;
  onus: OnuDetail[];
  stats: OltStats;
}

export interface OnuDetail {
  onu_id: string;
  name: string;
  mac: string;
  status: string;
  rx: string;
  tx?: string | null;
  distance?: string | null;
  temperature?: string | null;
  pon: string;
  olt_id?: string | null;
  olt_name?: string | null;
  [key: string]: any;
}

export interface OltTestResult {
  success: boolean;
  info?: OltSystemInfo | null;
  error?: string | null;
  [key: string]: any;
}

export interface OltOnuHistoryEntry {
  id: string;
  olt_id: string;
  tenant_id: string;
  onu_id: string;
  pon: string;
  mac?: string | null;
  name?: string | null;
  status: string;
  rx_power?: number | null;
  tx_power?: number | null;
  distance?: number | null;
  temperature?: number | null;
  recorded_at: string;
}

export interface OltPublicToken {
  id: string;
  olt_id: string;
  tenant_id?: string;
  token: string;
  description?: string | null;
  enabled: boolean;
  expires_at?: string | null;
  created_at: string;
}

/** Unwrap { status, data } envelope from backend responses */
function unwrap<T>(res: any): T {
  if (res && typeof res === 'object' && 'data' in res) return res.data as T;
  return res as T;
}

export const olt = {
  list: (): Promise<Olt[]> =>
    safeInvoke('list_olts', { token: getTokenOrThrow() }).then((r: any) => unwrap<Olt[]>(r)),

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
      host: data.host,
      port: data.port,
      username: data.username,
      password: data.password,
    }).then((r: any) => unwrap<Olt>(r)),

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
    }) as Promise<OltTestResult>,

  get: (id: string): Promise<Olt> =>
    safeInvoke('get_olt', { token: getTokenOrThrow(), id }).then((r: any) => unwrap<Olt>(r)),

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
    }).then((r: any) => unwrap<Olt>(r)),

  delete: (id: string): Promise<void> =>
    safeInvoke('delete_olt', { token: getTokenOrThrow(), id }) as Promise<void>,

  stats: (id: string, forceRefresh?: boolean): Promise<OltStatsResponse> =>
    safeInvoke('get_olt_stats', {
      token: getTokenOrThrow(),
      id,
      force_refresh: forceRefresh ?? false,
    }) as Promise<OltStatsResponse>,

  details: (id: string): Promise<OltDetails> =>
    safeInvoke('get_olt_details', { token: getTokenOrThrow(), id }) as Promise<OltDetails>,

  rebootOnu: (id: string, onuId: string, onuName: string): Promise<any> =>
    safeInvoke('reboot_olt_onu', {
      token: getTokenOrThrow(),
      id,
      onu_id: onuId,
      onu_name: onuName,
    }),

  onuHistory: (id: string, limit?: number): Promise<OltOnuHistoryEntry[]> =>
    safeInvoke('list_olt_onu_history', {
      token: getTokenOrThrow(),
      id,
      limit: limit ?? 200,
    }).then((r: any) => unwrap<OltOnuHistoryEntry[]>(r)),

  listPublicTokens: (id: string): Promise<OltPublicToken[]> =>
    safeInvoke('list_olt_public_tokens', { token: getTokenOrThrow(), id }).then((r: any) =>
      unwrap<OltPublicToken[]>(r),
    ),

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
    }).then((r: any) => unwrap<OltPublicToken>(r)),

  deletePublicToken: (id: string, tokenId: string): Promise<void> =>
    safeInvoke('delete_olt_public_token', {
      token: getTokenOrThrow(),
      id,
      token_id: tokenId,
    }) as Promise<void>,
};
