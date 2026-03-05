<script lang="ts">
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/ui/Icon.svelte';

  let {
    title,
    description,
    roadmap = [],
    nextTarget,
    icon = 'radio',
    statusLabel = 'Coming Soon',
  }: {
    title: string;
    description: string;
    roadmap?: string[];
    nextTarget: string;
    icon?: string;
    statusLabel?: string;
  } = $props();
</script>

<div class="service-page fade-in">
  <section class="hero card">
    <div>
      <h1>{title}</h1>
      <p>{description}</p>
      <span class="status soon">{statusLabel}</span>
    </div>
    <div class="hero-actions">
      <button class="btn btn-secondary" type="button" onclick={() => goto('/dashboard/services/order')}>
        <Icon name="arrow-left" size={15} />
        Service Types
      </button>
      <button class="btn btn-secondary" type="button" onclick={() => goto('/dashboard/services')}>
        <Icon name="layers" size={15} />
        My Services
      </button>
    </div>
  </section>

  <section class="card content">
    <div class="service-head">
      <span class="service-icon"><Icon name={icon} size={18} /></span>
      <h3>Planned Scope</h3>
    </div>

    <ul>
      {#each roadmap as item}
        <li>{item}</li>
      {/each}
    </ul>

    <div class="next-box">
      <strong>Next rollout target</strong>
      <p>{nextTarget}</p>
    </div>
  </section>
</div>

<style>
  .service-page {
    max-width: 1080px;
    margin: 0 auto;
    padding: clamp(1rem, 2.2vw, 1.7rem);
    display: grid;
    gap: 0.85rem;
  }

  .card {
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: var(--bg-surface);
    padding: 1rem;
  }

  .hero {
    display: flex;
    justify-content: space-between;
    gap: 0.8rem;
    flex-wrap: wrap;
  }

  .hero h1 {
    margin: 0;
    font-size: clamp(1.2rem, 2.2vw, 1.6rem);
  }

  .hero p {
    margin: 0.3rem 0 0;
    color: var(--text-secondary);
    max-width: 680px;
  }

  .status {
    margin-top: 0.6rem;
    display: inline-flex;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    padding: 0.18rem 0.56rem;
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .status.soon {
    border-color: color-mix(in srgb, #f59e0b 50%, var(--border-color));
    color: #fbbf24;
    background: color-mix(in srgb, #f59e0b 14%, transparent);
  }

  .hero-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .service-head {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
  }

  .service-icon {
    width: 32px;
    height: 32px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: var(--bg-secondary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .content h3 {
    margin: 0;
    font-size: 1rem;
  }

  .content ul {
    margin: 0.65rem 0 0;
    padding-left: 1.1rem;
    color: var(--text-secondary);
    display: grid;
    gap: 0.45rem;
  }

  .next-box {
    margin-top: 0.9rem;
    border: 1px dashed var(--border-color);
    border-radius: 10px;
    padding: 0.7rem;
  }

  .next-box strong {
    font-size: 0.88rem;
  }

  .next-box p {
    margin: 0.25rem 0 0;
    color: var(--text-secondary);
    font-size: 0.82rem;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    border-radius: 10px;
    border: 1px solid transparent;
    padding: 0.58rem 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-secondary {
    background: var(--bg-secondary);
    border-color: var(--border-color);
    color: var(--text-primary);
  }
</style>
