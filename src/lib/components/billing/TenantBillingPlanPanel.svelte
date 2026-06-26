<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';

  let { openSubscription }: { openSubscription: () => void } = $props();
</script>

<div class="billing-plan-hero">
  <div>
    <span class="section-label">
      {$t('admin.settings.billing_plan.eyebrow')}
    </span>
    <h3>
      {$t('admin.settings.billing_plan.title')}
    </h3>
    <p class="muted">
      {$t('admin.settings.billing_plan.description')}
    </p>
  </div>

  <div class="billing-plan-actions">
    <button class="btn btn-primary btn-sm" type="button" onclick={openSubscription}>
      <Icon name="credit-card" size={16} />
      {$t('admin.settings.billing_plan.primary_cta')}
    </button>
    <button class="btn btn-secondary btn-sm" type="button" onclick={openSubscription}>
      <Icon name="arrow-right" size={16} />
      {$t('admin.settings.billing_plan.secondary_cta')}
    </button>
  </div>
</div>

<div class="billing-plan-grid">
  <article class="billing-plan-card">
    <div class="billing-plan-card__icon">
      <Icon name="layers" size={18} />
    </div>
    <div>
      <h4>
        {$t('admin.settings.billing_plan.cards.plan.title')}
      </h4>
      <p>
        {$t('admin.settings.billing_plan.cards.plan.desc')}
      </p>
    </div>
  </article>

  <article class="billing-plan-card">
    <div class="billing-plan-card__icon">
      <Icon name="receipt" size={18} />
    </div>
    <div>
      <h4>
        {$t('admin.settings.billing_plan.cards.payments.title')}
      </h4>
      <p>
        {$t('admin.settings.billing_plan.cards.payments.desc') ||
          "Review your tenant's SaaS billing history and upcoming platform payment responsibilities."}
      </p>
    </div>
  </article>

  <article class="billing-plan-card">
    <div class="billing-plan-card__icon">
      <Icon name="folder-kanban" size={18} />
    </div>
    <div>
      <h4>
        {$t('admin.settings.billing_plan.cards.operations.title')}
      </h4>
      <p>
        {$t('admin.settings.billing_plan.cards.operations.desc')}
      </p>
    </div>
  </article>
</div>

<style>
  .billing-plan-hero {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    padding: 1.25rem;
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    background: var(--bg-surface);
  }

  .billing-plan-hero h3 {
    margin: 0.35rem 0 0.5rem;
    font-size: 1.2rem;
    font-weight: 750;
    color: var(--text-primary);
  }

  .billing-plan-actions {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .billing-plan-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1rem;
    margin-top: 1rem;
  }

  .billing-plan-card {
    display: flex;
    gap: 0.9rem;
    padding: 1rem 1.05rem;
    border-radius: var(--radius-lg);
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.02);
  }

  .billing-plan-card h4 {
    margin: 0 0 0.35rem;
    font-size: 0.98rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .billing-plan-card p {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.55;
  }

  .billing-plan-card__icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2.25rem;
    height: 2.25rem;
    border-radius: 999px;
    color: var(--color-primary);
    background: color-mix(in srgb, var(--color-primary) 10%, transparent);
    flex-shrink: 0;
  }

  @media (max-width: 900px) {
    .billing-plan-hero {
      flex-direction: column;
    }

    .billing-plan-actions {
      width: 100%;
      justify-content: stretch;
    }

    .billing-plan-actions :global(.btn) {
      flex: 1 1 100%;
      justify-content: center;
    }

    .billing-plan-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
