import { getTokenOrThrow, safeInvoke } from './core';
import type {
  WhatsAppEventDefinition,
  WhatsAppTestSendRequest,
  WhatsAppTestSendResponse,
} from './types';

export const whatsapp = {
  sendTest: ({
    phone,
    message,
    eventCode,
  }: WhatsAppTestSendRequest): Promise<WhatsAppTestSendResponse> =>
    safeInvoke('send_test_whatsapp', { token: getTokenOrThrow(), phone, message, eventCode }),

  listEvents: (): Promise<WhatsAppEventDefinition[]> =>
    safeInvoke('list_whatsapp_events', { token: getTokenOrThrow() }),
};
