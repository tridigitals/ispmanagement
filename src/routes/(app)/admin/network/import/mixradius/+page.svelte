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
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
  }

  .mixradius-wizard-loader__bar,
  .mixradius-wizard-loader__body {
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-hover) 72%, var(--bg-surface));
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
</style>
