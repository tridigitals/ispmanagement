import { getTokenOrThrow, safeInvoke } from './core';
import type {
  PaginatedResponse,
  SupportTicket,
  SupportTicketDetail,
  SupportTicketListItem,
  SupportTicketMessage,
  SupportTicketStats,
} from './types';

export const support = {
  list: (params?: {
    status?: string;
    search?: string;
    page?: number;
    perPage?: number;
  }): Promise<PaginatedResponse<SupportTicketListItem>> =>
    safeInvoke('list_support_tickets', {
      token: getTokenOrThrow(),
      status: params?.status,
      search: params?.search,
      page: params?.page,
      per_page: params?.perPage,
    }),

  stats: (): Promise<SupportTicketStats> =>
    safeInvoke('get_support_ticket_stats', { token: getTokenOrThrow() }),

  create: (
    subject: string,
    message: string,
    priority?: string,
    attachmentIds?: string[],
  ): Promise<SupportTicketDetail> =>
    safeInvoke('create_support_ticket', {
      token: getTokenOrThrow(),
      subject,
      message,
      priority,
      attachmentIds,
      attachment_ids: attachmentIds,
    }),

  get: (id: string): Promise<SupportTicketDetail> =>
    safeInvoke('get_support_ticket', { token: getTokenOrThrow(), id }),

  reply: (
    id: string,
    message: string,
    isInternal?: boolean,
    attachmentIds?: string[],
  ): Promise<SupportTicketMessage> =>
    safeInvoke('reply_support_ticket', {
      token: getTokenOrThrow(),
      id,
      message,
      isInternal,
      is_internal: isInternal,
      attachmentIds,
      attachment_ids: attachmentIds,
    }),

  update: (
    id: string,
    data: { status?: string; priority?: string; assignedTo?: string | null },
  ): Promise<SupportTicket> =>
    safeInvoke('update_support_ticket', {
      token: getTokenOrThrow(),
      id,
      status: data.status,
      priority: data.priority,
      assignedTo: data.assignedTo ?? undefined,
      assigned_to: data.assignedTo ?? undefined,
    }),
};
