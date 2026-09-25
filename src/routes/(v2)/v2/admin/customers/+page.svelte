<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page as pageStore } from '$app/stores';
  import { get as deriveStore } from 'svelte/store';
  import { api } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import type { MessageTemplate } from '$lib/api/types';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Field from '$lib/components/ds/Field.svelte';
  import { exportCsvRows } from '$lib/utils/tabularExport';
  import AppShell from '$lib/components/ds/AppShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Icon from '$lib/components/ds/Icon.svelte';
  import DataTable from '$lib/components/ds/DataTable.svelte';
  import RowActions from '$lib/components/ds/RowActions.svelte';
  import { formatRelative } from '$lib/components/ds/format';
  import type { Column } from '$lib/components/ds/table-types';
  import type { CustomerListItem } from '$lib/api/types';

  import { t } from 'svelte-i18n';
  import { get as getStore } from 'svelte/store';
  const columns = $derived<Column[]>([
    { key: 'name', label: $t('admin.customers.columns.customer') },
    { key: 'contact', label: $t('admin.customers.columns.contact'), hideSm: true },
    { key: 'services', label: $t('admin.customers.columns.service') },
    { key: 'updated', label: $t('admin.customers.columns.updated'), hideSm: true },
    { key: 'actions', label: $t('common.actions'), align: 'right', width: '120px' },
  ]);

  let rows = $state<CustomerListItem[]>([]);
  let total = $state(0);
  let page = $state(1);
  let perPage = $state(25);
  let q = $state('');
  let statusFilter = $state<'all' | 'active' | 'inactive'>('all');
  let serviceFilter = $state<'all' | 'active' | 'inactive' | 'none'>('all');
  let loading = $state(true);
  let err = $state('');

  /* Hitungan per filter LAYANAN, bukan per `customers.is_active`.
     Alasan: di tenant ini 548/548 pelanggan is_active=true sementara 542
     langganan berstatus suspended, jadi chip "Aktif 548" tidak membedakan
     apa pun. Yang menentukan pendapatan adalah status langganan. */
  let counts = $state({ all: 0, svcActive: 0, svcInactive: 0, svcNone: 0, pending: 0 });

  const canManage = $derived($can('manage', 'customers'));
  /* Buat pesanan instalasi = izin `create` pada resource `orders`, sama seperti
     legacy (app)/admin/customers — resource izinnya beda dari `customers`. */
  const canCreateOrders = $derived($can('create', 'orders'));

  /* ── Modal tambah pelanggan ─────────────────────────────────────────── */
  let showCreate = $state(false);
  let creating = $state(false);
  let cName = $state('');
  let cEmail = $state('');
  let cPhone = $state('');
  let cNotes = $state('');
  let cPass = $state('');
  let cPass2 = $state('');

  function openCreate() {
    cName = ''; cEmail = ''; cPhone = ''; cNotes = ''; cPass = ''; cPass2 = '';
    showCreate = true;
  }

  async function submitCreate() {
    if (!cName.trim()) return;
    if (!cEmail.trim()) {
      toast.error($t('admin.customers.new.portal.validation.email_required'));
      return;
    }
    if (!cPass || cPass.length < 6) {
      toast.error($t('admin.customers.new.portal.validation.password_min'));
      return;
    }
    if (cPass !== cPass2) {
      toast.error($t('admin.customers.new.portal.validation.password_mismatch'));
      return;
    }
    creating = true;
    try {
      await api.customers.createWithPortal({
        name: cName.trim(),
        email: cEmail.trim(),
        phone: cPhone.trim() || null,
        notes: cNotes.trim() || null,
        portal_email: cEmail.trim(),
        portal_name: cName.trim(),
        portal_password: cPass,
      });
      showCreate = false;
      toast.success($t('admin.customers.toasts.created'));
      page = 1;
      await load();
    } catch (e) {
      toast.error($t('admin.customers.toasts.create_failed', { values: { message: String((e as Error)?.message ?? e) } }));
    } finally {
      creating = false;
    }
  }

  /* ── Hapus pelanggan (konfirmasi) ───────────────────────────────────── */
  let deleteTarget = $state<CustomerListItem | null>(null);
  let deleteOpen = $state(false);

  async function confirmDelete() {
    if (!deleteTarget) return;
    const c = deleteTarget;
    deleteTarget = null;
    deleteOpen = false;
    try {
      await api.customers.delete(c.id);
      toast.success($t('admin.customers.toasts.deleted') || $t('common.deleted'));
      await load();
    } catch (e) {
      toast.error(String((e as Error)?.message ?? e));
    }
  }

  /* ── Komunikasi WhatsApp / Email ────────────────────────────────────── */
  let waTemplates = $state<MessageTemplate[]>([]);
  let emailTemplates = $state<MessageTemplate[]>([]);
  let waTarget = $state<CustomerListItem | null>(null);
  let waOpen = $state(false);
  let waTemplateId = $state('custom');
  let waMessage = $state('');
  let waSending = $state(false);
  let waReady = $state(false);
  let waReason = $state('');

  let emTarget = $state<CustomerListItem | null>(null);
  let emOpen = $state(false);
  let emTemplateId = $state('custom');
  let emSubject = $state('');
  let emBody = $state('');
  let emSending = $state(false);

  async function loadCommunication() {
    try {
      const [wa, em, ready] = await Promise.all([
        api.messageTemplates.list({ channel: 'whatsapp', status: 'active', target: 'customer', triggerMode: 'manual' }),
        api.messageTemplates.list({ channel: 'email', status: 'active', target: 'customer', triggerMode: 'manual' }),
        api.whatsapp.readiness(),
      ]);
      waTemplates = wa;
      emailTemplates = em;
      waReady = !!ready?.ready;
      waReason = ready?.reason || '';
    } catch {
      waTemplates = [];
      emailTemplates = [];
    }
  }

  function renderTpl(body: string, c: CustomerListItem) {
    const tenant = (deriveStore(pageStore).data as { tenant?: { name?: string } } | undefined)?.tenant?.name || '';
    const values: Record<string, string> = {
      'tenant.name': tenant,
      'customer.id': c.id,
      'customer.name': c.name,
      'customer.email': c.email || '',
      'customer.phone': c.phone || '',
      'customer.status': c.is_active ? 'active' : 'inactive',
      'customer.notes': c.notes || '',
    };
    return body.replace(/\{\{\s*([\w.]+)\s*\}\}/g, (_m, k) => values[k] ?? '');
  }

  function buildWaMessage(c: CustomerListItem, tplId: string) {
    const t0 = waTemplates.find((x) => x.id === tplId);
    if (t0?.whatsapp_body) return renderTpl(t0.whatsapp_body, c);
    if (tplId === 'payment_reminder')
      return $t('admin.customers.communication.tpl_payment_reminder', { values: { name: c.name } });
    if (tplId === 'installation_followup')
      return $t('admin.customers.communication.tpl_install_followup', { values: { name: c.name } });
    if (tplId === 'service_check')
      return $t('admin.customers.communication.tpl_service_check', { values: { name: c.name } });
    return $t('admin.customers.communication.fallback_whatsapp', { values: { name: c.name } });
  }

  function waTplOptions() {
    return [
      ...waTemplates.map((x) => ({ value: x.id, label: x.name })),
      { value: 'custom', label: $t('admin.customers.communication.custom_message') },
    ];
  }

  function openWhatsApp(c: CustomerListItem) {
    if (!c.phone) {
      toast.error($t('admin.customers.communication.phone_not_set'));
      return;
    }
    if (!waReady) {
      toast.error(waReason || $t('admin.customers.communication.gateway_not_ready'));
      return;
    }
    waTarget = c;
    waTemplateId = waTemplates[0]?.id || 'custom';
    waMessage = buildWaMessage(c, waTemplateId);
    waOpen = true;
  }

  function applyWaTemplate(id: string) {
    waTemplateId = id;
    if (waTarget && id !== 'custom') waMessage = buildWaMessage(waTarget, id);
  }

  async function sendWhatsApp() {
    if (!waTarget || waSending) return;
    const message = waMessage.trim();
    if (!message) {
      toast.error($t('admin.customers.communication.whatsapp_body_required'));
      return;
    }
    waSending = true;
    try {
      const res = await api.whatsapp.sendCustomer({
        customerId: waTarget.id,
        message,
        template: waTemplateId,
        templateId: waTemplateId === 'custom' ? null : waTemplateId,
      });
      if (!res.ok) {
        toast.error((res as { error?: string }).error || $t('admin.customers.communication.whatsapp_failed'));
        return;
      }
      toast.success($t('admin.customers.communication.whatsapp_sent'));
      waOpen = false;
      waTarget = null;
    } catch (e) {
      toast.error(String((e as Error)?.message ?? e));
    } finally {
      waSending = false;
    }
  }

  function buildEmail(c: CustomerListItem, tplId: string) {
    const t0 = emailTemplates.find((x) => x.id === tplId);
    if (t0) {
      return {
        subject: renderTpl(t0.email_subject || '', c),
        body: renderTpl(t0.email_body || '', c),
      };
    }
    return {
      subject: $t('admin.customers.communication.fallback_email_subject', { values: { name: c.name } }),
      body: $t('admin.customers.communication.fallback_email_body', { values: { name: c.name } }),
    };
  }

  function openEmail(c: CustomerListItem) {
    if (!c.email) {
      toast.error($t('admin.customers.communication.email_not_set'));
      return;
    }
    emTarget = c;
    emTemplateId = emailTemplates[0]?.id || 'custom';
    const built = buildEmail(c, emTemplateId);
    emSubject = built.subject;
    emBody = built.body;
    emOpen = true;
  }

  function applyEmailTemplate(id: string) {
    emTemplateId = id;
    if (emTarget && id !== 'custom') {
      const built = buildEmail(emTarget, id);
      emSubject = built.subject;
      emBody = built.body;
    }
  }

  async function sendEmail() {
    if (!emTarget || emSending) return;
    const subject = emSubject.trim();
    const body = emBody.trim();
    if (!subject) {
      toast.error($t('admin.customers.communication.email_subject_required'));
      return;
    }
    if (!body) {
      toast.error($t('admin.customers.communication.email_body_required'));
      return;
    }
    emSending = true;
    try {
      await api.customerCommunication.sendEmail({
        customerId: emTarget.id,
        subject,
        body,
        templateId: emTemplateId === 'custom' ? null : emTemplateId,
      });
      toast.success($t('admin.customers.communication.email_queued'));
      emOpen = false;
      emTarget = null;
    } catch (e) {
      toast.error(String((e as Error)?.message ?? e));
    } finally {
      emSending = false;
    }
  }

  /* ── Ekspor CSV baris aktif (respek filter + kata kunci) ────────────── */
  async function exportCsv() {
    try {
      const res = await api.customers.list({
        q: q || undefined,
        page: 1,
        perPage: 2000,
        status: statusFilter,
        service: serviceFilter,
        installation: activeChip === 'pending' ? 'pending' : 'all',
      });
      const data = res.data ?? [];
      if (!data.length) {
        toast.error($t('common.nothing_to_export'));
        return;
      }
      exportCsvRows(
        data.map((c: CustomerListItem) => ({
          name: c.name ?? '',
          email: c.email ?? '',
          phone: c.phone ?? '',
          service_status: c.service_status ?? '',
          active_subscriptions: c.active_subscriptions ?? 0,
          pending_installations: c.pending_installations ?? 0,
          is_active: c.is_active ? '1' : '0',
          created_at: c.created_at ?? '',
        })),
        'pelanggan',
      );
    } catch (e) {
      toast.error(String((e as Error)?.message ?? e));
    }
  }

  /* ── Buatan tagihan: lewat halaman Tagihan terfilter pelanggan ──────── */
  async function createInvoiceFor(c: CustomerListItem) {
    try {
      const subsRes = await api.customers.subscriptions.list(c.id, { page: 1, per_page: 50 });
      const subs = (subsRes.data ?? []) as { id: string; status: string }[];
      const sub = subs.find((s) => s.status === 'active' || s.status === 'suspended');
      if (!sub) {
        toast.error($t('admin.customers.list_v2.no_sub_for_invoice'));
        return;
      }
      const inv = await api.payment.createInvoiceForCustomerSubscription(sub.id);
      toast.success($t('admin.customers.list_v2.invoice_created'));
      goto(`/v2/admin/invoices/${(inv as { id: string }).id}`);
    } catch (e) {
      toast.error(String((e as Error)?.message ?? e));
    }
  }

  /* Chip filter cepat. Menggantikan 3 dropdown terpisah di halaman lama:
     pilihan yang sering dipakai jadi satu klik, dan jumlahnya terlihat. */
  const chips = $derived([
    { key: 'all', label: $t('common.all'), count: counts.all },
    { key: 'svc-active', label: $t('admin.customers.list_v2.chip_active'), count: counts.svcActive },
    { key: 'svc-inactive', label: $t('admin.customers.list_v2.chip_inactive'), count: counts.svcInactive },
    { key: 'pending', label: $t('admin.customers.list_v2.chip_pending'), count: counts.pending },
    { key: 'svc-none', label: $t('admin.customers.list_v2.chip_none'), count: counts.svcNone },
  ]);

  let activeChip = $state('all');

  function applyChip(key: string) {
    activeChip = key;
    page = 1;

    statusFilter = 'all';
    serviceFilter =
      key === 'svc-active'
        ? 'active'
        : key === 'svc-inactive'
          ? 'inactive'
          : key === 'svc-none'
            ? 'none'
            : 'all';

    load();
  }

  /* Placeholder hasil impor MixRadius: baris ini bukan pelanggan nyata,
     jadi aksinya disembunyikan supaya tidak ada yang mengirim WA ke sana. */
  function isPlaceholder(c: CustomerListItem): boolean {
    return /unassigned|system import/i.test(c.name ?? '');
  }

  function serviceLabel(c: CustomerListItem): string {
    const tt = getStore(t);
    if (c.pending_installations > 0) return tt('admin.customers.list_v2.chip_pending');
    if (c.active_subscriptions > 0) return tt('admin.customers.list_v2.n_active', { values: { n: c.active_subscriptions } });
    if (c.subscription_count > 0) return tt('admin.customers.list_v2.chip_inactive');
    return tt('admin.customers.list_v2.no_service');
  }

  function serviceTone(c: CustomerListItem) {
    if (c.pending_installations > 0) return 'warning' as const;
    if (c.active_subscriptions > 0) return 'positive' as const;
    if (c.subscription_count > 0) return 'negative' as const;
    return 'neutral' as const;
  }

  let searchTimer: ReturnType<typeof setTimeout> | undefined;

  function onSearch(value: string) {
    q = value;
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      page = 1;
      load();
    }, 300);
  }

  async function load() {
    loading = true;
    err = '';

    try {
      const res = await api.customers.list({
        q: q || undefined,
        page,
        perPage,
        status: statusFilter,
        service: serviceFilter,
        installation: activeChip === 'pending' ? 'pending' : 'all',
      });
      rows = res.data ?? [];
      total = res.total ?? 0;
    } catch (e) {
      err = 'Gagal memuat daftar pelanggan';
      console.warn('list customers gagal', e);
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    /* Hitungan chip diambil dari endpoint list dengan per_page=1: yang dipakai
       hanya `total`, jadi biaya transfernya satu baris per chip. Ini jauh lebih
       murah daripada menarik 548 baris hanya untuk dihitung di klien. */
    const countOf = (service: 'all' | 'active' | 'inactive' | 'none', installation?: 'pending') =>
      api.customers
        .list({ page: 1, perPage: 1, service, installation: installation ?? 'all' })
        .then((r) => r.total ?? 0)
        .catch((e) => {
          console.error('hitung chip pelanggan gagal:', e);
          return 0;
        });

    if (deriveStore(pageStore).url.searchParams.get('new') === '1') {
      openCreate();
      goto('/v2/admin/customers', { replaceState: true, keepFocus: true });
    }
    void loadCommunication();
    await Promise.all([
      load(),
      Promise.all([
        countOf('all'),
        countOf('active'),
        countOf('inactive'),
        countOf('none'),
        countOf('all', 'pending'),
      ]).then(([all, svcActive, svcInactive, svcNone, pending]) => {
        counts = { all, svcActive, svcInactive, svcNone, pending };
      }),
    ]);
  });
