import type { LayoutPreset, RotateMode } from '$lib/constants/wallboard';
import type { LocalWallboardConfig, WallboardSlot } from './wallboardConfig';

type WallboardConfigSetters = {
  setLayout: (v: LayoutPreset) => void;
  setRotateMode: (v: RotateMode) => void;
  setRotateMs: (v: number) => void;
  setStatusFilter: (v: 'all' | 'offline' | 'online') => void;
  setPollMs: (v: number) => void;
  setCriticalSoundEnabled: (v: boolean) => void;
  setSlotsAll: (v: (WallboardSlot | null)[]) => void;
};

export function applyLocalWallboardConfigState(
  conf: LocalWallboardConfig,
  setters: WallboardConfigSetters,
) {
  if (conf.layout) setters.setLayout(conf.layout);
  if (conf.rotateMode) setters.setRotateMode(conf.rotateMode);
  if (conf.rotateMs != null) setters.setRotateMs(conf.rotateMs);
  if (conf.statusFilter) setters.setStatusFilter(conf.statusFilter);
  if (conf.pollMs != null) setters.setPollMs(conf.pollMs);
  if (conf.criticalSoundEnabled != null) setters.setCriticalSoundEnabled(conf.criticalSoundEnabled);
  if (conf.slotsAll) setters.setSlotsAll(conf.slotsAll);
}

export function applyRemoteWallboardConfigState(
  conf: { layout?: LayoutPreset; slotsAll?: (WallboardSlot | null)[]; remoteLoaded: true },
  setters: Pick<WallboardConfigSetters, 'setLayout' | 'setSlotsAll'> & {
    setRemoteLoaded: (v: boolean) => void;
  },
) {
  if (conf.layout) setters.setLayout(conf.layout);
  if (conf.slotsAll) setters.setSlotsAll(conf.slotsAll);
  setters.setRemoteLoaded(conf.remoteLoaded);
}

export function buildLocalWallboardConfigPayload(args: {
  layout: LayoutPreset;
  slotsAll: (WallboardSlot | null)[];
  rotateMode: RotateMode;
  rotateMs: number;
  focusMode: boolean;
  statusFilter: 'all' | 'offline' | 'online';
  pollMs: number;
  criticalSoundEnabled: boolean;
}) {
  return {
    layout: args.layout,
    slotsAll: args.slotsAll,
    rotateMode: args.rotateMode,
    rotateMs: args.rotateMs,
    focusMode: args.focusMode,
    statusFilter: args.statusFilter,
    pollMs: args.pollMs,
    criticalSoundEnabled: args.criticalSoundEnabled,
  };
}

export function persistLocalWallboardConfigState(
  args: Parameters<typeof buildLocalWallboardConfigPayload>[0] & {
    persist: (payload: ReturnType<typeof buildLocalWallboardConfigPayload>) => void;
  },
) {
  args.persist(
    buildLocalWallboardConfigPayload({
      layout: args.layout,
      slotsAll: args.slotsAll,
      rotateMode: args.rotateMode,
      rotateMs: args.rotateMs,
      focusMode: args.focusMode,
      statusFilter: args.statusFilter,
      pollMs: args.pollMs,
      criticalSoundEnabled: args.criticalSoundEnabled,
    }),
  );
}

export function loadLocalWallboardConfigState(
  loadConfig: () => LocalWallboardConfig,
  setters: WallboardConfigSetters,
) {
  applyLocalWallboardConfigState(loadConfig(), setters);
}

export async function loadRemoteWallboardConfigState(
  loadConfig: () => Promise<{ layout?: LayoutPreset; slotsAll?: (WallboardSlot | null)[]; remoteLoaded: true }>,
  setters: Pick<WallboardConfigSetters, 'setLayout' | 'setSlotsAll'> & {
    setRemoteLoaded: (v: boolean) => void;
  },
) {
  const conf = await loadConfig();
  applyRemoteWallboardConfigState(conf, setters);
}
