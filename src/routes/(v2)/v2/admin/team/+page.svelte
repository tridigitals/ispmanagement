<script lang="ts">
  /*
    Anggota tim v2.

    Versi lama: `(app)/admin/team/+page.svelte` — 1.204 baris. Tiga masalah
    yang terukur, bukan soal selera:

    1. PELANGGAN DIHITUNG SEBAGAI ANGGOTA TIM.
       `team_service.rs:34 list_members()` mengembalikan seluruh baris
       `tenant_members` tanpa memfilter role. Halaman memakainya apa adanya
       (baris 80: `total: teamMembers.length`).

       GET /api/team tenant "ISP Management" (2026-09-04) = 6 baris:
         Owner 1 · Technician 1 · Member 1 · Customer 3
       Layar lama menulis "6 members". Yang benar: 3 staf.

       Yang membuatnya aneh, sistem lain sudah sepakat Customer bukan staf:
       dropdown filter role halaman ini SUDAH membuang Customer (baris 96),
       dan backend menolak menetapkan role Customer lewat modul tim
       (`http/team.rs:207`). Hanya kueri pengisi tabelnya yang tidak ikut.

    2. TAB ARSIP TIDAK PERNAH BISA JALAN DI LUAR TAURI.
       `api.team.listDeleted/restore/hardDelete` memanggil `safeInvoke`, tapi
       `commandMap` di `src/lib/api/core.ts` tidak punya entri untuk ketiganya
       — padahal rutenya hidup (`bootstrap/http.rs:683`, diverifikasi
       GET /api/team/deleted -> 200 []). Tanpa entri, safeInvoke jatuh ke
       cabang mock dan melempar "not implemented in HTTP API yet". Ketiga
       entri itu ditambahkan bersama halaman ini.

    3. TIDAK ADA INFORMASI KEAMANAN AKUN.
       Keenam akun tenant ini `two_factor_enabled = false` dan
       `email_verified_at = NULL`, termasuk Owner dengan kewenangan penuh.
       Halaman lama tidak punya kolom untuk itu, jadi tidak ada tempat yang
       memberi tahu. Kolomnya kini ikut di-SELECT `list_members()`.

    Aturan roster pindah ke `$lib/utils/teamRoster` (21 tes unit).
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { Role, TeamMember } from '$lib/api/types';
  import { can, user } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import Modal from '$lib/components/ui/Modal.svelte';
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
  } from '$lib/components/ds';
  import {
    canManage,
    initials,
    isCustomerAccount,
    roleTone,
    securityFlags,
    staffOnly,
    summarize,
  } from '$lib/utils/teamRoster';

  let rows = $state<TeamMember[]>([]);
  let roles = $state<Role[]>([]);
  let archived = $state<TeamMember[]>([]);
  let loading = $state(true);
  let loadingArchive = $state(false);
  let showArchive = $state(false);
  let search = $state('');
  let roleFilter = $state('all');
  let statusFilter = $state<'all' | 'active' | 'inactive'>('all');

  let inviteOpen = $state(false);
  let inviteEmail = $state('');
  let inviteName = $state('');
  let inviteRoleId = $state('');
  let invitePassword = $state('');
  let inviting = $state(false);

  let editOpen = $state(false);
  let editTarget = $state<TeamMember | null>(null);
  let editRoleId = $state('');
  let savingRole = $state(false);

  let removeTarget = $state<TeamMember | null>(null);
  let removing = $state(false);
  let purgeTarget = $state<TeamMember | null>(null);
  let purging = $state(false);
  let busyId = $state<string | null>(null);

  /* Staf saja. Ini perbedaan inti dari halaman lama. */
  const staff = $derived(staffOnly(rows) as TeamMember[]);
  const ringkasan = $derived(summarize(rows));
  const pelanggan = $derived(rows.filter(isCustomerAccount) as TeamMember[]);

  const myLevel = $derived.by(() => {
    const me = rows.find((m) => m.email === $user?.email);
    if (!me) return 0;
    if (typeof me.role_level === 'number') return me.role_level;
    return roles.find((r) => r.id === me.role_id)?.level ?? 0;
  });

  /* Role yang bisa dipilih: Customer dikecualikan karena backend menolaknya,
     dan role di atas level sendiri dibuang karena backend juga menolaknya
     (`enforce_member_role_change_permissions`). Halaman lama menampilkan
     semua role lalu membiarkan permintaan gagal dengan pesan 403. */
  const assignableRoles = $derived(
    roles
      .filter((r) => r.name.toLowerCase() !== 'customer')
      .filter((r) => (r.level ?? 0) <= myLevel)
      .sort((a, b) => (b.level ?? 0) - (a.level ?? 0)),
  );

  const roleFilterOptions = $derived([
    { value: 'all', label: 'Semua role' },
    ...roles
      .filter((r) => r.name.toLowerCase() !== 'customer')
      .map((r) => ({ value: r.id, label: r.name })),
  ]);

  const terlihat = $derived.by(() => {
    const q = search.trim().toLowerCase();
    const base = showArchive ? (archived as TeamMember[]) : staff;
    return base.filter((m) => {
      const cocokCari =
        !q ||
        (m.name ?? '').toLowerCase().includes(q) ||
        (m.email ?? '').toLowerCase().includes(q);
      const cocokRole = roleFilter === 'all' || m.role_id === roleFilter;
      const cocokStatus =
        showArchive ||
        statusFilter === 'all' ||
        (statusFilter === 'active' ? m.is_active !== false : m.is_active === false);
      return cocokCari && cocokRole && cocokStatus;
    });
  });

  const peringatan = $derived(
    securityFlags(staff)
      .slice(0, 4)
      .map((f) => ({
        icon: (f.kind === 'no_2fa_privileged' ? 'shield' : f.kind === 'unverified_email' ? 'mail' : 'users') as
          | 'shield'
          | 'mail'
          | 'users',
        title:
          f.kind === 'no_2fa_privileged'
            ? 'Akun berkuasa tanpa 2FA'
            : f.kind === 'unverified_email'
              ? 'Email belum diverifikasi'
              : 'Staf nonaktif masih terdaftar',
        detail: f.text,
        action: f.kind === 'no_2fa_privileged' ? 'Buka pengaturan keamanan' : 'Tinjau akun',
        href: f.kind === 'no_2fa_privileged' ? '/admin/settings' : undefined,
        severity: (f.kind === 'no_2fa_privileged' ? 'high' : 'medium') as 'high' | 'medium',
      })),
  );

  const columns = $derived<Column[]>(
    showArchive
      ? [
          { key: 'member', label: 'Anggota', width: '34%' },
          { key: 'role', label: 'Role' },
          { key: 'deleted', label: 'Dihapus', hideSm: true },
          { key: 'actions', label: '', align: 'right', width: '120px' },
        ]
      : [
          { key: 'member', label: 'Anggota', width: '32%' },
          { key: 'role', label: 'Role' },
          { key: 'security', label: 'Keamanan akun', hideSm: true },
          { key: 'joined', label: 'Bergabung', hideSm: true },
          { key: 'actions', label: '', align: 'right', width: '120px' },
        ],
  );

  function tanggal(v: string | null | undefined) {
    if (!v) return '—';
    return new Date(v).toLocaleDateString('id-ID', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
    });
  }

  async function load() {
    loading = true;
    try {
      const [m, r] = await Promise.all([api.team.list(), api.roles.list()]);
      rows = m;
      roles = r;
      if (!inviteRoleId) {
        const member = r.find((x) => x.name === 'Member');
        inviteRoleId = member?.id ?? r.find((x) => x.name.toLowerCase() !== 'customer')?.id ?? '';
      }
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, 'Gagal memuat data tim'));
    } finally {
      loading = false;
    }
  }

  async function loadArchive() {
    loadingArchive = true;
    try {
      archived = await api.team.listDeleted();
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, 'Gagal memuat arsip anggota'));
    } finally {
      loadingArchive = false;
    }
  }

  async function toggleArchive() {
    showArchive = !showArchive;
    if (showArchive && !archived.length) await loadArchive();
  }

  async function kirimUndangan() {
    if (!inviteEmail || !inviteName || !inviteRoleId) return;
    inviting = true;
    try {
      await api.team.add(inviteEmail, inviteName, inviteRoleId, invitePassword || undefined);
      toast.success('Anggota ditambahkan');
      inviteOpen = false;
      inviteEmail = '';
      inviteName = '';
      invitePassword = '';
      await load();
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, 'Gagal menambah anggota'));
    } finally {
      inviting = false;
    }
  }

  function bukaEdit(m: TeamMember) {
    editTarget = m;
    editRoleId = m.role_id ?? '';
    editOpen = true;
  }

  async function simpanRole() {
    const target = editTarget;
    if (!target || !editRoleId) return;
    savingRole = true;
    try {
      await api.team.updateRole(target.id, editRoleId);
      toast.success('Role diperbarui');
      editOpen = false;
      editTarget = null;
      await load();
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, 'Gagal memperbarui role'));
    } finally {
      savingRole = false;
    }
  }

  async function hapus() {
    const target = removeTarget;
    if (!target) return;
    removing = true;
    try {
      await api.team.remove(target.id);
      toast.success('Anggota dipindahkan ke arsip');
      removeTarget = null;
      rows = rows.filter((m) => m.id !== target.id);
      if (archived.length) await loadArchive();
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, 'Gagal menghapus anggota'));
    } finally {
      removing = false;
    }
  }

  async function pulihkan(m: TeamMember) {
    busyId = m.id;
    try {
      await api.team.restore(m.id);
      toast.success('Anggota dipulihkan');
      await Promise.all([load(), loadArchive()]);
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, 'Gagal memulihkan anggota'));
    } finally {
      busyId = null;
    }
  }

  async function hapusPermanen() {
    const target = purgeTarget;
    if (!target) return;
    purging = true;
    try {
      await api.team.hardDelete(target.id);
      toast.success('Keanggotaan dihapus permanen');
      purgeTarget = null;
      await loadArchive();
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, 'Gagal menghapus permanen'));
    } finally {
      purging = false;
    }
  }

  onMount(() => {
    if (!$can('read', 'team')) {
      goto('/unauthorized');
      return;
    }
    void load();
  });
