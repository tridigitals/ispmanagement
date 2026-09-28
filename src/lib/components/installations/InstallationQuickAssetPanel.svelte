<script lang="ts">
  /**
    Panel "Buat aset FTTH dari instalasi ini" — satu komponen, dua halaman.

    Dipakai oleh:
      - (v2)/v2/admin/network/installations/+page.svelte   ← yang benar-benar tampil
      - (app)/admin/network/installations/InstallationDetailModal.svelte

    Catatan temuan (verifikasi 2026-09): rute (app) di atas DI-REDIRECT ke v2 oleh
    legacyV2Redirect.ts, jadi jalur kedua saat ini dorman — dipertahankan sebagai
    fallback karena redirect itu memang dirancang bisa di-rollback. Semua
    permukaan (app) lain juga dorman untuk alasan yang sama.

    Kenapa digabung: sebelumnya markup form ini ada DUA kali dengan isi berbeda.
    Halaman (app) mengisi nama dari pelanggan+lokasi; halaman v2 sempat tidak
    mengisi apa pun. Akibatnya layout mendarat di dua tempat dan detail aset bisa
    hilang di salah satunya. Logika sudah disatukan lebih dulu di
    `$lib/utils/installationQuickAsset`; komponen ini menyatukan tampilannya.

    Komponen ini TIDAK mengambil data sendiri. Pemanggil yang bertanggung jawab
    memuat konteks instalasi dan menyediakan aksi `onSubmit` — jadi tidak ada
    fetch ganda dan tidak ada asumsi soal bentuk baris instalasi.
  */
  import { t } from 'svelte-i18n';

  import Button from '$lib/components/ds/Button.svelte';
  import Field from '$lib/components/ds/Field.svelte';
  import type { InstallationQuickAssetDraft } from '$lib/utils/installationQuickAsset';
  import { IQA } from './quickAssetPanelStyles';

  interface Props {
    /** Draf terkini; pemanggil menyimpan state-nya (bindable). */
    draft: InstallationQuickAssetDraft;
    /** Pesan galat siap-tampil; null/'' bila valid. */
    error?: string | null;
    /** Opsi induk (ODP). Sudah termasuk opsi kosong dari pemanggil. */
    parentOptions?: { value: string; label: string }[];
    /** Sedang menyimpan. */
    busy?: boolean;
    /** Perbarui satu field draf (pemanggil menormalisasi nilainya). */
    onfield: (field: keyof InstallationQuickAssetDraft, value: string) => void;
    /** Kirim — pemanggil yang memvalidasi ulang & memanggil API. */
    onSubmit: () => void;
    /** Tutup form. */
    onCancel: () => void;
  }

  let {
    draft = $bindable(),
    error = null,
    parentOptions = [],
    busy = false,
    onfield,
    onSubmit,
    onCancel,
  }: Props = $props();

  /* Label & teks dibaca dari kunci i18n yang SAMA di kedua halaman
     (`admin.network.installations.v2.*`), supaya terjemahan tidak bercabang. */
  const L = {
    title: 'admin.network.installations.v2.create_asset_title',
    prefill: 'admin.network.installations.v2.create_asset_prefill',
    type: 'admin.network.installations.v2.qa_type',
    name: 'admin.network.installations.v2.qa_name',
    serial: 'admin.network.installations.v2.qa_serial',
    code: 'admin.network.installations.v2.qa_code',
    lat: 'admin.network.installations.v2.qa_lat',
    lng: 'admin.network.installations.v2.qa_lng',
    parent: 'admin.network.installations.v2.qa_parent',
    submit: 'admin.network.installations.v2.qa_submit',
    cancel: 'common.cancel',
    loading: 'common.loading',
  } as const;

  const TYPE_OPTIONS = [
    { value: 'ont', label: 'ONT' },
    { value: 'onu', label: 'ONU' },
    { value: 'odp', label: 'ODP' },
  ];

  const text = (key: string, fallback: string) => {
    const value = $t(key);
    // Kunci yang belum diterjemahkan dikembalikan apa adanya oleh svelte-i18n.
    return !value || value === key ? fallback : value;
  };
</script>

<div class={IQA.card}>
  <div class={IQA.cardTitle}>{text(L.title, 'Buat aset FTTH dari instalasi ini')}</div>
  <p class={IQA.cardNote}>
    {text(L.prefill, 'Terisi otomatis dari work order — ubah bila perlu.')}
  </p>

  <div class={IQA.grid}>
    <Field
      id="qa-type"
      label={text(L.type, 'Tipe')}
      type="select"
      stacked
      value={draft.asset_type}
      options={TYPE_OPTIONS}
      onchange={(v) => onfield('asset_type', String(v ?? 'ont'))}
    />
    <Field
      id="qa-name"
      label={text(L.name, 'Nama aset')}
      stacked
      value={draft.name}
      onchange={(v) => onfield('name', String(v ?? ''))}
    />
    <Field
      id="qa-serial"
      label={text(L.serial, 'Nomor seri')}
      stacked
      value={draft.serial_number}
      onchange={(v) => onfield('serial_number', String(v ?? ''))}
    />
    <Field
      id="qa-code"
      label={text(L.code, 'Kode')}
      stacked
      value={draft.code}
      onchange={(v) => onfield('code', String(v ?? ''))}
    />
    <Field
      id="qa-lat"
      label={text(L.lat, 'Lintang')}
      stacked
      value={draft.latitude}
      onchange={(v) => onfield('latitude', String(v ?? ''))}
    />
    <Field
      id="qa-lng"
      label={text(L.lng, 'Bujur')}
      stacked
      value={draft.longitude}
      onchange={(v) => onfield('longitude', String(v ?? ''))}
    />
    <Field
      id="qa-parent"
      label={text(L.parent, 'Induk (ODP, opsional)')}
      type="select"
      stacked
      value={draft.parent_asset_id}
      options={parentOptions}
      onchange={(v) => onfield('parent_asset_id', String(v ?? ''))}
    />
  </div>

  {#if error}
    <p class={IQA.error}>{error}</p>
  {/if}

  <div class={IQA.actions}>
    <Button variant="ghost" size="sm" onclick={onCancel} disabled={busy}>
      {text(L.cancel, 'Batal')}
    </Button>
    <Button variant="primary" size="sm" disabled={busy || !!error} onclick={onSubmit}>
      {busy ? text(L.loading, 'Memuat…') : text(L.submit, 'Buat & pilih')}
    </Button>
  </div>
</div>
