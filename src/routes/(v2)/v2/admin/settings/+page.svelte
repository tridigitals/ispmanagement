<script lang="ts">
  /*
    Pengaturan v2.

    Versi lama: `(app)/admin/settings/+page.svelte` 2.098 baris (900 script,
    492 CSS scoped) + 4 komponen tab (SettingsEmailTab 746, SettingsPaymentTab
    859, SettingsServiceTab 1.158, SettingsCompanyTab 535).

    TIGA BUG STRUKTURAL yang diperbaiki di sini, semuanya terukur lebih dulu:

    1. SIMPAN MEMBUANG PERUBAHAN DI BAGIAN LAIN.
       Baris 738 versi lama: `keysToSave = categories[activeTab].keys` — hanya
       key tab yang sedang dibuka yang dikirim. Tapi `hasChanges` (baris 575)
       dihitung dari SELURUH kategori, dan jalur mobile mempertahankan edit
       lintas tab (baris 1600, tanpa `discard`) sementara jalur desktop
       membuangnya (baris 911, `discard: true`).
       Terkonfirmasi lewat probe di viewport 390px: edit "ISP Management" di tab
       Umum -> pindah ke tab Jaringan lewat FAB -> tombol Simpan MASIH menyala
       -> menekannya menyimpan key Jaringan lalu `loadSettings()` menimpa state,
       sehingga edit Umum hilang tanpa pesan apa pun.
       Di sini: `save()` mengirim SEMUA key yang berubah dari SEMUA bagian, dan
       SaveBar menyebut bagian mana saja yang ikut tersimpan.

    2. NILAI DEFAULT TERSEBAR SEBAGAI 27 CABANG IF.
       `buildLocalSettingsFromData()` versi lama berisi 27 baris
       `if (key === '...' && !val) val = '...'`. Key baru yang lupa didaftarkan
       di sana tampil kosong dan tersimpan sebagai string kosong.
       Di sini default hidup di `settingsSchema.ts` bersama definisi field-nya.

    3. TIDAK ADA VALIDASI LINTAS FIELD.
       SLA "terlampaui" bisa disimpan lebih kecil dari SLA "peringatan"; versi
       lama hanya menutupinya di pratinjau (`slaBreachPreview` mengalikan warn
       x 2) sehingga yang tersimpan tetap nilai tidak masuk akal. Hal sama untuk
       ambang CPU dan latensi. Sekarang `validate()` menahan simpan dan
       menunjukkan galat di field yang salah.

    Yang TIDAK dipindahkan ke skema: merek/domain, tagihan & paket, email,
    pembayaran, layanan, WhatsApp, notifikasi event. Ketujuhnya bukan daftar key
    sederhana melainkan panel dengan alur sendiri (upload logo, status domain,
    uji SMTP, matriks event x kanal). Memaksakannya ke skema hanya memindahkan
    kerumitan. Bagian itu masih dilayani halaman lama dan ditandai jelas.
  */
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import type { Setting } from '$lib/api/client';
  import {
    AppShell,
    Button,
    Card,
    Field,
    PageHeader,
    SaveBar,
    Tabs,
    AttentionPanel,
    Icon,
  } from '$lib/components/ds';
  import {
    PANEL_SECTIONS,
    SETTING_SECTIONS,
    initialValue,
    isVisible,
    schemaKeys,
    validate,
  } from '$lib/utils/settingsSchema';

  type Values = Record<string, string>;

  let loading = $state(true);
  let saving = $state(false);
  let active = $state(SETTING_SECTIONS[0].id);

  /** Nilai dari server, dipakai sebagai pembanding untuk menghitung perubahan. */
  let baseline = $state<Values>({});
  let values = $state<Values>({});

  const errors = $derived(validate(values));
  const sections = $derived(SETTING_SECTIONS);
  const activeSection = $derived(sections.find((s) => s.id === active) ?? sections[0]);

  /* Semua key yang berbeda dari baseline, LINTAS bagian — bukan hanya bagian
     yang sedang dibuka. Inilah perbedaan inti dari versi lama. */
  const changedKeys = $derived(schemaKeys().filter((k) => (values[k] ?? '') !== (baseline[k] ?? '')));

  const changedLabels = $derived(
    changedKeys.map((k) => {
      for (const s of sections) {
        const f = s.fields.find((x) => x.key === k);
        if (f) return f.label;
      }
      return k;
    }),
  );

  /* Bagian yang punya perubahan tapi tidak sedang dibuka. Versi lama tidak
     pernah menampilkan ini, padahal justru di situ datanya hilang. */
  const changedElsewhere = $derived(
    sections
      .filter((s) => s.id !== active && s.fields.some((f) => changedKeys.includes(f.key)))
      .map((s) => s.label),
  );

  const blockingErrors = $derived(
    Object.entries(errors).filter(([k]) => changedKeys.includes(k)),
  );

  const tabItems = $derived([
    ...sections.map((s) => {
      const n = s.fields.filter((f) => changedKeys.includes(f.key)).length;
      return { id: s.id, label: s.label, count: n > 0 ? n : null };
    }),
  ]);

  const visibleFields = $derived(activeSection.fields.filter((f) => isVisible(f, values)));

  function applyServerData(rows: Setting[]) {
    const byKey = new Map(rows.map((r) => [r.key, r.value ?? '']));
    const next: Values = {};

    for (const s of SETTING_SECTIONS) {
      for (const f of s.fields) {
        next[f.key] = initialValue(f, byKey.get(f.key));
      }
    }

    /* baseline = apa yang ADA DI SERVER, bukan hasil fallback. Kalau fallback
       ikut masuk baseline, field yang belum pernah disimpan akan terlihat
       "tidak berubah" padahal DB-nya kosong — dan tombol Simpan tidak akan
       pernah menyala untuk menyimpannya. */
    const base: Values = {};
    for (const s of SETTING_SECTIONS) {
      for (const f of s.fields) base[f.key] = (byKey.get(f.key) ?? '').trim();
    }

    values = next;
    baseline = base;
  }

  onMount(async () => {
    try {
      const rows = await api.settings.getAll();
      applyServerData(rows);
    } catch (error: any) {
      toast.error(error?.message || 'Gagal memuat pengaturan');
    } finally {
      loading = false;
    }
  });

  function set(key: string, value: string) {
    values = { ...values, [key]: value };
  }

  async function save() {
    if (blockingErrors.length > 0) {
      toast.error(`${blockingErrors.length} field masih perlu diperbaiki`);
      return;
    }

    saving = true;
    try {
      /* SEMUA key yang berubah, dari SEMUA bagian. */
      const results = await Promise.allSettled(
        changedKeys.map((k) => api.settings.upsert(k, values[k] ?? '')),
      );

      const failed = results.filter((r) => r.status === 'rejected').length;

      /* Baca ulang dari server, jangan asumsikan simpan berhasil. Kalau ada
         yang gagal, baseline baru akan menunjukkan key mana yang masih
         berbeda, jadi SaveBar tetap menyala untuk key itu saja. */
      const rows = await api.settings.getAll();
      applyServerData(rows);

      if (failed > 0) {
        toast.error(`${failed} dari ${results.length} pengaturan gagal disimpan`);
      } else {
        toast.success(`${results.length} pengaturan disimpan`);
      }
    } catch (error: any) {
      toast.error(error?.message || 'Gagal menyimpan pengaturan');
    } finally {
      saving = false;
    }
  }

  function reset() {
    const next: Values = { ...values };
    for (const s of SETTING_SECTIONS) {
      for (const f of s.fields) next[f.key] = initialValue(f, baseline[f.key]);
    }
    values = next;
  }
