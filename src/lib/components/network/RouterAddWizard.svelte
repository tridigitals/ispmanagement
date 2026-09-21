<script lang="ts">
  import { t } from 'svelte-i18n';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Field from '$lib/components/ds/Field.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import { mikrotik } from '$lib/api/mikrotik';
  import { pppoe } from '$lib/api/pppoe';
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
    // mengimport akun PPPoE terpilih dengan router_id yang baru dibuat.
    selectedUsernames = [...selected];
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

      <div class="flex items-center justify-between">
        <Button variant="ghost" type="button" onclick={() => (step = 2)}>
          {$t('common.back')}
        </Button>
        <div class="flex items-center gap-2">
          {#if !previewDone}
            <Button variant="secondary" type="button" onclick={() => void loadPreview()} disabled={busy}>
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
