<script lang="ts">
  import { onMount } from 'svelte';
  import { page as pageStore } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';
  import { can } from '$lib/stores/auth';
  import { secureGetItem } from '$lib/utils/tauri-store';
  import {
    api,
    type Customer,
    type CustomerListItem,
    type CustomerRegistrationInvitePolicy,
    type CustomerRegistrationInviteSummary,
    type CustomerRegistrationInviteView,
    type CustomerSummary,
    type MessageTemplate,
    type PaginatedResponse,
  } from '$lib/api/client';

  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import TableToolbar from '$lib/components/ui/TableToolbar.svelte';
  import StatsCard from '$lib/components/dashboard/StatsCard.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';

  const IMPORT_PLACEHOLDER_CUSTOMER_NAME = 'Imported (Unassigned)';

  const columns = $derived.by(() => [
    { key: 'name', label: $t('admin.customers.columns.customer') || 'Customer' },
    { key: 'contact', label: $t('admin.customers.columns.contact') || 'Contact' },
    { key: 'status', label: $t('admin.customers.columns.status') || 'Status' },
    { key: 'health', label: $t('admin.customers.columns.health') || 'Health' },
    { key: 'service', label: $t('admin.customers.columns.service') || 'Service' },
    { key: 'updated_at', label: $t('admin.customers.columns.updated') || 'Updated' },
    { key: 'actions', label: '', align: 'right' as const },
  ]);

  type CustomerStatusFilter = 'all' | 'active' | 'inactive';
  type CustomerServiceFilter = 'all' | 'active' | 'inactive' | 'none';
  type CustomerInstallationFilter = 'all' | 'pending';

  let customers = $state<CustomerListItem[]>([]);
  let total = $state(0);
  let customerSummary = $state<CustomerSummary>({
    total: 0,
    active: 0,
    inactive: 0,
    pending_installation: 0,
  });
  let lifecycleIssueCount = $state(0);
  let loading = $state(true);
  let error = $state('');

  let q = $state('');
  let statusFilter = $state<CustomerStatusFilter>('all');
  let serviceFilter = $state<CustomerServiceFilter>('all');
  let installationFilter = $state<CustomerInstallationFilter>('all');
  let page = $state(0); // Table is 0-based
  let perPage = $state(10);

  let showCreate = $state(false);
  let creating = $state(false);
  let createName = $state('');
  let createEmail = $state('');
  let createPhone = $state('');
  let createNotes = $state('');
  let createPortalPassword = $state('');
  let createPortalPasswordConfirm = $state('');

  let showDelete = $state(false);
  let deleting = $state(false);
  let deleteTarget = $state<Customer | null>(null);

  let showInviteModal = $state(false);
  let inviteGenerating = $state(false);
  let inviteLoading = $state(false);
  let inviteRevokingId = $state<string | null>(null);
  let inviteExpiresInHours = $state(24);
  let inviteMaxUses = $state(1);
  let inviteNote = $state('');
  let inviteIncludeInactive = $state(true);
  let inviteRows = $state<CustomerRegistrationInviteView[]>([]);
  let generatedInviteUrl = $state('');
  let generatedInviteExpiresAt = $state('');
  let invitePolicyLoading = $state(false);
  let invitePolicySaving = $state(false);
  let inviteSummaryLoading = $state(false);
  let invitePolicyExpiresInHours = $state(24);
  let invitePolicyMaxUses = $state(1);
  let inviteSummary = $state<CustomerRegistrationInviteSummary | null>(null);
  let whatsappGatewayReady = $state(false);
  let whatsappGatewayReason = $state('WhatsApp gateway is not configured');
  let whatsappGatewayProvider = $state('');
  let whatsappSending = $state(false);
  let showWhatsAppCompose = $state(false);
  let whatsappTarget = $state<CustomerListItem | null>(null);
  let whatsappTemplate = $state('greeting');
  let whatsappMessage = $state('');
  let messageTemplateOptions = $state<MessageTemplate[]>([]);
  let emailTemplateOptions = $state<MessageTemplate[]>([]);
  let selectedMessageTemplateId = $state('custom');
  let emailSending = $state(false);
  let showEmailCompose = $state(false);
  let emailTarget = $state<CustomerListItem | null>(null);
  let selectedEmailTemplateId = $state('custom');
  let emailSubject = $state('');
  let emailBody = $state('');
  const canReadCustomers = $derived($can('read', 'customers') || $can('manage', 'customers'));
  const canManageCustomers = $derived($can('manage', 'customers'));
  const canCreateOrders = $derived($can('create', 'orders'));

  let stats = $derived({
    total: customerSummary.total,
    active: customerSummary.active,
    inactive: customerSummary.inactive,
    pendingInstallation: customerSummary.pending_installation,
  });
  let totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  onMount(async () => {
    if (!canReadCustomers) {
      goto('/unauthorized');
      return;
    }
    hydrateUrlState();
    await Promise.all([
      canManageCustomers ? loadInvites() : Promise.resolve(),
      canManageCustomers ? loadWhatsAppReadiness() : Promise.resolve(),
      canManageCustomers ? loadMessageTemplates() : Promise.resolve(),
      load(),
      loadCustomerSummary(),
      loadLifecycleReconciliationSummary(),
    ]);
  });

  function isCustomerStatusFilter(value: string | null): value is CustomerStatusFilter {
    return value === 'all' || value === 'active' || value === 'inactive';
  }

  function isCustomerServiceFilter(value: string | null): value is CustomerServiceFilter {
    return value === 'all' || value === 'active' || value === 'inactive' || value === 'none';
  }

  function isCustomerInstallationFilter(value: string | null): value is CustomerInstallationFilter {
    return value === 'all' || value === 'pending';
  }

  function hydrateUrlState() {
    if (typeof window === 'undefined') return;

    const params = new URLSearchParams(window.location.search);
    const nextStatus = params.get('status');
    const nextService = params.get('service');
    const nextInstallation = params.get('installation');
    q = params.get('q') || '';
    statusFilter = isCustomerStatusFilter(nextStatus) ? nextStatus : 'all';
    serviceFilter = isCustomerServiceFilter(nextService) ? nextService : 'all';
    installationFilter = isCustomerInstallationFilter(nextInstallation) ? nextInstallation : 'all';
    page = Math.max(0, Number(params.get('page') || '1') - 1);
    perPage = Math.max(1, Number(params.get('per_page') || perPage));
  }

  function syncUrlState() {
    if (typeof window === 'undefined') return;

    const url = new URL(window.location.href);
    if (q.trim()) url.searchParams.set('q', q.trim());
    else url.searchParams.delete('q');

    if (statusFilter !== 'all') url.searchParams.set('status', statusFilter);
    else url.searchParams.delete('status');

    if (serviceFilter !== 'all') url.searchParams.set('service', serviceFilter);
    else url.searchParams.delete('service');

    if (installationFilter !== 'all') url.searchParams.set('installation', installationFilter);
    else url.searchParams.delete('installation');

    if (page > 0) url.searchParams.set('page', String(page + 1));
    else url.searchParams.delete('page');

    if (perPage !== 10) url.searchParams.set('per_page', String(perPage));
    else url.searchParams.delete('per_page');

    window.history.replaceState(window.history.state, '', url);
  }

  async function refreshCustomers(options: { sync?: boolean } = {}) {
    if (options.sync !== false) syncUrlState();
    await Promise.all([load(), loadCustomerSummary(), loadLifecycleReconciliationSummary()]);
  }

  async function setStatusFilter(next: CustomerStatusFilter) {
    if (statusFilter === next) return;
    statusFilter = next;
    page = 0;
    await refreshCustomers();
  }

  async function setServiceFilter(next: CustomerServiceFilter) {
    if (serviceFilter === next) return;
    serviceFilter = next;
    page = 0;
    await refreshCustomers();
  }

  async function setInstallationFilter(next: CustomerInstallationFilter) {
    if (installationFilter === next) return;
    installationFilter = next;
    page = 0;
    await refreshCustomers();
  }

  async function load() {
    loading = true;
    error = '';
    try {
      const res: PaginatedResponse<CustomerListItem> = await api.customers.list({
        q,
        status: statusFilter,
        service: serviceFilter,
        installation: installationFilter,
        page: page + 1,
        perPage,
      });
      customers = res.data;
      total = res.total;
    } catch (e: any) {
      error = String(e?.message || e || 'Failed to load customers');
      toast.error(get(t)('admin.customers.toasts.load_failed') || 'Failed to load customers');
    } finally {
      loading = false;
    }
  }

  async function loadCustomerSummary() {
    try {
      customerSummary = await api.customers.summary();
    } catch (e: any) {
      toast.error(e?.message || 'Failed to load customer summary');
    }
  }

  async function loadLifecycleReconciliationSummary() {
    try {
      const report = await api.customers.reconciliation.report({
        page: 1,
        perPage: 1,
      });
      lifecycleIssueCount = report.total_issues;
    } catch {
      lifecycleIssueCount = 0;
    }
  }

  function isSystemImportPlaceholder(customer: Customer): boolean {
    return customer.name === IMPORT_PLACEHOLDER_CUSTOMER_NAME;
  }

  function openCustomer(c: Customer) {
    const base = $pageStore.url.pathname.replace(/\/$/, '');
    goto(`${base}/${c.id}`);
  }

  function serviceStatusLabel(c: CustomerListItem) {
    if (c.pending_installations > 0) return `${c.pending_installations} ${$t('admin.customers.status.pending_install')}`;
    if (c.service_status === 'active') return `${c.active_subscriptions} ${$t('common.active')}`;
    if (c.service_status === 'inactive') return `${c.subscription_count} ${$t('common.inactive')}`;
    return $t('admin.customers.status.no_service') || 'No service';
  }

  function customerHealthLabel(c: CustomerListItem) {
    if (!c.is_active) return $t('admin.customers.status.inactive') || 'Inactive';
    if (c.pending_installations > 0) return $t('admin.customers.status.pending_installation') || 'Pending installation';
    if (c.service_status === 'none') return $t('admin.customers.status.no_service') || 'No service';
    if (c.service_status === 'inactive') return $t('admin.customers.status.service_inactive') || 'Service inactive';
    return $t('admin.customers.status.healthy') || 'Healthy';
  }

  function customerHealthTone(c: CustomerListItem) {
    if (!c.is_active) return 'muted';
    if (
      c.pending_installations > 0 ||
      c.service_status === 'none' ||
      c.service_status === 'inactive'
    ) {
      return 'warning';
    }
    return 'healthy';
  }

  async function goToMobilePage(nextPage: number) {
    page = Math.min(totalPages - 1, Math.max(0, nextPage));
    await refreshCustomers();
  }

  function adminBasePath() {
    return $pageStore.url.pathname.replace(/\/admin\/customers\/?$/, '/admin');
  }

  function openAddService(c: CustomerListItem) {
    const base = $pageStore.url.pathname.replace(/\/$/, '');
    goto(`${base}/${c.id}#subscriptions`);
  }

  function openCreateInvoice(c: CustomerListItem) {
    goto(`${adminBasePath()}/invoices?customer_id=${encodeURIComponent(c.id)}`);
  }

  function openLifecycleReconciliation() {
    goto(`${adminBasePath()}/customers/lifecycle-reconciliation`);
  }

  async function loadWhatsAppReadiness() {
    try {
      const readiness = await api.whatsapp.readiness();
      whatsappGatewayReady = readiness.ready;
      whatsappGatewayReason = readiness.reason || '';
      whatsappGatewayProvider = readiness.provider || '';
    } catch (e: any) {
      whatsappGatewayReady = false;
      whatsappGatewayReason = e?.message || 'Failed to check WhatsApp gateway';
      whatsappGatewayProvider = '';
    }
  }

  async function loadMessageTemplates() {
    try {
      messageTemplateOptions = await api.messageTemplates.list({
        channel: 'whatsapp',
        status: 'active',
        target: 'customer',
        triggerMode: 'manual',
      });
      emailTemplateOptions = await api.messageTemplates.list({
        channel: 'email',
        status: 'active',
        target: 'customer',
        triggerMode: 'manual',
      });
    } catch (e: any) {
      messageTemplateOptions = [];
      emailTemplateOptions = [];
      toast.error(e?.message || $t('admin.customers.communication.load_templates_failed'));
    }
  }

  function openWhatsAppApp(c: CustomerListItem) {
    if (!c.phone) {
      toast.error($t('admin.customers.communication.phone_not_set'));
      return;
    }
    const digits = c.phone.replace(/[^\d+]/g, '');
    window.open(`https://wa.me/${digits.replace(/^\+/, '')}`, '_blank', 'noopener,noreferrer');
  }

  function whatsappActionTitle(c: CustomerListItem) {
    if (!c.phone) return $t('admin.customers.communication.phone_not_set');
    if (!whatsappGatewayReady)
      return whatsappGatewayReason || $t('admin.customers.communication.gateway_not_ready');
    return $t('admin.customers.communication.actions.send_whatsapp');
  }

  function emailActionTitle(c: CustomerListItem) {
    if (!c.email) return $t('admin.customers.communication.email_not_set');
    return $t('admin.customers.communication.actions.send_email');
  }

  function buildWhatsAppTemplate(c: CustomerListItem, template: string) {
    const savedTemplate = messageTemplateOptions.find((item) => item.id === template);
    if (savedTemplate?.whatsapp_body) {
      return renderCustomerTemplate(savedTemplate.whatsapp_body, c);
    }
    if (template === 'payment_reminder') {
      return `Halo ${c.name}, kami ingin mengingatkan tagihan layanan internet Anda. Jika sudah melakukan pembayaran, mohon abaikan pesan ini. Terima kasih.`;
    }
    if (template === 'installation_followup') {
      return `Halo ${c.name}, kami ingin konfirmasi jadwal instalasi layanan internet Anda. Mohon balas pesan ini jika ada perubahan jadwal.`;
    }
    if (template === 'service_check') {
      return `Halo ${c.name}, kami ingin memastikan layanan internet Anda berjalan normal. Jika ada kendala, silakan balas pesan ini.`;
    }
    if (template === 'custom') {
      return whatsappMessage;
    }
    return `Halo ${c.name}, kami dari Tri Digitals ingin menghubungi Anda terkait layanan internet.`;
  }

  function buildEmailTemplate(c: CustomerListItem, templateId: string) {
    const savedTemplate = emailTemplateOptions.find((item) => item.id === templateId);
    if (savedTemplate) {
      return {
        subject: renderCustomerTemplate(savedTemplate.email_subject || '', c),
        body: renderCustomerTemplate(savedTemplate.email_body || '', c),
      };
    }
    if (templateId === 'custom') {
      return { subject: emailSubject, body: emailBody };
    }
    return {
      subject: ($t('admin.customers.communication.fallback_email_subject') || '').replace(
        '{name}',
        c.name,
      ),
      body: ($t('admin.customers.communication.fallback_email_body') || '').replace(
        '{name}',
        c.name,
      ),
    };
  }

  function currentTenantName() {
    const pageData = get(pageStore).data as { tenant?: { name?: string } } | undefined;
    if (pageData?.tenant?.name) return pageData.tenant.name;
    if (typeof localStorage === 'undefined') return '';
    try {
      return JSON.parse(secureGetItem('auth_tenant') || '{}')?.name || '';
    } catch {
      return '';
    }
  }

  function renderCustomerTemplate(body: string, c: CustomerListItem) {
    const values: Record<string, string> = {
      'tenant.name': currentTenantName(),
      'customer.id': c.id,
      'customer.name': c.name,
      'customer.email': c.email || '',
      'customer.phone': c.phone || '',
      'customer.status': c.is_active ? 'active' : 'inactive',
      'customer.notes': c.notes || '',
    };
    return body.replace(/\{\{\s*([\w.]+)\s*\}\}/g, (_match, key) => values[key] ?? '');
  }

  function applyWhatsAppTemplate(template = whatsappTemplate) {
    if (!whatsappTarget) return;
    whatsappTemplate = template;
    selectedMessageTemplateId = template;
    whatsappMessage = buildWhatsAppTemplate(whatsappTarget, template);
  }

  function openWhatsAppCompose(c: CustomerListItem) {
    if (!c.phone) {
      toast.error($t('admin.customers.communication.phone_not_set'));
      return;
    }
    if (!whatsappGatewayReady) {
      toast.error(whatsappGatewayReason || $t('admin.customers.communication.gateway_not_ready'));
      return;
    }

    whatsappTarget = c;
    showWhatsAppCompose = true;
    applyWhatsAppTemplate(messageTemplateOptions[0]?.id || 'custom');
  }

  function applyEmailTemplate(templateId = selectedEmailTemplateId) {
    if (!emailTarget) return;
    selectedEmailTemplateId = templateId;
    const next = buildEmailTemplate(emailTarget, templateId);
    emailSubject = next.subject;
    emailBody = next.body;
  }

  function openEmailCompose(c: CustomerListItem) {
    if (!c.email) {
      toast.error($t('admin.customers.communication.email_not_set'));
      return;
    }
    emailTarget = c;
    showEmailCompose = true;
    applyEmailTemplate(emailTemplateOptions[0]?.id || 'custom');
  }

  async function sendCustomerEmail() {
    if (!emailTarget || emailSending) return;
    const subject = emailSubject.trim();
    const body = emailBody.trim();
    if (!subject) {
      toast.error($t('admin.customers.communication.email_subject_required'));
      return;
    }
    if (!body) {
      toast.error($t('admin.customers.communication.email_body_required'));
      return;
    }

    emailSending = true;
    try {
      await api.customerCommunication.sendEmail({
        customerId: emailTarget.id,
        templateId: selectedEmailTemplateId === 'custom' ? null : selectedEmailTemplateId,
        subject,
        body,
      });
      toast.success($t('admin.customers.communication.email_queued'));
      showEmailCompose = false;
      emailTarget = null;
      emailSubject = '';
      emailBody = '';
    } catch (e: any) {
      toast.error(e?.message || $t('admin.customers.communication.email_send_failed'));
    } finally {
      emailSending = false;
    }
  }

  async function sendCustomerWhatsApp() {
    if (!whatsappTarget || whatsappSending) return;
    const message = whatsappMessage.trim();
    if (!message) {
      toast.error($t('admin.customers.communication.whatsapp_body_required'));
      return;
    }

    whatsappSending = true;
    try {
      const result = await api.whatsapp.sendCustomer({
        customerId: whatsappTarget.id,
        message,
        template: whatsappTemplate,
        templateId: selectedMessageTemplateId === 'custom' ? null : selectedMessageTemplateId,
      });
      if (!result.ok) {
        toast.error(result.error || $t('admin.customers.communication.whatsapp_failed'));
        return;
      }
      toast.success($t('admin.customers.communication.whatsapp_sent'));
      showWhatsAppCompose = false;
      whatsappTarget = null;
      whatsappMessage = '';
    } catch (e: any) {
      toast.error(e?.message || $t('admin.customers.communication.whatsapp_send_failed'));
    } finally {
      whatsappSending = false;
      await loadWhatsAppReadiness();
    }
  }

  async function createCustomer() {
    if (!createName.trim()) return;
    if (!createEmail.trim()) {
      toast.error(
        get(t)('admin.customers.new.portal.validation.email_required') ||
          'Email wajib diisi jika ingin membuat akun login.',
      );
      return;
    }
    if (!createPortalPassword || createPortalPassword.length < 6) {
      toast.error(
        get(t)('admin.customers.new.portal.validation.password_min') ||
          'Password minimal 6 karakter.',
      );
      return;
    }
    if (createPortalPassword !== createPortalPasswordConfirm) {
      toast.error(
        get(t)('admin.customers.new.portal.validation.password_mismatch') ||
          'Konfirmasi password tidak sama.',
      );
      return;
    }
    creating = true;
    try {
      await api.customers.createWithPortal({
        name: createName.trim(),
        email: createEmail.trim() || null,
        phone: createPhone.trim() || null,
        notes: createNotes.trim() || null,
        portal_email: createEmail.trim(),
        portal_name: createName.trim(),
        portal_password: createPortalPassword,
      });

      showCreate = false;
      createName = '';
      createEmail = '';
      createPhone = '';
      createNotes = '';
      createPortalPassword = '';
      createPortalPasswordConfirm = '';
      page = 0;
      await refreshCustomers();
      toast.success(get(t)('admin.customers.toasts.created') || 'Customer created');
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.toasts.create_failed', { values: { message: e?.message || e } }) ||
          `Failed to create customer: ${e?.message || e}`,
      );
    } finally {
      creating = false;
    }
  }

  function confirmDelete(c: Customer) {
    deleteTarget = c;
    showDelete = true;
  }

  async function doDelete() {
    const c = deleteTarget;
    if (!c) return;
    deleting = true;
    try {
      await api.customers.delete(c.id);
      showDelete = false;
      deleteTarget = null;
      await refreshCustomers();
      toast.success(get(t)('admin.customers.toasts.deleted') || 'Customer deleted');
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.toasts.delete_failed', { values: { message: e?.message || e } }) ||
          `Failed to delete: ${e?.message || e}`,
      );
    } finally {
      deleting = false;
    }
  }

  function isInviteExpired(invite: CustomerRegistrationInviteView) {
    const ts = new Date(invite.expires_at).getTime();
    return Number.isFinite(ts) && ts <= Date.now();
  }

  function isInviteUsedOut(invite: CustomerRegistrationInviteView) {
    return invite.used_count >= invite.max_uses;
  }

  function inviteStatus(invite: CustomerRegistrationInviteView) {
    if (invite.is_revoked) return 'revoked';
    if (isInviteUsedOut(invite)) return 'used';
    if (isInviteExpired(invite)) return 'expired';
    return 'active';
  }

  function inviteStatusLabel(invite: CustomerRegistrationInviteView) {
    const s = inviteStatus(invite);
    if (s === 'revoked') return $t('admin.customers.invite.revoked') || 'Revoked';
    if (s === 'used') return 'Used';
    if (s === 'expired') return 'Expired';
    return 'Active';
  }

  async function loadInvites() {
    if (!canManageCustomers) return;
    inviteLoading = true;
    try {
      inviteRows = await api.customers.invites.list({
        include_inactive: inviteIncludeInactive,
        limit: 50,
      });
    } catch (e: any) {
      toast.error(e?.message || $t('admin.customers.invite.toasts.load_failed') || 'Failed to load customer invite links');
    } finally {
      inviteLoading = false;
    }
  }

  async function loadInvitePolicy() {
    if (!canManageCustomers) return;
    invitePolicyLoading = true;
    try {
      const policy: CustomerRegistrationInvitePolicy = await api.customers.invites.getPolicy();
      invitePolicyExpiresInHours = policy.default_expires_in_hours || 24;
      invitePolicyMaxUses = policy.default_max_uses || 1;
      inviteExpiresInHours = invitePolicyExpiresInHours;
      inviteMaxUses = invitePolicyMaxUses;
    } catch (e: any) {
      const msg = String(e?.message || '');
      const isMissingEndpoint = msg.includes('404') || msg.toLowerCase().includes('not found');
      if (!isMissingEndpoint) {
        toast.error(msg || $t('admin.customers.invite.toasts.policy_failed') || 'Failed to load invite defaults');
      }
    } finally {
      invitePolicyLoading = false;
    }
  }

  async function saveInvitePolicy() {
    if (invitePolicySaving) return;
    invitePolicySaving = true;
    try {
      const nextExpires = Math.min(720, Math.max(1, Math.trunc(invitePolicyExpiresInHours || 24)));
      const nextMaxUses = Math.min(100, Math.max(1, Math.trunc(invitePolicyMaxUses || 1)));
      const policy = await api.customers.invites.updatePolicy({
        default_expires_in_hours: nextExpires,
        default_max_uses: nextMaxUses,
      });
      invitePolicyExpiresInHours = policy.default_expires_in_hours;
      invitePolicyMaxUses = policy.default_max_uses;
      inviteExpiresInHours = policy.default_expires_in_hours;
      inviteMaxUses = policy.default_max_uses;
      toast.success($t('admin.customers.invite.toasts.policy_updated') || 'Invite defaults updated');
    } catch (e: any) {
      toast.error(e?.message || $t('admin.customers.invite.toasts.policy_failed') || 'Failed to update invite defaults');
    } finally {
      invitePolicySaving = false;
    }
  }

  async function loadInviteSummary() {
    if (!canManageCustomers) return;
    inviteSummaryLoading = true;
    try {
      inviteSummary = await api.customers.invites.summary();
    } catch (e: any) {
      const msg = String(e?.message || '');
      const isMissingEndpoint = msg.includes('404') || msg.toLowerCase().includes('not found');
      if (!isMissingEndpoint) {
        toast.error(msg || $t('admin.customers.invite.toasts.summary_failed') || 'Failed to load invite summary');
      }
    } finally {
      inviteSummaryLoading = false;
    }
  }

  function openInviteModal() {
    showInviteModal = true;
    generatedInviteUrl = '';
    generatedInviteExpiresAt = '';
    inviteExpiresInHours = invitePolicyExpiresInHours || 24;
    inviteMaxUses = invitePolicyMaxUses || 1;
    inviteNote = '';
    void Promise.all([loadInvitePolicy(), loadInviteSummary(), loadInvites()]);
  }

  async function generateInvite() {
    if (inviteGenerating) return;
    inviteGenerating = true;
    try {
      const nextExpires = Math.min(720, Math.max(1, Math.trunc(inviteExpiresInHours || 24)));
      const nextMaxUses = Math.min(100, Math.max(1, Math.trunc(inviteMaxUses || 1)));
      const res = await api.customers.invites.create({
        expires_in_hours: nextExpires,
        max_uses: nextMaxUses,
        note: inviteNote.trim() || null,
      });
      inviteExpiresInHours = nextExpires;
      inviteMaxUses = nextMaxUses;
      generatedInviteUrl = res.invite_url;
      generatedInviteExpiresAt = res.invite.expires_at;
      inviteNote = '';
      toast.success($t('admin.customers.invite.toasts.generated') || 'Invite link generated');
      await Promise.all([loadInvites(), loadInviteSummary()]);
    } catch (e: any) {
      const msg = String(e?.message || '');
      if (msg.toLowerCase().includes('custom domain') && msg.toLowerCase().includes('required')) {
        toast.error(
          $t('admin.customers.invite.toasts.domain_required') || 'Custom domain belum diset — wajib sebelum generate invite link.',
          {
            action: { label: 'Settings', onClick: () => goto('/admin/settings') },
            duration: 10000,
          },
        );
      } else {
        toast.error(msg || $t('admin.customers.invite.toasts.generate_failed') || 'Failed to generate invite link');
      }
    } finally {
      inviteGenerating = false;
    }
  }

  async function revokeInvite(inviteId: string) {
    if (!inviteId || inviteRevokingId) return;
    inviteRevokingId = inviteId;
    try {
      await api.customers.invites.revoke(inviteId);
      toast.success($t('admin.customers.invite.toasts.revoked') || 'Invite link revoked');
      await Promise.all([loadInvites(), loadInviteSummary()]);
    } catch (e: any) {
      toast.error(e?.message || $t('admin.customers.invite.toasts.revoke_failed') || 'Failed to revoke invite');
    } finally {
      inviteRevokingId = null;
    }
  }

  async function copyInviteLink(link: string) {
    if (!link) return;
    try {
      await navigator.clipboard.writeText(link);
      toast.success(get(t)('common.copied') || 'Copied');
    } catch {
      toast.error(get(t)('common.copy_failed') || 'Copy failed');
    }
  }
