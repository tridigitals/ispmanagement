<script lang="ts">
  /*
    Pengumuman v2.

    Versi lama: `(app)/admin/announcements/+page.svelte` — 1.293 baris.
    Temuan yang dikunci gelombang ini (dibuktikan di data produksi 2026-09-04,
    detail di `$lib/utils/announcementInsights`):

    1. SANITIZER BODY NO-OP (KEAMANAN).
       Body dirender `{@html}` dan `sanitizeHtml` lama tidak membuang apa pun
       — 10/10 payload XSS lolos, termasuk <script> telanjang. Token sesi ada
       di localStorage, jadi dampak-nya pencurian sesi. Diganti DOMPurify
       (allowlist eksplisit) + 32 tes unit.

    2. SCOPE GLOBAL MENGABAIKAN AUDIENS.
       Keempat cabang pengirim menjalankan `SELECT id FROM users WHERE
       is_active = true` tanpa membaca `audience` — "hanya admin" global tetap
       terkirim ke 18 user aktif lintas tenant. Diperbaiki di Rust lewat
       `global_recipient_ids()`.

    3. JADWAL ULANG TIDAK PERNAH TERKIRIM.
       `UPDATE announcements` tidak menyentuh `notified_at`, sementara
       penjadwal hanya memilih baris `notified_at IS NULL`. Menggeser
       `starts_at` ke masa depan kini mereset `notified_at`
       (`should_reschedule_delivery`, 5 tes unit Rust).

    4. PILIHAN AUDIENS MENYESATKAN DI UI.
       Dropdown menawarkan 5 audiens padahal di tenant ini jangkauannya
       admins=1, customers=2, active_subscribers=0. Sekarang dropdown
       menampilkan jumlah penerima nyata (GET /announcements/admin/reach) dan
       memperingatkan pilihan yang nol.

    5. TANGGAL TAK VALID DIAM-DIAM JADI "TERBIT SEKARANG".
       `toIsoOrNull` lama mengembalikan null untuk input rusak dan server
       memaknai null sebagai now(). `parseWaktu`/`validateDraft` memisahkan
       "kosong" dari "rusak".

    6. TOMBOL EDIT TIDAK MENGEDIT APA PUN.
       Rute `[id]` lama tidak pernah memanggil `updateAdmin`. Di sini edit
       nyata: modal yang sama mengisi form dari baris dan mengirim PUT.
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import type { Component } from 'svelte';
  import { api } from '$lib/api/client';
  import type { Announcement, CreateAnnouncementDto } from '$lib/api/types';
  import { can, isSuperAdmin } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import Modal from '$lib/components/ui/Modal.svelte';
  import DateTimeLocalInput from '$lib/components/ui/DateTimeLocalInput.svelte';
  import {
    AppShell,
    AttentionPanel,
    Badge,
    Button,
    Card,
    DataTable,
    Field,
    Icon,
    PageHeader,
    RowActions,
    StatTile,
    type Column,
    type FieldOption,
  } from '$lib/components/ds';
  import type { AttentionItem } from '$lib/components/ds/AttentionPanel.svelte';
  import { t } from 'svelte-i18n';
  import {
    alreadyDelivered,
    announcementStatus,
    audienceOptions,
    bodyExcerpt,
    deliveryLabels,
    editDeliveryWarning,
    portalCoverageGap,
    severityTone,
    statusCounts,
    statusLabel,
    statusTone,
    toIso,
    validateDraft,
    type DraftIssue,
  } from '$lib/utils/announcementInsights';

  let rows = $state<Announcement[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let pageNum = $state(1);
  const perPage = 20;

  let search = $state('');
  let statusFilter = $state('all');
  let severityFilter = $state('all');
  let modeFilter = $state('all');
  let scopeFilter = $state('tenant');

  // Jangkauan audiens dari server; null sampai terisi.
  let reach = $state<Record<string, number>>({});

  // ---- form (create & edit) ----
  let formOpen = $state(false);
  let editTarget = $state<Announcement | null>(null);
  let saving = $state(false);
  let fScope = $state<'tenant' | 'global'>('tenant');
  let fAudience = $state('all');
  let fSeverity = $state('info');
  let fMode = $state<'post' | 'banner'>('post');
  let fTitle = $state('');
  let fBody = $state('');
  let fStarts = $state('');
  let fEnds = $state('');
  let fInApp = $state(true);
  let fEmail = $state(false);
  let fEmailForce = $state(true);
  let fCoverFile = $state<File | null>(null);
  let fCoverPreview = $state('');
  let issues = $state<DraftIssue[]>([]);

  let Editor = $state<Component | null>(null);

  let deleteTarget = $state<Announcement | null>(null);
  let deleteOpen = $state(false);
  let deleting = $state(false);

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));
  const counts = $derived(statusCounts(rows));
  const ringkas = $derived(
    audienceOptions(reach, $t).find((o) => o.value === fAudience) ?? null,
  );
  const gapPortal = $derived(
    portalCoverageGap(reach.total_customers ?? 0, reach.portal_accounts ?? 0, $t),
  );

  const peringatan = $derived<AttentionItem[]>(
    (() => {
      const out: AttentionItem[] = [];
      if (gapPortal) {
        out.push({
          icon: 'mail',
          title: $t('announcements.v2.attn_nop_title'),
          detail: gapPortal,
          action: $t('announcements.v2.attn_nop_action'),
          severity: 'medium',
        });
      }
      const tanpaKanal = rows.filter((r) => !r.deliver_in_app && !r.deliver_email).length;
      if (tanpaKanal > 0) {
        out.push({
          icon: 'alert',
          title: $t('announcements.v2.attn_noch_title'),
          detail: $t('announcements.v2.attn_noch_detail', { values: { n: tanpaKanal } }),
          action: $t('announcements.v2.attn_noch_action'),
          severity: 'high',
        });
      }
      return out;
    })(),
  );

  const severityOpts = $derived<FieldOption[]>([
    { value: 'info', label: $t('announcements.severity.info') },
    { value: 'success', label: $t('announcements.severity.success') },
    { value: 'warning', label: $t('announcements.severity.warning') },
    { value: 'error', label: $t('announcements.v2.sev_error') },
  ]);
  const modeOpts = $derived<FieldOption[]>([
    { value: 'post', label: $t('announcements.v2.mode_post') },
    { value: 'banner', label: $t('announcements.modes.banner') },
  ]);
  const scopeOpts = $derived<FieldOption[]>([
    { value: 'tenant', label: $t('announcements.v2.scope_tenant') },
    { value: 'global', label: $t('announcements.v2.scope_global') },
  ]);

  const columns: Column[] = [
    { key: 'title', label: $t('announcements.title') },
    { key: 'status', label: $t('admin.customers.columns.status'), hideSm: true },
    { key: 'audience', label: $t('superadmin.audit_logs.columns.audience'), hideSm: true },
    { key: 'channels', label: $t('announcements.v2.col_channel'), hideSm: true },
    { key: 'schedule', label: $t('common.schedule'), hideSm: true },
    { key: 'actions', label: '', width: '120px' },
  ];

  function tanggal(v: string | null): string {
    if (!v) return '—';
    return new Date(v).toLocaleString('id-ID', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  async function load() {
    loading = true;
    try {
      const [res, r] = await Promise.all([
        api.announcements.listAdmin({
          scope: scopeFilter as 'tenant' | 'global' | 'all',
          page: pageNum,
          per_page: perPage,
          search: search || undefined,
          status: statusFilter !== 'all' ? statusFilter : undefined,
          severity: severityFilter !== 'all' ? severityFilter : undefined,
          mode: modeFilter !== 'all' ? modeFilter : undefined,
        }),
        api.announcements.reach(scopeFilter === 'global' ? 'global' : 'tenant'),
      ]);
      rows = res.data;
      total = res.total;
      reach = r;
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, 'Gagal memuat pengumuman'));
    } finally {
      loading = false;
    }
  }

  function bukaBaru() {
    editTarget = null;
    fScope = 'tenant';
    fAudience = 'all';
    fSeverity = 'info';
    fMode = 'post';
    fTitle = '';
    fBody = '';
    fStarts = '';
    fEnds = '';
    fInApp = true;
    fEmail = false;
    fEmailForce = true;
    fCoverFile = null;
    issues = [];
    formOpen = true;
    void ensureEditor();
  }

  function bukaEdit(a: Announcement) {
    editTarget = a;
    fScope = a.tenant_id === null ? 'global' : 'tenant';
    fAudience = a.audience;
    fSeverity = a.severity as typeof fSeverity;
    fMode = a.mode;
    fTitle = a.title;
    fBody = a.body;
    fStarts = '';
    fEnds = '';
    fInApp = a.deliver_in_app;
    fEmail = a.deliver_email;
    fEmailForce = a.deliver_email_force ?? true;
    fCoverFile = null;
    issues = [];
    formOpen = true;
    void ensureEditor();
  }

  async function ensureEditor() {
    if (Editor) return;
    try {
      // Dynamic import: bundle editor teks kaya (~ribuan baris) tidak ikut
      // halaman ini sampai form pertama kali dibuka.
      const { default: EditorComponent } = await import('$lib/components/ui/RichTextEditor.svelte');
      Editor = EditorComponent;
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, 'Editor gagal dimuat'));
    }
  }

  function onPickCover(e: Event) {
    const input = e.target as HTMLInputElement;
    const f = (input.files || [])[0] || null;
    fCoverFile = f;
    if (fCoverPreview) URL.revokeObjectURL(fCoverPreview);
    fCoverPreview = f ? URL.createObjectURL(f) : '';
  }

  async function simpan() {
    // Validasi DULU, upload cover SESUDAH — urutan lama meninggalkan berkas
    // yatim di storage saat server menolak.
    const draft = {
      title: fTitle,
      body: fBody,
      startsAt: fStarts,
      endsAt: fEnds,
      deliverInApp: fInApp,
      deliverEmail: fEmail,
      scope: fScope,
    };
    issues = validateDraft(draft, $t);
    if (issues.length) {
      toast.error(issues[0].message);
      return;
    }

    saving = true;
    try {
      let coverFileId: string | null | undefined;
      if (fCoverFile) {
        const rec = await api.storage.uploadFile(fCoverFile);
        coverFileId = rec.id;
      }

      const startsAt = toIso(fStarts);
      const endsAt = toIso(fEnds);

      if (editTarget) {
        await api.announcements.updateAdmin(editTarget.id, {
          title: fTitle.trim(),
          body: fBody,
          severity: fSeverity as CreateAnnouncementDto['severity'],
          audience: fAudience as CreateAnnouncementDto['audience'],
          mode: fMode,
          format: 'html',
          deliver_in_app: fInApp,
          deliver_email: fEmail,
          deliver_email_force: fEmailForce,
          starts_at: startsAt ?? undefined,
          ends_at: endsAt ?? (fEnds ? null : undefined),
          cover_file_id: coverFileId,
        });
        toast.success($t('announcements.v2.t_updated'));
      } else {
        await api.announcements.createAdmin({
          scope: fScope,
          title: fTitle.trim(),
          body: fBody,
          severity: fSeverity as CreateAnnouncementDto['severity'],
          audience: fAudience as CreateAnnouncementDto['audience'],
          mode: fMode,
          format: 'html',
          deliver_in_app: fInApp,
          deliver_email: fEmail,
          deliver_email_force: fEmailForce,
          starts_at: startsAt,
          ends_at: endsAt,
          cover_file_id: coverFileId ?? null,
        });
        toast.success($t('announcements.v2.t_created'));
      }
      formOpen = false;
      if (fCoverPreview) URL.revokeObjectURL(fCoverPreview);
      fCoverPreview = '';
      await load();
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, 'Gagal menyimpan pengumuman'));
    } finally {
      saving = false;
    }
  }

  async function hapus() {
    if (!deleteTarget) return;
    deleting = true;
    try {
      await api.announcements.deleteAdmin(deleteTarget.id);
      toast.success($t('announcements.v2.t_deleted'));
      deleteTarget = null;
      deleteOpen = false;
      await load();
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, 'Gagal menghapus'));
    } finally {
      deleting = false;
    }
  }

  onMount(() => {
    if (!$can('manage', 'announcements')) {
      goto('/unauthorized');
      return;
    }
    void load();
  });
</script>

<AppShell title={ $t('announcements.title') }>
  <PageHeader
    title={ $t('announcements.title') }
    eyebrow={ $t('admin.eyebrows.communication') }
    desc={ $t('announcements.v2.desc') }
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={() => void load()}>{ $t('announcements.v2.reload') }</Button>
      <Button variant="primary" icon="plus" onclick={bukaBaru}>{ $t('announcements.v2.create') }</Button>
    {/snippet}
  </PageHeader>

  <Card>
    <div class="grid grid-cols-2 gap-6 sm:grid-cols-4">
      <StatTile label={ $t('announcements.v2.live') } value={String(counts.active)} hint={ $t('announcements.v2.h_live') } tone="positive" />
      <StatTile
        label={ $t('announcements.status.scheduled') }
        value={String(counts.scheduled)}
        hint={counts.scheduled > 0 ? $t('announcements.v2.h_sched') : $t('announcements.v2.h_none')}
        tone={counts.scheduled > 0 ? 'warning' : 'neutral'}
      />
      <StatTile label={ $t('announcements.v2.expired') } value={String(counts.expired)} hint={ $t('announcements.v2.h_exp') } />
      <StatTile
        label={ $t('announcements.v2.reach_tile') }
        value={reach.all != null ? String(reach.all) : '—'}
        hint={ $t('announcements.v2.h_reach') }
      />
    </div>
  </Card>

  {#if peringatan.length}
    <div class="mt-4">
      <AttentionPanel items={peringatan} title={ $t('admin.network.dhcp_static.v2.attention') } />
    </div>
  {/if}

  <div class="mt-4">
    <Card>
      <div class="mb-3 flex flex-wrap items-center gap-2">
        <div class="relative min-w-[220px] flex-1">
          <Icon
            name="search"
            size={15}
            class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-400"
          />
          <input
            bind:value={search}
            placeholder={ $t('announcements.v2.search_ph') }
            aria-label={ $t('announcements.v2.search_aria') }
            class="focus-ring h-9 w-full rounded-lg border-0 bg-white pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400"
          />
        </div>

        <select
          bind:value={statusFilter}
          aria-label={ $t('announcements.v2.f_status') }
          class="focus-ring h-9 rounded-lg border-0 bg-white px-2.5 text-base text-ink-900 ring-1 ring-inset ring-ink-200"
        >
          <option value="all">{ $t('announcements.v2.all_status') }</option>
          <option value="active">{ $t('announcements.v2.live') }</option>
          <option value="scheduled">{ $t('announcements.status.scheduled') }</option>
          <option value="expired">{ $t('announcements.v2.expired') }</option>
        </select>

        <select
          bind:value={severityFilter}
          aria-label={ $t('announcements.v2.f_sev') }
          class="focus-ring h-9 rounded-lg border-0 bg-white px-2.5 text-base text-ink-900 ring-1 ring-inset ring-ink-200"
        >
          <option value="all">{ $t('announcements.v2.all_sev') }</option>
          <option value="info">{ $t('announcements.severity.info') }</option>
          <option value="success">{ $t('announcements.severity.success') }</option>
          <option value="warning">{ $t('announcements.severity.warning') }</option>
          <option value="error">{ $t('announcements.v2.sev_error') }</option>
        </select>

        <select
          bind:value={modeFilter}
          aria-label={ $t('announcements.v2.f_mode') }
          class="focus-ring h-9 rounded-lg border-0 bg-white px-2.5 text-base text-ink-900 ring-1 ring-inset ring-ink-200"
        >
          <option value="all">{ $t('announcements.v2.mode_post_banner') }</option>
          <option value="post">{ $t('announcements.v2.mode_post') }</option>
          <option value="banner">{ $t('announcements.modes.banner') }</option>
        </select>

        {#if $isSuperAdmin}
          <select
            bind:value={scopeFilter}
            aria-label={ $t('announcements.v2.f_scope') }
            class="focus-ring h-9 rounded-lg border-0 bg-white px-2.5 text-base text-ink-900 ring-1 ring-inset ring-ink-200"
          >
            <option value="tenant">{ $t('announcements.v2.scope_tenant') }</option>
            <option value="global">{ $t('admin.email_outbox.scope.global') }</option>
            <option value="all">{ $t('admin.customers.billing.filters.all') }</option>
          </select>
        {/if}

        <Button
          variant="ghost"
          icon="search"
          onclick={() => {
            pageNum = 1;
            void load();
          }}>Terapkan</Button
        >
      </div>

      <DataTable
        {columns}
        {rows}
        {loading}
        emptyTitle="Belum ada pengumuman"
        emptyHint={search
          ? $t('announcements.v2.empty_hint1')
          : $t('announcements.v2.empty_hint2')}
        footNote={`${rows.length} dari ${total} pengumuman · halaman ${pageNum}/${totalPages}`}
      >
        {#snippet cell(a, c)}
          {#if c.key === 'title'}
            <div class="min-w-0 max-w-[420px]">
              <div class="flex items-center gap-1.5">
                <span class="truncate font-medium text-ink-900">{a.title}</span>
                {#if a.mode === 'banner'}
                  <Badge label={ $t('announcements.modes.banner') } tone="neutral" />
                {/if}
                {#if a.tenant_id === null}
                  <Badge label={ $t('admin.email_outbox.scope.global') } tone="warning" />
                {/if}
              </div>
              <div class="truncate text-sm text-ink-500">{bodyExcerpt(a.body)}</div>
            </div>
          {:else if c.key === 'status'}
            <div class="flex flex-wrap items-center gap-1.5">
              <Badge label={statusLabel(announcementStatus(a), $t)} tone={statusTone(announcementStatus(a))} />
              <Badge label={a.severity} tone={severityTone(a.severity)} />
            </div>
          {:else if c.key === 'audience'}
            <span class="text-sm text-ink-600">
              {audienceOptions(reach, $t).find((o) => o.value === a.audience)?.label ?? a.audience}
            </span>
          {:else if c.key === 'channels'}
            <div class="flex flex-wrap gap-1 text-sm text-ink-600">
              {#if deliveryLabels(a).length === 0}
                <span class="inline-flex items-center gap-1 font-medium text-red-700">
                  <Icon name="alert" size={13} /> { $t('announcements.v2.no_channel') }
                </span>
              {:else}
                {#each deliveryLabels(a, $t) as l (l)}
                  <span class="inline-flex items-center gap-1">
                    <Icon name={l === 'Email' ? 'mail' : 'bell'} size={13} />{l === 'Email' ? $t('admin.message_templates.channels.email') : $t('announcements.v2.inapp')}
                  </span>
                {/each}
              {/if}
              {#if alreadyDelivered(a)}
                <span class="text-ink-400">· { $t('announcements.v2.delivered') } {tanggal(a.notified_at)}</span>
              {/if}
            </div>
          {:else if c.key === 'schedule'}
            <div class="text-sm text-ink-500">
              <div>{tanggal(a.starts_at)}</div>
              <div class="text-ink-400">{ $t('announcements.v2.range_sep') } {a.ends_at ? tanggal(a.ends_at) : $t('announcements.v2.no_limit')}</div>
            </div>
          {:else if c.key === 'actions'}
            <RowActions
              primary={{ label: $t('admin.customers.detail.v2.act_edit'), icon: 'cog', onclick: () => bukaEdit(a) }}
              rest={[
                {
                  label: $t('common.delete'),
                  icon: 'close',
                  danger: true,
                  onclick: () => {
                    deleteTarget = a;
                    deleteOpen = true;
                  },
                },
              ]}
            />
          {/if}
        {/snippet}
      </DataTable>

      {#if totalPages > 1}
        <div class="mt-3 flex items-center justify-end gap-2">
          <Button
            variant="ghost"
            size="sm"
            disabled={pageNum <= 1 || loading}
            onclick={() => {
              pageNum -= 1;
              void load();
            }}>Sebelumnya</Button
          >
          <span class="num text-sm text-ink-500">{pageNum} / {totalPages}</span>
          <Button
            variant="ghost"
            size="sm"
            disabled={pageNum >= totalPages || loading}
            onclick={() => {
              pageNum += 1;
              void load();
            }}>Berikutnya</Button
          >
        </div>
      {/if}
    </Card>
  </div>
</AppShell>

<Modal bind:show={formOpen} title={editTarget ? $t('announcements.v2.edit_title') : $t('announcements.v2.create')} width="720px">
  {#if editTarget && editDeliveryWarning(editTarget, $t)}
    <p class="mb-3 rounded-lg bg-amber-50 px-3 py-2 text-sm text-amber-900 ring-1 ring-inset ring-amber-200">
      {editDeliveryWarning(editTarget, $t)}
    </p>
  {/if}

  <div class="space-y-3">
    {#if $isSuperAdmin && !editTarget}
      <Field stacked
        id="f-scope"
        label={ $t('announcements.fields.scope') }
        value={fScope}
        type="select"
        options={scopeOpts}
        help="Global menjangkau semua tenant; audiens tetap dihormati."
        onchange={(v) => (fScope = v as 'tenant' | 'global')}
      />
    {/if}

    <Field stacked
      id="f-audience"
      label={ $t('superadmin.audit_logs.columns.audience') }
      value={fAudience}
      type="select"
      options={audienceOptions(reach).map((o) => ({
        value: o.value,
        label: o.recipients != null ? `${o.label} — ${o.recipients} penerima` : o.label,
      }))}
      error={ringkas?.warning ?? null}
      help="Jumlah penerima dihitung dari akun yang benar-benar ada, bukan perkiraan."
      onchange={(v) => (fAudience = v)}
    />

    <div class="grid grid-cols-2 gap-3">
      <Field stacked
        id="f-severity"
        label={ $t('announcements.v2.sev_label') }
        value={fSeverity}
        type="select"
        options={severityOpts}
        onchange={(v) => (fSeverity = v)}
      />
      <Field stacked
        id="f-mode"
        label={ $t('announcements.v2.mode_label') }
        value={fMode}
        type="select"
        options={modeOpts}
        onchange={(v) => (fMode = v as 'post' | 'banner')}
      />
    </div>

    <Field stacked
      id="f-title"
      label={ $t('admin.team.hard_delete.title') }
      value={fTitle}
      error={issues.find((i) => i.field === 'title')?.message ?? null}
      onchange={(v) => (fTitle = v)}
    />

    {#if Editor}
      <Editor bind:value={fBody} label={ $t('announcements.v2.f_body') } placeholder={ $t('announcements.v2.body_ph') } minHeight={200} />
    {:else}
      <Field stacked
        id="f-body"
        label={ $t('announcements.v2.f_body') }
        value={fBody}
        type="textarea"
        rows={6}
        placeholder={ $t('announcements.v2.editor_loading') }
        onchange={(v) => (fBody = v)}
      />
    {/if}

    <div class="grid grid-cols-2 gap-3">
      <DateTimeLocalInput id="f-starts" bind:value={fStarts} label={ $t('announcements.v2.starts') } />
      <DateTimeLocalInput id="f-ends" bind:value={fEnds} label={ $t('announcements.v2.ends') } />
    </div>
    {#each issues.filter((i) => i.field === 'startsAt' || i.field === 'endsAt') as i (i.field)}
      <p class="text-sm text-red-700">{i.message}</p>
    {/each}

    <div class="grid grid-cols-2 gap-3">
      <Field stacked
        id="f-inapp"
        label={ $t('announcements.v2.inapp') }
        value={String(fInApp)}
        type="toggle"
        onchange={(v) => (fInApp = v === 'true')}
      />
      <Field stacked
        id="f-email"
        label={ $t('admin.message_templates.channels.email') }
        value={String(fEmail)}
        type="toggle"
        onchange={(v) => (fEmail = v === 'true')}
      />
    </div>
    {#if issues.some((i) => i.field === 'delivery')}
      <p class="text-sm text-red-700">{issues.find((i) => i.field === 'delivery')?.message}</p>
    {/if}

    {#if fEmail}
      <Field stacked
        id="f-email-force"
        label={ $t('announcements.v2.force_email') }
        value={String(fEmailForce)}
        type="toggle"
        help="Hanya perlu bila pengaturan email tenant sedang dimatikan."
        onchange={(v) => (fEmailForce = v === 'true')}
      />
    {/if}

    <div>
      <label class="mb-1 block text-sm font-medium text-ink-700" for="f-cover">{ $t('announcements.v2.cover') }</label>
      <input
        id="f-cover"
        type="file"
        accept="image/*"
        onchange={onPickCover}
        class="block w-full cursor-pointer rounded-lg border border-dashed border-ink-300 bg-white p-2 text-sm text-ink-700
          file:mr-2 file:rounded-md file:border-0 file:bg-ink-100 file:px-2.5 file:py-1 file:text-sm file:text-ink-700"
      />
      {#if fCoverPreview}
        <img src={fCoverPreview} alt="Pratinjau cover" class="mt-2 h-24 rounded-lg object-cover" />
      {/if}
    </div>
  </div>

  {#snippet footer()}
    <Button variant="ghost" onclick={() => (formOpen = false)}>{ $t('common.cancel') }</Button>
    <Button variant="primary" icon="check" loading={saving} onclick={() => void simpan()}>
      {editTarget ? $t('announcements.v2.save_changes') : $t('announcements.v2.publish')}
    </Button>
  {/snippet}
</Modal>

<Modal bind:show={deleteOpen} title={ $t('announcements.v2.delete_title') } width="420px">
  <p class="text-sm text-ink-700">
    { $t('announcements.title') } <b>{deleteTarget?.title}</b> akan dihapus permanen. Notifikasi yang sudah
    terlanjur terkirim ke penerima tidak ikut terhapus.
  </p>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (deleteOpen = false)}>{ $t('common.cancel') }</Button>
    <Button variant="danger" icon="close" loading={deleting} onclick={() => void hapus()}>{ $t('common.delete') }</Button>
  {/snippet}
</Modal>
