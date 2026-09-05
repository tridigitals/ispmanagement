<script lang="ts">
  /*
    Cadangan v2.

    Versi lama (`(app)/admin/backups/+page.svelte`, 107 baris) hanya menampilkan
    status "fitur cadangan dinonaktifkan" — tidak ada API yang dipanggil. v2
    mempertahankan perilaku itu dengan komponen DS.
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { can } from '$lib/stores/auth';
  import { AppShell, Card, Icon, PageHeader } from '$lib/components/ds';

  onMount(() => {
    if (
      !$can('read', 'backups') &&
      !$can('create', 'backups') &&
      !$can('download', 'backups') &&
      !$can('restore', 'backups') &&
      !$can('delete', 'backups')
    ) {
      goto('/unauthorized');
    }
  });
</script>

<AppShell title="Cadangan">
  <PageHeader
    title="Cadangan"
    eyebrow="Sistem"
    desc="Salinan cadangan basis data dan berkas. Fitur ini sedang dinonaktifkan di tenant ini."
  />

  <div class="mt-4">
    <Card>
      <div class="flex items-start gap-3">
        <span class="grid size-11 shrink-0 place-items-center rounded-xl bg-ink-50 text-ink-500">
          <Icon name="shield" size={22} />
        </span>
        <div class="min-w-0">
          <h2 class="text-base font-semibold text-ink-900">Cadangan dinonaktifkan</h2>
          <p class="mt-1 text-sm text-ink-500">
            Layanan cadangan belum diaktifkan untuk tenant ini. Hubungi administrator
            sistem untuk mengaktifkannya kembali.
          </p>
        </div>
      </div>
    </Card>
  </div>
</AppShell>
