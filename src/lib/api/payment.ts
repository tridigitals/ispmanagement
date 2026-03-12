import { getTokenOrThrow, safeInvoke } from './core';
import type {
  BankAccount,
  BillingCollectionLogView,
  BillingCollectionRunResult,
  BulkGenerateInvoicesResult,
  FxRate,
  Invoice,
  InvoiceReminderLogView,
} from './types';

export const payment = {
  listBanks: (): Promise<BankAccount[]> =>
    safeInvoke('list_bank_accounts', { token: getTokenOrThrow() }),

  createBank: (
    bank_name: string,
    account_number: string,
    account_holder: string,
  ): Promise<BankAccount> =>
    safeInvoke('create_bank_account', {
      token: getTokenOrThrow(),
      bankName: bank_name,
      accountNumber: account_number,
      accountHolder: account_holder,
    }),

  deleteBank: (id: string): Promise<void> =>
    safeInvoke('delete_bank_account', { token: getTokenOrThrow(), id }),

  createInvoiceForPlan: (planId: string, billingCycle: 'monthly' | 'yearly'): Promise<Invoice> =>
    safeInvoke('create_invoice_for_plan', { token: getTokenOrThrow(), planId, billingCycle }),

  createInvoiceForCustomerSubscription: (subscriptionId: string): Promise<Invoice> =>
    safeInvoke('create_invoice_for_customer_subscription', {
      token: getTokenOrThrow(),
      subscriptionId,
      subscription_id: subscriptionId,
    }),

  createInvoiceForInstallationWorkOrder: (workOrderId: string): Promise<Invoice> =>
    safeInvoke('create_invoice_for_installation_work_order', {
      token: getTokenOrThrow(),
      workOrderId,
      work_order_id: workOrderId,
    }),

  getInvoice: (id: string): Promise<Invoice> =>
    safeInvoke('get_invoice', { token: getTokenOrThrow(), id }),

  listInvoices: (): Promise<Invoice[]> => safeInvoke('list_invoices', { token: getTokenOrThrow() }),

  listCustomerPackageInvoices: (params?: {
    sort_by?: 'invoice_number' | 'description' | 'amount' | 'status' | 'due_date' | 'created_at';
    sort_dir?: 'asc' | 'desc';
  }): Promise<Invoice[]> =>
    safeInvoke('list_customer_package_invoices', {
      token: getTokenOrThrow(),
      sort_by: params?.sort_by,
      sort_dir: params?.sort_dir,
    }),

  generateDueCustomerPackageInvoices: (): Promise<BulkGenerateInvoicesResult> =>
    safeInvoke('generate_due_customer_package_invoices', { token: getTokenOrThrow() }),

  listBillingCollectionLogs: (filters?: {
    action?: string;
    result?: string;
    from?: string;
    to?: string;
    search?: string;
    limit?: number;
  }): Promise<BillingCollectionLogView[]> =>
    safeInvoke('list_billing_collection_logs', {
      token: getTokenOrThrow(),
      action: filters?.action ?? null,
      result: filters?.result ?? null,
      from: filters?.from ?? null,
      to: filters?.to ?? null,
      search: filters?.search ?? null,
      limit: filters?.limit ?? 200,
    }),

  listInvoiceReminderLogs: (filters?: {
    reminderCode?: string;
    status?: string;
    from?: string;
    to?: string;
    search?: string;
    limit?: number;
  }): Promise<InvoiceReminderLogView[]> =>
    safeInvoke('list_invoice_reminder_logs', {
      token: getTokenOrThrow(),
      reminderCode: filters?.reminderCode ?? null,
      reminder_code: filters?.reminderCode ?? null,
      status: filters?.status ?? null,
      from: filters?.from ?? null,
      to: filters?.to ?? null,
      search: filters?.search ?? null,
      limit: filters?.limit ?? 200,
    }),

  runBillingCollectionNow: (): Promise<BillingCollectionRunResult> =>
    safeInvoke('run_billing_collection_now', { token: getTokenOrThrow() }),

  listAllInvoices: (): Promise<Invoice[]> =>
    safeInvoke('list_all_invoices', { token: getTokenOrThrow() }),

  getFxRate: (baseCurrency: string, quoteCurrency: string): Promise<FxRate> =>
    safeInvoke('get_fx_rate', { token: getTokenOrThrow(), baseCurrency, quoteCurrency }),

  payMidtrans: (id: string): Promise<string> =>
    safeInvoke('pay_invoice_midtrans', { token: getTokenOrThrow(), id }),

  checkStatus: (id: string): Promise<string> =>
    safeInvoke('check_payment_status', { token: getTokenOrThrow(), id }),

  submitPaymentProof: (invoiceId: string, filePath: string): Promise<void> =>
    safeInvoke('submit_payment_proof', { token: getTokenOrThrow(), invoiceId, filePath }),

  verifyPayment: (
    invoiceId: string,
    status: 'paid' | 'failed',
    rejectionReason?: string,
  ): Promise<void> =>
    safeInvoke('verify_payment', { token: getTokenOrThrow(), invoiceId, status, rejectionReason }),

  verifyCustomerPackagePayment: (
    invoiceId: string,
    status: 'paid' | 'failed',
    rejectionReason?: string,
  ): Promise<void> =>
    safeInvoke('verify_customer_package_payment', {
      token: getTokenOrThrow(),
      id: invoiceId,
      invoiceId,
      invoice_id: invoiceId,
      status,
      rejectionReason,
      rejection_reason: rejectionReason,
    }),
};
