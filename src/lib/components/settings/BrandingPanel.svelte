<script lang="ts">
  /*
    Panel "Merek & Domain" v2.

    Menggantikan markup inline pada tab `branding` halaman lama
    ((app)/admin/settings/+page.svelte baris ~934). Peta field identik:
    nama tenant, logo, dan domain kustom (termasuk banner PRO + status panel
    verifikasi domain). Bedanya: state tidak lagi lokal — semuanya lewat
    `settingsStore` sehingga SaveBar global melihat perubahan di sini
    (halaman lama menghitung hasChanges dengan logika tenantPatch sendiri;
    di sini `changedKeys()` sudah memasukkan 'tenant' dan 'app_logo_path').
  */
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { toast } from '$lib/stores/toast';
  import CustomDomainStatusPanel from '$lib/components/domain/CustomDomainStatusPanel.svelte';
  import { Button, Field, Icon } from '$lib/components/ds';
  import { changeSetting, settingsStore, uploadLogo } from '$lib/stores/settingsStore.svelte';

  let uploading = $state(false);

  function edit(key: string, value: string) {
    if (!changeSetting(key, value)) {
      toast.error(settingsStore.emailReadiness.reason || $t('admin.settings.security.require_email_verification_not_ready'));
    }
  }

  async function pickLogo(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    uploading = true;
    try {
      const reader = new FileReader();
      const dataUrl = await new Promise<string>((resolve, reject) => {
        reader.onload = () => resolve(String(reader.result));
        reader.onerror = reject;
        reader.readAsDataURL(file);
      });
      await uploadLogo(dataUrl);
      toast.success($t('admin.settings.toasts.logo_uploaded'));
    } catch (error: any) {
      toast.error(error?.message || $t('admin.settings.toasts.logo_upload_failed'));
    } finally {
      uploading = false;
    }
  }
</script>

<div class="space-y-6">
  <Field
    id="set-tenant_name"
    label={ $t('admin.settings.keys.tenant_name') || 'Nama tenant' }
    type="text"
    value={ settingsStore.values['tenant_name'] ?? '' }
    onchange={(v) => edit('tenant_name', v)}
  />

  <div>
    <div class="flex items-center justify-between gap-4">
      <div>
        <p class="text-base font-medium text-ink-900">{ $t('common.logo') || 'Logo' }</p>
        <p class="mt-0.5 text-sm text-ink-500">{ $t('admin.settings.branding.logo_help') || 'PNG/JPG persegi, tampil di sidebar dan halaman login.' }</p>
      </div>
      <label class="focus-ring inline-flex cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium ring-1 ring-inset ring-ink-200 hover:bg-ink-50">
        <Icon name="plus" size={14} class="text-ink-400" />
        { uploading ? ($t('common.uploading') || 'Mengunggah…') : ($t('admin.settings.branding.choose_logo') || 'Pilih berkas') }
        <input type="file" accept="image/*" class="sr-only" onchange={pickLogo} disabled={uploading} />
      </label>
    </div>
    {#if settingsStore.logoBase64}
      <img
        src={settingsStore.logoBase64}
        alt={ $t('common.logo') || 'Logo' }
        class="mt-3 h-16 w-16 rounded-lg object-contain ring-1 ring-ink-200"
      />
    {/if}
  </div>

  <div class="border-t border-ink-100 pt-6">
    {#if settingsStore.customDomainAccess}
      <div class="mt-3 max-w-md">
        <Field
          id="set-custom_domain"
          label={ $t('admin.settings.keys.custom_domain') || 'Domain kustom' }
          type="text"
          value={ settingsStore.values['custom_domain'] ?? '' }
          placeholder={ $t('admin.settings.placeholders.custom_domain') || 'billing.usahaku.com' }
          help={ String($t('admin.settings.branding.custom_domain_help_prefix') || "Point your domain's CNAME record to") +
            ' ' +
            (import.meta.env.VITE_CNAME_DNS_TARGET || 'cname.tridigitals.com') +
            ' ' +
            String($t('admin.settings.branding.custom_domain_help_suffix') || '') }
          onchange={(v) => edit('custom_domain', v)}
        />
      </div>
      <div class="mt-2 max-w-md">
        <CustomDomainStatusPanel
          customDomain={settingsStore.tenantInfo?.custom_domain || settingsStore.values['custom_domain'] || null}
          status={settingsStore.tenantInfo?.custom_domain_status || 'none'}
          failureReason={settingsStore.tenantInfo?.custom_domain_failure_reason || null}
          verifiedAt={settingsStore.tenantInfo?.custom_domain_verified_at || null}
        />
      </div>
    {:else}
      <div class="mt-3 flex items-start gap-3 rounded-lg bg-amber-50 p-4 ring-1 ring-inset ring-amber-200">
        <Icon name="lock" size={18} class="mt-0.5 shrink-0 text-amber-600" />
        <div class="min-w-0 flex-1">
          <p class="text-base font-medium text-amber-900">{ $t('admin.settings.branding.custom_domain_pro_title') }</p>
          <p class="mt-0.5 text-sm text-amber-800">{ $t('admin.settings.branding.custom_domain_pro_desc') }</p>
          <Button
            class="mt-3"
            variant="secondary"
            size="sm"
            onclick={() => void goto('/v2/admin/subscription')}
          >
            { $t('common.upgrade_plan') || 'Upgrade paket' }
          </Button>
        </div>
      </div>
      <div class="mt-3 max-w-md">
        <Field
          id="set-custom_domain_locked"
          label={ $t('admin.settings.keys.custom_domain') || 'Domain kustom' }
          type="text"
          value={ settingsStore.values['custom_domain'] ?? '' }
          placeholder={ $t('admin.settings.placeholders.locked') || 'Terkunci oleh paket' }
          disabled={true}
          onchange={() => {}}
        />
      </div>
    {/if}
  </div>
</div>
