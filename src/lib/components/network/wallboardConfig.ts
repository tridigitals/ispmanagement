import {
  ALERT_SOUND_KEY,
  FOCUS_MODE_KEY,
  KEEP_AWAKE_KEY,
  POLL_MS_KEY,
  ROTATE_MODE_KEY,
  ROTATE_MS_KEY,
  SETTINGS_LAYOUT_KEY,
  SETTINGS_SLOTS_KEY,
  STATUS_FILTER_KEY,
  WALLBOARD_POLL_MS_OPTIONS,
  WALLBOARD_ROTATE_MS_OPTIONS,
  isLayoutPreset,
  type LayoutPreset,
  type RotateMode,
} from '$lib/constants/wallboard';

export type WallboardSlot = {
  routerId: string;
  iface: string;
  warn_below_rx_bps?: number | null;
  warn_below_tx_bps?: number | null;
};

export type LocalWallboardConfig = {
  layout?: LayoutPreset;
  rotateMode?: RotateMode;
  rotateMs?: number;
  focusMode?: boolean;
  statusFilter?: 'all' | 'offline' | 'online';
  pollMs?: number;
  criticalSoundEnabled?: boolean;
  slotsAll?: (WallboardSlot | null)[];
};

function normalizeSlots(value: unknown): (WallboardSlot | null)[] {
  if (!Array.isArray(value)) return [];
  return value.map((it) => {
    if (!it) return null;

    // Back-compat: old format was just routerId strings.
    if (typeof it === 'string') {
      return { routerId: it, iface: 'ether1', warn_below_rx_bps: null, warn_below_tx_bps: null };
    }

    if (typeof it === 'object') {
      const row = it as {
        routerId?: unknown;
        iface?: unknown;
        warn_below_rx_bps?: unknown;
        warn_below_tx_bps?: unknown;
      };
      if (typeof row.routerId === 'string' && typeof row.iface === 'string') {
        return {
          routerId: row.routerId,
          iface: row.iface,
          warn_below_rx_bps: typeof row.warn_below_rx_bps === 'number' ? row.warn_below_rx_bps : null,
          warn_below_tx_bps: typeof row.warn_below_tx_bps === 'number' ? row.warn_below_tx_bps : null,
        };
      }
    }

    return null;
  });
}

export function persistWallboardLocalConfig(config: {
  layout: LayoutPreset;
  slotsAll: (WallboardSlot | null)[];
  rotateMode: RotateMode;
  rotateMs: number;
  focusMode: boolean;
  statusFilter: 'all' | 'offline' | 'online';
  pollMs: number;
  criticalSoundEnabled: boolean;
}) {
  try {
    localStorage.setItem('mikrotik_wallboard_layout', config.layout);
    localStorage.setItem('mikrotik_wallboard_slots', JSON.stringify(config.slotsAll));
    localStorage.setItem(ROTATE_MODE_KEY, config.rotateMode);
    localStorage.setItem(ROTATE_MS_KEY, String(config.rotateMs));
    localStorage.setItem(FOCUS_MODE_KEY, config.focusMode ? '1' : '0');
    localStorage.setItem(STATUS_FILTER_KEY, config.statusFilter);
    localStorage.setItem(POLL_MS_KEY, String(config.pollMs));
    localStorage.setItem(ALERT_SOUND_KEY, config.criticalSoundEnabled ? '1' : '0');
    localStorage.setItem(KEEP_AWAKE_KEY, '1');
  } catch {
    // ignore
  }
}

export function loadWallboardLocalConfig(): LocalWallboardConfig {
  const out: LocalWallboardConfig = {};
  try {
    const l = localStorage.getItem('mikrotik_wallboard_layout');
    if (isLayoutPreset(l)) out.layout = l;

    const rm = localStorage.getItem(ROTATE_MODE_KEY);
    if (rm === 'manual' || rm === 'auto') out.rotateMode = rm;

    const rms = Number(localStorage.getItem(ROTATE_MS_KEY) || 10000);
    if ((WALLBOARD_ROTATE_MS_OPTIONS as readonly number[]).includes(rms)) out.rotateMs = rms;

    const sf = localStorage.getItem(STATUS_FILTER_KEY);
    if (sf === 'all' || sf === 'online' || sf === 'offline') out.statusFilter = sf;

    const pm = Number(localStorage.getItem(POLL_MS_KEY) || 1000);
    if ((WALLBOARD_POLL_MS_OPTIONS as readonly number[]).includes(pm)) out.pollMs = pm;

    const sd = localStorage.getItem(ALERT_SOUND_KEY);
    if (sd === '0' || sd === '1') out.criticalSoundEnabled = sd === '1';

    const slots = localStorage.getItem('mikrotik_wallboard_slots');
    if (slots) {
      out.slotsAll = normalizeSlots(JSON.parse(slots));
    }
  } catch {
    // ignore
  }
  return out;
}

export async function loadWallboardRemoteConfig(args: {
  canUseTenantSettings: boolean;
  getValue: (key: string) => Promise<string | null>;
}): Promise<{ layout?: LayoutPreset; slotsAll?: (WallboardSlot | null)[]; remoteLoaded: true }> {
  if (!args.canUseTenantSettings) {
    return { remoteLoaded: true };
  }

  try {
    const [remoteLayout, remoteSlots] = await Promise.all([
      args.getValue(SETTINGS_LAYOUT_KEY),
      args.getValue(SETTINGS_SLOTS_KEY),
    ]);

    const out: { layout?: LayoutPreset; slotsAll?: (WallboardSlot | null)[]; remoteLoaded: true } = {
      remoteLoaded: true,
    };
    if (isLayoutPreset(remoteLayout)) out.layout = remoteLayout;
    if (remoteSlots) out.slotsAll = normalizeSlots(JSON.parse(remoteSlots));
    return out;
  } catch {
    // ignore (wallboard should always load)
    return { remoteLoaded: true };
  }
}

export function createWallboardRemotePersister(args: {
  canUseTenantSettings: () => boolean;
  remoteLoaded: () => boolean;
  getLayout: () => LayoutPreset;
  getSlotsAll: () => (WallboardSlot | null)[];
  upsert: (key: string, value: string, description: string) => Promise<unknown>;
  delayMs?: number;
}) {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let lastPayload: string | null = null;

  async function persistRemoteNow() {
    if (!args.remoteLoaded() || !args.canUseTenantSettings()) return;
    const layout = args.getLayout();
    const slotsAll = args.getSlotsAll();
    const payload = JSON.stringify({ layout, slots: slotsAll });
    if (payload === lastPayload) return;
    lastPayload = payload;

    try {
      await Promise.all([
        args.upsert(SETTINGS_LAYOUT_KEY, layout, 'Wallboard layout preset (tenant scoped)'),
        args.upsert(SETTINGS_SLOTS_KEY, JSON.stringify(slotsAll), 'Wallboard interface tiles (tenant scoped)'),
      ]);
    } catch {
      // ignore: avoid spamming toasts on background saves
    }
  }

  function schedulePersistRemote() {
    if (!args.remoteLoaded()) return;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => void persistRemoteNow(), args.delayMs ?? 700);
  }

  function clearScheduledPersist() {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
  }

  return {
    persistRemoteNow,
    schedulePersistRemote,
    clearScheduledPersist,
  };
}
