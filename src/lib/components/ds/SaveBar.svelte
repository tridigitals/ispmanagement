<!--
  Ds/SaveBar — bilah simpan yang menempel di bawah viewport.

  Ini menjawab bug struktural di halaman pengaturan lama, bukan sekadar
  memindahkan tombol. Di sana:
    - `hasChanges` (baris 575) dihitung dari SELURUH kategori, tapi
      `saveChanges()` (baris 738) hanya mengirim `categories[activeTab].keys`.
    - Jalur mobile mempertahankan edit lintas tab (baris 1600 tanpa `discard`),
      jalur desktop membuangnya (baris 911 `discard: true`).
    Hasilnya, di mobile tombol Simpan menyala karena ada edit di tab lain, lalu
    menekannya menyimpan tab yang sedang dibuka dan `loadSettings()` menimpa
    sisanya. Edit hilang tanpa pesan.

  Komponen ini menutup celah itu dengan menampilkan APA yang akan disimpan:
  jumlah perubahan dan nama field-nya, bukan tombol yang cuma menyala. Kalau
  perubahan ada di bagian lain, pengguna melihatnya sebelum menekan Simpan.
-->
<script lang="ts">
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';

  interface Props {
    /** Label field yang berubah. Panjangnya = jumlah perubahan. */
    changes: string[];
    saving?: boolean;
    /** Nama bagian tempat perubahan berada, kalau berbeda dari yang dibuka. */
    elsewhere?: string[];
    onsave: () => void;
    onreset: () => void;
  }

  let { changes, saving = false, elsewhere = [], onsave, onreset }: Props = $props();

  const count = $derived(changes.length);
  /* Tiga nama pertama saja; sisanya diringkas. Menyebut semua nama di bilah
     sempit membuatnya tidak terbaca. */
  const preview = $derived(
    changes.slice(0, 3).join(', ') + (count > 3 ? ` +${count - 3} lainnya` : ''),
  );
</script>

{#if count > 0}
  <div
    class="sticky bottom-0 z-20 -mx-5 mt-6 border-t border-ink-200 bg-white/95 px-5 py-3 backdrop-blur lg:-mx-7 lg:px-7"
    role="region"
    aria-label="Perubahan belum disimpan"
  >
    <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
      <div class="flex min-w-0 flex-1 items-center gap-2.5">
        <span class="size-2 shrink-0 rounded-full bg-amber-500"></span>
        <div class="min-w-0">
          <div class="text-base font-medium text-ink-900">
            {count} perubahan belum disimpan
          </div>
          <div class="truncate text-sm text-ink-500">{preview}</div>
        </div>
      </div>

      <div class="flex shrink-0 items-center gap-2">
        <Button variant="ghost" onclick={onreset} disabled={saving}>Batalkan</Button>
        <Button variant="primary" icon="check" onclick={onsave} loading={saving}>Simpan</Button>
      </div>
    </div>

    {#if elsewhere.length > 0}
      <!-- Inilah yang tidak pernah ditampilkan halaman lama: perubahan yang
           tersimpan di state tapi berada di bagian yang tidak terlihat. -->
      <div
        class="mt-2.5 flex items-start gap-2 rounded-lg bg-amber-50/70 px-3 py-2 ring-1 ring-inset ring-amber-200"
      >
        <Icon name="alert" size={14} class="mt-0.5 shrink-0 text-amber-700" />
        <p class="text-sm text-amber-900">
          Termasuk perubahan di bagian {elsewhere.join(', ')}. Semuanya ikut tersimpan.
        </p>
      </div>
    {/if}
  </div>
{/if}
