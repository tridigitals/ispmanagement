<script lang="ts">
  /*
    Pengaturan v2 — SATU halaman utk seluruh pengaturan tenant.

    Versi lama: `(app)/admin/settings/+page.svelte` 2.098 baris (900 script,
    492 CSS scoped) + 4 komponen tab (SettingsEmailTab 746, SettingsPaymentTab
    859, SettingsServiceTab 1.158, SettingsCompanyTab 535) + panel inline
    branding/billing_plan/whatsapp/event_notifications.

    TIGA BUG STRUKTURAL yang diperbaiki di sini, semuanya terukur lebih dulu:

    1. SIMPAN MEMBUANG PERUBAHAN DI BAGIAN LAIN.
       Baris 738 versi lama: `keysToSave = categories[activeTab].keys` — hanya
       key tab yang sedang dibuka yang dikirim. Tapi `hasChanges` (baris 575)
       dihitung dari SELURUH kategori, dan jalur mobile mempertahankan edit
       lintas tab sementara jalur desktop membuangnya (`discard: true`).
       Di sini `saveAllSettings()` mengirim SEMUA key yang berubah dari SEMUA
       bagian (settingsStore), dan SaveBar menyebut bagian yang ikut tersimpan.

    2. NILAI DEFAULT TERSEBAR SEBAGAI 27 CABANG IF.
       Sekarang default hidup di settingsSchema.ts bersama field-nya; key panel
       lama memakai FALLBACKS di settingsStore.

    3. TIDAK ADA VALIDASI LINTAS FIELD.
       validate() menahan simpan (SLA/CPU/latensi/kredensial storage) dengan
       pesan terjemahan.

    Tujuh panel yang dulu "masih di halaman lama" kini jadi TAB NYATA di sini:
    branding & billing plan dirender komponen DS baru; email/payment/service/
    whatsapp/event_notifications me-mount komponen panel legacy yang sama
    (diprop-drill dari store bersama) — tidak ada duplikasi markup, tidak ada
    dua sumber kebenaran. Halaman lama tetap utuh sbg fallback rollback cutover.
  */
  import { goto } from '$app/navigation';
  import { onMount, tick, type Component } from 'svelte';
  import { locale as i18nLocale, t, waitLocale } from 'svelte-i18n';
  import { AppShell, Button, Card, Field, Icon, PageHeader, SaveBar, Tabs } from '$lib/components/ds';
  import { toast } from '$lib/stores/toast';
  import { appSettings } from '$lib/stores/settings';
  import { can } from '$lib/stores/auth';
  import {
    PANEL_SETTING_KEYS,
    changeSetting,
    changedKeys,
    discardSettings,
    loadSettings,
    saveAllSettings,
    settingsStore,
  } from '$lib/stores/settingsStore.svelte';
  import { SETTING_SECTIONS, applyLabels, isVisible, validate } from '$lib/utils/settingsSchema';

  type Values = Record<string, string>;

  let saving = $derived(settingsStore.saving);
  let active = $state('general');

  /* ---------------------------------------------------------------- tabs -- */

  const PANEL_IDS = [
    'branding',
    'billing_plan',
    'email',
    'payment',
    'service',
    'whatsapp',
    'event_notifications',
  ] as const;
  type PanelId = (typeof PANEL_IDS)[number];

  /* Urutan tab = urutan halaman lama, satu baris waktu bagi user. */
  const TAB_ORDER = [
    'general',
    'company',
    'branding',
    'billing_plan',
    'security',
    'network',
    'storage',
    'email',
    'payment',
    'service',
    'whatsapp',
    'event_notifications',
  ] as const;

  /* Label & pesan dirender dari kamus $t; default Indonesia skema hanya
     fallback kalau key terjemahan belum ada. svelte-i18n mem-format string,
     bukan objek, jadi kamus disusun per-key di sini. */
  const g = (path: string, fallback: string): string => {
    const v = $t(path);
    return typeof v === 'string' && v && !v.startsWith('admin.') ? v : fallback;
  };

  const dict = $derived.by(() => {
    const sections: Record<string, { label: string; desc: string }> = {};
    for (const s of SETTING_SECTIONS) {
      sections[s.id] = {
        label: g(`admin.settings_v2.schema.sections.${s.id}.label`, s.label),
        desc: g(`admin.settings_v2.schema.sections.${s.id}.desc`, s.desc),
      };
    }
    const fields: Record<
      string,
      { label: string; help?: string; suffix?: string; options?: Record<string, string> }
    > = {};
    for (const s of SETTING_SECTIONS) {
      for (const f of s.fields) {
        const entry: {
          label: string;
          help?: string;
          suffix?: string;
          options?: Record<string, string>;
        } = { label: g(`admin.settings_v2.schema.fields.${f.key}.label`, f.label) };
        if (f.help) entry.help = g(`admin.settings_v2.schema.fields.${f.key}.help`, f.help);
        if (f.suffix) entry.suffix = g(`admin.settings_v2.schema.units.${f.suffix}`, f.suffix);
        if (f.options) {
          entry.options = {};
          for (const o of f.options)
            entry.options[o.value] = g(
              `admin.settings_v2.schema.fields.${f.key}.options.${o.value}`,
              o.label,
            );
        }
        fields[f.key] = entry;
      }
    }
    return { sections, fields };
  });
  const sections = $derived(applyLabels(dict));
  const schemaById = $derived(new Map(sections.map((s) => [s.id, s])));

  function panelMeta(id: PanelId) {
    const legacy = {
      branding: ['admin.settings_v2.schema.panels.branding.label', 'Merek & Domain'],
      billing_plan: ['admin.settings_v2.schema.panels.billing_plan.label', 'Tagihan & Paket'],
      email: ['admin.settings_v2.schema.panels.email.label', 'Email'],
      payment: ['admin.settings_v2.schema.panels.payment.label', 'Pembayaran'],
      service: ['admin.settings_v2.schema.panels.service.label', 'Layanan'],
      whatsapp: ['admin.settings_v2.schema.panels.whatsapp.label', 'WhatsApp'],
      event_notifications: [
        'admin.settings_v2.schema.panels.event_notifications.label',
        'Notifikasi Event',
      ],
    }[id];
    return g(legacy[0], legacy[1]);
  }

  const values = $derived(settingsStore.values);

  function validationMessages() {
    return {
      slaBreach: (warn: number) => $t('admin.settings_v2.err_sla_breach', { values: { warn } }),
      cpuHot: (risk: number) => $t('admin.settings_v2.err_cpu_hot', { values: { risk } }),
      latencyHot: (risk: number) => $t('admin.settings_v2.err_lat_hot', { values: { risk } }),
      credentialRequired: (label: string, driver: string) =>
        $t('admin.settings_v2.err_cred_required', { values: { label, driver } }),
    };
  }

  const errors = $derived(validate(values as Values, validationMessages()));
  const changed = $derived(changedKeys());

  function prettyKey(k: string): string {
    return k.replace(/_/g, ' ').replace(/\b\w/g, (l) => l.toUpperCase());
  }

  const changedLabels = $derived(
    changed.map((k) => {
      if (k === 'tenant') return $t('admin.settings.keys.tenant_name');
      if (k === 'app_logo_path') return $t('common.logo');
      for (const s of sections) {
        const f = s.fields.find((x) => x.key === k);
        if (f) return f.label;
      }
      return prettyKey(k);
    }),
  );

  const sectionOfKey = (k: string): string | null => {
    if (k === 'tenant' || k === 'app_logo_path' || k === 'tenant_name' || k === 'custom_domain')
      return panelMeta('branding');
    if (k === 'enforce_2fa') return schemaById.get('security')?.label ?? null;
    for (const s of sections) if (s.fields.some((f) => f.key === k)) return s.label;
    for (const [pid, keys] of Object.entries(PANEL_SETTING_KEYS)) {
      if (keys.includes(k)) return panelMeta(pid as PanelId);
    }
    if (['tenant_name', 'custom_domain', 'app_logo_path'].includes(k)) return panelMeta('branding');
    if (k === 'enforce_2fa') return schemaById.get('security')?.label ?? 'Security';
    return null;
  };

  const changedElsewhere = $derived(
    [...new Set(changed.map(sectionOfKey).filter((x): x is string => Boolean(x) && x !== tabLabel(active)))],
  );

  function tabLabel(id: string): string {
    const s = schemaById.get(id);
    if (s) return s.label;
    return panelMeta(id as PanelId);
  }

  const tabItems = $derived(
    TAB_ORDER.map((id) => {
      const label = tabLabel(id);
      const n = changed.filter((k) => sectionOfKey(k) === label).length;
      return { id, label, count: n > 0 ? n : null };
    }),
  );

  const isPanelTab = $derived((id: string): boolean => (PANEL_IDS as readonly string[]).includes(id));
  const activeSection = $derived(schemaById.get(active) ?? null);
  const visibleFields = $derived(
    activeSection ? activeSection.fields.filter((f) => isVisible(f, values as Values)) : [],
  );

  /* ------------------------------------------------- panel mounting (lazy) -- */

  const panelComp = $state<Record<string, Component<any> | null>>({});
  const panelFailed = $state<Record<string, boolean>>({});

  const PANEL_LOADERS: Record<PanelId, () => Promise<{ default: Component<any> }>> = {
    branding: () => import('$lib/components/settings/BrandingPanel.svelte'),
    billing_plan: () => import('$lib/components/billing/TenantBillingPlanPanel.svelte'),
    email: () => import('../../../../(app)/admin/settings/SettingsEmailTab.svelte'),
    payment: () => import('../../../../(app)/admin/settings/SettingsPaymentTab.svelte'),
    service: () => import('../../../../(app)/admin/settings/SettingsServiceTab.svelte'),
    whatsapp: () => import('$lib/components/settings/WhatsAppGatewayTab.svelte'),
    event_notifications: () => import('$lib/components/settings/NotificationEventsTab.svelte'),
  };

  async function ensurePanel(id: string) {
    if (!isPanelTab(id) || panelComp[id] || panelFailed[id]) return;
    try {
      const mod = await PANEL_LOADERS[id as PanelId]();
      panelComp[id] = mod.default;
    } catch (error: any) {
      panelFailed[id] = true;
      toast.error(error?.message || $t('admin.settings_v2.toast_panel_load_failed'));
    }
  }

  $effect(() => {
    if (!settingsStore.loading && isPanelTab(active)) void ensurePanel(active);
  });

  /* ----------------------------------------------- handlers utk panel lama -- */

  function editSetting(key: string, value: unknown) {
    if (!changeSetting(key, value)) {
      toast.error(
        settingsStore.emailReadiness.reason ||
          $t('admin.settings.security.require_email_verification_not_ready'),
      );
    }
  }

  /* state milik panel email */
  const emailProviderOptions = [
    { value: 'smtp', label: 'SMTP' },
    { value: 'resend', label: 'Resend API' },
  ];
  const smtpEncryptionOptions = [
    { value: 'starttls', label: 'STARTTLS' },
    { value: 'tls', label: 'TLS/SSL' },
    { value: 'none', label: 'None' },
  ];
  let testEmailAddress = $state('');
  let sendingTestEmail = $state(false);
  let testingSmtp = $state(false);

  async function sendTestEmail() {
    if (!testEmailAddress) return;
    sendingTestEmail = true;
    try {
      const { api } = await import('$lib/api/client');
      toast.success(await api.settings.sendTestEmail(testEmailAddress));
    } catch (error: any) {
      toast.error(error?.message || $t('admin.settings.toasts.send_test_failed'));
    } finally {
      sendingTestEmail = false;
    }
  }

  async function testSmtpConnection() {
    testingSmtp = true;
    try {
      const { api } = await import('$lib/api/client');
      const r = await api.settings.testSmtpConnection();
      toast.success(`${r.message} (${r.host}:${r.port}, ${r.encryption}, ${r.duration_ms}ms)`);
    } catch (error: any) {
      toast.error(error?.message || $t('admin.settings.email.smtp_test.failed'));
    } finally {
      testingSmtp = false;
    }
  }

  /* state milik panel payment: rekening manual = JSON string di values, jadi
     discard otomatis ikut reset store. */
  let newBank = $state({ bank_name: '', account_number: '', account_holder: '' });
  let showAddBank = $state(false);
  const bankAccounts = $derived.by(() => {
    try {
      const json = (values['payment_manual_accounts'] ?? '').trim();
      return json ? JSON.parse(json) : [];
    } catch {
      return [];
    }
  });
  function addBankAccount() {
    if (!newBank.bank_name || !newBank.account_number || !newBank.account_holder) return;
    const next = [...bankAccounts, { ...newBank, id: crypto.randomUUID() }];
    newBank = { bank_name: '', account_number: '', account_holder: '' };
    showAddBank = false;
    editSetting('payment_manual_accounts', JSON.stringify(next));
  }
  function removeBankAccount(id: string) {
    editSetting(
      'payment_manual_accounts',
      JSON.stringify(bankAccounts.filter((b: any) => b.id !== id)),
    );
  }

  /* helper panel service */
  function formatLastRunAt(value?: string) {
    if (!value) return '-';
    const dt = new Date(value);
    if (Number.isNaN(dt.getTime())) return value;
    return new Intl.DateTimeFormat($i18nLocale || undefined, {
      year: 'numeric',
      month: 'short',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      timeZone: $appSettings.app_timezone || 'UTC',
    }).format(dt);
  }

  /* -------------------------------------------------------------- actions -- */

  async function selectTab(id: string) {
    active = id;
    await ensurePanel(id);
  }

  async function save() {
    const blocking = Object.entries(errors).filter(([k]) => changed.includes(k));
    if (blocking.length > 0) {
      toast.error($t('admin.settings_v2.toast_blocking', { values: { n: blocking.length } }));
      return;
    }
    const prevLocale = settingsStore.baseline['default_locale'] ?? '';
    try {
      const r = await saveAllSettings();
      await appSettings.refresh();
      if (r.savedCount > 0) {
        toast.success($t('admin.settings_v2.toast_saved', { values: { n: r.savedCount } }));
      }
      if (!r.ok) {
        toast.error(
          $t('admin.settings_v2.toast_partial', { values: { failed: r.failedCount, total: r.totalCount } }),
        );
      }
      if ((values['default_locale'] ?? '') !== prevLocale) {
        void i18nLocale.set(values['default_locale']);
        void waitLocale();
      }
    } catch (error: any) {
      toast.error(error?.message || $t('admin.settings_v2.toast_save_failed'));
    }
  }

  function reset() {
    discardSettings();
    toast.info($t('admin.settings_v2.toast_discarded'));
  }

  /* deep-link hash: /admin/settings#email -> redirect ke halaman ini dengan
     hash sama (lihat (app)/+layout.svelte). Hash lama = id tab baru, 1:1. */
  function tabFromHash(): string | null {
    if (typeof window === 'undefined') return null;
    const id = decodeURIComponent(window.location.hash.replace(/^#/, '')).trim();
    return (TAB_ORDER as readonly string[]).includes(id) ? id : null;
  }

  function onHashChange() {
    const id = tabFromHash();
    if (id && id !== active) void selectTab(id);
  }

  onMount(() => {
    /* Cleanup HANYA terdaftar kalau returned synchronously — pasang dulu,
       baru load async. (onMount async = promise returned = listener bocor.) */
    window.addEventListener('hashchange', onHashChange);
    void (async () => {
      try {
        await loadSettings();
      } catch (error: any) {
        toast.error(error?.message || $t('admin.settings_v2.toast_load_failed'));
      }
      await tick();
      const fromHash = tabFromHash();
      if (fromHash) {
        active = fromHash;
        void ensurePanel(fromHash);
      }
    })();
    return () => window.removeEventListener('hashchange', onHashChange);
  });
</script>

<AppShell title={ $t('admin.settings_v2.title') }>
  <PageHeader
    title={ $t('admin.settings_v2.title') }
    eyebrow={ $t('admin.settings_v2.eyebrow') }
    desc={ $t('admin.settings_v2.desc') }
  />

  {#if settingsStore.loading}
    <Card>
      <div class="space-y-4 py-2">
        {#each Array(6) as _}
          <div class="grid gap-2 sm:grid-cols-[15rem_1fr] sm:gap-6">
            <div class="skeleton h-4 w-40 rounded"></div>
            <div class="skeleton h-9 w-full max-w-md rounded-lg"></div>
          </div>
        {/each}
      </div>
    </Card>
  {:else}
    <Tabs items={tabItems} {active} panelId="settings-panel" onselect={(id) => void selectTab(id)} />

    <div id="settings-panel" role="tabpanel">
      {#if activeSection}
        <Card>
          <div class="mb-4 border-b border-ink-100 pb-4">
            <h2 class="text-base font-semibold text-ink-900">{activeSection.label}</h2>
            <p class="mt-0.5 text-sm text-ink-500">{activeSection.desc}</p>
          </div>

          <div class="divide-y divide-ink-100">
            {#each visibleFields as f (f.key)}
              <Field
                id={`set-${f.key}`}
                label={f.label}
                type={f.type}
                value={values[f.key] ?? ''}
                help={f.help}
                error={errors[f.key] ?? null}
                options={f.options}
                placeholder={f.placeholder}
                suffix={f.suffix}
                min={f.min}
                max={f.max}
                rows={f.rows}
                dirty={changed.includes(f.key)}
                onchange={(v) => editSetting(f.key, v)}
              />
            {/each}

            {#if active === 'security'}
              <Field
                id="set-enforce_2fa"
                label={ $t('admin.settings.sections.enforce_2fa') }
                type="toggle"
                value={ values['enforce_2fa'] ?? 'false' }
                dirty={changed.includes('enforce_2fa')}
                onchange={(v) => editSetting('enforce_2fa', v)}
              />
              {#if !settingsStore.emailReadiness.ready}
                <p class="py-3 text-sm text-amber-700">
                  <Icon name="alert" size={14} class="mr-1 inline-flex align-text-bottom" />
                  { settingsStore.emailReadiness.reason ||
                    $t('admin.settings.security.require_email_verification_not_ready') }
                </p>
              {/if}
            {/if}
          </div>

          {#if activeSection && visibleFields.length < activeSection.fields.length}
            <p class="mt-4 border-t border-ink-100 pt-4 text-sm text-ink-400">
              { $t('admin.settings_v2.hidden_note', {
                values: { n: activeSection.fields.length - visibleFields.length },
              }) }
            </p>
          {/if}
        </Card>
      {:else if isPanelTab(active)}
        <Card title={panelMeta(active as PanelId)}>
          {#if panelComp[active]}
            {@const Panel = panelComp[active]}
            {#if active === 'branding'}
              <Panel />
            {:else if active === 'billing_plan'}
              <Panel openSubscription={() => void goto('/v2/admin/subscription')} />
            {:else if active === 'email'}
              <Panel
                localSettings={values}
                {emailProviderOptions}
                {smtpEncryptionOptions}
                bind:testEmailAddress
                {sendingTestEmail}
                {testingSmtp}
                canReadEmailOutbox={ $can('read', 'email_outbox') }
                handleChange={editSetting}
                onSendTestEmail={sendTestEmail}
                onTestSmtpConnection={testSmtpConnection}
                onViewOutbox={() => void goto('/v2/admin/email-outbox')}
              />
            {:else if active === 'payment'}
              <Panel
                localSettings={values}
                {bankAccounts}
                bind:newBank
                bind:showAddBank
                handleChange={editSetting}
                {addBankAccount}
                {removeBankAccount}
              />
            {:else if active === 'service'}
              <Panel
                localSettings={values}
                formattedLastRunAt={formatLastRunAt(values['customer_invoice_last_run_at'])}
                billingLogsPath="/v2/admin/invoices/collection"
                invoicesPath="/v2/admin/invoices"
                handleChange={editSetting}
              />
            {:else if active === 'whatsapp'}
              <Panel
                localSettings={values}
                handleChange={editSetting}
                eventScope="tenant"
                title={ $t('admin.settings.whatsapp.title') }
                description={ $t('admin.settings.whatsapp.description') }
              />
            {:else if active === 'event_notifications'}
              <Panel
                localSettings={values}
                handleChange={editSetting}
                eventSettingsKey="wa_events_tenant"
                eventScope="tenant"
                emailReady={settingsStore.emailReadiness.ready}
                emailReadinessReason={settingsStore.emailReadiness.reason}
                title={ $t('admin.settings.event_notifications.title') }
                description={ $t('admin.settings.event_notifications.description') }
              />
            {/if}
          {:else if panelFailed[active]}
            <div class="flex items-center justify-between gap-4 py-2">
              <p class="text-sm text-red-600">{ $t('admin.settings_v2.toast_panel_load_failed') }</p>
              <Button
                variant="secondary"
                icon="refresh"
                onclick={() => {
                  panelFailed[active] = false;
                  void ensurePanel(active);
                }}
              >
                { $t('common.retry') }
              </Button>
            </div>
          {:else}
            <div class="flex justify-center py-10" aria-busy="true">
              <span class="skeleton h-6 w-6 animate-spin rounded-full border-2 border-ink-200 border-t-ink-700"></span>
            </div>
          {/if}
        </Card>
      {/if}
    </div>

    <SaveBar
      changes={changedLabels}
      elsewhere={changedElsewhere}
      {saving}
      onsave={save}
      onreset={reset}
    />
  {/if}
</AppShell>
