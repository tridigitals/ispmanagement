import { describe, expect, it } from 'vitest';
import {
  isCustomerMessage,
  messageAuthorName,
  ticketCategoryLabel,
  ticketIcon,
  ticketPriorityLabel,
  ticketPriorityTone,
  ticketStatusLabel,
  ticketStatusTone,
} from './supportTicketInsights';

describe('status tiket', () => {
  it('memetakan status ke label Indonesia', () => {
    expect(ticketStatusLabel('open')).toBe('Terbuka');
    expect(ticketStatusLabel('pending')).toBe('Menunggu');
    expect(ticketStatusLabel('closed')).toBe('Ditutup');
    expect(ticketStatusLabel('resolved')).toBe('Selesai');
  });
  it('fallback ke nilai asli / dash untuk kosong', () => {
    expect(ticketStatusLabel('status-baru')).toBe('status-baru');
    expect(ticketStatusLabel(null)).toBe('—');
    expect(ticketStatusLabel('')).toBe('—');
  });
  it('tone konsisten', () => {
    expect(ticketStatusTone('open')).toBe('info');
    expect(ticketStatusTone('pending')).toBe('warning');
    expect(ticketStatusTone('closed')).toBe('neutral');
    expect(ticketStatusTone('resolved')).toBe('positive');
    expect(ticketStatusTone('x')).toBe('neutral');
  });
});

describe('prioritas tiket', () => {
  it('memetakan prioritas ke label', () => {
    expect(ticketPriorityLabel('low')).toBe('Rendah');
    expect(ticketPriorityLabel('normal')).toBe('Normal');
    expect(ticketPriorityLabel('high')).toBe('Tinggi');
    expect(ticketPriorityLabel('urgent')).toBe('Urgent');
  });
  it('tone: urgent negative, high warning, normal info, low neutral', () => {
    expect(ticketPriorityTone('urgent')).toBe('negative');
    expect(ticketPriorityTone('high')).toBe('warning');
    expect(ticketPriorityTone('normal')).toBe('info');
    expect(ticketPriorityTone('low')).toBe('neutral');
    expect(ticketPriorityTone('x')).toBe('neutral');
  });
});

describe('kategori', () => {
  it('memetakan kategori', () => {
    expect(ticketCategoryLabel('billing')).toBe('Tagihan');
    expect(ticketCategoryLabel('technical')).toBe('Teknis');
    expect(ticketCategoryLabel('installation')).toBe('Instalasi');
    expect(ticketCategoryLabel('general')).toBe('Umum');
  });
  it('fallback Umum untuk null dan nilai asing', () => {
    expect(ticketCategoryLabel(null)).toBe('Umum');
    expect(ticketCategoryLabel('lain')).toBe('lain');
  });
});

describe('pesan: customer vs staf', () => {
  it('pesan customer bila author == pembuat tiket', () => {
    expect(isCustomerMessage('u1', 'u1')).toBe(true);
    expect(isCustomerMessage('u1', 'u2')).toBe(false);
    expect(isCustomerMessage(null, 'u1')).toBe(false);
    expect(isCustomerMessage('u1', null)).toBe(false);
  });
  it('nama penulis dengan fallback peran', () => {
    expect(messageAuthorName({ authorName: 'Budi', isCustomer: true })).toBe('Budi');
    expect(messageAuthorName({ authorName: null, isCustomer: true })).toBe('Pelanggan');
    expect(messageAuthorName({ authorName: '', isCustomer: false })).toBe('Staf');
  });
  it('ikon: internal eye-off, customer user, staf headphones', () => {
    expect(ticketIcon({ isInternal: true, isCustomer: false })).toBe('eye-off');
    expect(ticketIcon({ isInternal: false, isCustomer: true })).toBe('user');
    expect(ticketIcon({ isInternal: false, isCustomer: false })).toBe('headphones');
  });
});