</script>

<AppShell title={ $t('admin.customers.title') }>
  <PageHeader
    title={ $t('admin.customers.title') }
    desc={ $t('admin.customers.list_v2.desc', { values: { a: counts.all, b: counts.svcActive, c: counts.pending } }) }
  >
    {#snippet actions()}
      <Button icon="download" onclick={() => void exportCsv()}>{ $t('admin.customers.list_v2.export') }</Button>
      {#if canCreateOrders}
        <Button icon="receipt" onclick={() => goto('/v2/admin/customers/orders/new')}>
          { $t('admin.customers.actions.create_order') }
        </Button>
      {/if}
      {#if canManage}
        <Button variant="primary" icon="plus" onclick={openCreate}>{ $t('admin.customers.list_v2.add') }</Button>
      {/if}
    {/snippet}
  </PageHeader>

  {#if err}
    <div
      role="alert"
      class="mb-4 rounded-lg border border-red-200 bg-red-50 px-3.5 py-2.5 text-base text-red-800"
    >
      {err}
    </div>
  {/if}

  <!-- Filter: chip + satu kotak cari. -->
  <div class="mb-3 flex flex-wrap items-center gap-2">
    {#each chips as chip}
      <button
        onclick={() => applyChip(chip.key)}
        aria-pressed={activeChip === chip.key}
        class="focus-ring flex h-8 items-center gap-1.5 rounded-lg px-3 text-base
          {activeChip === chip.key
          ? 'bg-ink-900 font-medium text-white'
          : 'bg-white text-ink-700 ring-1 ring-inset ring-ink-200 hover:bg-ink-50'}"
      >
        {chip.label}
        <span
          class="num text-sm {activeChip === chip.key ? 'text-ink-300' : 'text-ink-400'}"
        >
          {chip.count}
        </span>
      </button>
    {/each}

    <div class="relative ml-auto w-full sm:w-64">
      <span class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-400">
        <Icon name="search" size={15} />
      </span>
      <input
        value={q}
        oninput={(e) => onSearch((e.currentTarget as HTMLInputElement).value)}
        type="search"
        placeholder={ $t('admin.customers.list_v2.search_ph') }
        aria-label={ $t('admin.customers.list_v2.search_aria') }
        class="h-8 w-full rounded-lg bg-white pr-3 pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400 focus:ring-brand-600 focus:outline-none"
      />
    </div>
  </div>

  <Card padded={false}>
    <DataTable
      {columns}
      rows={rows}
      {loading}
      pageSize={perPage}
      page={page}
      {total}
      onpage={(p) => {
        page = p;
        load();
      }}
      onpagesize={(n) => {
        perPage = n;
        page = 1;
        load();
      }}
      emptyTitle={ $t('admin.customers.list_v2.empty_title') }
      emptyHint={q ? $t('common.no_results_for', { values: { q } }) : $t('admin.customers.list_v2.empty_hint')}
      footNote={ $t('admin.customers.list_v2.foot', { values: { n: total } }) }
    >
      {#snippet cell(c: CustomerListItem, col: Column)}
        {#if col.key === 'name'}
          <div class="flex items-center gap-1.5">
            <span class="font-medium text-ink-900">{c.name}</span>
            {#if !c.is_active}
              <Badge tone="negative" label={ $t('admin.customers.list_v2.disabled') } />
            {/if}
          </div>
          <div class="num text-sm text-ink-400">{c.customer_number || '—'}</div>
        {:else if col.key === 'contact'}
          <div class="text-ink-700">{c.email || '—'}</div>
          <div class="num text-sm text-ink-400">{c.phone || '—'}</div>
        {:else if col.key === 'services'}
          <Badge tone={serviceTone(c)} label={serviceLabel(c)} />
        {:else if col.key === 'updated'}
          <span class="text-sm text-ink-500">{formatRelative(c.updated_at)}</span>
        {:else if col.key === 'actions'}
          {#if isPlaceholder(c)}
            <div class="text-right text-sm text-ink-400">—</div>
          {:else}
            <RowActions
              primary={{
                label: $t('common.open'),
                icon: 'chevronRight',
                onclick: () => goto(`/v2/admin/customers/${c.id}`),
              }}
              rest={[
                /* Buat pesanan = izin create/orders, BUKAN manage/customers —
                   jadi dipisah dari blok canManage di bawah supaya staf yang
                   boleh membuat order tetap bisa, walau tak boleh mengelola
                   pelanggan. Router wizard membaca ?customer_id= untuk prefill. */
                ...(canCreateOrders
                  ? [
                      {
                        label: $t('admin.customers.actions.create_order'),
                        icon: 'clipboard' as const,
                        onclick: () =>
                          goto(`/v2/admin/customers/orders/new?customer_id=${encodeURIComponent(c.id)}`),
                      },
                    ]
                  : []),
                ...(canManage
                  ? [
                      { label: $t('admin.customers.list_v2.act_service'), icon: 'wifi' as const, onclick: () => goto(`/v2/admin/customers/${c.id}?tab=subscriptions`) },
                      { label: $t('admin.customers.list_v2.act_invoice'), icon: 'receipt' as const, onclick: () => void createInvoiceFor(c) },
                      { label: $t('admin.customers.list_v2.act_wa'), icon: 'inbox' as const, onclick: () => openWhatsApp(c) },
                      { label: $t('admin.customers.list_v2.act_email'), icon: 'mail' as const, onclick: () => openEmail(c) },
                      { label: $t('admin.customers.list_v2.act_delete'), icon: 'close' as const, danger: true, onclick: () => ((deleteTarget = c), (deleteOpen = true)) },
                    ]
                  : []),
              ]}
            />
          {/if}
        {/if}
      {/snippet}
    </DataTable>
  </Card>
</AppShell>

<!-- Modal tambah pelanggan -->
<Modal bind:show={showCreate} title={ $t('admin.customers.new.title') } width="560px">
  <div class="space-y-3 py-1">
    <Field stacked id="nc-name" label={ $t('admin.customers.fields.name') } value={cName}
      placeholder={ $t('admin.settings.company.name_placeholder') } onchange={(v) => (cName = v)} />
    <div class="grid gap-x-5 sm:grid-cols-2">
      <Field stacked id="nc-email" label={ $t('admin.customers.fields.email') } value={cEmail} type="email"
        placeholder="customer@example.com" onchange={(v) => (cEmail = v)} />
      <Field stacked id="nc-phone" label={ $t('admin.customers.fields.phone') } value={cPhone}
        placeholder="+62..." onchange={(v) => (cPhone = v)} />
    </div>
    <Field stacked id="nc-notes" label={ $t('admin.customers.fields.notes') } value={cNotes} type="textarea" rows={3}
      onchange={(v) => (cNotes = v)} />
    <div class="rounded-lg bg-ink-50 p-3 ring-1 ring-inset ring-ink-200">
      <div class="mb-2 text-sm font-medium text-ink-700">{ $t('admin.customers.new.portal.title') }</div>
      <div class="mb-3 text-sm text-ink-500">{ $t('admin.customers.new.portal.subtitle') }</div>
      <div class="grid gap-x-5 sm:grid-cols-2">
        <Field stacked id="nc-pass" label={ $t('admin.customers.new.portal.password') } value={cPass} type="password"
          onchange={(v) => (cPass = v)} />
        <Field stacked id="nc-pass2" label={ $t('admin.customers.new.portal.password_confirm') } value={cPass2} type="password"
          onchange={(v) => (cPass2 = v)} />
      </div>
    </div>
  </div>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (showCreate = false)}>{ $t('common.cancel') }</Button>
    <Button variant="primary" loading={creating}
      disabled={!cName.trim() || !cEmail.trim() || !cPass || !cPass2}
      onclick={() => void submitCreate()}>{ $t('common.create') }</Button>
  {/snippet}
</Modal>

<!-- Modal hapus pelanggan -->
<Modal bind:show={deleteOpen} title={ $t('admin.customers.delete.title') } width="480px">
  <p class="py-2 text-ink-700">{ $t('admin.customers.delete.message') }</p>
  <p class="pb-2 text-sm font-medium text-ink-900">{deleteTarget?.name}</p>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => ((deleteTarget = null), (deleteOpen = false))}>{ $t('common.cancel') }</Button>
    <Button variant="danger" onclick={() => void confirmDelete()}>{ $t('common.delete') }</Button>
  {/snippet}
</Modal>

<!-- Modal kirim WhatsApp -->
<Modal bind:show={waOpen} title={ $t('admin.customers.communication.title_whatsapp') } width="560px">
  <div class="space-y-3 py-1">
    <div class="text-sm text-ink-500">{waTarget?.name} · <span class="num">{waTarget?.phone}</span></div>
    <Field stacked id="wa-tpl" label={ $t('admin.customers.communication.template') } value={waTemplateId} type="select"
      options={waTplOptions()} onchange={applyWaTemplate} />
    <Field stacked id="wa-msg" label={ $t('admin.customers.communication.body') } value={waMessage} type="textarea" rows={5}
      help={`${waMessage.length} ${$t('admin.customers.communication.characters')}`} onchange={(v) => (waMessage = v)} />
  </div>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (waOpen = false)}>{ $t('common.cancel') }</Button>
    <Button variant="primary" loading={waSending} onclick={() => void sendWhatsApp()}>{ $t('admin.customers.communication.actions.send_whatsapp') }</Button>
  {/snippet}
</Modal>

<!-- Modal kirim email -->
<Modal bind:show={emOpen} title={ $t('admin.customers.communication.title_email') } width="560px">
  <div class="space-y-3 py-1">
    <div class="text-sm text-ink-500">{emTarget?.name} · <span class="num">{emTarget?.email}</span></div>
    <Field stacked id="em-tpl" label={ $t('admin.customers.communication.template') } value={emTemplateId} type="select"
      options={[...emailTemplates.map((x) => ({ value: x.id, label: x.name })), { value: 'custom', label: $t('admin.customers.communication.custom_email') }]}
      onchange={applyEmailTemplate} />
    <Field stacked id="em-subj" label={ $t('admin.customers.communication.subject') } value={emSubject} onchange={(v) => (emSubject = v)} />
    <Field stacked id="em-body" label={ $t('admin.customers.communication.body') } value={emBody} type="textarea" rows={7} onchange={(v) => (emBody = v)} />
  </div>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (emOpen = false)}>{ $t('common.cancel') }</Button>
    <Button variant="primary" loading={emSending} onclick={() => void sendEmail()}>{ $t('admin.customers.communication.actions.send_email') }</Button>
  {/snippet}
</Modal>
