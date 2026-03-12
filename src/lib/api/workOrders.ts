import { getTokenOrThrow, safeInvoke } from './core';
import type {
  InstallationWorkOrderView,
  TeamMember,
  WorkOrderRescheduleRequestView,
} from './types';

export const workOrders = {
  list: (params?: {
    status?: string;
    assigned_to?: string;
    include_closed?: boolean;
    limit?: number;
  }): Promise<InstallationWorkOrderView[]> =>
    safeInvoke('list_installation_work_orders', {
      token: getTokenOrThrow(),
      ...(params || {}),
    }),

  assignees: (): Promise<TeamMember[]> =>
    safeInvoke('list_installation_assignees', {
      token: getTokenOrThrow(),
    }),

  assign: (id: string, payload: { assigned_to: string; scheduled_at?: string; notes?: string }) =>
    safeInvoke('assign_installation_work_order', {
      token: getTokenOrThrow(),
      id,
      ...payload,
    }),

  claim: (id: string, notes?: string) =>
    safeInvoke('claim_installation_work_order', {
      token: getTokenOrThrow(),
      id,
      notes: notes ?? undefined,
    }),

  release: (id: string, notes?: string) =>
    safeInvoke('release_installation_work_order', {
      token: getTokenOrThrow(),
      id,
      notes: notes ?? undefined,
    }),

  start: (id: string, notes?: string) =>
    safeInvoke('start_installation_work_order', {
      token: getTokenOrThrow(),
      id,
      notes: notes ?? undefined,
    }),

  complete: (id: string, notes?: string) =>
    safeInvoke('complete_installation_work_order', {
      token: getTokenOrThrow(),
      id,
      notes: notes ?? undefined,
    }),

  cancel: (id: string, notes?: string) =>
    safeInvoke('cancel_installation_work_order', {
      token: getTokenOrThrow(),
      id,
      notes: notes ?? undefined,
    }),

  reopen: (id: string, notes?: string) =>
    safeInvoke('reopen_installation_work_order', {
      token: getTokenOrThrow(),
      id,
      notes: notes ?? undefined,
    }),

  getRescheduleRequest: (id: string): Promise<WorkOrderRescheduleRequestView | null> =>
    safeInvoke('get_pending_work_order_reschedule_request', {
      token: getTokenOrThrow(),
      id,
    }),

  approveReschedule: (id: string, payload?: { scheduled_at?: string; notes?: string }) =>
    safeInvoke('approve_work_order_reschedule_request', {
      token: getTokenOrThrow(),
      id,
      ...(payload || {}),
    }),

  rejectReschedule: (id: string, payload: { notes: string }) =>
    safeInvoke('reject_work_order_reschedule_request', {
      token: getTokenOrThrow(),
      id,
      ...payload,
    }),
};
