<script lang="ts">
  /*
    Impor MixRadius v2 — memuat wizard impor yang sama dengan versi lama
    (`(app)/admin/network/import/mixradius/+page.svelte`, 69 baris) secara lazy.
  */
  import { onMount } from 'svelte';
  import MixRadiusImportWizard from '$lib/components/network/mixradius/MixRadiusImportWizard.svelte';
  import { AppShell, Card, PageHeader, TableSkeleton } from '$lib/components/ds';

  let wizardLoading = $state(true);

  onMount(() => {
    // Wizard dimuat statis di sini (legacy memuatnya lazy lewat modul
    // colocated); penanda loading dilepas setelah mount pertama.
    wizardLoading = false;
  });
</script>

<AppShell title="Impor MixRadius">
  <PageHeader
    title="Impor MixRadius"
    eyebrow="Jaringan · Pusat impor"
    desc="Migrasi backup .sql/.sql.gz MixRadius ke paket, pelanggan, langganan, dan PPPoE."
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
