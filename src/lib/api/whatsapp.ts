import { getTokenOrThrow, safeInvoke } from './core';
import type {
  WhatsAppCustomerSendRequest,
  WhatsAppEventDefinition,
  WhatsAppGatewayReadiness,
  WhatsAppTestSendRequest,
  WhatsAppTestSendResponse,
} from './types';

export const whatsapp = {
  readiness: (): Promise<WhatsAppGatewayReadiness> =>
    safeInvoke('get_whatsapp_gateway_readiness', { token: getTokenOrThrow() }),

  sendCustomer: ({
    customerId,
    message,
    template,
    templateId,
  }: WhatsAppCustomerSendRequest): Promise<WhatsAppTestSendResponse> =>
    safeInvoke('send_customer_whatsapp', {
      token: getTokenOrThrow(),
      customer_id: customerId,
      customerId,
      message,
      template,
      template_id: templateId,
      templateId,
    }),

  sendTest: ({
    phone,
    message,
    eventCode,
  }: WhatsAppTestSendRequest): Promise<WhatsAppTestSendResponse> =>
    safeInvoke('send_test_whatsapp', { token: getTokenOrThrow(), phone, message, eventCode }),

  listEvents: (): Promise<WhatsAppEventDefinition[]> =>
    safeInvoke('list_whatsapp_events', { token: getTokenOrThrow() }),
};
