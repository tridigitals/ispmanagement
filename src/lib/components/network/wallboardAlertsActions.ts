import { api } from '$lib/api/client';
import { toast } from '$lib/stores/toast';
import type { WallboardAlertRow, WallboardIncidentRow } from './wallboardDerivations';

export async function loadWallboardAlerts(silent = true): Promise<WallboardAlertRow[]> {
  try {
    return ((await api.mikrotik.alerts.list({ activeOnly: true, limit: 300 })) as any) || [];
  } catch (e: any) {
    if (!silent) toast.error(e?.message || e);
    return [];
  }
}

export async function loadWallboardIncidents(silent = true): Promise<WallboardIncidentRow[]> {
  try {
    return ((await api.mikrotik.incidents.list({ activeOnly: false, limit: 500 })) as any) || [];
  } catch (e: any) {
    if (!silent) toast.error(e?.message || e);
    return [];
  }
}

export async function ackWallboardIncident(id: string, ackedLabel: string) {
  await api.mikrotik.incidents.ack(id);
  toast.success(ackedLabel);
}

export async function resolveWallboardIncident(id: string, resolvedLabel: string) {
  await api.mikrotik.incidents.resolve(id);
  toast.success(resolvedLabel);
}

export async function ackWallboardAlerts(ids: string[], ackedLabel: string) {
  for (const id of ids) {
    await api.mikrotik.alerts.ack(id);
  }
  toast.success(ackedLabel);
}

export async function muteWallboardRouter(routerId: string, minutes: number, snoozedLabel: string) {
  const until = new Date(Date.now() + minutes * 60 * 1000).toISOString();
  await api.mikrotik.routers.update(routerId, {
    maintenance_until: until,
    maintenance_reason: `Snoozed from wallboard for ${minutes}m`,
  });
  toast.success(snoozedLabel);
}

export async function unmuteWallboardRouter(routerId: string, unmutedLabel: string) {
  await api.mikrotik.routers.update(routerId, {
    maintenance_until: null,
    maintenance_reason: null,
  });
  toast.success(unmutedLabel);
}
