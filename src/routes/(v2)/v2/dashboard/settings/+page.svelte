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
  import { t } from 'svelte-i18n';

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
      toast.success($t('profile.portal_settings.t_saved'));
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
        toast.success($t('profile.portal_settings.t_2fa_off'));
      } else {
        await api.auth.enable2FA();
        twofaEnabled = true;
        toast.success($t('profile.portal_settings.t_2fa_on'));
      }
    } catch (e: any) {
      toast.error(e?.message || 'Gagal mengubah 2FA.');
    }
  }

  function changePassword() {
    toast.error($t('profile.portal_settings.t_pw_hint'));
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

<PortalShell title={ $t('profile.portal_settings.title') }>
  <PageHeader title={ $t('profile.portal_settings.title') } desc={ $t('profile.portal_settings.desc') } />

  <Card title={ $t('profile.portal_settings.tab_profile') } class="mb-4">
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
      <Field id="s-name" label={ $t('profile.portal_settings.name') } type="text" value={name} placeholder={ $t('profile.portal_settings.name_ph') } onchange={(v) => (name = v)} />
      <Field id="s-email" label="Email" type="email" value={email} placeholder={ $t('profile.portal_settings.email_ph') } onchange={(v) => (email = v)} />
      <Field id="s-phone" label={ $t('profile.portal_settings.phone') } type="text" value={phone} placeholder="0812xxxx" onchange={(v) => (phone = v)} />
      <div class="flex items-end">
        <Button loading={saving} disabled={saving} onclick={saveProfile}>
          {saving ? $t('common.saving') : $t('profile.portal_settings.save')}
        </Button>
      </div>
    </div>
  </Card>

  <div class="grid gap-4 md:grid-cols-2">
    <Card title={ $t('profile.portal_settings.tab_security') }
      >
      <div class="flex items-center justify-between py-2">
        <div>
          <p class="text-sm font-medium">Two-Factor Authentication</p>
          <p class="text-xs text-ink-500">{twofaEnabled ? $t('profile.portal_settings.twofa_on') : $t('profile.portal_settings.twofa_off')}</p>
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
        { $t('profile.portal_settings.change_pw') }
      </Button>
    </Card>

    <Card title={ $t('profile.portal_settings.tab_notif') }
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

    <Card title={ $t('profile.portal_settings.tab_lang') } class="md:col-span-2">
      <div class="max-w-xs">
        <Field
          id="s-lang"
          label={ $t('profile.portal_settings.ui_lang') }
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
