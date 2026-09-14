<script lang="ts">
  /*
    Panel "Merek & Domain" v2.

    Peta field identik dengan tab `branding` lama: nama tenant, logo, domain
    kustom (banner PRO + panel status verifikasi). State lewat `settingsStore`
    supaya SaveBar global melihat 'tenant' + 'app_logo_path'.

    Layout mengikuti tab skema: baris Field label-kiri (grid 15rem/1fr), dua
    seksi dengan subjudul, jadi tab ini terasa satu keluarga dengan tab lain.
  */
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { toast } from '$lib/stores/toast';
  import CustomDomainStatusPanel from '$lib/components/domain/CustomDomainStatusPanel.svelte';
  import { Button, Field, Icon } from '$lib/components/ds';
  import { changeSetting, settingsStore, uploadLogo } from '$lib/stores/settingsStore.svelte';

  let uploading = $state(false);
  let cnameCopied = $state(false);

  const cnameTarget = import.meta.env.VITE_CNAME_DNS_TARGET || 'cname.tridigitals.com';

  const tenantDirty = $derived(
    (settingsStore.values['tenant_name'] ?? '') !== (settingsStore.tenantInfo?.name ?? ''),
  );
  const domainDirty = $derived(
    (settingsStore.values['custom_domain'] ?? '') !== (settingsStore.tenantInfo?.custom_domain ?? ''),
  );

  function edit(key: string, value: string) {
    if (!changeSetting(key, value)) {
      toast.error(settingsStore.emailReadiness.reason || $t('admin.settings.security.require_email_verification_not_ready'));
    }
  }

  async function copyCname() {
    try {
      await navigator.clipboard.writeText(cnameTarget);
      cnameCopied = true;
      setTimeout(() => (cnameCopied = false), 1600);
    } catch {
      toast.error($t('admin.settings.branding.copy_failed'));
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
      input.value = '';
    }
  }
</script>

