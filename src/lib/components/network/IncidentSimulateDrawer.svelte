<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';

  type RouterOption = {
    id: string;
    name: string;
    identity?: string | null;
  };

  let {
    open = false,
    busy = false,
    routers = [],
    routerId = '',
    incidentType = 'offline',
    severity = 'warning',
    interfaceName = '',
    message = '',
    onClose,
    onSubmit,
    onRouterChange,
    onTypeChange,
    onSeverityChange,
    onInterfaceChange,
    onMessageChange,
  }: {
    open?: boolean;
    busy?: boolean;
    routers?: RouterOption[];
    routerId?: string;
    incidentType?: string;
    severity?: string;
    interfaceName?: string;
    message?: string;
    onClose: () => void;
    onSubmit: () => void | Promise<void>;
    onRouterChange: (value: string) => void;
    onTypeChange: (value: string) => void;
    onSeverityChange: (value: string) => void;
    onInterfaceChange: (value: string) => void;
    onMessageChange: (value: string) => void;
  } = $props();
</script>

{#if open}
  <button class="drawer-backdrop" type="button" onclick={onClose} aria-label={$t('common.close') || 'Close'}></button>
  <aside class="drawer simulate-drawer" aria-label={$t('admin.network.incidents.actions.simulate') || 'Simulate'}>
    <div class="drawer-head">
      <div>
        <div class="drawer-title">{$t('admin.network.incidents.actions.simulate') || 'Simulate'}</div>
        <div class="drawer-sub">{$t('admin.network.incidents.simulate.subtitle') || 'Create test incident manually'}</div>
      </div>
      <button class="icon-btn" type="button" onclick={onClose} disabled={busy}>
        <Icon name="x" size={16} />
      </button>
    </div>
    <div class="drawer-body">
      <div class="field">
        <label for="sim-router">{$t('admin.network.incidents.simulate.router') || 'Router'}</label>
        <select
          id="sim-router"
          class="input"
          value={routerId}
          onchange={(e) => onRouterChange((e.currentTarget as HTMLSelectElement).value)}
          disabled={busy}
        >
          {#each routers as router}
            <option value={router.id}>{router.identity || router.name}</option>
          {/each}
        </select>
      </div>
      <div class="field">
        <label for="sim-type">{$t('admin.network.incidents.simulate.type') || 'Incident type'}</label>
        <select
          id="sim-type"
          class="input"
          value={incidentType}
          onchange={(e) => onTypeChange((e.currentTarget as HTMLSelectElement).value)}
          disabled={busy}
        >
          <option value="offline">offline</option>
          <option value="cpu">cpu</option>
          <option value="latency">latency</option>
          <option value="interface_down">interface_down</option>
        </select>
      </div>
      <div class="field">
        <label for="sim-sev">{$t('admin.network.incidents.simulate.severity') || 'Severity'}</label>
        <select
          id="sim-sev"
          class="input"
          value={severity}
          onchange={(e) => onSeverityChange((e.currentTarget as HTMLSelectElement).value)}
          disabled={busy}
        >
          <option value="info">info</option>
          <option value="warning">warning</option>
          <option value="critical">critical</option>
        </select>
      </div>
      <div class="field">
        <label for="sim-iface">{$t('admin.network.incidents.simulate.interface') || 'Interface (optional)'}</label>
        <input
          id="sim-iface"
          class="input"
          type="text"
          value={interfaceName}
          oninput={(e) => onInterfaceChange((e.currentTarget as HTMLInputElement).value)}
          placeholder="ether1"
          disabled={busy}
        />
      </div>
      <div class="field">
        <label for="sim-msg">{$t('admin.network.incidents.simulate.message') || 'Message (optional)'}</label>
        <textarea
          id="sim-msg"
          class="textarea"
          rows="4"
          value={message}
          oninput={(e) => onMessageChange((e.currentTarget as HTMLTextAreaElement).value)}
          placeholder={$t('admin.network.incidents.simulate.message_placeholder') || 'Optional simulation message'}
          disabled={busy}
        ></textarea>
      </div>
    </div>
    <div class="drawer-actions">
      <button class="btn ghost" type="button" onclick={onClose} disabled={busy}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn ghost" type="button" onclick={() => void onSubmit()} disabled={busy || !routerId}>
        <Icon name="activity" size={16} />
        {busy
          ? $t('common.saving') || 'Saving...'
          : $t('admin.network.incidents.actions.simulate') || 'Simulate'}
      </button>
    </div>
  </aside>
{/if}

<style>
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    border: 0;
    background: rgba(0, 0, 0, 0.45);
    z-index: 50;
  }
  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    width: min(560px, 92vw);
    height: 100vh;
    background: var(--bg-surface);
    border-left: 1px solid var(--border-color);
    z-index: 51;
    display: grid;
    grid-template-rows: auto 1fr auto;
  }
  .simulate-drawer {
    width: min(500px, 92vw);
  }
  .drawer-head {
    padding: 16px;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  .drawer-title {
    font-size: 0.78rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }
  .drawer-sub {
    margin-top: 6px;
    font-size: 1.05rem;
    font-weight: 900;
    color: var(--text-primary);
  }
  .drawer-body {
    padding: 16px;
    display: grid;
    gap: 14px;
    overflow: auto;
  }
  .field {
    display: grid;
    gap: 6px;
  }
  .field label {
    color: var(--text-secondary);
    font-size: 0.8rem;
    font-weight: 700;
  }
  .input,
  .textarea {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 10px 12px;
    outline: none;
  }
  .input:focus,
  .textarea:focus {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border-color));
  }
  .drawer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
    padding: 12px 16px;
    border-top: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card) 80%, transparent);
  }
  .icon-btn {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .icon-btn:hover {
    background: var(--bg-hover);
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    font-weight: 800;
    cursor: pointer;
  }
  .btn:hover {
    background: var(--bg-hover);
  }
</style>
