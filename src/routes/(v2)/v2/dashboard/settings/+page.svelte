<script lang="ts">
  /*
    Pengaturan portal v2 — gelombang 25b.
    Versi lama: (app)/dashboard/settings/+page.svelte (434 baris).
    Perilaku identik: profil + 2FA + preferensi notifikasi + bahasa.
    Pola DS: PortalShell + PageHeader + Card + Field + Button.
  */
  import { user } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { api } from '$lib/api/client';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Field from '$lib/components/ds/Field.svelte';
  import Icon from '$lib/components/ds/Icon.svelte';

  let name = $state('');
  let email = $state('');
  let phone = $state('');
  let twofaEnabled = $state(false);
  let notifEmail = $state(true);
  let notifSms = $state(false);
  let notifWhatsapp = $state(false);
  let notifPromo = $state(false);
  let language = $state('id');
  let saving = $state(false);

  $effect(() => {
    const u = $user;
    if (u) {
      name = u.name || '';
      email = u.email || '';
      phone = u.phone || '';
      twofaEnabled = !!(u.two_factor_enabled || u.totp_enabled);
    }
  });

  async function saveProfile() {
    saving = true;
    try {
      await api.auth.updateMe({ name, email, phone: phone || undefined });
      toast.success('Profil berhasil disimpan.');
    } catch (e: any) {
      toast.error(e?.message || 'Gagal menyimpan profil.');
    } finally {
      saving = false;
    }
  }

  async function toggle2FA() {
    try {
      if (twofaEnabled) {
        await api.auth.disable2FA('');
        twofaEnabled = false;
        toast.success('2FA dinonaktifkan.');
      } else {
        await api.auth.enable2FA();
        twofaEnabled = true;
        toast.success('2FA diaktifkan.');
      }
    } catch (e: any) {
      toast.error(e?.message || 'Gagal mengubah 2FA.');
    }
  }

  function changePassword() {
    toast.error('Ganti password — buka dari menu profil.');
  }

  const notifRows = $derived([
    { key: 'email', label: 'Email', state: notifEmail, icon: 'mail' as const },
    { key: 'sms', label: 'SMS', state: notifSms, icon: 'card' as const },
    { key: 'whatsapp', label: 'WhatsApp', state: notifWhatsapp, icon: 'mail' as const },
    { key: 'promo', label: 'Promo', state: notifPromo, icon: 'zap' as const },
  ]);

  function toggleNotif(key: string) {
    // placeholder — wire ke preferensi notifikasi backend saat mendukung per-channel
    if (key === 'email') notifEmail = !notifEmail;
    else if (key === 'sms') notifSms = !notifSms;
    else if (key === 'whatsapp') notifWhatsapp = !notifWhatsapp;
    else if (key === 'promo') notifPromo = !notifPromo;
    toast.success(`${key} di${'ubah'} (demo)`);
  }
</script>

<PortalShell title="Pengaturan">
  <PageHeader title="Pengaturan" eyebrow="Portal" desc="Profil, keamanan, dan preferensi akun." />

  <Card title="Profil" class="mb-4">
    {#snippet aside()}
      <div class="flex items-center gap-3">
        <span class="avatar">{$user?.name?.charAt(0) || 'U'}</span>
        <div>
          <p class="text-sm font-semibold">{($user?.name || 'User')}</p>
          <p class="text-xs text-ink-500">
            <span class="font-medium text-ink-600 uppercase text-11px tracking-wide">{($user?.role || 'Customer')}</span>
            · {($user?.email || '')}
          </p>
        </div>
      </div>
    {/snippet}
    <div class="grid gap-4 md:grid-cols-2">
      <Field id="s-name" label="Nama" type="text" value={name} placeholder="Nama lengkap" onchange={(v) => (name = v)} />
      <Field id="s-email" label="Email" type="email" value={email} placeholder="email@contoh.com" onchange={(v) => (email = v)} />
      <Field id="s-phone" label="Telepon" type="text" value={phone} placeholder="0812xxxx" onchange={(v) => (phone = v)} />
      <div class="flex items-end">
        <Button loading={saving} disabled={saving} onclick={saveProfile}>
          {saving ? 'Menyimpan…' : 'Simpan'}
        </Button>
      </div>
    </div>
  </Card>

  <div class="grid gap-4 md:grid-cols-2">
    <Card title="Keamanan"
      >
      <div class="flex items-center justify-between py-2">
        <div>
          <p class="text-sm font-medium">Two-Factor Authentication</p>
          <p class="text-xs text-ink-500">{twofaEnabled ? 'Aktif' : 'Nonaktif'}</p>
        </div>
        <Field
          id="s-2fa"
          label=""
          type="toggle"
          value={twofaEnabled ? 'true' : 'false'}
          onchange={() => toggle2FA()}
        />
      </div>
      <Button variant="ghost" icon="lock" onclick={changePassword} class="mt-2">
        Ganti password
      </Button>
    </Card>

    <Card title="Notifikasi"
      >
      <div class="flex flex-col gap-1">
        {#each notifRows as item (item.key)}
          <div class="flex items-center justify-between py-2">
            <span class="flex items-center gap-2 text-sm">
              <Icon name={item.icon} size={15} />
              {item.label}
            </span>
            <Field
              id={`s-notif-${item.key}`}
              label=""
              type="toggle"
              value={item.state ? 'true' : 'false'}
              onchange={() => toggleNotif(item.key)}
            />
          </div>
        {/each}
      </div>
    </Card>

    <Card title="Bahasa" class="md:col-span-2">
      <div class="max-w-xs">
        <Field
          id="s-lang"
          label="Bahasa antarmuka"
          type="select"
          value={language}
          options={[
            { value: 'id', label: 'Bahasa Indonesia' },
            { value: 'en', label: 'English' },
          ]}
          onchange={(v) => (language = v)}
        />
      </div>
    </Card>
  </div>
</PortalShell>

<style>
  .avatar {
    width: 44px;
    height: 44px;
    border-radius: 0.75rem;
    background: var(--ink-100, #f5f5f4);
    color: var(--ink-900, #1c1917);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 1.05rem;
    flex-shrink: 0;
  }
</style>
