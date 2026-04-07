type InstallationInvoiceContext = {
  workOrderStatus?: string | null;
  subscriptionStatus?: string | null;
  hasCustomerPackageInvoice?: boolean | null;
};

export function shouldAllowInstallationInvoiceCreation(
  context: InstallationInvoiceContext,
): boolean {
  const workOrderStatus = `${context.workOrderStatus || ''}`.trim().toLowerCase();
  if (workOrderStatus !== 'completed') return false;
  if (context.hasCustomerPackageInvoice) return false;

  const subscriptionStatus = `${context.subscriptionStatus || ''}`.trim().toLowerCase();
  return (
    subscriptionStatus === 'pending_installation' ||
    subscriptionStatus === 'suspended' ||
    subscriptionStatus === 'grace_active'
  );
}
