<script lang="ts">
  /*
    Role & izin v2.

    Versi lama: `(app)/admin/roles/+page.svelte` — 977 baris. Empat masalah yang
    terukur di data produksi, tiga di antaranya bukan soal tampilan:

    1. KATALOG IZIN MENAWARKAN CENTANG YANG TIDAK BISA DISIMPAN.
       `GET /api/permissions` mengembalikan 83 baris. Enam di antaranya alias
       murni (`network_routers:read|manage|manage_radius_secret`,
       `storage:read|upload|delete`): `normalize_permission_keys` menulisnya
       sebagai kunci granular lain, jadi kunci aslinya tidak pernah kembali dari
       DB. Probe: kirim 83 kunci → tersimpan 77. Centangnya kosong lagi saat
       dimuat ulang dan terlihat seperti gagal simpan.
       Diperbaiki di sumbernya: `list_permissions()` menyaring alias
       (`RoleService::is_alias_permission_key`), bukan di UI.

    2. IZIN YANG TERCENTANG TIDAK SELALU BERLAKU.
       `has_permission`/`get_user_permissions` membandingkan
       `role_permissions.permission_id` mentah dengan string "resource:action".
       Itu hanya kebetulan cocok untuk baris yang dibuat `seed_permissions()`.
       Baris dari migrasi memakai id gaya `perm_communication_templates_read`,
       sehingga izin communication_templates milik Admin/CS tidak pernah cocok
       walau tercentang di layar ini. Kedua kueri kini JOIN ke `permissions` dan
       membandingkan `p.resource || ':' || p.action`.

    3. GAGAL HAPUS/SIMPAN TERLIHAT SEPERTI SUKSES ATAU ERROR SERVER.
       `delete_role` mengembalikan Ok(true) untuk role yang tidak ada (toast
       "berhasil", baris hilang dari tabel) dan Ok(false) untuk role sistem —
       tidak bisa dibedakan dari sukses. Menghapus role yang masih dipakai
       anggota memicu FK violation → HTTP 500 "A database error occurred."
       Sekarang: 404 NotFound, 403 Forbidden, 409 Conflict dengan jumlah anggota.

    4. LAYAR TIDAK MENUNJUKKAN APA YANG MEMBLOKIR PENYUNTINGAN.
       Sembilan role di DB ini SEMUANYA `is_system = true`, jadi setiap
       penyuntingan oleh non-super-admin ditolak backend — tapi UI lama baru
       memberi tahu setelah Simpan gagal. Di sini tombol mati dengan alasan.

    Aturan pengelompokan + izin pindah ke `$lib/utils/rolesMatrix` (19 tes unit).
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import { extractApiErrorMessage } from '$lib/api/core';
  import type { Permission, Role, TeamMember } from '$lib/api/types';
  import { can, user } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import Modal from '$lib/components/ui/Modal.svelte';
  import {
    AppShell,
    Badge,
    Button,
    Card,
    DataTable,
    Icon,
    PageHeader,
    RowActions,
    StatTile,
    type Column,
  } from '$lib/components/ds';
  import {
    canDeleteRole,
    canEditRole,
    groupCoverage,
    groupPermissions,
    levelTone,
    summarizeRoles,
    type PermissionGroup,
  } from '$lib/utils/rolesMatrix';

  let roles = $state<Role[]>([]);
  let permissions = $state<Permission[]>([]);
  let members = $state<TeamMember[]>([]);
  let loading = $state(true);
  let search = $state('');

  /* Editor */
  let editOpen = $state(false);
  let editing = $state<Role | null>(null);
  let form = $state({ name: '', description: '', level: 0 });
  let checked = $state<Set<string>>(new Set());
  let saving = $state(false);
  let openGroups = $state<Set<string>>(new Set());

  let deleteTarget = $state<Role | null>(null);
  let deleting = $state(false);

  const isSuperAdmin = $derived($user?.is_super_admin === true);

  /* `User` tidak membawa level role, jadi diturunkan dari baris keanggotaan saya
     (pola yang sama dipakai halaman Anggota tim). Fallback ke pencocokan nama role
     tenant kalau daftar anggota tidak bisa dibaca karena izin. */
  const myLevel = $derived.by(() => {
    const me = members.find((m) => m.email === $user?.email);
    if (me && typeof me.role_level === 'number') return me.role_level;
    if (me?.role_id) return roles.find((r) => r.id === me.role_id)?.level ?? 0;
    const named = $user?.tenant_role ?? $user?.role;
    return roles.find((r) => r.name === named)?.level ?? 0;
  });

  /* Berapa anggota memakai role ini. Dipakai untuk memblokir hapus SEBELUM
     backend menolak dengan 409, dan untuk menandai role yang tidak terpakai. */
  const memberCounts = $derived.by(() => {
    const counts: Record<string, number> = {};
    for (const m of members) {
      if (!m.role_id) continue;
      counts[m.role_id] = (counts[m.role_id] ?? 0) + 1;
    }
    return counts;
  });

  const ringkasan = $derived(summarizeRoles(roles, memberCounts));
  const groups = $derived(groupPermissions(permissions));

  const terlihat = $derived.by(() => {
    const q = search.trim().toLowerCase();
    const list = q
      ? roles.filter(
          (r) =>
            r.name.toLowerCase().includes(q) || (r.description ?? '').toLowerCase().includes(q)
        )
      : roles;
    return [...list].sort((a, b) => b.level - a.level || a.name.localeCompare(b.name));
  });

  const columns: Column[] = [
    { key: 'role', label: 'Role' },
    { key: 'level', label: 'Level', num: true, width: '90px' },
    { key: 'members', label: 'Dipakai', num: true, width: '110px' },
    { key: 'perms', label: 'Izin', num: true, width: '110px' },
    { key: 'origin', label: 'Asal', hideSm: true, width: '130px' },
    { key: 'actions', label: '', align: 'right', width: '150px' },
  ];

  async function load() {
    loading = true;
    try {
      /* Tiga permintaan paralel. `members` bukan hiasan: tanpa itu tombol hapus
         tidak bisa tahu role masih dipakai, dan itulah penyebab 500 yang lama. */
      const [r, p, m] = await Promise.all([
        api.roles.list(),
        api.roles.getPermissions(),
        $can('read', 'team') ? api.team.list() : Promise.resolve([] as TeamMember[]),
      ]);
      roles = r ?? [];
      permissions = p ?? [];
      members = m ?? [];
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, 'Gagal memuat role'));
    } finally {
      loading = false;
    }
  }

  function bukaEdit(r: Role) {
    editing = r;
    form = { name: r.name, description: r.description ?? '', level: r.level };
    checked = new Set(r.permissions ?? []);
    /* Buka grup yang sudah punya izin supaya operator langsung melihat
       keadaan sekarang, bukan sembilan accordion tertutup. */
    openGroups = new Set(
      groups.filter((g) => groupCoverage(g, checked).granted > 0).map((g) => g.key)
    );
    editOpen = true;
  }

  function bukaBaru() {
    editing = null;
    form = { name: '', description: '', level: 0 };
    checked = new Set();
    openGroups = new Set();
    editOpen = true;
  }

  function toggleGroup(key: string) {
    const next = new Set(openGroups);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    openGroups = next;
  }

  function togglePerm(key: string) {
    const next = new Set(checked);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    checked = next;
  }

  function toggleGroupAll(g: PermissionGroup) {
    const cov = groupCoverage(g, checked);
    const next = new Set(checked);
    if (cov.granted === cov.total) for (const i of g.items) next.delete(i.key);
    else for (const i of g.items) next.add(i.key);
    checked = next;
  }

  const editGuard = $derived(
    editing
      ? canEditRole(editing, { level: myLevel, isSuperAdmin, canUpdate: $can('update', 'roles') })
      : { allowed: $can('create', 'roles'), reason: 'Tidak punya izin membuat role' }
  );

  async function simpan() {
    if (!editGuard.allowed) return;
    const nama = form.name.trim();
    if (!nama) {
      toast.error('Nama role wajib diisi');
      return;
    }

    saving = true;
    try {
      const perms = [...checked];
      if (editing) {
        await api.roles.update(editing.id, nama, form.description.trim(), form.level, perms);
        toast.success('Role diperbarui');
      } else {
        await api.roles.create(nama, form.description.trim() || undefined, form.level, perms);
        toast.success('Role dibuat');
      }
      editOpen = false;
      await load();
    } catch (e: unknown) {
      /* Backend kini mengirim 403/404/409 dengan pesan yang bisa dibaca
         (bukan lagi 500 "A database error occurred"), jadi tampilkan apa adanya. */
      toast.error(extractApiErrorMessage(e, 'Gagal menyimpan role'));
    } finally {
      saving = false;
    }
  }

  async function hapus() {
    const target = deleteTarget;
    if (!target) return;
    deleting = true;
    try {
      await api.roles.delete(target.id);
      toast.success('Role dihapus');
      deleteTarget = null;
      await load();
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, 'Gagal menghapus role'));
    } finally {
      deleting = false;
    }
  }

  onMount(() => {
    if (!$can('read', 'roles')) {
      goto('/unauthorized');
      return;
    }
    void load();
  });