</script>

<div class="page-content fade-in">
  <div class="page-header">
    <div>
      <h1>{$t('admin.customers.title')}</h1>
      <p class="subtitle">{$t('admin.customers.subtitle')}</p>
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" onclick={() => refreshCustomers()} disabled={loading}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh')}
      </button>
      {#if canCreateOrders}
        <button class="btn btn-secondary" onclick={() => goto('/admin/customers/orders/new')}>
          <Icon name="file-text" size={16} />
          {$t('admin.customers.actions.create_order')}
        </button>
      {/if}
      {#if canManageCustomers}
        <button class="btn btn-secondary" onclick={openInviteModal}>
          <Icon name="link" size={16} />
          {$t('admin.customers.actions.invite_link')}
        </button>
        <button class="btn btn-primary" onclick={() => (showCreate = true)}>
          <Icon name="plus" size={16} />
          {$t('admin.customers.actions.new')}
        </button>
      {/if}
    </div>
  </div>

  <div class="stats-grid customer-stats-grid">
    <button
      class="stat-filter"
      class:active={statusFilter === 'all' && installationFilter === 'all'}
      onclick={async () => {
        statusFilter = 'all';
        installationFilter = 'all';
        page = 0;
        await refreshCustomers();
      }}
    >
      <StatsCard
        title={$t('admin.customers.stats.total')}
        value={stats.total}
        icon="users"
        color="blue"
      />
    </button>
    <button
      class="stat-filter"
      class:active={statusFilter === 'active' && installationFilter === 'all'}
      onclick={() => setStatusFilter('active')}
    >
      <StatsCard
        title={$t('admin.customers.stats.active')}
        value={stats.active}
        icon="check-circle"
        color="green"
      />
    </button>
    <button
      class="stat-filter"
      class:active={statusFilter === 'inactive' && installationFilter === 'all'}
      onclick={() => setStatusFilter('inactive')}
    >
      <StatsCard
        title={$t('admin.customers.stats.inactive')}
        value={stats.inactive}
        icon="x-circle"
        color="orange"
      />
    </button>
    <button
      class="stat-filter"
      class:active={installationFilter === 'pending'}
      onclick={() => setInstallationFilter('pending')}
    >
      <StatsCard
        title={$t('admin.customers.stats.pending_installation')}
        value={stats.pendingInstallation}
        icon="wrench"
        color="orange"
      />
    </button>
  </div>

  <div class="card table-card">
    <TableToolbar
      bind:searchQuery={q}
      placeholder={$t('admin.customers.search')}
      onsearch={() => {
        page = 0;
        refreshCustomers();
      }}
    >
      {#snippet filters()}
        <div class="toolbar-filters">
          <label class="customer-filter-field">
            <span>{$t('admin.customers.filters.status')}</span>
            <select
              class="customer-filter-select"
              aria-label={$t('admin.customers.filters.aria_status')}
              value={statusFilter}
              onchange={(event) =>
                setStatusFilter(
                  (event.currentTarget as HTMLSelectElement).value as CustomerStatusFilter,
                )}
            >
              <option value="all">{$t('admin.customers.filters.all_customers')}</option>
              <option value="active">{$t('common.active')}</option>
              <option value="inactive">{$t('common.inactive')}</option>
            </select>
          </label>
          <label class="customer-filter-field">
            <span>{$t('admin.customers.filters.service')}</span>
            <select
              class="customer-filter-select"
              aria-label={$t('admin.customers.filters.aria_service')}
              value={serviceFilter}
              onchange={(event) =>
                setServiceFilter(
                  (event.currentTarget as HTMLSelectElement).value as CustomerServiceFilter,
                )}
            >
              <option value="all">{$t('admin.customers.filters.all_services')}</option>
              <option value="active">{$t('admin.customers.filters.active_service')}</option>
              <option value="inactive">{$t('admin.customers.filters.inactive_service')}</option>
              <option value="none">{$t('admin.customers.filters.no_service')}</option>
            </select>
          </label>
          <label class="customer-filter-field">
            <span>{$t('admin.customers.filters.installation')}</span>
            <select
              class="customer-filter-select"
              aria-label={$t('admin.customers.filters.aria_installation')}
              value={installationFilter}
              onchange={(event) =>
                setInstallationFilter(
                  (event.currentTarget as HTMLSelectElement).value as CustomerInstallationFilter,
                )}
            >
              <option value="all">{$t('admin.customers.filters.all_installations')}</option>
              <option value="pending">{$t('admin.customers.filters.pending_installation')}</option>
            </select>
          </label>
        </div>
      {/snippet}
      {#snippet actions()}
        <div class="customer-toolbar-actions">
          <span class="customer-toolbar-results">
            {total}
            {$t('admin.customers.results')}
          </span>
          <button
            class="customer-toolbar-shortcut"
            class:attention={lifecycleIssueCount > 0}
            onclick={openLifecycleReconciliation}
          >
            <Icon name="shield-check" size={16} />
            <span>{$t('admin.customers.actions.lifecycle_reconciliation')}</span>
            {#if lifecycleIssueCount > 0}
              <span class="toolbar-alert-count pulse-red">{lifecycleIssueCount}</span>
            {/if}
          </button>
        </div>
      {/snippet}
    </TableToolbar>

    {#if error}
      <div class="error-banner">
        <Icon name="alert-triangle" size={18} />
        <span>{error}</span>
      </div>
    {/if}

    <div class="mobile-customer-list">
      {#if loading}
        <div class="mobile-empty">{$t('common.loading')}</div>
      {:else if customers.length === 0}
        <div class="mobile-empty">{$t('admin.customers.empty')}</div>
      {:else}
        {#each customers as c (c.id)}
          <article class="mobile-customer-card">
            <div class="mobile-customer-head">
              <button
                class="linkish"
                onclick={() => openCustomer(c)}
                disabled={isSystemImportPlaceholder(c)}
              >
                <div class="name">{c.name}</div>
                <div class="sub">{c.email || c.phone || $t('admin.customers.no_contact') || 'No contact'}</div>
              </button>
              <span
                class="pill"
                class:pill-green={customerHealthTone(c) === 'healthy'}
                class:pill-warning={customerHealthTone(c) === 'warning'}
                class:pill-gray={customerHealthTone(c) === 'muted'}
              >
                {customerHealthLabel(c)}
              </span>
            </div>
            <div class="mobile-customer-meta">
              <span>{c.email || '—'}</span>
              <span>{c.phone || '—'}</span>
              <span>{serviceStatusLabel(c)}</span>
            </div>
            <div class="mobile-customer-actions">
              {#if !isSystemImportPlaceholder(c)}
                <button class="btn btn-secondary" onclick={() => openCustomer(c)}>
                  <Icon name="arrow-right" size={15} />
                  {$t('common.open')}
                </button>
              {/if}
              {#if canManageCustomers && !isSystemImportPlaceholder(c)}
                <button class="btn-icon" title={$t('admin.customers.actions.add_service')} onclick={() => openAddService(c)}>
                  <Icon name="wifi" size={16} />
                </button>
                <button
                  class="btn-icon"
                  title={$t('admin.customers.actions.create_invoice')}
                  onclick={() => openCreateInvoice(c)}
                >
                  <Icon name="receipt" size={16} />
                </button>
                <button
                  class="btn-icon"
                  title={whatsappActionTitle(c)}
                  disabled={!c.phone || !whatsappGatewayReady}
                  onclick={() => openWhatsAppCompose(c)}
                >
                  <Icon name="message-circle" size={16} />
                </button>
                <button
                  class="btn-icon"
                  title={emailActionTitle(c)}
                  disabled={!c.email}
                  onclick={() => openEmailCompose(c)}
                >
                  <Icon name="mail" size={16} />
                </button>
                <button
                  class="btn-icon danger"
                  title={$t('common.delete')}
                  onclick={() => confirmDelete(c)}
                >
                  <Icon name="trash-2" size={16} />
                </button>
              {/if}
            </div>
          </article>
        {/each}
        <div class="mobile-pager">
          <button
            class="btn btn-secondary"
            disabled={page === 0}
            onclick={() => goToMobilePage(page - 1)}
          >
            <Icon name="chevron-left" size={16} />
            Prev
          </button>
          <span class="mono">{page + 1} / {totalPages}</span>
          <button
            class="btn btn-secondary"
            disabled={page + 1 >= totalPages}
            onclick={() => goToMobilePage(page + 1)}
          >
            Next
            <Icon name="chevron-right" size={16} />
          </button>
        </div>
      {/if}
    </div>

    <div class="desktop-customer-table">
      <Table
        {columns}
        data={customers}
        keyField="id"
        {loading}
        emptyText={$t('admin.customers.empty')}
        pagination
        serverSide
        pageSize={perPage}
        count={total}
        onchange={(p) => {
          page = p;
          refreshCustomers();
        }}
        onpageSizeChange={(s) => {
          perPage = s;
          page = 0;
          refreshCustomers();
        }}
      >
        {#snippet cell({ item, key })}
          {@const c = item as CustomerListItem}
          {#if key === 'name'}
            {#if isSystemImportPlaceholder(c)}
              <div>
                <div class="name">{c.name}</div>
                <div class="sub">
                  {$t('admin.network.pppoe.import.fields.unassigned')}
                </div>
              </div>
            {:else}
              <button class="linkish" onclick={() => openCustomer(c)}>
                <div class="name">{c.name}</div>
                <div class="sub">{c.email || c.phone || ''}</div>
              </button>
            {/if}
          {:else if key === 'contact'}
            <div class="contact">
              <div>{c.email || '—'}</div>
              <div class="sub">{c.phone || '—'}</div>
            </div>
          {:else if key === 'status'}
            {#if c.is_active}
              <span class="pill pill-green">{$t('common.active')}</span>
            {:else}
              <span class="pill pill-gray">{$t('common.inactive')}</span>
            {/if}
          {:else if key === 'health'}
            <span
              class="pill"
              class:pill-green={customerHealthTone(c) === 'healthy'}
              class:pill-warning={customerHealthTone(c) === 'warning'}
              class:pill-gray={customerHealthTone(c) === 'muted'}
            >
              {customerHealthLabel(c)}
            </span>
          {:else if key === 'service'}
            <div>
              <span
                class="pill"
                class:pill-green={c.service_status === 'active'}
                class:pill-warning={c.pending_installations > 0}
                class:pill-gray={c.service_status === 'none'}
              >
                {serviceStatusLabel(c)}
              </span>
              {#if c.subscription_count > 0 && c.active_subscriptions !== c.subscription_count}
                <div class="sub">{c.subscription_count} {$t('admin.customers.status.total_services')}</div>
              {/if}
            </div>
          {:else if key === 'updated_at'}
            <span class="mono">{new Date(c.updated_at).toLocaleString()}</span>
          {:else if key === 'actions'}
            <div class="row-actions">
              {#if !isSystemImportPlaceholder(c)}
                <button
                  class="btn-icon"
                  title={$t('common.open')}
                  onclick={() => openCustomer(c)}
                >
                  <Icon name="arrow-right" size={16} />
                </button>
              {/if}
              {#if canManageCustomers && !isSystemImportPlaceholder(c)}
                <button class="btn-icon" title={$t('admin.customers.actions.add_service')} onclick={() => openAddService(c)}>
                  <Icon name="wifi" size={16} />
                </button>
                <button
                  class="btn-icon"
                  title={$t('admin.customers.actions.create_invoice')}
                  onclick={() => openCreateInvoice(c)}
                >
                  <Icon name="receipt" size={16} />
                </button>
                <button
                  class="btn-icon"
                  title={whatsappActionTitle(c)}
                  disabled={!c.phone || !whatsappGatewayReady}
                  onclick={() => openWhatsAppCompose(c)}
                >
                  <Icon name="message-circle" size={16} />
                </button>
                <button
                  class="btn-icon"
                  title={emailActionTitle(c)}
                  disabled={!c.email}
                  onclick={() => openEmailCompose(c)}
                >
                  <Icon name="mail" size={16} />
                </button>
                <button
                  class="btn-icon danger"
                  title={$t('common.delete')}
                  onclick={() => confirmDelete(c)}
                >
                  <Icon name="trash-2" size={16} />
                </button>
              {:else if isSystemImportPlaceholder(c)}
                <span class="mono">—</span>
              {/if}
            </div>
          {:else}
            {item[key] ?? ''}
          {/if}
        {/snippet}
      </Table>
    </div>
  </div>
</div>

<Modal
  show={showCreate}
  title={$t('admin.customers.new.title')}
  onclose={() => (showCreate = false)}
>
  <div class="form">
    <label>
      <span>{$t('admin.customers.fields.name')}</span>
      <input class="input" bind:value={createName} placeholder={$t('admin.settings.company.name_placeholder')} />
    </label>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.fields.email')}</span>
        <input class="input" bind:value={createEmail} placeholder="customer@example.com" />
      </label>
      <label>
        <span>{$t('admin.customers.fields.phone')}</span>
        <input class="input" bind:value={createPhone} placeholder="+62..." />
      </label>
    </div>
    <label>
      <span>{$t('admin.customers.fields.notes')}</span>
      <textarea class="input" rows="4" bind:value={createNotes}></textarea>
    </label>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.new.portal.password')}</span>
        <input class="input" type="text" bind:value={createPortalPassword} />
      </label>
      <label>
        <span>{$t('admin.customers.new.portal.password_confirm')}</span>
        <input class="input" type="text" bind:value={createPortalPasswordConfirm} />
      </label>
    </div>

    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showCreate = false)}>
        {$t('common.cancel')}
      </button>
      <button
        class="btn btn-primary"
        onclick={createCustomer}
        disabled={creating ||
          !createName.trim() ||
          !createEmail.trim() ||
          !createPortalPassword ||
          !createPortalPasswordConfirm ||
          createPortalPassword !== createPortalPasswordConfirm}
      >
        <Icon name="plus" size={16} />
        {$t('common.create')}
      </button>
    </div>
  </div>
</Modal>

<Modal
  show={showInviteModal}
  title={$t('admin.customers.invite.title')}
  width="760px"
  onclose={() => (showInviteModal = false)}
>
  <div class="form invite-modal-shell">
    <section class="invite-section invite-generate-card">
      <div class="invite-section-head invite-section-heading-block">
        <div>
          <strong>{$t('admin.customers.invite.generate')}</strong>
          <p class="invite-section-copy">
            {$t('admin.customers.invite.share_hint')}
          </p>
        </div>
      </div>

      <div class="invite-overview-grid">
        <section class="invite-section invite-section-subtle">
          <div class="invite-section-head">
            <strong>{$t('admin.customers.invite.default_policy')}</strong>
            {#if invitePolicyLoading}
              <span class="muted">{$t('common.loading')}</span>
            {/if}
          </div>
          <div class="grid2">
            <label>
              <span>{$t('admin.customers.invite.default_expiry')}</span>
              <input
                class="input"
                type="number"
                min="1"
                max="720"
                bind:value={invitePolicyExpiresInHours}
              />
            </label>
            <label>
              <span>{$t('admin.customers.invite.default_max_uses')}</span>
              <input
                class="input"
                type="number"
                min="1"
                max="100"
                bind:value={invitePolicyMaxUses}
              />
            </label>
          </div>
          <div class="actions actions-inline">
            <button
              class="btn btn-secondary"
              onclick={saveInvitePolicy}
              disabled={invitePolicySaving}
            >
              <Icon name="save" size={14} />
              {invitePolicySaving ? ($t('common.saving') || 'Saving...') : ($t('admin.customers.invite.save_defaults') || 'Save defaults')}
            </button>
          </div>
        </section>

        <section class="invite-section invite-section-subtle">
          <div class="invite-section-head">
            <strong>{$t('admin.customers.invite.invite_summary')}</strong>
          </div>
          {#if inviteSummaryLoading}
            <div class="muted">{$t('common.loading')}</div>
          {:else if inviteSummary}
            <div class="invite-summary-grid">
              <div class="invite-summary-item">
                <small>{$t('common.total')}</small>
                <strong>{inviteSummary.total}</strong>
              </div>
              <div class="invite-summary-item">
                <small>{$t('common.active')}</small>
                <strong>{inviteSummary.active}</strong>
              </div>
              <div class="invite-summary-item">
                <small>{$t('admin.customers.invite.used_up')}</small>
                <strong>{inviteSummary.used_up}</strong>
              </div>
              <div class="invite-summary-item">
                <small>{$t('admin.customers.invite.expired')}</small>
                <strong>{inviteSummary.expired}</strong>
              </div>
              <div class="invite-summary-item">
                <small>{$t('admin.customers.invite.revoked')}</small>
                <strong>{inviteSummary.revoked}</strong>
              </div>
              <div class="invite-summary-item">
                <small>{$t('admin.customers.invite.utilization')}</small>
                <strong>{inviteSummary.utilization_percent.toFixed(1)}%</strong>
              </div>
            </div>
          {/if}
        </section>
      </div>

      <div class="grid2">
        <label>
          <span>{$t('admin.customers.invite.expire_hours')}</span>
          <input class="input" type="number" min="1" max="720" bind:value={inviteExpiresInHours} />
        </label>
        <label>
          <span>{$t('admin.customers.invite.max_uses')}</span>
          <input class="input" type="number" min="1" max="100" bind:value={inviteMaxUses} />
        </label>
      </div>
      <label>
        <span>{$t('admin.customers.invite.note_optional')}</span>
        <input
          class="input"
          bind:value={inviteNote}
          placeholder={$t('admin.customers.invite.note_placeholder')}
        />
      </label>

      <div class="actions actions-inline">
        <button class="btn btn-primary" onclick={generateInvite} disabled={inviteGenerating}>
          <Icon name="plus" size={16} />
          {inviteGenerating ? ($t('admin.customers.invite.generating') || 'Generating...') : ($t('admin.customers.invite.generate_btn') || 'Generate Invite Link')}
        </button>
      </div>

      {#if generatedInviteUrl}
        <div class="invite-result-panel">
          <div class="invite-result-head">
            <div>
              <strong>{$t('admin.customers.invite.generated_link')}</strong>
              <div class="sub">{$t('admin.customers.invite.share_hint')}</div>
            </div>
            <small class="sub">
              Expires: {new Date(generatedInviteExpiresAt).toLocaleString()}
            </small>
          </div>
          <div class="invite-copy-row">
            <input class="input mono" readonly value={generatedInviteUrl} />
            <button class="btn btn-secondary" onclick={() => copyInviteLink(generatedInviteUrl)}>
              <Icon name="copy" size={16} />
              {$t('common.copy')}
            </button>
          </div>
        </div>
      {/if}
    </section>

    <section class="invite-section invite-history-section">
      <div class="invite-history-toolbar">
        <div>
          <strong>{$t('admin.customers.invite.recent_links')}</strong>
          <div class="sub">{$t('admin.customers.invite.recent_hint')}</div>
        </div>
        <label class="inline-check">
          <input
            type="checkbox"
            bind:checked={inviteIncludeInactive}
            onchange={() => loadInvites()}
          />
          <span>{$t('admin.customers.invite.show_inactive')}</span>
        </label>
      </div>

      {#if inviteLoading}
        <div class="muted">{$t('common.loading')}</div>
      {:else if inviteRows.length === 0}
        <div class="invite-empty-state">
          <Icon name="link" size={18} />
          <span>{$t('admin.customers.invite.no_links')}</span>
        </div>
      {:else}
        <div class="invite-list">
          {#each inviteRows as invite}
            <div class="invite-item">
              <div class="invite-item-main">
                <div class="invite-meta">
                  <span
                    class="pill"
                    class:pill-green={inviteStatus(invite) === 'active'}
                    class:pill-warning={inviteStatus(invite) === 'used' ||
                      inviteStatus(invite) === 'expired'}
                    class:pill-gray={inviteStatus(invite) === 'revoked'}
                  >
                    {inviteStatusLabel(invite)}
                  </span>
                  <span class="mono">
                    {$t('admin.customers.invite.uses')}: {invite.used_count}/{invite.max_uses}
                  </span>
                </div>
                <div class="sub">
                  {$t('admin.customers.invite.created_at')}: {new Date(invite.created_at).toLocaleString()} · {$t('admin.customers.invite.expires_at')}: {new Date(
                    invite.expires_at,
                  ).toLocaleString()}
                </div>
                {#if invite.note}
                  <div class="sub">{invite.note}</div>
                {/if}
                {#if invite.invite_url}
                  <a
                    class="invite-item-link mono"
                    href={invite.invite_url}
                    target="_blank"
                    rel="noreferrer"
                  >
                    {invite.invite_url}
                  </a>
                {:else}
                  <div class="sub">{$t('admin.customers.invite.link_only_available')}</div>
                {/if}
              </div>
              <div class="invite-item-actions">
                <button
                  class="btn btn-secondary"
                  onclick={() => invite.invite_url && copyInviteLink(invite.invite_url)}
                  disabled={!invite.invite_url}
                >
                  <Icon name="copy" size={14} />
                  {$t('common.copy')}
                </button>
                {#if inviteStatus(invite) === 'active'}
                  <button
                    class="btn btn-secondary"
                    onclick={() => revokeInvite(invite.id)}
                    disabled={inviteRevokingId === invite.id}
                  >
                    <Icon name="x" size={14} />
                    {inviteRevokingId === invite.id ? ($t('admin.customers.invite.revoking') || 'Revoking...') : ($t('admin.customers.invite.revoke') || 'Revoke')}
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  </div>
</Modal>

<Modal
  show={showWhatsAppCompose}
  title={$t('admin.customers.communication.title_whatsapp')}
  onclose={() => {
    showWhatsAppCompose = false;
    whatsappTarget = null;
  }}
>
  <div class="form">
    {#if whatsappTarget}
      <div class="compose-target">
        <div>
          <strong>{whatsappTarget.name}</strong>
          <span>{whatsappTarget.phone}</span>
        </div>
        <span class="pill" class:pill-green={whatsappGatewayReady}>
          {whatsappGatewayReady
            ? `${whatsappGatewayProvider || 'gateway'} ${$t('admin.customers.communication.gateway_ready')}`
            : whatsappGatewayReason ||
              $t('admin.customers.communication.gateway_not_ready') ||
              'Gateway not ready'}
        </span>
      </div>
      <label>
        <span>{$t('admin.customers.communication.template')}</span>
        <select
          class="input"
          bind:value={selectedMessageTemplateId}
          onchange={(event) => applyWhatsAppTemplate(event.currentTarget.value)}
        >
          {#each messageTemplateOptions as template}
            <option value={template.id}>{template.name}</option>
          {/each}
          <option value="custom"
            >{$t('admin.customers.communication.custom_message')}</option
          >
        </select>
      </label>
      <label>
        <span>{$t('admin.customers.communication.message')}</span>
        <textarea class="input" rows="7" bind:value={whatsappMessage}></textarea>
      </label>
      <div class="compose-footnote">
        <span
          >{whatsappMessage.trim().length}
          {$t('admin.customers.communication.characters')}</span
        >
        {#if !whatsappGatewayReady}
          <span>{whatsappGatewayReason}</span>
        {/if}
      </div>
      <div class="actions">
        <button
          class="btn btn-secondary"
          onclick={() => whatsappTarget && openWhatsAppApp(whatsappTarget)}
        >
          <Icon name="external-link" size={16} />
          {$t('admin.customers.communication.actions.open_whatsapp_app')}
        </button>
        <button
          class="btn btn-primary"
          onclick={sendCustomerWhatsApp}
          disabled={!whatsappGatewayReady || whatsappSending || !whatsappMessage.trim()}
        >
          <Icon name="send" size={16} />
          {whatsappSending
            ? $t('admin.customers.communication.actions.sending') || 'Sending...'
            : $t('admin.customers.communication.actions.send') || 'Send'}
        </button>
      </div>
    {/if}
  </div>
</Modal>

<Modal
  show={showEmailCompose}
  title={$t('admin.customers.communication.title_email')}
  onclose={() => {
    showEmailCompose = false;
    emailTarget = null;
  }}
>
  <div class="form">
    {#if emailTarget}
      <div class="compose-target">
        <div>
          <strong>{emailTarget.name}</strong>
          <span>{emailTarget.email}</span>
        </div>
        <span class="pill pill-green"
          >{$t('admin.customers.communication.email_outbox')}</span
        >
      </div>
      <label>
        <span>{$t('admin.customers.communication.template')}</span>
        <select
          class="input"
          bind:value={selectedEmailTemplateId}
          onchange={(event) => applyEmailTemplate(event.currentTarget.value)}
        >
          {#each emailTemplateOptions as template}
            <option value={template.id}>{template.name}</option>
          {/each}
          <option value="custom"
            >{$t('admin.customers.communication.custom_email')}</option
          >
        </select>
      </label>
      <label>
        <span>{$t('admin.customers.communication.subject')}</span>
        <input class="input" bind:value={emailSubject} />
      </label>
      <label>
        <span>{$t('admin.customers.communication.body')}</span>
        <textarea class="input" rows="9" bind:value={emailBody}></textarea>
      </label>
      <div class="compose-footnote">
        <span
          >{emailBody.trim().length}
          {$t('admin.customers.communication.characters')}</span
        >
        <span
          >{$t('admin.customers.communication.queued_through_outbox')}</span
        >
      </div>
      <div class="actions">
        <button
          class="btn btn-secondary"
          onclick={() => {
            showEmailCompose = false;
            emailTarget = null;
          }}
        >
          {$t('common.cancel')}
        </button>
        <button
          class="btn btn-primary"
          onclick={sendCustomerEmail}
          disabled={emailSending || !emailSubject.trim() || !emailBody.trim()}
        >
          <Icon name="send" size={16} />
          {emailSending
            ? $t('admin.customers.communication.actions.sending') || 'Sending...'
            : $t('admin.customers.communication.actions.send_email') || 'Send Email'}
        </button>
      </div>
    {/if}
  </div>
</Modal>

<ConfirmDialog
  show={showDelete}
  title={$t('admin.customers.delete.title')}
  message={$t('admin.customers.delete.message')}
  confirmText={$t('common.delete')}
  cancelText={$t('common.cancel')}
  loading={deleting}
  confirmationKeyword={deleteTarget?.name || ''}
  onconfirm={doDelete}
  oncancel={() => (showDelete = false)}
/>

<style>
  .page-content {
    padding: 1.25rem 1.5rem 1.5rem;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 0.9rem;
  }

  .subtitle {
    color: var(--text-secondary);
    margin-top: 0.35rem;
  }

  .header-actions {
    display: flex;
    gap: 0.55rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .btn {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    padding: 0.55rem 0.9rem;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    font-weight: 650;
    font-size: 0.9rem;
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      transform 0.02s ease;
    user-select: none;
  }

  .btn:hover {
    background: var(--bg-hover);
  }

  .btn:active {
    transform: translateY(1px);
  }

  .btn:disabled {
    opacity: 0.7;
    cursor: wait;
  }

  .btn-primary {
    background: rgba(99, 102, 241, 0.95);
    border-color: rgba(99, 102, 241, 0.55);
    color: white;
  }

  .btn-primary:hover {
    background: rgba(99, 102, 241, 1);
  }

  .btn-secondary {
    background: var(--bg-surface);
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .customer-stats-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .stat-filter {
    display: block;
    width: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
    border-radius: var(--radius-lg);
  }

  .stat-filter.active {
    outline: 2px solid color-mix(in srgb, var(--color-primary) 42%, transparent);
    outline-offset: 2px;
  }

  .table-card {
    padding: 1rem;
    overflow: hidden;
  }

  .customer-toolbar-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .customer-toolbar-results {
    color: var(--text-secondary);
    font-size: 0.92rem;
    white-space: nowrap;
  }

  .customer-toolbar-shortcut {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    min-height: 38px;
    padding: 0.55rem 0.9rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    font-weight: 600;
    cursor: pointer;
    transition:
      border-color 0.2s ease,
      background 0.2s ease,
      box-shadow 0.2s ease,
      transform 0.2s ease;
  }

  .customer-toolbar-shortcut:hover {
    border-color: color-mix(in srgb, var(--color-primary) 35%, var(--border-color));
    background: color-mix(in srgb, var(--bg-tertiary) 82%, var(--color-primary) 18%);
  }

  .customer-toolbar-shortcut:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--color-primary) 42%, transparent);
    outline-offset: 2px;
  }

  .customer-toolbar-shortcut.attention {
    border-color: rgba(239, 68, 68, 0.7);
    background: color-mix(in srgb, var(--bg-tertiary) 75%, rgba(127, 29, 29, 0.55));
    box-shadow: 0 0 0 1px rgba(239, 68, 68, 0.18);
    animation: toolbar-attention 1s ease-in-out infinite;
  }

  .customer-toolbar-shortcut.attention:hover {
    border-color: rgba(248, 113, 113, 0.95);
    background: color-mix(in srgb, var(--bg-tertiary) 68%, rgba(127, 29, 29, 0.7));
  }

  .toolbar-alert-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.5rem;
    height: 1.5rem;
    padding: 0 0.4rem;
    border-radius: 999px;
    background: #ef4444;
    color: #fff;
    font-size: 0.75rem;
    font-weight: 700;
    line-height: 1;
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.08);
  }

  .pulse-red {
    animation: pulse-red 0.95s ease-in-out infinite;
  }

  @keyframes toolbar-attention {
    0%,
    100% {
      box-shadow:
        0 0 0 1px rgba(239, 68, 68, 0.18),
        0 0 0 0 rgba(239, 68, 68, 0.08);
    }
    50% {
      box-shadow:
        0 0 0 1px rgba(248, 113, 113, 0.42),
        0 0 0 6px rgba(239, 68, 68, 0.12);
    }
  }

  @keyframes pulse-red {
    0%,
    100% {
      transform: scale(1);
      filter: brightness(1);
    }
    50% {
      transform: scale(1.08);
      filter: brightness(1.15);
    }
  }

  .toolbar-filters {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
    padding: 0.35rem;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: var(--bg-surface);
  }

  .customer-filter-field {
    display: grid;
    gap: 0.25rem;
    min-width: 150px;
    padding: 0.25rem 0.3rem;
  }

  .customer-filter-field span {
    color: var(--text-secondary);
    font-size: 0.72rem;
    font-weight: 700;
    line-height: 1;
  }

  .customer-filter-select {
    min-height: 38px;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    padding: 0.45rem 2rem 0.45rem 0.7rem;
    font-size: 0.86rem;
    font-weight: 700;
    outline: none;
    cursor: pointer;
  }

  .customer-filter-select:focus {
    border-color: color-mix(in srgb, var(--color-primary) 58%, var(--border-color));
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 16%, transparent);
  }

  .customer-filter-select:hover {
    background: var(--bg-hover);
  }

  .customer-filter-select option {
    background: var(--bg-surface);
    color: var(--text-primary);
  }

  .error-banner {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding: 0.75rem 0.9rem;
    border-radius: 12px;
    border: 1px solid rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.08);
    color: var(--text-primary);
    margin-bottom: 0.75rem;
  }

  .linkish {
    border: none;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
    padding: 0;
  }

  .linkish:disabled {
    cursor: default;
  }

  .name {
    font-weight: 650;
  }

  .sub {
    color: var(--text-secondary);
    font-size: 0.85rem;
    margin-top: 0.15rem;
  }

  .mono {
    font-variant-numeric: tabular-nums;
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .btn-icon {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 10px;
    padding: 0.4rem 0.45rem;
    cursor: pointer;
  }

  .btn-icon:hover {
    background: var(--bg-hover);
  }

  .btn-icon:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .btn-icon.danger {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgb(239, 68, 68);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    font-size: 0.8rem;
    font-weight: 650;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
  }

  .pill-green {
    border-color: rgba(34, 197, 94, 0.35);
    background: rgba(34, 197, 94, 0.12);
    color: rgb(34, 197, 94);
  }

  .pill-gray {
    border-color: rgba(148, 163, 184, 0.35);
    background: rgba(148, 163, 184, 0.12);
    color: rgba(148, 163, 184, 1);
  }

  .pill-warning {
    border-color: rgba(245, 158, 11, 0.34);
    background: rgba(245, 158, 11, 0.1);
    color: rgb(245, 158, 11);
  }

  .mobile-customer-list {
    display: none;
  }

  .mobile-customer-card {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    padding: 0.85rem;
    display: grid;
    gap: 0.75rem;
  }

  .mobile-customer-head {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: flex-start;
  }

  .mobile-customer-meta {
    display: grid;
    gap: 0.3rem;
    color: var(--text-secondary);
    font-size: 0.86rem;
    min-width: 0;
  }

  .mobile-customer-meta span {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .mobile-customer-actions,
  .mobile-pager {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .mobile-pager {
    justify-content: space-between;
    padding-top: 0.15rem;
  }

  .mobile-empty {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    color: var(--text-secondary);
    padding: 1rem;
    text-align: center;
  }

  .form {
    display: grid;
    gap: 0.9rem;
  }

  label > span {
    display: block;
    margin-bottom: 0.35rem;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .input {
    width: 100%;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    padding: 0.65rem 0.75rem;
    outline: none;
  }

  textarea.input {
    resize: vertical;
  }

  .grid2 {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .compose-target {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    padding: 0.8rem;
  }

  .compose-target div {
    min-width: 0;
  }

  .compose-target strong,
  .compose-target div span {
    display: block;
  }

  .compose-target div span {
    color: var(--text-secondary);
    font-size: 0.88rem;
    overflow-wrap: anywhere;
  }

  .compose-footnote {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    color: var(--text-secondary);
    font-size: 0.82rem;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }

  .muted {
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .invite-section {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 0.9rem;
    background: var(--bg-surface);
    display: grid;
    gap: 0.75rem;
  }

  .invite-section-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.6rem;
  }

  .invite-section-heading-block {
    align-items: flex-start;
  }

  .invite-section-copy {
    margin: 0.3rem 0 0;
    color: var(--text-secondary);
    font-size: 0.88rem;
    line-height: 1.5;
  }

  .invite-modal-shell {
    gap: 1rem;
  }

  .invite-overview-grid {
    display: grid;
    grid-template-columns: minmax(0, 1.1fr) minmax(0, 1fr);
    gap: 0.85rem;
  }

  .invite-generate-card {
    padding: 1rem;
    gap: 0.9rem;
  }

  .invite-history-section {
    gap: 0.85rem;
  }

  .invite-section-subtle {
    background: color-mix(in srgb, var(--bg-surface) 88%, var(--bg-tertiary) 12%);
  }

  .actions-inline {
    margin-top: 0;
  }

  .invite-summary-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.6rem;
  }

  .invite-summary-item {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 0.55rem 0.6rem;
    background: color-mix(in srgb, var(--bg-surface) 84%, rgba(99, 102, 241, 0.06));
    display: grid;
    gap: 0.2rem;
  }

  .invite-summary-item small {
    color: var(--text-secondary);
    font-size: 0.75rem;
  }

  .invite-summary-item strong {
    font-size: 0.98rem;
  }

  .invite-result-panel {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 0.9rem;
    background: color-mix(in srgb, var(--bg-surface) 82%, rgba(99, 102, 241, 0.08));
    display: grid;
    gap: 0.6rem;
  }

  .invite-result-head {
    display: flex;
    justify-content: space-between;
    gap: 0.6rem;
    flex-wrap: wrap;
  }

  .invite-copy-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.6rem;
    align-items: center;
  }

  .invite-history-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .inline-check {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .invite-list {
    display: grid;
    gap: 0.65rem;
    max-height: 320px;
    overflow: auto;
    padding-right: 0.25rem;
  }

  .invite-item {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 0.8rem;
    display: flex;
    justify-content: space-between;
    gap: 0.9rem;
    align-items: flex-start;
    background: color-mix(in srgb, var(--bg-surface) 90%, var(--bg-tertiary) 10%);
  }

  .invite-item-main {
    min-width: 0;
    display: grid;
    gap: 0.32rem;
  }

  .invite-item-actions {
    display: inline-flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .invite-meta {
    display: inline-flex;
    gap: 0.55rem;
    align-items: center;
    margin-bottom: 0.25rem;
    flex-wrap: wrap;
  }

  .invite-item-link {
    display: inline-block;
    margin-top: 0.2rem;
    color: var(--text-secondary);
    text-decoration: none;
    overflow-wrap: anywhere;
  }

  .invite-item-link:hover {
    color: var(--text-primary);
    text-decoration: underline;
  }

  .invite-empty-state {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    border: 1px dashed var(--border-color);
    border-radius: 12px;
    padding: 0.85rem 0.9rem;
    color: var(--text-secondary);
  }

  @media (max-width: 900px) {
    .page-content {
      padding: 1rem;
    }

    .stats-grid {
      grid-template-columns: 1fr;
    }
    .page-header {
      flex-direction: column;
      align-items: stretch;
    }
    .header-actions {
      justify-content: stretch;
    }
    .header-actions .btn {
      flex: 1 1 calc(50% - 0.55rem);
      min-width: 0;
    }
    .toolbar-filters {
      width: 100%;
      padding: 0.45rem;
    }
    .customer-filter-field {
      flex: 1 1 180px;
      min-width: 0;
    }
    .customer-filter-select {
      width: 100%;
    }
    .grid2 {
      grid-template-columns: 1fr;
    }
    .compose-target,
    .compose-footnote,
    .actions {
      flex-direction: column;
      align-items: stretch;
    }
    .invite-summary-grid {
      grid-template-columns: 1fr 1fr;
    }
    .invite-overview-grid {
      grid-template-columns: 1fr;
    }
    .invite-copy-row {
      grid-template-columns: 1fr;
    }
    .invite-history-toolbar {
      flex-direction: column;
      align-items: stretch;
    }
    .invite-item {
      flex-direction: column;
      align-items: stretch;
    }
    .invite-item-actions {
      justify-content: flex-start;
    }
  }

  @media (max-width: 720px) {
    .table-card {
      padding: 0.9rem;
    }
    .customer-toolbar-actions {
      width: 100%;
      flex-direction: column-reverse;
      align-items: stretch;
      justify-content: flex-start;
    }
    .customer-toolbar-results {
      white-space: normal;
      text-align: left;
    }
    .customer-toolbar-shortcut {
      width: 100%;
      justify-content: center;
    }
    .desktop-customer-table {
      display: none;
    }
    .mobile-customer-list {
      display: grid;
      gap: 0.75rem;
    }
    .mobile-customer-head {
      flex-direction: column;
      align-items: stretch;
    }
    .mobile-customer-actions .btn {
      flex: 1 1 auto;
    }
  }
</style>
