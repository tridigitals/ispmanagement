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
    category?: string;
    page?: number;
    perPage?: number;
  }): Promise<PaginatedResponse<SupportTicketListItem>> =>
    safeInvoke('list_support_tickets', {
      token: getTokenOrThrow(),
      status: params?.status,
      search: params?.search,
      category: params?.category,
      page: params?.page,
      per_page: params?.perPage,
    }),

  stats: (): Promise<SupportTicketStats> =>
    safeInvoke('get_support_ticket_stats', { token: getTokenOrThrow() }),

  create: (
    subject: string,
    message: string,
    priority?: string,
    category?: string,
    subscriptionId?: string,
    attachmentIds?: string[],
  ): Promise<SupportTicketDetail> =>
    safeInvoke('create_support_ticket', {
      token: getTokenOrThrow(),
      subject,
      message,
      priority,
      category,
      subscriptionId,
      subscription_id: subscriptionId,
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
    data: { status?: string; priority?: string; category?: string; assignedTo?: string | null },
  ): Promise<SupportTicket> =>
    safeInvoke('update_support_ticket', {
      token: getTokenOrThrow(),
      id,
      status: data.status,
      priority: data.priority,
      category: data.category,
      assignedTo: data.assignedTo ?? undefined,
      assigned_to: data.assignedTo ?? undefined,
    }),

  submitSatisfaction: (
    id: string,
    rating: number,
    comment?: string,
  ): Promise<void> =>
    safeInvoke('submit_ticket_satisfaction', {
      token: getTokenOrThrow(),
      ticketId: id,
      ticket_id: id,
      rating,
      comment,
    }),
};