</script>

<AppShell title="Role & izin">
  <PageHeader
    title="Role & izin"
    eyebrow="Organisasi"
    desc="Apa yang boleh dilakukan tiap role. Penetapan role ke orang ada di halaman Anggota tim."
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={() => void load()}>Muat ulang</Button>
      {#if $can('create', 'roles')}
        <Button variant="primary" icon="plus" onclick={bukaBaru}>Role baru</Button>
      {/if}
    {/snippet}
  </PageHeader>

  <Card>
    <div class="grid grid-cols-2 gap-6 sm:grid-cols-4">
      <StatTile label="Role" value={String(ringkasan.total)} hint="tersedia di tenant ini" />
      <StatTile
        label="Role sistem"
        value={String(ringkasan.system)}
        hint={ringkasan.custom === 0
          ? 'semua role bawaan — hanya Super Admin yang bisa mengubah'
          : 'hanya Super Admin yang bisa mengubah'}
        tone={ringkasan.custom === 0 ? 'warning' : 'neutral'}
      />
      <StatTile label="Role kustom" value={String(ringkasan.custom)} hint="dibuat sendiri" />
      <StatTile
        label="Tidak terpakai"
        value={String(ringkasan.unused)}
        hint={`${ringkasan.totalAssigned} keanggotaan terisi`}
      />
    </div>
  </Card>

  {#if !isSuperAdmin && ringkasan.custom === 0 && !loading}
    <!-- Dinyatakan di muka, bukan setelah Simpan gagal: semua role di tenant ini
         adalah role sistem, jadi non-super-admin tidak bisa mengubah apa pun. -->
    <p class="mt-4 flex items-start gap-2 rounded-xl bg-amber-50 px-4 py-3 text-sm text-amber-900">
      <Icon name="alert" size={16} class="mt-0.5 shrink-0" />
      <span>
        Semua {ringkasan.total} role di sini adalah role sistem. Mengubahnya butuh Super Admin —
        buat role kustom kalau perlu izin yang berbeda.
      </span>
    </p>
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
            placeholder="Cari nama role"
            aria-label="Cari role"
            class="focus-ring h-9 w-full rounded-lg border-0 bg-white pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400"
          />
        </div>
        <span class="text-sm text-ink-500">
          {permissions.length} izin bisa diatur
        </span>
      </div>

      <DataTable
        {columns}
        rows={terlihat}
        {loading}
        emptyTitle="Tidak ada role yang cocok"
        emptyHint="Coba kata kunci lain."
        footNote={`${terlihat.length} dari ${ringkasan.total} role`}
      >
        {#snippet cell(r, c)}
          {#if c.key === 'role'}
            <div class="min-w-0">
              <div class="truncate font-medium text-ink-900">{r.name}</div>
              {#if r.description}
                <div class="truncate text-sm text-ink-500">{r.description}</div>
              {/if}
            </div>
          {:else if c.key === 'level'}
            <Badge label={String(r.level)} tone={levelTone(r.level)} />
          {:else if c.key === 'members'}
            {@const n = memberCounts[r.id] ?? 0}
            <span class={n === 0 ? 'text-ink-400' : 'text-ink-900'}>
              {n === 0 ? '—' : n}
            </span>
          {:else if c.key === 'perms'}
            <span class="text-ink-900">{r.permissions?.length ?? 0}</span>
          {:else if c.key === 'origin'}
            <span class="text-sm text-ink-500">{r.is_system ? 'Sistem' : 'Kustom'}</span>
          {:else if c.key === 'actions'}
            {@const eg = canEditRole(r, {
              level: myLevel,
              isSuperAdmin,
              canUpdate: $can('update', 'roles'),
            })}
            {@const dg = canDeleteRole(
              r,
              { level: myLevel, isSuperAdmin, canDelete: $can('delete', 'roles') },
              memberCounts[r.id] ?? 0
            )}
            <RowActions
              primary={{
                label: eg.allowed ? 'Ubah izin' : 'Lihat izin',
                icon: eg.allowed ? 'cog' : 'search',
                onclick: () => bukaEdit(r),
              }}
              rest={[
                {
                  label: 'Hapus role',
                  icon: 'close' as const,
                  danger: true,
                  disabled: !dg.allowed,
                  disabledReason: dg.reason,
                  onclick: () => (deleteTarget = r),
                },
              ]}
            />
          {/if}
        {/snippet}
      </DataTable>
    </Card>
  </div>
</AppShell>

<Modal bind:show={editOpen} title={editing ? `Izin ${editing.name}` : 'Role baru'} width="720px">
  <div class="space-y-4">
    {#if editing && !editGuard.allowed}
      <p class="flex items-start gap-2 rounded-lg bg-ink-50 px-3 py-2 text-sm text-ink-700">
        <Icon name="lock" size={15} class="mt-0.5 shrink-0" />
        <span>Mode baca. {editGuard.reason}.</span>
      </p>
    {/if}

    <div class="grid gap-3 sm:grid-cols-[1fr_120px]">
      <label class="block">
        <span class="mb-1 block text-sm font-medium text-ink-700">Nama role</span>
        <input
          bind:value={form.name}
          disabled={!editGuard.allowed}
          placeholder="misal Dispatcher"
          class="focus-ring h-9 w-full rounded-lg border-0 bg-white px-2.5 text-base text-ink-900 ring-1 ring-inset ring-ink-200 disabled:bg-ink-50 disabled:text-ink-500"
        />
      </label>
      <label class="block">
        <span class="mb-1 block text-sm font-medium text-ink-700">Level</span>
        <input
          type="number"
          bind:value={form.level}
          disabled={!editGuard.allowed}
          min="0"
          max="100"
          class="focus-ring h-9 w-full rounded-lg border-0 bg-white px-2.5 text-base tabular-nums text-ink-900 ring-1 ring-inset ring-ink-200 disabled:bg-ink-50 disabled:text-ink-500"
        />
      </label>
    </div>

    <label class="block">
      <span class="mb-1 block text-sm font-medium text-ink-700">Keterangan</span>
      <input
        bind:value={form.description}
        disabled={!editGuard.allowed}
        placeholder="Untuk apa role ini dipakai"
        class="focus-ring h-9 w-full rounded-lg border-0 bg-white px-2.5 text-base text-ink-900 ring-1 ring-inset ring-ink-200 disabled:bg-ink-50 disabled:text-ink-500"
      />
    </label>

    <p class="text-sm text-ink-500">
      Level menentukan siapa boleh mengubah siapa: seseorang hanya bisa menyunting role dengan
      level di bawah levelnya sendiri.
    </p>

    <div class="border-t border-ink-100 pt-3">
      <div class="mb-2 flex items-center justify-between">
        <span class="text-sm font-medium text-ink-700">
          Izin · {checked.size} dipilih dari {permissions.length}
        </span>
      </div>

      <div class="max-h-[320px] space-y-1.5 overflow-y-auto pr-1">
        {#each groups as g (g.key)}
          {@const cov = groupCoverage(g, checked)}
          <div class="rounded-lg ring-1 ring-ink-100">
            <div class="flex items-center gap-2 px-3 py-2">
              <button
                type="button"
                onclick={() => toggleGroup(g.key)}
                aria-expanded={openGroups.has(g.key)}
                class="focus-ring flex flex-1 items-center gap-2 rounded text-left"
              >
                <Icon
                  name={openGroups.has(g.key) ? 'more' : 'plus'}
                  size={13}
                  class="text-ink-400"
                />
                <span class="text-base font-medium text-ink-900">{g.label}</span>
                <span
                  class="text-sm {cov.granted === 0
                    ? 'text-ink-400'
                    : cov.granted === cov.total
                      ? 'text-emerald-700'
                      : 'text-ink-600'}"
                >
                  {cov.granted}/{cov.total}
                </span>
              </button>
              {#if editGuard.allowed}
                <button
                  type="button"
                  onclick={() => toggleGroupAll(g)}
                  class="focus-ring rounded px-1.5 py-0.5 text-sm text-ink-500 hover:text-ink-900"
                >
                  {cov.granted === cov.total ? 'Kosongkan' : 'Pilih semua'}
                </button>
              {/if}
            </div>

            {#if openGroups.has(g.key)}
              <div class="grid gap-1 border-t border-ink-100 px-3 py-2 sm:grid-cols-2">
                {#each g.items as item (item.key)}
                  <label
                    class="flex cursor-pointer items-start gap-2 rounded px-1 py-1 text-sm hover:bg-ink-50"
                  >
                    <input
                      type="checkbox"
                      checked={checked.has(item.key)}
                      disabled={!editGuard.allowed}
                      onchange={() => togglePerm(item.key)}
                      class="focus-ring mt-0.5 size-4 shrink-0 rounded"
                    />
                    <span class="min-w-0">
                      <span class="text-ink-900">{item.label}</span>
                      <span class="ml-1 text-ink-400">{item.resource}</span>
                    </span>
                  </label>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  </div>

  {#snippet footer()}
    <Button variant="ghost" onclick={() => (editOpen = false)}>
      {editGuard.allowed ? 'Batal' : 'Tutup'}
    </Button>
    {#if editGuard.allowed}
      <Button variant="primary" loading={saving} onclick={() => void simpan()}>
        {editing ? 'Simpan perubahan' : 'Buat role'}
      </Button>
    {/if}
  {/snippet}
</Modal>

<Modal
  show={deleteTarget !== null}
  title="Hapus role"
  width="420px"
  onclose={() => (deleteTarget = null)}
>
  <p class="text-base text-ink-700">
    Hapus role <span class="font-medium text-ink-900">{deleteTarget?.name}</span>? Tindakan ini
    tidak bisa dibatalkan.
  </p>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (deleteTarget = null)}>Batal</Button>
    <Button variant="danger" loading={deleting} onclick={() => void hapus()}>Hapus</Button>
  {/snippet}
</Modal>