</script>

<AppShell title="Anggota tim">
  <PageHeader
    title="Anggota tim"
    eyebrow="Organisasi"
    desc="Staf yang punya akses ke panel ini. Akun pelanggan dikelola di modul Pelanggan."
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={() => void load()}>Muat ulang</Button>
      {#if $can('create', 'team')}
        <Button variant="primary" icon="plus" onclick={() => (inviteOpen = true)}>
          Tambah anggota
        </Button>
      {/if}
    {/snippet}
  </PageHeader>

  <Card>
    <div class="grid grid-cols-2 gap-6 sm:grid-cols-4">
      <StatTile
        label="Staf"
        value={String(ringkasan.staff)}
        hint={ringkasan.customers > 0
          ? `${ringkasan.rows} baris keanggotaan, ${ringkasan.customers} di antaranya akun pelanggan`
          : 'punya akses panel admin'}
      />
      <StatTile
        label="Aktif"
        value={String(ringkasan.staffActive)}
        hint={`dari ${ringkasan.staff} staf`}
        tone="positive"
      />
      <StatTile
        label="Nonaktif"
        value={String(ringkasan.staffInactive)}
        hint={ringkasan.staffInactive > 0 ? 'akun tidak bisa masuk' : 'semua staf bisa masuk'}
        tone={ringkasan.staffInactive > 0 ? 'warning' : 'neutral'}
      />
      <StatTile
        label="Akun pelanggan"
        value={String(ringkasan.customers)}
        hint={ringkasan.customers > 0 ? 'bukan anggota tim — dulu ikut terhitung' : 'tidak ada'}
      />
    </div>
  </Card>

  {#if peringatan.length}
    <div class="mt-4">
      <AttentionPanel items={peringatan} title="Perlu perhatian" />
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
            placeholder="Cari nama atau email"
            aria-label="Cari anggota"
            class="focus-ring h-9 w-full rounded-lg border-0 bg-white pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400"
          />
        </div>

        <select
          bind:value={roleFilter}
          aria-label="Filter role"
          class="focus-ring h-9 rounded-lg border-0 bg-white px-2.5 text-base text-ink-900 ring-1 ring-inset ring-ink-200"
        >
          {#each roleFilterOptions as o (o.value)}
            <option value={o.value}>{o.label}</option>
          {/each}
        </select>

        {#if !showArchive}
          <select
            bind:value={statusFilter}
            aria-label="Filter status"
            class="focus-ring h-9 rounded-lg border-0 bg-white px-2.5 text-base text-ink-900 ring-1 ring-inset ring-ink-200"
          >
            <option value="all">Semua status</option>
            <option value="active">Aktif</option>
            <option value="inactive">Nonaktif</option>
          </select>
        {/if}

        <Button variant={showArchive ? 'primary' : 'ghost'} icon="folder" onclick={toggleArchive}>
          {showArchive ? 'Kembali ke daftar aktif' : 'Arsip'}
        </Button>
      </div>

      {#if showArchive}
        <p class="mb-3 rounded-lg bg-ink-50 px-3 py-2 text-sm text-ink-600">
          Keanggotaan yang dihapus. Memulihkan mengembalikan akses panel dengan role terakhirnya.
        </p>
      {/if}

      <DataTable
        {columns}
        rows={terlihat}
        loading={showArchive ? loadingArchive : loading}
        emptyTitle={showArchive ? 'Arsip kosong' : 'Tidak ada staf yang cocok'}
        emptyHint={showArchive
          ? 'Belum ada anggota tim yang dihapus.'
          : search || roleFilter !== 'all' || statusFilter !== 'all'
            ? 'Coba hapus filter atau ubah kata kunci.'
            : 'Tambahkan anggota untuk memberi akses panel.'}
        footNote={showArchive
          ? `${terlihat.length} keanggotaan diarsipkan`
          : `${terlihat.length} dari ${ringkasan.staff} staf` +
            (ringkasan.customers > 0
              ? ` · ${ringkasan.customers} akun pelanggan disembunyikan dari daftar ini`
              : '')}
      >
        {#snippet cell(m, c)}
          {#if c.key === 'member'}
            <div class="flex min-w-0 items-center gap-2.5">
              <span
                class="flex size-8 shrink-0 items-center justify-center rounded-full bg-ink-100 text-xs font-semibold text-ink-600"
                aria-hidden="true"
              >
                {initials(m.name)}
              </span>
              <div class="min-w-0">
                <div class="truncate font-medium text-ink-900">
                  {m.name || 'Tanpa nama'}
                  {#if m.email === $user?.email}
                    <span class="ml-1 text-xs font-normal text-ink-400">(Anda)</span>
                  {/if}
                </div>
                <div class="truncate text-sm text-ink-500">{m.email}</div>
              </div>
            </div>
          {:else if c.key === 'role'}
            <div class="flex flex-wrap items-center gap-1.5">
              <Badge
                label={m.role_name || m.role || 'Tanpa role'}
                tone={roleTone(m)}
              />
              {#if m.is_active === false}
                <Badge label="Nonaktif" tone="neutral" />
              {/if}
            </div>
          {:else if c.key === 'security'}
            <div class="flex flex-wrap items-center gap-2 text-sm">
              {#if m.two_factor_enabled === true}
                <span class="inline-flex items-center gap-1 text-emerald-700">
                  <Icon name="shield" size={13} /> 2FA
                </span>
              {:else if m.two_factor_enabled === false}
                <span
                  class="inline-flex items-center gap-1 {(m.role_level ?? 0) >= 50
                    ? 'font-medium text-red-700'
                    : 'text-ink-500'}"
                >
                  <Icon name="shield" size={13} /> Tanpa 2FA
                </span>
              {/if}
              {#if m.email_verified_at === null}
                <span class="text-amber-800">Email belum diverifikasi</span>
              {/if}
            </div>
          {:else if c.key === 'joined'}
            <span class="text-sm text-ink-500">{tanggal(m.created_at)}</span>
          {:else if c.key === 'deleted'}
            <span class="text-sm text-ink-500">{tanggal(m.deleted_at)}</span>
          {:else if c.key === 'actions'}
            {#if showArchive}
              <RowActions
                primary={{
                  label: busyId === m.id ? 'Memulihkan…' : 'Pulihkan',
                  icon: 'refresh',
                  onclick: () => void pulihkan(m),
                }}
                rest={$can('delete', 'team')
                  ? [
                      {
                        label: 'Hapus permanen',
                        icon: 'close' as const,
                        danger: true,
                        onclick: () => (purgeTarget = m),
                      },
                    ]
                  : []}
              />
            {:else}
              <RowActions
                primary={{
                  label: 'Ubah role',
                  icon: 'cog',
                  onclick: () => bukaEdit(m),
                  disabled: !$can('update', 'team') || !canManage(myLevel, m),
                  disabledReason: !$can('update', 'team')
                    ? 'Anda tidak punya izin mengubah anggota tim'
                    : 'Role anggota ini setara atau lebih tinggi dari Anda',
                }}
                rest={$can('delete', 'team') && canManage(myLevel, m)
                  ? [
                      {
                        label: 'Hapus dari tim',
                        icon: 'close' as const,
                        danger: true,
                        onclick: () => (removeTarget = m),
                      },
                    ]
                  : []}
              />
            {/if}
          {/if}
        {/snippet}
      </DataTable>

      {#if !showArchive && pelanggan.length}
        <!-- Transparansi: pelanggan tidak dibuang diam-diam, tapi juga tidak
             dihitung sebagai staf. Halaman lama mencampur keduanya. -->
        <details class="mt-3 border-t border-ink-100 pt-3">
          <summary
            class="focus-ring cursor-pointer rounded text-sm font-medium text-ink-600 hover:text-ink-900"
          >
            {pelanggan.length} akun pelanggan punya baris keanggotaan di tenant ini
          </summary>
          <ul class="mt-2 space-y-1 text-sm text-ink-500">
            {#each pelanggan as p (p.id)}
              <li class="flex flex-wrap items-center gap-2">
                <span class="text-ink-700">{p.name || 'Tanpa nama'}</span>
                <span>{p.email}</span>
                <Badge label={p.role_name || p.role || 'Customer'} tone="neutral" />
              </li>
            {/each}
          </ul>
          <p class="mt-2 text-sm text-ink-500">
            Kelola akun ini dari modul Pelanggan. Role Customer tidak bisa ditetapkan dari halaman
            tim.
          </p>
        </details>
      {/if}
    </Card>
  </div>
</AppShell>

<Modal bind:show={inviteOpen} title="Tambah anggota tim" width="440px">
  <div class="ds-scope space-y-1">
    <Field
      id="inv-nama"
      label="Nama"
      value={inviteName}
      onchange={(v) => (inviteName = v)}
      placeholder="Nama lengkap"
    />
    <Field
      id="inv-email"
      label="Email"
      type="email"
      value={inviteEmail}
      onchange={(v) => (inviteEmail = v)}
      placeholder="nama@perusahaan.com"
      help="Dipakai untuk masuk ke panel."
    />
    <Field
      id="inv-role"
      label="Role"
      type="select"
      value={inviteRoleId}
      options={assignableRoles.map((r) => ({ value: r.id, label: `${r.name} (level ${r.level})` }))}
      onchange={(v) => (inviteRoleId = v)}
      help="Hanya role di level Anda atau di bawahnya. Role Customer dikelola dari modul Pelanggan."
    />
    <Field
      id="inv-pass"
      label="Kata sandi awal"
      type="password"
      value={invitePassword}
      onchange={(v) => (invitePassword = v)}
      help="Kosongkan untuk mengirim undangan lewat email."
    />
  </div>
  {#snippet footer()}
    <div class="ds-scope flex justify-end gap-2">
      <Button variant="ghost" onclick={() => (inviteOpen = false)}>Batal</Button>
      <Button
        variant="primary"
        loading={inviting}
        disabled={!inviteName || !inviteEmail || !inviteRoleId}
        onclick={() => void kirimUndangan()}
      >
        Tambah
      </Button>
    </div>
  {/snippet}
</Modal>

<Modal bind:show={editOpen} title="Ubah role anggota" width="420px">
  <div class="ds-scope">
    {#if editTarget}
      <p class="mb-3 text-sm text-ink-600">
        {editTarget.name || editTarget.email} — saat ini
        <span class="font-medium text-ink-900">{editTarget.role_name || editTarget.role}</span>
      </p>
      <Field
        id="edit-role"
        label="Role baru"
        type="select"
        value={editRoleId}
        options={assignableRoles.map((r) => ({
          value: r.id,
          label: `${r.name} (level ${r.level})`,
        }))}
        onchange={(v) => (editRoleId = v)}
        help="Mengubah role langsung mengubah izin yang dimiliki akun ini."
      />
    {/if}
  </div>
  {#snippet footer()}
    <div class="ds-scope flex justify-end gap-2">
      <Button variant="ghost" onclick={() => (editOpen = false)}>Batal</Button>
      <Button
        variant="primary"
        loading={savingRole}
        disabled={!editRoleId || editRoleId === editTarget?.role_id}
        onclick={() => void simpanRole()}
      >
        Simpan
      </Button>
    </div>
  {/snippet}
</Modal>

<Modal
  show={removeTarget !== null}
  title="Hapus dari tim"
  width="420px"
  onclose={() => (removeTarget = null)}
>
  <div class="ds-scope text-sm text-ink-700">
    {#if removeTarget}
      <p>
        <span class="font-medium text-ink-900">{removeTarget.name || removeTarget.email}</span>
        kehilangan akses ke panel. Keanggotaannya masuk arsip dan bisa dipulihkan.
      </p>
      <p class="mt-2 text-ink-500">Akun penggunanya sendiri tidak dihapus.</p>
    {/if}
  </div>
  {#snippet footer()}
    <div class="ds-scope flex justify-end gap-2">
      <Button variant="ghost" onclick={() => (removeTarget = null)}>Batal</Button>
      <Button variant="danger" loading={removing} onclick={() => void hapus()}>Hapus</Button>
    </div>
  {/snippet}
</Modal>

<Modal
  show={purgeTarget !== null}
  title="Hapus permanen"
  width="420px"
  onclose={() => (purgeTarget = null)}
>
  <div class="ds-scope text-sm text-ink-700">
    {#if purgeTarget}
      <p>
        Baris keanggotaan
        <span class="font-medium text-ink-900">{purgeTarget.name || purgeTarget.email}</span>
        dihapus permanen dan tidak bisa dipulihkan.
      </p>
    {/if}
  </div>
  {#snippet footer()}
    <div class="ds-scope flex justify-end gap-2">
      <Button variant="ghost" onclick={() => (purgeTarget = null)}>Batal</Button>
      <Button variant="danger" loading={purging} onclick={() => void hapusPermanen()}>
        Hapus permanen
      </Button>
    </div>
  {/snippet}
</Modal>
