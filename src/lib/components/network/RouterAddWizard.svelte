<script lang="ts">
  import { t } from 'svelte-i18n';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Field from '$lib/components/ds/Field.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import { mikrotik } from '$lib/api/mikrotik';
  import { pppoe } from '$lib/api/pppoe';
  import { ispPackages } from '$lib/api/ispPackages';
  import {
    pppoeActionLabel,
    pppoeActionTone,
    pppoeDefaultSelection,
    pppoeSummary,
    type PppoeCandidate,
  } from '$lib/utils/pppoeImportInsights';

  let {
    show = $bindable(false),
    formName = $bindable(''),
    formHost = $bindable(''),
    formPort = $bindable(8728),
    formUsername = $bindable(''),
    formPassword = $bindable(''),
    formLatitude = $bindable(''),
    formLongitude = $bindable(''),
    formEnabled = $bindable(true),
    selectedUsernames = $bindable([] as string[]),
    selectedProfileNames = $bindable([] as string[]),
    poolByProfileOut = $bindable({} as Record<string, string>),
    onSubmit,
  }: {
    show?: boolean;
    formName?: string;
    formHost?: string;
    formPort?: number;
    formUsername?: string;
    formPassword?: string;
    formLatitude?: string;
    formLongitude?: string;
    formEnabled?: boolean;
    selectedUsernames?: string[];
    selectedProfileNames?: string[];
    poolByProfileOut?: Record<string, string>;
    onSubmit: () => void;
  } = $props();

  type Step = 1 | 2 | 3 | 4;
  let step = $state<Step>(1);
  let busy = $state(false);
  let error = $state('');

  // Step 1 — hasil test adhoc
  let testOk = $state(false);
  let testIdentity = $state<string | null>(null);
  let testVersion = $state<string | null>(null);

  // Step 3 — import
  let includeDisabled = $state(false);
  let candidates = $state<PppoeCandidate[]>([]);
  let selected = $state<Set<string>>(new Set());
  let previewDone = $state(false);
  let importTab = $state<'plans' | 'accounts'>('accounts');

  // Step 3 tab plan — profil & pool dari router
  type AdhocProfileRow = {
    name: string;
    rate_limit?: string | null;
    local_address?: string | null;
    remote_address?: string | null;
    dns_server?: string | null;
    default_profile: boolean;
  };
  type AdhocPoolRow = { name: string; ranges?: string | null; comment?: string | null };
  let profilesDone = $state(false);
  let profileRows = $state<AdhocProfileRow[]>([]);
  let poolRows = $state<AdhocPoolRow[]>([]);
  let selectedProfiles = $state<Set<string>>(new Set());
  let poolByProfile = $state<Record<string, string>>({});
  let existingPlanNames = $state<Set<string>>(new Set());

  const planNewCount = $derived(
    [...selectedProfiles].filter((n) => !existingPlanNames.has(n)).length,
  );
  const planExistingCount = $derived(
    [...selectedProfiles].filter((n) => existingPlanNames.has(n)).length,
  );

  const summary = $derived(pppoeSummary(candidates));

  function reset() {
    step = 1;
    busy = false;
    error = '';
    testOk = false;
    testIdentity = null;
    testVersion = null;
    candidates = [];
    selected = new Set();
    previewDone = false;
    includeDisabled = false;
    importTab = 'accounts';
    profilesDone = false;
    profileRows = [];
    poolRows = [];
    selectedProfiles = new Set();
    poolByProfile = {};
    existingPlanNames = new Set();
  }

  function close() {
    show = false;
    reset();
  }

  $effect(() => {
    if (!show) reset();
  });

  async function testConnection() {
    error = '';
    if (!formHost.trim() || !formUsername.trim()) {
      error = $t('admin.network.routers.wizard.err_required');
      return;
    }
    busy = true;
    try {
      const res = await mikrotik.routers.adhocTest({
        host: formHost.trim(),
        port: formPort || 8728,
        username: formUsername.trim(),
        password: formPassword,
      });
      testOk = !!res?.ok;
      testIdentity = res?.identity ?? null;
      testVersion = res?.ros_version ?? null;
      if (!testOk) {
        error = res?.error || $t('admin.network.routers.wizard.err_conn');
      }
    } catch (e: any) {
      testOk = false;
      error = e?.message ?? String(e);
    } finally {
      busy = false;
    }
  }

  function gotoStep2() {
    if (!testOk) return;
    error = '';
    step = 2;
  }

  function gotoStep3() {
    error = '';
    step = 3;
    if (!previewDone) void loadPreview();
    if (!profilesDone) void loadPlansPreview();
  }

  async function loadPlansPreview() {
    error = '';
    busy = true;
    try {
      const [res, pkgs] = await Promise.all([
        mikrotik.routers.adhocProfilesPools({
          host: formHost.trim(),
          port: formPort || 8728,
          username: formUsername.trim(),
          password: formPassword,
        }),
        ispPackages.packages.list({ per_page: 500 }),
      ]);
      profileRows = res?.profiles ?? [];
      poolRows = res?.pools ?? [];
      // Profilkandidat = non-default & belum ada plan dengan nama sama.
      const existing = new Set<string>(
        pkgs.data.map((p) => p.name.toLowerCase()),
      );
      existingPlanNames = existing;
      const actionable = profileRows.filter(
        (p) => !p.default_profile && !existing.has(p.name.toLowerCase()),
      );
      selectedProfiles = new Set(actionable.map((p) => p.name));
      // Default mapping pool: pakai remote-address profil bila cocok nama pool.
      const poolNames = new Set(poolRows.map((p) => p.name));
      const init: Record<string, string> = {};
      for (const p of actionable) {
        if (p.remote_address && poolNames.has(p.remote_address)) init[p.name] = p.remote_address;
      }
      poolByProfile = init;
      profilesDone = true;
    } catch (e: any) {
      error = e?.message ?? String(e);
      profileRows = [];
      poolRows = [];
      selectedProfiles = new Set();
      profilesDone = false;
    } finally {
      busy = false;
    }
  }

  function toggleProfile(name: string) {
    const next = new Set(selectedProfiles);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    selectedProfiles = next;
  }

  async function loadPreview() {
    error = '';
    busy = true;
    try {
      const rows = (await pppoe.import.previewAdhoc({
        host: formHost.trim(),
        port: formPort || 8728,
        username: formUsername.trim(),
        password: formPassword,
        include_disabled: includeDisabled,
      })) as PppoeCandidate[];
      candidates = rows || [];
      selected = new Set(pppoeDefaultSelection(candidates));
      previewDone = true;
    } catch (e: any) {
      error = e?.message ?? String(e);
      candidates = [];
      selected = new Set();
      previewDone = false;
    } finally {
      busy = false;
    }
  }

  function toggle(username: string) {
    const next = new Set(selected);
    if (next.has(username)) next.delete(username);
    else next.add(username);
    selected = next;
  }

  function toggleAll() {
    const actionable = candidates.filter(
      (c) => c.action === 'new' || c.action === 'update',
    );
    if (selected.size === actionable.length) selected = new Set();
    else selected = new Set(actionable.map((c) => c.username));
  }

  function finish() {
    // Kirim seleksi ke halaman; router disimpan via onSubmit, lalu halaman
    // membuat plan terpilih + mengimport akun PPPoE dengan router_id baru.
    selectedUsernames = [...selected];
    selectedProfileNames = [...selectedProfiles];
    poolByProfileOut = { ...poolByProfile };
    onSubmit();
    close();
  }
