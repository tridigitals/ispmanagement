export const SETTINGS_LAYOUT_KEY = 'mikrotik_wallboard_layout';
export const SETTINGS_SLOTS_KEY = 'mikrotik_wallboard_slots_json';
export const ROTATE_MODE_KEY = 'mikrotik_wallboard_rotate_mode';
export const ROTATE_MS_KEY = 'mikrotik_wallboard_rotate_ms';
export const FOCUS_MODE_KEY = 'mikrotik_wallboard_focus_mode';
export const STATUS_FILTER_KEY = 'mikrotik_wallboard_status_filter';
export const POLL_MS_KEY = 'mikrotik_wallboard_poll_ms';
export const ALERT_SOUND_KEY = 'mikrotik_wallboard_alert_sound';
export const KEEP_AWAKE_KEY = 'mikrotik_wallboard_keep_awake';

export const WALLBOARD_LAYOUT_PRESETS = ['2x2', '3x2', '3x3', '4x3'] as const;
export const WALLBOARD_ROTATE_MODES = ['manual', 'auto'] as const;
export const WALLBOARD_STATUS_FILTERS = ['all', 'online', 'offline'] as const;
export const WALLBOARD_ROTATE_MS_OPTIONS = [5000, 10000, 15000, 30000, 60000] as const;
export const WALLBOARD_POLL_MS_OPTIONS = [1000, 2000, 5000] as const;

export type LayoutPreset = (typeof WALLBOARD_LAYOUT_PRESETS)[number];
export type RotateMode = (typeof WALLBOARD_ROTATE_MODES)[number];
export type StatusFilter = (typeof WALLBOARD_STATUS_FILTERS)[number];

export function isLayoutPreset(v: string | null): v is LayoutPreset {
  return !!v && (WALLBOARD_LAYOUT_PRESETS as readonly string[]).includes(v);
}

export function isRotateMode(v: string | null): v is RotateMode {
  return !!v && (WALLBOARD_ROTATE_MODES as readonly string[]).includes(v);
}

export function isStatusFilter(v: string | null): v is StatusFilter {
  return !!v && (WALLBOARD_STATUS_FILTERS as readonly string[]).includes(v);
}

