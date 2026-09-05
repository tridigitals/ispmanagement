/**
 * Helper murni detail tiket dukungan v2 (gelombang 23).
 *
 * Label/tone status & prioritas, plus predikat "pesan ini dari
 * pelanggan/staf" dulu hidup sebagai `$t()` inline + ternari di
 * komponen 1.068 baris. Dipindah ke sini agar teruji.
 */
import type { StatusTone } from '$lib/components/ds/tokens';

export type TicketStatus = 'open' | 'pending' | 'closed' | 'resolved';
export type TicketPriority = 'low' | 'normal' | 'high' | 'urgent';
export type TicketCategory = 'general' | 'billing' | 'technical' | 'installation';

export const TICKET_STATUS_LABEL: Record<TicketStatus, string> = {
  open: 'Terbuka',
  pending: 'Menunggu',
  closed: 'Ditutup',
  resolved: 'Selesai',
};

export const TICKET_PRIORITY_LABEL: Record<TicketPriority, string> = {
  low: 'Rendah',
  normal: 'Normal',
  high: 'Tinggi',
  urgent: 'Urgent',
};

export const TICKET_CATEGORY_LABEL: Record<TicketCategory, string> = {
  general: 'Umum',
  billing: 'Tagihan',
  technical: 'Teknis',
  installation: 'Instalasi',
};

export function ticketStatusLabel(status: string | null | undefined): string {
  if (!status) return '—';
  return TICKET_STATUS_LABEL[status as TicketStatus] || status;
}

export function ticketStatusTone(status: string | null | undefined): StatusTone {
  switch (status) {
    case 'open':
      return 'info';
    case 'pending':
      return 'warning';
    case 'closed':
      return 'neutral';
    case 'resolved':
      return 'positive';
    default:
      return 'neutral';
  }
}

export function ticketPriorityLabel(priority: string | null | undefined): string {
  if (!priority) return '—';
  return TICKET_PRIORITY_LABEL[priority as TicketPriority] || priority;
}

export function ticketPriorityTone(priority: string | null | undefined): StatusTone {
  switch (priority) {
    case 'urgent':
      return 'negative';
    case 'high':
      return 'warning';
    case 'normal':
      return 'info';
    case 'low':
      return 'neutral';
    default:
      return 'neutral';
  }
}

export function ticketCategoryLabel(category: string | null | undefined): string {
  if (!category) return 'Umum';
  return TICKET_CATEGORY_LABEL[category as TicketCategory] || category;
}

/** Siapa pengirim pesan: customer (pembuat tiket) vs staf/teknisi. */
export function isCustomerMessage(
  createdBy: string | null | undefined,
  authorId: string | null | undefined,
): boolean {
  return Boolean(createdBy && authorId && authorId === createdBy);
}

export function messageAuthorName(opts: {
  authorName: string | null | undefined;
  isCustomer: boolean;
}): string {
  if (opts.authorName) return opts.authorName;
  return opts.isCustomer ? 'Pelanggan' : 'Staf';
}

export function ticketIcon(opts: { isInternal: boolean; isCustomer: boolean }): string {
  if (opts.isInternal) return 'eye-off';
  return opts.isCustomer ? 'user' : 'headphones';
}
