<script lang="ts">
  /*
    Impor MixRadius v2 — memuat wizard impor yang sama dengan versi lama
    (`(app)/admin/network/import/mixradius/+page.svelte`, 69 baris) secara lazy.
  */
  import { onMount } from 'svelte';
  import MixRadiusImportWizard from '$lib/components/network/mixradius/MixRadiusImportWizard.svelte';
  import { AppShell, Card, PageHeader, TableSkeleton } from '$lib/components/ds';
  import { t } from 'svelte-i18n';

  let wizardLoading = $state(true);

  onMount(() => {
    // Wizard dimuat statis di sini (legacy memuatnya lazy lewat modul
    // colocated); penanda loading dilepas setelah mount pertama.
    wizardLoading = false;
  });
</script>

<AppShell title={ $t('network.miximport.title') }>
  <PageHeader
    title={ $t('network.miximport.title') }
    eyebrow={ $t('admin.eyebrows.network_import') }
    desc={ $t('network.miximport.desc') }
  />

  <div class="mt-4">
    {#if wizardLoading}
      <Card>
        <div aria-busy="true">
          <TableSkeleton rows={6} cols={3} />
        </div>
      </Card>
    {:else}
      <MixRadiusImportWizard />
    {/if}
  </div>
</AppShell>
