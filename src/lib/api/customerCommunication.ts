import { getTokenOrThrow, safeInvoke } from './core';
import type { CustomerEmailSendRequest, CustomerEmailSendResponse } from './types';

export const customerCommunication = {
  sendEmail: (payload: CustomerEmailSendRequest): Promise<CustomerEmailSendResponse> =>
    safeInvoke('send_customer_email', {
      token: getTokenOrThrow(),
      payload,
    }),
};
