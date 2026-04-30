<script lang="ts">
  import { onMount } from 'svelte';
  import { loadMixradiusImportWizard } from './mixradiusImportPageModules';

  let MixRadiusImportWizardComponent = $state<any>(null);
  let wizardLoading = $state(false);

  onMount(() => {
    void ensureWizardLoaded();
  });

  async function ensureWizardLoaded() {
    if (MixRadiusImportWizardComponent || wizardLoading) return;

    wizardLoading = true;
    try {
      const { WizardComponent } = await loadMixradiusImportWizard();
      MixRadiusImportWizardComponent = WizardComponent;
    } finally {
      wizardLoading = false;
    }
  }
</script>

{#if MixRadiusImportWizardComponent}
  <MixRadiusImportWizardComponent />
{:else}
  <div class="mixradius-wizard-loader" aria-busy={wizardLoading}>
    <div class="mixradius-wizard-loader__card">
      <div class="mixradius-wizard-loader__bar"></div>
      <div class="mixradius-wizard-loader__body"></div>
      <div class="mixradius-wizard-loader__body mixradius-wizard-loader__body--short"></div>
    </div>
  </div>
{/if}

<style>
  .mixradius-wizard-loader {
    padding: 1.5rem;
  }

  .mixradius-wizard-loader__card {
    display: grid;
    gap: 1rem;
    padding: 1.25rem;
    border-radius: 18px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
  }

  .mixradius-wizard-loader__bar,
  .mixradius-wizard-loader__body {
    border-radius: 14px;
    background:
      linear-gradient(
        90deg,
        rgba(255, 255, 255, 0.04) 0%,
        rgba(255, 255, 255, 0.12) 50%,
        rgba(255, 255, 255, 0.04) 100%
      );
    background-size: 200% 100%;
    animation: mixradius-loader-shimmer 1.2s ease-in-out infinite;
  }

  :global([data-theme='light']) .mixradius-wizard-loader__bar,
  :global([data-theme='light']) .mixradius-wizard-loader__body {
    background:
      linear-gradient(
        90deg,
        rgba(0, 0, 0, 0.05) 0%,
        rgba(0, 0, 0, 0.1) 50%,
        rgba(0, 0, 0, 0.05) 100%
      );
    background-size: 200% 100%;
  }

  .mixradius-wizard-loader__bar {
    height: 3rem;
    width: min(20rem, 100%);
  }

  .mixradius-wizard-loader__body {
    min-height: 14rem;
  }

  .mixradius-wizard-loader__body--short {
    min-height: 7rem;
  }

  @keyframes mixradius-loader-shimmer {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }
</style>
