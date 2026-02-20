<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can, user, tenant } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';

  type RouterRow = { id: string; name: string };
  type CustomerRow = { id: string; name: string };
  type LocationRow = { id: string; label: string };

  type Candidate = {
    username: string;
    router_secret_id?: string | null;
    profile_name?: string | null;
    remote_address?: string | null;
    disabled: boolean;
    comment?: string | null;
    password_available: boolean;
    action: 'new' | 'update' | 'same';
    existing_account_id?: string | null;
  };

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);

  let step = $state<1 | 2 | 3>(1);
  let loading = $state(false);

  let routers = $state<RouterRow[]>([]);
  let routerId = $state('');
  let includeDisabled = $state(false);

  let customers = $state<CustomerRow[]>([]);
  let customerId = $state('');
  let locations = $state<LocationRow[]>([]);
  let locationId = $state('');

  let candidates = $state<Candidate[]>([]);
  let selected = $state<Set<string>>(new Set());

  let result = $state<any | null>(null);

  const columns = $derived([
    { key: 'pick', label: '', width: '46px' },
    { key: 'username', label: $t('admin.customers.pppoe.columns.username') || 'Username' },
    { key: 'profile', label: $t('admin.customers.pppoe.fields.profile') || 'Profile', class: 'mono' },
    { key: 'remote', label: $t('admin.customers.pppoe.fields.remote_address') || 'Remote', class: 'mono' },
    { key: 'disabled', label: $t('common.status') || 'Status', width: '110px' },
    { key: 'action', label: $t('common.action') || 'Action', width: '110px' },
    { key: 'pw', label: $t('admin.network.pppoe.import.columns.password') || 'Password', width: '120px' },
  ]);

  const tableData = $derived.by(() =>
    candidates.map((c) => ({
      id: c.username,
      username: c.username,
      profile: c.profile_name || '—',
      remote: c.remote_address || '—',
      disabled: c.disabled,
      action: c.action,
      pw: c.password_available,
    })),
  );

  onMount(async () => {
    if (!$can('manage', 'pppoe') && !$can('read', 'pppoe')) {
      goto('/unauthorized');
      return;
    }
    await loadRouters();
    await loadCustomers();
  });

  async function loadRouters() {
    try {
      routers = (await api.mikrotik.routers.list()) as any;
      if (!routerId && routers.length) routerId = routers[0].id;
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function loadCustomers() {
    try {
      const res = await api.customers.list({ page: 1, perPage: 1000 });
      customers = (res.data || []).map((c) => ({ id: c.id, name: c.name }));
    } catch {
      customers = [];
    }
  }

  async function loadLocationsForCustomer(cid: string) {
    locations = [];
    locationId = '';
    if (!cid) return;
    try {
      locations = (await api.customers.locations.list(cid)) as any;
      if (locations.length) locationId = locations[0].id;
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  function resetPreview() {
    candidates = [];
    selected = new Set();
    result = null;
    step = 1;
  }

  function toggleAll(kind: 'new_update' | 'all' | 'none') {
    const next = new Set<string>();
    if (kind === 'none') {
      selected = next;
      return;
    }

    for (const c of candidates) {
      if (kind === 'all') next.add(c.username);
      else if (c.action === 'new' || c.action === 'update') next.add(c.username);
    }
    selected = next;
  }

  async function scan() {
    if (!routerId) {
      toast.error($t('admin.network.pppoe.import.errors.select_router') || 'Select a router');
      return;
    }
    loading = true;
    try {
      const rows = (await api.pppoe.import.preview(routerId, {
        include_disabled: includeDisabled,
      })) as any[];
      candidates = (rows || []) as any;

      // Default select: new + update
      const next = new Set<string>();
      for (const c of candidates) {
        if (c.action === 'new' || c.action === 'update') next.add(c.username);
      }
      selected = next;
      step = 2;
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  async function runImport() {
    if (!$can('manage', 'pppoe')) {
      toast.error($t('admin.network.pppoe.import.errors.permission') || $t('common.forbidden') || 'Forbidden');
      return;
    }
    if (!routerId) return;
    const usernames = Array.from(selected);
    if (usernames.length === 0) {
      toast.error($t('admin.network.pppoe.import.errors.select_one') || 'Select at least one item');
      return;
    }

    // Professional default: mapping optional. If one is set, require both.
    if ((customerId && !locationId) || (!customerId && locationId)) {
      toast.error(
        $t('admin.network.pppoe.import.errors.customer_location_pair') ||
          'Select both customer and location, or leave both empty.',
      );
      return;
    }

    loading = true;
    try {
      result = await api.pppoe.import.run(routerId, {
        usernames,
        customer_id: customerId || undefined,
        location_id: locationId || undefined,
      });
      step = 3;
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="page-content fade-in">
  <div class="head">
    <button class="back" type="button" onclick={() => goto(`${tenantPrefix}/admin/network/pppoe`)}>
      <Icon name="arrow-left" size={16} />
      {$t('common.back') || 'Back'}
    </button>

    <div class="title-wrap">
      <div class="kicker">
        <span class="dot"></span>
        {$t('sidebar.sections.network') || 'Network'}
      </div>
      <h1 class="title">{$t('admin.network.pppoe.import.title') || 'Import PPPoE from MikroTik'}</h1>
      <div class="sub">
        {$t('admin.network.pppoe.import.subtitle') ||
          'Scan /ppp/secret and import selected accounts into the database (per-router).'}
      </div>
    </div>

    <div class="steps">
      <div class="step" class:active={step === 1}>1. {$t('admin.network.pppoe.import.steps.select') || 'Select'}</div>
      <div class="step" class:active={step === 2}>2. {$t('admin.network.pppoe.import.steps.preview') || 'Preview'}</div>
      <div class="step" class:active={step === 3}>3. {$t('admin.network.pppoe.import.steps.import') || 'Import'}</div>
    </div>
  </div>

  {#if step === 1}
    <div class="card">
      <div class="grid2">
        <label class="field">
          <span class="label">{$t('admin.customers.pppoe.fields.router') || 'Router'}</span>
          <select class="select" bind:value={routerId} disabled={loading}>
            <option value="">{($t('common.select') || 'Select') + '...'}</option>
            {#each routers as r}
              <option value={r.id}>{r.name}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span class="label">{$t('admin.network.pppoe.import.fields.include_disabled') || 'Include disabled'}</span>
          <label class="check">
            <input type="checkbox" bind:checked={includeDisabled} />
            <span class="muted">
              {$t('admin.network.pppoe.import.fields.include_disabled_hint') ||
                'Also scan secrets that are disabled on the router.'}
            </span>
          </label>
        </label>
      </div>

      <div class="divider"></div>

      <div class="grid2">
        <label class="field">
          <span class="label">{$t('admin.network.pppoe.import.fields.customer_optional') || 'Assign to customer (optional)'}</span>
          <select
            class="select"
            bind:value={customerId}
            disabled={loading}
            onchange={() => void loadLocationsForCustomer(customerId)}
          >
            <option value="">{$t('admin.network.pppoe.import.fields.unassigned') || 'Unassigned (recommended for bulk import)'}</option>
            {#each customers as c}
              <option value={c.id}>{c.name}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span class="label">{$t('admin.network.pppoe.import.fields.location_optional') || 'Location (optional)'}</span>
          <select class="select" bind:value={locationId} disabled={loading || !customerId}>
            <option value="">{($t('common.select') || 'Select') + '...'}</option>
            {#each locations as l}
              <option value={l.id}>{l.label}</option>
            {/each}
          </select>
          {#if customerId}
            <div class="muted small">
              {$t('admin.network.pppoe.import.fields.location_hint') ||
                'If you pick a customer, you must also pick a location.'}
            </div>
          {/if}
        </label>
      </div>

      <div class="actions">
        <button class="btn ghost" type="button" onclick={resetPreview} disabled={loading}>
          <Icon name="eraser" size={16} />
          {$t('common.clear') || 'Clear'}
        </button>
        <button class="btn" type="button" onclick={scan} disabled={loading || !routerId}>
          <Icon name="search" size={16} />
          {$t('admin.network.pppoe.import.actions.scan') || 'Scan secrets'}
        </button>
      </div>
    </div>
  {:else if step === 2}
    <div class="card">
      <div class="preview-head">
          <div class="summary">
            <div class="pill">{candidates.length} {$t('admin.network.pppoe.import.summary.items') || 'items'}</div>
            <div class="pill ok">
              {candidates.filter((c) => c.action === 'new').length}
              {$t('admin.network.pppoe.import.labels.new') || 'New'}
            </div>
            <div class="pill warn">
              {candidates.filter((c) => c.action === 'update').length}
              {$t('admin.network.pppoe.import.labels.update') || 'Update'}
            </div>
            <div class="pill off">
              {candidates.filter((c) => c.action === 'same').length}
              {$t('admin.network.pppoe.import.labels.same') || 'Same'}
            </div>
          </div>
          <div class="preview-actions">
            <button class="btn ghost" type="button" onclick={() => toggleAll('new_update')} disabled={loading}>
              {$t('admin.network.pppoe.import.actions.select_new_update') || 'Select new+update'}
            </button>
            <button class="btn ghost" type="button" onclick={() => toggleAll('all')} disabled={loading}>
            {$t('common.select_all') || 'Select all'}
            </button>
            <button class="btn ghost" type="button" onclick={() => toggleAll('none')} disabled={loading}>
              {$t('common.clear') || 'Clear'}
            </button>
          <button class="btn ghost" type="button" onclick={scan} disabled={loading}>
            <Icon name="refresh-cw" size={16} />
            {$t('common.refresh') || 'Refresh'}
          </button>
          <button class="btn" type="button" onclick={runImport} disabled={loading || selected.size === 0}>
            <Icon name="download" size={16} />
            {$t('admin.network.pppoe.import.actions.import') || 'Import selected'}
            <span class="count">{selected.size}</span>
          </button>
        </div>
      </div>

      <div class="table-wrap">
        <Table
          columns={columns}
          data={tableData}
          keyField="id"
          pagination={true}
          pageSize={12}
          searchable={true}
          searchPlaceholder={$t('admin.network.pppoe.import.search') || 'Search username...'}
          mobileView="scroll"
        >
          {#snippet cell({ item, key }: any)}
            {#if key === 'pick'}
              <input
                type="checkbox"
                checked={selected.has(item.id)}
                onchange={(e) => {
                  const on = (e.currentTarget as HTMLInputElement).checked;
                  const next = new Set(selected);
                  if (on) next.add(item.id);
                  else next.delete(item.id);
                  selected = next;
                }}
              />
            {:else if key === 'disabled'}
              {#if item.disabled}
                <span class="pill off">{$t('admin.network.pppoe.import.labels.disabled') || 'Disabled'}</span>
              {:else}
                <span class="pill ok">{$t('admin.network.pppoe.import.labels.enabled') || 'Enabled'}</span>
              {/if}
            {:else if key === 'action'}
              {#if item.action === 'new'}
                <span class="pill ok">{$t('admin.network.pppoe.import.labels.new') || 'New'}</span>
              {:else if item.action === 'update'}
                <span class="pill warn">{$t('admin.network.pppoe.import.labels.update') || 'Update'}</span>
              {:else}
                <span class="pill">{$t('admin.network.pppoe.import.labels.same') || 'Same'}</span>
              {/if}
            {:else if key === 'pw'}
              {#if item.pw}
                <span class="pill ok">OK</span>
              {:else}
                <span class="pill warn">Missing</span>
              {/if}
            {:else}
              {item[key] ?? ''}
            {/if}
          {/snippet}
        </Table>
      </div>

      <div class="actions">
        <button class="btn ghost" type="button" onclick={() => (step = 1)} disabled={loading}>
          <Icon name="arrow-left" size={16} />
          {$t('common.back') || 'Back'}
        </button>
      </div>
    </div>
  {:else}
    <div class="card">
      <div class="result">
        <div class="result-title">
          <Icon name="check-circle" size={18} />
          {$t('admin.network.pppoe.import.done') || 'Import completed'}
        </div>
        <div class="result-grid">
          <div class="stat">
            <div class="k">{$t('admin.network.pppoe.import.result.created') || 'Created'}</div>
            <div class="v">{result?.created ?? 0}</div>
          </div>
          <div class="stat">
            <div class="k">{$t('admin.network.pppoe.import.result.updated') || 'Updated'}</div>
            <div class="v">{result?.updated ?? 0}</div>
          </div>
          <div class="stat">
            <div class="k">{$t('admin.network.pppoe.import.result.skipped') || 'Skipped'}</div>
            <div class="v">{result?.skipped ?? 0}</div>
          </div>
          <div class="stat">
            <div class="k">
              {$t('admin.network.pppoe.import.result.missing_password') || 'Missing password'}
            </div>
            <div class="v">{result?.missing_password ?? 0}</div>
          </div>
        </div>

        {#if result?.errors?.length}
          <div class="error-box">
            <div class="k">Errors</div>
            <ul class="err-list">
              {#each result.errors as e}
                <li><span class="mono">{e.username}</span>: {e.message}</li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>

      <div class="actions">
        <button class="btn ghost" type="button" onclick={resetPreview}>
          <Icon name="refresh-cw" size={16} />
          {$t('admin.network.pppoe.import.actions.import_more') || 'Import more'}
        </button>
        <button class="btn" type="button" onclick={() => goto(`${tenantPrefix}/admin/network/pppoe`)}>
          <Icon name="arrow-right" size={16} />
          {$t('admin.network.pppoe.import.actions.go_to_list') || 'Go to PPPoE list'}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .page-content {
    padding: 24px;
    max-width: 1280px;
    margin: 0 auto;
  }

  .head {
    display: grid;
    gap: 12px;
    margin-bottom: 14px;
  }

  .back {
    width: fit-content;
    display: inline-flex;
    align-items: center;
    gap: 10px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-card);
    color: var(--text-primary);
    padding: 10px 12px;
    cursor: pointer;
  }

  .back:hover {
    background: var(--bg-hover);
  }

  .title-wrap {
    display: grid;
    gap: 6px;
  }

  .kicker {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    color: var(--text-secondary);
    font-weight: 900;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-size: 0.75rem;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: rgba(99, 102, 241, 0.9);
    box-shadow: 0 0 0 6px rgba(99, 102, 241, 0.12);
  }

  .title {
    margin: 0;
    color: var(--text-primary);
    font-size: 1.65rem;
    font-weight: 1000;
    letter-spacing: 0.01em;
  }

  .sub {
    color: var(--text-secondary);
    font-weight: 650;
    max-width: 80ch;
  }

  .steps {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }

  .step {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 8px 12px;
    font-weight: 850;
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--bg-card), transparent 6%);
  }

  .step.active {
    border-color: rgba(99, 102, 241, 0.55);
    background: rgba(99, 102, 241, 0.14);
    color: var(--text-primary);
  }

  .card {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 18px;
    padding: 14px;
  }

  .grid2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .field {
    display: grid;
    gap: 6px;
  }

  .label {
    font-size: 12px;
    opacity: 0.8;
    font-weight: 800;
    color: var(--text-secondary);
  }

  .select {
    padding: 0.55rem 0.7rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    outline: none;
  }

  .muted {
    color: var(--text-secondary);
  }

  .muted.small {
    font-size: 12px;
  }

  .check {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 10px 12px;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-card), transparent 6%);
  }

  .divider {
    height: 1px;
    background: var(--border-color);
    margin: 12px 0;
    opacity: 0.7;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 12px;
    flex-wrap: wrap;
  }

  .table-wrap {
    margin-top: 12px;
    overflow: auto;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card), transparent 8%);
  }

  .preview-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .summary {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }

  .preview-actions {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }

  .count {
    margin-left: 8px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    height: 22px;
    padding: 0 6px;
    border-radius: 999px;
    background: rgba(99, 102, 241, 0.2);
    border: 1px solid rgba(99, 102, 241, 0.4);
    font-weight: 900;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    padding: 4px 8px;
    border-radius: 999px;
    font-weight: 900;
    font-size: 0.72rem;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-hover), transparent 15%);
    color: var(--text-secondary);
  }

  .pill.ok {
    border-color: rgba(34, 197, 94, 0.28);
    background: rgba(34, 197, 94, 0.12);
    color: rgba(34, 197, 94, 0.95);
  }

  .pill.warn {
    border-color: rgba(245, 158, 11, 0.35);
    background: rgba(245, 158, 11, 0.12);
    color: rgba(245, 158, 11, 0.95);
  }

  .pill.off {
    border-color: rgba(148, 163, 184, 0.28);
    background: rgba(148, 163, 184, 0.12);
    color: rgba(148, 163, 184, 0.95);
  }

  .result {
    display: grid;
    gap: 12px;
  }

  .result-title {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-weight: 950;
    color: var(--text-primary);
  }

  .result-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
  }

  .stat {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 12px;
    background: color-mix(in srgb, var(--bg-card), transparent 6%);
  }

  .stat .k {
    font-size: 12px;
    color: var(--text-secondary);
    font-weight: 850;
  }

  .stat .v {
    margin-top: 4px;
    font-size: 1.35rem;
    font-weight: 1000;
    color: var(--text-primary);
  }

  .error-box {
    border: 1px solid rgba(239, 68, 68, 0.35);
    background: rgba(239, 68, 68, 0.08);
    border-radius: 16px;
    padding: 12px;
  }

  .err-list {
    margin: 8px 0 0;
    padding-left: 18px;
    color: var(--text-secondary);
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
  }

  @media (max-width: 900px) {
    .grid2 {
      grid-template-columns: 1fr;
    }
    .result-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