<div class="space-y-2">
  <section>
    <h3 class="text-base font-semibold text-ink-900">{ $t('admin.settings.branding.section_brand_title') }</h3>
    <p class="mt-0.5 text-sm text-ink-500">{ $t('admin.settings.branding.section_brand_desc') }</p>

    <Field
      id="set-tenant_name"
      label={ $t('admin.settings.keys.tenant_name') }
      type="text"
      value={ settingsStore.values['tenant_name'] ?? '' }
      dirty={tenantDirty}
      onchange={(v) => edit('tenant_name', v)}
    />

    <!-- Baris logo: grid yang sama dgn Field (label-kiri) agar rata kolom. -->
    <div class="grid gap-1.5 border-t border-ink-100 py-3.5 sm:grid-cols-[minmax(0,15rem)_minmax(0,1fr)] sm:gap-x-6">
      <div class="flex items-start gap-1.5 sm:pt-1.5">
        <span class="text-base font-medium text-ink-800" id="set-logo-label">{ $t('common.logo') }</span>
        {#if settingsStore.logoDirty}
          <span
            class="mt-1.5 size-1.5 shrink-0 rounded-full bg-amber-500"
            title={ $t('components.save_bar.unsaved_field') }
            aria-label={ $t('components.save_bar.unsaved_field') }
          ></span>
        {/if}
      </div>

      <div class="min-w-0">
        <div class="flex flex-wrap items-center gap-4">
          <!-- Kotak cerewet papan catur: transparansi logo terlihat, bukan kotak putih kosong. -->
          <div
            class="grid size-16 shrink-0 place-items-center rounded-lg ring-1 ring-ink-200"
            style="background: {settingsStore.logoBase64
              ? 'white'
              : 'repeating-conic-gradient(#f1f5f9 0% 25%, #ffffff 0% 50%) 50% / 14px 14px'}"
            aria-hidden="true"
          >
            {#if settingsStore.logoBase64}
              <img src={settingsStore.logoBase64} alt="" class="max-h-12 max-w-12 object-contain" />
            {:else}
              <Icon name="image" size={20} class="text-ink-300" />
            {/if}
          </div>

          <div class="min-w-0">
            <label
              for="set-logo-file"
              class="focus-ring inline-flex cursor-pointer items-center gap-2 rounded-lg bg-white px-3 py-2 text-sm font-medium text-ink-800 ring-1 ring-inset ring-ink-200 transition-colors hover:bg-ink-50"
            >
              <Icon name={uploading ? 'refresh' : 'plus'} size={14} class={uploading ? 'animate-spin text-ink-400' : 'text-ink-400'} />
              { uploading ? ($t('common.uploading') || 'Mengunggah…') : ($t('admin.settings.branding.choose_logo') || 'Pilih berkas') }
            </label>
            <input type="file" id="set-logo-file" accept="image/png,image/jpeg,image/webp,image/svg+xml" class="sr-only" onchange={pickLogo} disabled={uploading} aria-labelledby="set-logo-label" />
            <p class="mt-1.5 text-sm text-ink-400">{ $t('admin.settings.branding.logo_help') }</p>
          </div>
        </div>

        {#if settingsStore.logoBase64}
          <!-- Pratinjau pemakaian nyata: chip navigasi + kartu login. -->
          <div class="mt-4 flex flex-wrap items-end gap-6">
            <div>
              <div class="mb-1.5 text-[11px] font-semibold tracking-wide text-ink-400 uppercase">{ $t('admin.settings.branding.preview_sidebar') }</div>
              <div class="flex items-center gap-2 rounded-lg bg-ink-900 px-2.5 py-2">
                <span class="grid size-6 place-items-center overflow-hidden rounded-md bg-white">
                  <img src={settingsStore.logoBase64} alt="" class="max-h-5 max-w-5 object-contain" />
                </span>
                <span class="text-sm font-medium text-white">{ settingsStore.values['tenant_name'] || settingsStore.tenantInfo?.name || '—' }</span>
              </div>
            </div>
            <div>
              <div class="mb-1.5 text-[11px] font-semibold tracking-wide text-ink-400 uppercase">{ $t('admin.settings.branding.preview_login') }</div>
              <div class="flex w-44 flex-col items-center gap-2 rounded-xl bg-white p-4 ring-1 ring-ink-200">
                <span class="grid size-12 place-items-center overflow-hidden rounded-lg ring-1 ring-ink-100">
                  <img src={settingsStore.logoBase64} alt="" class="max-h-10 max-w-10 object-contain" />
                </span>
                <div class="h-2 w-20 rounded-full bg-ink-200"></div>
                <div class="h-2 w-14 rounded-full bg-ink-100"></div>
              </div>
            </div>
          </div>
        {/if}
      </div>
    </div>
  </section>

  <div class="border-t border-ink-100" role="presentation"></div>

  <section>
    <h3 class="text-base font-semibold text-ink-900">{ $t('admin.settings.branding.section_domain_title') }</h3>
    <p class="mt-0.5 text-sm text-ink-500">{ $t('admin.settings.branding.section_domain_desc') }</p>

    {#if settingsStore.customDomainAccess}
      <Field
        id="set-custom_domain"
        label={ $t('admin.settings.keys.custom_domain') }
        type="text"
        value={ settingsStore.values['custom_domain'] ?? '' }
        placeholder={ $t('admin.settings.placeholders.custom_domain') }
        help={ $t('admin.settings.branding.custom_domain_help_prefix') + ' ' + cnameTarget + ' ' + $t('admin.settings.branding.custom_domain_help_suffix') }
        dirty={domainDirty}
        onchange={(v) => edit('custom_domain', v)}
      />

      <div class="-mt-2 mb-4 flex items-center gap-2 sm:pl-[calc(15rem+1.5rem)]">
        <code class="rounded-md bg-ink-50 px-2 py-1 font-mono text-sm text-ink-700 ring-1 ring-inset ring-ink-200">{cnameTarget}</code>
        <button
          type="button"
          class="focus-ring inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-sm font-medium text-ink-500 transition-colors hover:bg-ink-50 hover:text-ink-800"
          onclick={copyCname}
        >
          <Icon name={cnameCopied ? 'check' : 'copy'} size={13} />
          { cnameCopied ? ($t('common.copied') || 'Disalin') : ($t('common.copy') || 'Salin') }
        </button>
      </div>

      <div class="pb-2 sm:pl-[calc(15rem+1.5rem)]">
        <CustomDomainStatusPanel
          customDomain={settingsStore.tenantInfo?.custom_domain || settingsStore.values['custom_domain'] || null}
          status={settingsStore.tenantInfo?.custom_domain_status || 'none'}
          failureReason={settingsStore.tenantInfo?.custom_domain_failure_reason || null}
          verifiedAt={settingsStore.tenantInfo?.custom_domain_verified_at || null}
        />
      </div>
    {:else}
      <div class="my-2 flex flex-wrap items-center gap-4 rounded-xl bg-amber-50 p-4 ring-1 ring-inset ring-amber-200">
        <span class="grid size-10 shrink-0 place-items-center rounded-lg bg-amber-100">
          <Icon name="lock" size={18} class="text-amber-700" />
        </span>
        <div class="min-w-0 flex-1">
          <p class="text-base font-medium text-amber-900">{ $t('admin.settings.branding.custom_domain_pro_title') }</p>
          <p class="mt-0.5 text-sm text-amber-800">{ $t('admin.settings.branding.custom_domain_pro_desc') }</p>
        </div>
        <Button variant="secondary" size="sm" onclick={() => void goto('/v2/admin/subscription')}>
          { $t('common.upgrade_plan') }
        </Button>
      </div>
      <Field
        id="set-custom_domain_locked"
        label={ $t('admin.settings.keys.custom_domain') }
        type="text"
        value={ settingsStore.values['custom_domain'] ?? '' }
        placeholder={ $t('admin.settings.placeholders.locked') }
        disabled={true}
        onchange={() => {}}
      />
    {/if}
  </section>
</div>