</script>

<AppShell title="Pengaturan">
  <PageHeader
    title="Pengaturan"
    eyebrow="Konfigurasi tenant"
    desc="Perubahan berlaku untuk seluruh pengguna di tenant ini."
  />

  {#if loading}
    <Card>
      <div class="space-y-4 py-2">
        {#each Array(6) as _}
          <div class="grid gap-2 sm:grid-cols-[15rem_1fr] sm:gap-6">
            <div class="skeleton h-4 w-40 rounded"></div>
            <div class="skeleton h-9 w-full max-w-md rounded-lg"></div>
          </div>
        {/each}
      </div>
    </Card>
  {:else}
    <Tabs items={tabItems} active={active} panelId="settings-panel" onselect={(id) => (active = id)} />

    <div id="settings-panel" role="tabpanel">
      <Card>
        <div class="mb-4 border-b border-ink-100 pb-4">
          <h2 class="text-base font-semibold text-ink-900">{activeSection.label}</h2>
          <p class="mt-0.5 text-sm text-ink-500">{activeSection.desc}</p>
        </div>

        <div class="divide-y divide-ink-100">
          {#each visibleFields as f (f.key)}
            <Field
              id={`set-${f.key}`}
              label={f.label}
              type={f.type}
              value={values[f.key] ?? ''}
              help={f.help}
              error={errors[f.key] ?? null}
              options={f.options}
              placeholder={f.placeholder}
              suffix={f.suffix}
              min={f.min}
              max={f.max}
              rows={f.rows}
              dirty={changedKeys.includes(f.key)}
              onchange={(v) => set(f.key, v)}
            />
          {/each}
        </div>

        {#if visibleFields.length < activeSection.fields.length}
          <!-- Field bersyarat yang sedang disembunyikan. Versi lama menampilkan
               semuanya sekaligus, termasuk kredensial S3 saat driver-nya
               "bawaan sistem" — form terlihat jauh lebih panjang dari yang
               sebenarnya perlu diisi. -->
          <p class="mt-4 border-t border-ink-100 pt-4 text-sm text-ink-400">
            {activeSection.fields.length - visibleFields.length} pengaturan lain muncul setelah
            opsi terkait diaktifkan.
          </p>
        {/if}
      </Card>
    </div>

    <SaveBar
      changes={changedLabels}
      elsewhere={changedElsewhere}
      {saving}
      onsave={save}
      onreset={reset}
    />

    <!-- Bagian yang masih dilayani halaman lama. Ditampilkan sebagai daftar
         eksplisit, bukan disembunyikan: pengguna perlu tahu ke mana harus pergi
         alih-alih mengira fiturnya hilang di versi baru. -->
    <div class="mt-8">
      <Card title="Bagian lain">
        {#snippet aside()}
          <span class="text-sm text-ink-400">masih memakai tampilan lama</span>
        {/snippet}
        <ul class="grid gap-2 sm:grid-cols-2">
          {#each PANEL_SECTIONS as s (s.id)}
            <li>
              <a
                href={`/admin/settings#${s.id}`}
                class="focus-ring flex items-start gap-3 rounded-lg p-3 ring-1 ring-inset ring-ink-200 hover:bg-ink-50"
              >
                <Icon name={s.icon} size={16} class="mt-0.5 shrink-0 text-ink-400" />
                <span class="min-w-0">
                  <span class="block text-base font-medium text-ink-900">{s.label}</span>
                  <span class="block text-sm text-ink-500">{s.desc}</span>
                </span>
              </a>
            </li>
          {/each}
        </ul>
      </Card>
    </div>
  {/if}
</AppShell>
