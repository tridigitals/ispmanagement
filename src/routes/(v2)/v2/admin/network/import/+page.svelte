<script lang="ts">
  /*
    Pusat impor v2 — daftar sumber impor dari buildImportCenterSources, sama
    seperti versi lama (`(app)/admin/network/import/+page.svelte`, 267 baris).
    href sumber ditulis ulang ke rute v2.
  */
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { buildImportCenterSources } from '$lib/components/network/import-center/importCenterTypes';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import {
    AppShell,
    Badge,
    Button,
    Card,
    Icon,
    PageHeader,
    StatTile,
  } from '$lib/components/ds';
  import { t } from 'svelte-i18n';

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);
  const sources = $derived.by(() =>
    buildImportCenterSources(tenantPrefix).map((s) => ({
      ...s,
      // buildImportCenterSources mengembalikan href /admin/... — petakan ke /v2/admin/...
      href: s.href.replace('/admin/', '/v2/admin/'),
    })),
  );
</script>

<AppShell title={ $t('network.import_v2.title') }>
  <PageHeader
    title={ $t('network.import_v2.title') }
    eyebrow={ $t('admin.eyebrows.network') }
    desc={ $t('network.import_v2.desc') }
  />

  <div class="mt-4 grid gap-4">
    <Card title={ $t('network.import_v2.sec_summary') }>
      <div class="grid grid-cols-2 gap-6">
        <StatTile
          label={ $t('network.import_v2.src_count') }
          value={String(sources.length)}
          hint={ $t('network.import_v2.src_hint') }
        />
        <StatTile label={ $t('network.import_v2.stages') } value="4" hint={ $t('network.import_v2.stages_hint') } />
      </div>
    </Card>

    <Card title={ $t('network.import_v2.sources') }>
      <p class="mb-2 text-sm text-ink-500">{ $t('network.import_v2.pick_hint') }</p>
      <ul class="divide-y divide-ink-100">
        {#each sources as source (source.key)}
          <li class="flex items-center gap-4 py-3">
            <span class="grid size-11 shrink-0 place-items-center rounded-xl bg-ink-50 text-ink-500">
              <Icon name="download" size={20} />
            </span>
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <span class="font-medium text-ink-900">{source.title}</span>
                <Badge
                  label={source.status === 'ready' ? $t('network.import_v2.ready') : $t('network.import_v2.soon')}
                  tone={source.status === 'ready' ? 'positive' : 'neutral'}
                />
              </div>
              <p class="mt-0.5 truncate text-sm text-ink-500">{source.description}</p>
            </div>
            <Button
              variant="secondary"
              size="sm"
              onclick={() => goto(source.href)}
              disabled={source.status !== 'ready'}
            >
              Buka wizard
            </Button>
          </li>
        {/each}
      </ul>
    </Card>
  </div>
</AppShell>
