<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';

  type RouterItem = {
    id: string;
    is_online: boolean;
  };

  type InterfaceItem = {
    name: string;
    interface_type?: string | null;
    running: boolean;
    disabled: boolean;
  };

  let {
    isEditing = false,
    currentIface = '',
    selectedRouterLabel = '—',
    routerSearch = $bindable(''),
    ifaceSearch = $bindable(''),
    selectedRouterId = $bindable<string | null>(null),
    routerList = [],
    interfaces = [],
    interfacesTotal = 0,
    interfacesLoading = false,
    routerTitle,
    onClose,
    onSelectRouter,
    onSelectInterface,
  }: {
    isEditing?: boolean;
    currentIface?: string;
    selectedRouterLabel?: string;
    routerSearch?: string;
    ifaceSearch?: string;
    selectedRouterId?: string | null;
    routerList?: RouterItem[];
    interfaces?: InterfaceItem[];
    interfacesTotal?: number;
    interfacesLoading?: boolean;
    routerTitle: (router: RouterItem) => string;
    onClose: () => void;
    onSelectRouter: (routerId: string) => void | Promise<void>;
    onSelectInterface: (iface: string) => void;
  } = $props();
</script>

<div class="picker-overlay" role="dialog" aria-modal="true">
  <button class="picker-backdrop" type="button" onclick={onClose} aria-label={$t('common.close') || 'Close'}></button>
  <div class="picker">
    <div class="picker-head">
      <h3>
        {isEditing
          ? ($t('admin.network.wallboard.edit_tile') || 'Edit interface tile')
          : ($t('admin.network.wallboard.pick_tile') || 'Add interface tile')}
      </h3>
      <button class="icon-x" type="button" onclick={onClose} title={$t('common.close') || 'Close'}>
        <Icon name="x" size={18} />
      </button>
    </div>
    <div class="picker-summary">
      <span class="picker-chip">
        <span class="k">Router</span>
        <span class="v mono">{selectedRouterLabel || '—'}</span>
      </span>
      <span class="picker-chip">
        <span class="k">Interface</span>
        <span class="v mono">{currentIface || '—'}</span>
      </span>
    </div>

    <div class="picker-body">
      <div class="picker-col">
        <div class="col-head">
          <span class="col-title">{$t('admin.network.wallboard.pick_router') || 'Router'}</span>
          <span class="muted">{routerList.length}</span>
        </div>
        <div class="pill small">
          <Icon name="search" size={16} />
          <input
            value={routerSearch}
            oninput={(e) => (routerSearch = (e.currentTarget as HTMLInputElement).value)}
            placeholder={$t('admin.network.wallboard.pick_search') || 'Search routers...'}
          />
        </div>
        <div class="picker-list">
          {#if routerList.length === 0}
            <div class="panel-empty muted">
              <Icon name="search" size={16} />
              {$t('admin.network.wallboard.empty') || 'No routers match your filters.'}
            </div>
          {:else}
            {#each routerList as r (r.id)}
              <button
                class="pick"
                class:active={selectedRouterId === r.id}
                type="button"
                onclick={() => void onSelectRouter(r.id)}
              >
                <span class="name">{routerTitle(r)}</span>
                <span class="spacer"></span>
                <span class="badge" class:ok={r.is_online} class:bad={!r.is_online}>
                  <span class="dot"></span>
                  {r.is_online
                    ? $t('admin.network.routers.badges.online') || 'Online'
                    : $t('admin.network.routers.badges.offline') || 'Offline'}
                </span>
              </button>
            {/each}
          {/if}
        </div>
      </div>

      <div class="picker-col">
        <div class="col-head">
          <span class="col-title">{$t('admin.network.wallboard.pick_interface') || 'Interface'}</span>
          {#if selectedRouterId}
            <span class="muted">{interfacesTotal}</span>
          {/if}
        </div>

        {#if !selectedRouterId}
          <div class="panel-empty muted">
            <Icon name="info" size={16} />
            {$t('admin.network.wallboard.pick_interface_hint') || 'Select a router first.'}
          </div>
        {:else}
          <div class="pill small">
            <Icon name="search" size={16} />
            <input
              value={ifaceSearch}
              oninput={(e) => (ifaceSearch = (e.currentTarget as HTMLInputElement).value)}
              placeholder={$t('admin.network.wallboard.pick_interface_search') || 'Search interfaces...'}
            />
          </div>

          {#if interfacesLoading}
            <div class="panel-empty muted">
              <Icon name="loader" size={16} />
              {$t('admin.network.wallboard.watch_loading') || 'Loading interfaces...'}
            </div>
          {:else}
            <div class="picker-list">
              {#if interfaces.length === 0}
                <div class="panel-empty muted">
                  <Icon name="search" size={16} />
                  {$t('admin.network.wallboard.watch_none') || 'No interfaces.'}
                </div>
              {:else}
                {#each interfaces as it (it.name)}
                  <button class="pick" type="button" onclick={() => onSelectInterface(it.name)}>
                    <span class="name mono">{it.name}</span>
                    <span class="muted">{it.interface_type || ''}</span>
                    <span class="spacer"></span>
                    {#if it.disabled}
                      <span class="tag">{$t('admin.network.wallboard.interface_state.disabled') || 'disabled'}</span>
                    {:else if it.running}
                      <span class="tag ok">{$t('admin.network.wallboard.interface_state.up') || 'up'}</span>
                    {:else}
                      <span class="tag">{$t('admin.network.wallboard.interface_state.down') || 'down'}</span>
                    {/if}
                  </button>
                {/each}
              {/if}
            </div>
          {/if}
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .picker-overlay {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: grid;
    place-items: center;
  }
  .picker-backdrop {
    position: absolute;
    inset: 0;
    border: none;
    background: rgba(0, 0, 0, 0.55);
  }
  .picker {
    position: relative;
    width: min(860px, calc(100vw - 24px));
    max-height: min(740px, calc(100vh - 24px));
    overflow: auto;
    border-radius: 18px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    box-shadow: var(--shadow-lg);
    padding: 14px;
  }
  .picker-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 10px;
  }
  .picker-head h3 {
    margin: 0;
    font-size: 20px;
    font-weight: 900;
  }
  .icon-x {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 55%, transparent);
    color: var(--text-primary);
    cursor: pointer;
  }
  .picker-summary {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-bottom: 10px;
  }
  .picker-chip {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 8px 10px;
    display: grid;
    gap: 2px;
    background: color-mix(in srgb, var(--bg-surface) 74%, transparent);
  }
  .picker-chip .k {
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-weight: 800;
    color: var(--text-muted);
  }
  .picker-chip .v {
    font-size: 12px;
    font-weight: 800;
    color: var(--text-primary);
  }
  .picker-body {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .picker-col {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 10px;
    background: color-mix(in srgb, var(--bg-surface) 80%, transparent);
    min-height: 380px;
    max-height: min(56vh, 520px);
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow: hidden;
  }
  .col-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    position: sticky;
    top: 0;
    z-index: 2;
    background: color-mix(in srgb, var(--bg-surface) 88%, transparent);
    padding-bottom: 6px;
  }
  .col-title {
    font-weight: 900;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
    min-width: 260px;
  }
  .pill.small {
    min-width: 0;
    padding: 8px 10px;
    border-radius: 12px;
  }
  .pill input {
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    width: 100%;
  }
  .picker-list {
    display: grid;
    gap: 8px;
    overflow: auto;
    padding-right: 2px;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px;
    border-radius: 16px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 70%, transparent);
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
  }
  .pick.active {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border-color));
    background: color-mix(in srgb, var(--accent) 14%, var(--bg-surface));
  }
  .pick:hover {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border-color));
  }
  .panel-empty {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px;
    border-radius: 14px;
    border: 1px dashed var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 45%, transparent);
  }
  .muted {
    color: var(--text-muted);
  }
  .spacer {
    flex: 1;
  }
  .name {
    font-weight: 800;
    font-size: 14px;
    line-height: 1.2;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    font-weight: 800;
    font-size: 12px;
    letter-spacing: 0.02em;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    box-shadow: none;
  }
  .badge.ok .dot {
    background: #2ecc71;
  }
  .badge.bad .dot {
    background: #ff6b6b;
  }
  .tag {
    padding: 3px 8px;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .tag.ok {
    border-color: color-mix(in srgb, #22c55e 45%, var(--border-color));
    color: #22c55e;
  }
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
  }

  @media (max-width: 920px) {
    .picker-summary {
      grid-template-columns: 1fr;
    }
    .picker-body {
      grid-template-columns: 1fr;
    }
    .picker-col {
      min-height: auto;
    }
  }
</style>