</script>

<Modal bind:show width="560px" onclose={() => (show = false)}>
  <div class="mb-4">
    <h2 class="text-lg font-semibold">
      {$t('admin.network.routers.wizard.title')}
    </h2>
    <ol class="mt-2 flex items-center gap-2 text-xs">
      {#each [1, 2, 3, 4] as s (s)}
        <li
          class="flex items-center gap-1.5 rounded-full px-2.5 py-1
            {s === step
              ? 'bg-primary text-primary-foreground'
              : s < step
                ? 'bg-muted text-muted-foreground line-through'
                : 'text-muted-foreground'}"
        >
          <span class="font-mono">{s}</span>
          {$t(`admin.network.routers.wizard.step${s}`)}
        </li>
      {/each}
    </ol>
  </div>

  {#if error}
    <div class="mb-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
      {error}
    </div>
  {/if}

  {#if step === 1}
    <div class="grid gap-3">
      <Field
        stacked
        id="rzw-name"
        label={$t('admin.network.routers.form.name')}
        value={formName}
        placeholder="POP Router 1"
        onchange={(v) => (formName = v)}
      />
      <div class="grid grid-cols-3 gap-3">
        <div class="col-span-2">
          <Field
            stacked
            id="rzw-host"
            label={$t('admin.network.routers.form.host')}
            value={formHost}
            placeholder="192.168.88.1"
            onchange={(v) => (formHost = v)}
          />
        </div>
        <Field
          stacked
          id="rzw-port"
          label={$t('admin.network.routers.form.port')}
          value={String(formPort)}
          type="number"
          min={1}
          max={65535}
          onchange={(v) => (formPort = Number(v) || 0)}
        />
      </div>
      <div class="grid grid-cols-2 gap-3">
        <Field
          stacked
          id="rzw-user"
          label={$t('admin.network.routers.form.username')}
          value={formUsername}
          placeholder="admin"
          onchange={(v) => (formUsername = v)}
        />
        <Field
          stacked
          id="rzw-pass"
          label={$t('admin.network.routers.form.password')}
          value={formPassword}
          type="password"
          onchange={(v) => (formPassword = v)}
        />
      </div>

      {#if testOk}
        <div class="flex items-center gap-2 rounded-md border border-emerald-500/40 bg-emerald-500/10 px-3 py-2 text-sm">
          <Badge tone="positive" label={$t('common.ok')} />
          <span>
            {testIdentity || formHost}
            {#if testVersion}
              <span class="text-muted-foreground">· RouterOS {testVersion}</span>
            {/if}
          </span>
        </div>
      {/if}

      <div class="flex items-center justify-between">
        <Button variant="secondary" type="button" onclick={testConnection} disabled={busy}>
          {busy
            ? $t('admin.network.routers.wizard.testing')
            : $t('admin.network.routers.wizard.test')}
        </Button>
        <Button variant="primary" type="button" onclick={gotoStep2} disabled={!testOk}>
          {$t('common.next')}
        </Button>
      </div>
    </div>
  {:else if step === 2}
    <div class="grid gap-3">
      <div class="grid grid-cols-2 gap-3">
        <Field
          stacked
          id="rzw-lat"
          label={$t('network.map.latitude')}
          value={formLatitude}
          placeholder="-6.200000"
          onchange={(v) => (formLatitude = v)}
        />
        <Field
          stacked
          id="rzw-lng"
          label={$t('network.map.longitude')}
          value={formLongitude}
          placeholder="106.816666"
          onchange={(v) => (formLongitude = v)}
        />
      </div>
      <p class="text-xs text-muted-foreground">
        {$t('admin.network.routers.wizard.location_hint')}
      </p>
      <div class="flex items-center justify-between">
        <Button variant="ghost" type="button" onclick={() => (step = 1)}>
          {$t('common.back')}
        </Button>
        <Button variant="primary" type="button" onclick={gotoStep3}>
          {$t('common.next')}
        </Button>
      </div>
    </div>
  {:else if step === 3}
    <div class="grid gap-3">
      <div class="flex gap-1 rounded-md border p-1 text-sm">
        <button
          type="button"
          class="flex-1 rounded px-3 py-1.5
            {importTab === 'plans' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground'}"
          onclick={() => (importTab = 'plans')}
        >
          {$t('admin.network.routers.wizard.tab_plans')}
          {#if profilesDone && selectedProfiles.size > 0}
            <span class="ml-1 font-mono text-xs">{selectedProfiles.size}</span>
          {/if}
        </button>
        <button
          type="button"
          class="flex-1 rounded px-3 py-1.5
            {importTab === 'accounts' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground'}"
          onclick={() => (importTab = 'accounts')}
        >
          {$t('admin.network.routers.wizard.tab_accounts')}
          {#if previewDone && selected.size > 0}
            <span class="ml-1 font-mono text-xs">{selected.size}</span>
          {/if}
        </button>
      </div>

      {#if importTab === 'plans'}
        {#if !profilesDone && busy}
          <p class="text-sm text-muted-foreground">
            {$t('admin.network.routers.wizard.loading')}
          </p>
        {:else if !profilesDone}
          <p class="text-sm text-muted-foreground">
            {$t('admin.network.routers.wizard.no_profiles')}
          </p>
        {:else if profileRows.length === 0}
          <p class="text-sm text-muted-foreground">
            {$t('admin.network.routers.wizard.no_profiles')}
          </p>
        {:else}
          <p class="text-xs text-muted-foreground">
            {$t('admin.network.routers.wizard.plans_hint')}
          </p>
          <div class="flex items-center gap-2 text-sm">
            <Badge tone="positive" label={`${planNewCount}`} />
            {#if planExistingCount > 0}
              <Badge tone="neutral" label={`↻${planExistingCount}`} />
            {/if}
            <span class="text-xs text-muted-foreground">
              {poolRows.length}
              {$t('admin.network.routers.wizard.pools_found')}
            </span>
          </div>
          <div class="max-h-56 overflow-y-auto rounded-md border">
            <table class="w-full text-sm">
              <thead class="sticky top-0 bg-muted text-left text-xs uppercase text-muted-foreground">
                <tr>
                  <th class="w-8 px-2 py-1.5"></th>
                  <th class="px-2 py-1.5">Profile</th>
                  <th class="px-2 py-1.5">Rate</th>
                  <th class="px-2 py-1.5">Pool</th>
                  <th class="px-2 py-1.5">Status</th>
                </tr>
              </thead>
              <tbody>
                {#each profileRows as p (p.name)}
                  {@const planExists = existingPlanNames.has(p.name.toLowerCase())}
                  {@const actionable = !p.default_profile}
                  <tr class="border-t" class:opacity-50={p.default_profile}>
                    <td class="px-2 py-1.5">
                      {#if actionable}
                        <input
                          type="checkbox"
                          checked={selectedProfiles.has(p.name)}
                          onchange={() => toggleProfile(p.name)}
                        />
                      {/if}
                    </td>
                    <td class="px-2 py-1.5 font-mono">{p.name}</td>
                    <td class="px-2 py-1.5 text-xs">{p.rate_limit || '—'}</td>
                    <td class="px-2 py-1.5">
                      {#if selectedProfiles.has(p.name)}
                        <select
                          class="rounded border bg-transparent px-1.5 py-0.5 text-xs"
                          value={poolByProfile[p.name] ?? ''}
                          onchange={(e) => {
                            poolByProfile = {
                              ...poolByProfile,
                              [p.name]: (e.currentTarget as HTMLSelectElement).value,
                            };
                          }}
                        >
                          <option value="">—</option>
                          {#each poolRows as pool (pool.name)}
                            <option value={pool.name}>{pool.name}</option>
                          {/each}
                        </select>
                      {:else}
                        <span class="text-xs text-muted-foreground">—</span>
                      {/if}
                    </td>
                    <td class="px-2 py-1.5">
                      {#if p.default_profile}
                        <Badge tone="neutral" label="default" />
                      {:else if planExists}
                        <Badge tone="neutral" label={$t('admin.network.routers.wizard.plan_exists')} />
                      {:else}
                        <Badge tone="positive" label={$t('admin.network.routers.wizard.plan_new')} />
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      {:else}
        <label class="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            bind:checked={includeDisabled}
            onchange={() => (previewDone = false)}
          />
          {$t('admin.network.routers.wizard.include_disabled')}
        </label>

        {#if !previewDone && busy}
          <p class="text-sm text-muted-foreground">
            {$t('admin.network.routers.wizard.loading')}
          </p>
        {:else if !previewDone}
          <p class="text-sm text-muted-foreground">
            {$t('admin.network.routers.wizard.no_secrets')}
          </p>
        {:else if candidates.length === 0}
          <p class="text-sm text-muted-foreground">
            {$t('admin.network.routers.wizard.no_secrets')}
          </p>
        {:else}
          <div class="flex items-center gap-2 text-sm">
            <Badge tone="positive" label={`+${summary.fresh}`} />
            <Badge tone="warning" label={`~${summary.updates}`} />
            <Badge tone="neutral" label={`=${summary.same}`} />
            <button class="ml-auto text-xs underline" type="button" onclick={toggleAll}>
              {selected.size === summary.fresh + summary.updates
                ? $t('admin.network.routers.wizard.select_none')
                : $t('admin.network.routers.wizard.select_all')}
            </button>
          </div>
          <div class="max-h-56 overflow-y-auto rounded-md border">
            <table class="w-full text-sm">
              <thead class="sticky top-0 bg-muted text-left text-xs uppercase text-muted-foreground">
                <tr>
                  <th class="w-8 px-2 py-1.5"></th>
                  <th class="px-2 py-1.5">Username</th>
                  <th class="px-2 py-1.5">Profile</th>
                  <th class="px-2 py-1.5">Remote</th>
                  <th class="px-2 py-1.5">Status</th>
                </tr>
              </thead>
              <tbody>
                {#each candidates as c (c.username)}
                  <tr class="border-t">
                    <td class="px-2 py-1.5">
                      {#if c.action !== 'same'}
                        <input
                          type="checkbox"
                          checked={selected.has(c.username)}
                          onchange={() => toggle(c.username)}
                        />
                      {/if}
                    </td>
                    <td class="px-2 py-1.5 font-mono">{c.username}</td>
                    <td class="px-2 py-1.5">{c.profile_name || '—'}</td>
                    <td class="px-2 py-1.5 font-mono text-xs">{c.remote_address || '—'}</td>
                    <td class="px-2 py-1.5">
                      <Badge tone={pppoeActionTone(c.action)} label={pppoeActionLabel(c.action, $t)} />
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      {/if}

      <div class="flex items-center justify-between">
        <Button variant="ghost" type="button" onclick={() => (step = 2)}>
          {$t('common.back')}
        </Button>
        <div class="flex items-center gap-2">
          {#if importTab === 'accounts' && !previewDone}
            <Button variant="secondary" type="button" onclick={() => void loadPreview()} disabled={busy}>
              {$t('admin.network.routers.wizard.load_preview')}
            </Button>
          {:else if importTab === 'plans' && !profilesDone}
            <Button variant="secondary" type="button" onclick={() => void loadPlansPreview()} disabled={busy}>
              {$t('admin.network.routers.wizard.load_preview')}
            </Button>
          {/if}
          <Button variant="primary" type="button" onclick={() => (step = 4)}>
            {$t('common.next')}
          </Button>
        </div>
      </div>
    </div>
  {:else}
    <div class="grid gap-3">
      <div class="rounded-md border px-3 py-2 text-sm">
        <div class="flex justify-between py-0.5">
          <span class="text-muted-foreground">{$t('admin.network.routers.wizard.summary_router')}</span>
          <span class="font-medium">{formName || formHost}</span>
        </div>
        <div class="flex justify-between py-0.5">
          <span class="text-muted-foreground">{$t('admin.network.routers.wizard.summary_host')}</span>
          <span class="font-mono">{formHost}:{formPort}</span>
        </div>
        <div class="flex justify-between py-0.5">
          <span class="text-muted-foreground">{$t('admin.network.routers.wizard.summary_identity')}</span>
          <span>{testIdentity || '—'}</span>
        </div>
        <div class="flex justify-between py-0.5">
          <span class="text-muted-foreground">{$t('admin.network.routers.wizard.summary_plans')}</span>
          <span class="font-medium">{selectedProfiles.size}</span>
        </div>
        <div class="flex justify-between py-0.5">
          <span class="text-muted-foreground">{$t('admin.network.routers.wizard.summary_import')}</span>
          <span class="font-medium">{selected.size}</span>
        </div>
        <div class="flex justify-between py-0.5">
          <span class="text-muted-foreground">{$t('admin.network.routers.wizard.summary_status')}</span>
          <span>{formEnabled ? $t('common.enabled') : $t('common.disabled')}</span>
        </div>
      </div>
      <p class="text-xs text-muted-foreground">
        {$t('admin.network.routers.wizard.import_note')}
      </p>
      <div class="flex items-center justify-between">
        <Button variant="ghost" type="button" onclick={() => (step = 3)}>
          {$t('common.back')}
        </Button>
        <Button variant="primary" type="button" onclick={finish} disabled={busy}>
          {$t('admin.network.routers.wizard.finish')}
        </Button>
      </div>
    </div>
  {/if}
</Modal>
