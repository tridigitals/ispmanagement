<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { user, isAuthenticated } from '$lib/stores/auth';
  import { get } from 'svelte/store';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { t } from 'svelte-i18n';

  onMount(() => {
    if (!$isAuthenticated) {
      // Redirect to login
      goto('/login');
      return;
    }

    const u = get(user);
    const role = String(u?.role || '').toLowerCase();
    const ctx = resolveTenantContext({
      hostname: typeof window !== 'undefined' ? window.location.hostname : 'localhost',
      userTenantSlug: u?.tenant_slug,
    });

    let target = role === 'admin' ? `${ctx.tenantPrefix}/admin` : `${ctx.tenantPrefix}/dashboard`;
    const current = typeof window !== 'undefined' ? window.location.pathname : '/dashboard';
    if (target === current) {
      target = `${ctx.tenantPrefix}/profile`;
    }
    if (target === current) {
      target = '/';
    }

    goto(target);
  });
</script>

<div class="redirect-container">
  <p>{$t('pages.dashboard.redirecting') || 'Redirecting to your dashboard...'}</p>
</div>

<style>
  .redirect-container {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background: var(--bg-primary);
    color: var(--text-primary);
  }
</style>
