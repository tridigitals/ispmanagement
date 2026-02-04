<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { t } from 'svelte-i18n';

  export let authPasswordMinLength: number;
  export let authPasswordRequireUppercase: boolean;
  export let authPasswordRequireNumber: boolean;
  export let authPasswordRequireSpecial: boolean;

  const dispatch = createEventDispatcher();

  function handleChange() {
    dispatch('change');
  }
</script>

<div class="card section fade-in">
  <div class="card-header">
    <h3>
      {$t('superadmin.settings.sections.password') || 'Password Policy'}
    </h3>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="min-pwd-length">
          {$t('superadmin.settings.password.min_length.label') || 'Minimum Length'}
        </label>
      </div>
      <div class="input-group">
        <input
          type="number"
          id="min-pwd-length"
          bind:value={authPasswordMinLength}
          on:input={handleChange}
          min="6"
          class="form-input"
        />
        <span class="input-suffix">{$t('common.units.chars') || 'chars'}</span>
      </div>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="require-uppercase">
          {$t('superadmin.settings.password.require_uppercase.label') || 'Require Uppercase'}
        </label>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          id="require-uppercase"
          bind:checked={authPasswordRequireUppercase}
          on:change={handleChange}
        />
        <span class="slider"></span>
      </label>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="require-number">
          {$t('superadmin.settings.password.require_number.label') || 'Require Number'}
        </label>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          id="require-number"
          bind:checked={authPasswordRequireNumber}
          on:change={handleChange}
        />
        <span class="slider"></span>
      </label>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="require-special">
          {$t('superadmin.settings.password.require_special.label') || 'Require Special Character'}
        </label>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          id="require-special"
          bind:checked={authPasswordRequireSpecial}
          on:change={handleChange}
        />
        <span class="slider"></span>
      </label>
    </div>
  </div>
</div>

<style>
  .card {
    background: var(--bg-surface);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
    margin-bottom: 1.5rem;
  }

  .card-header {
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border-color);
    background: rgba(0, 0, 0, 0.2);
  }

  .card-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .card-body {
    padding: 1.5rem;
  }

  .setting-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 1.25rem 0;
    border-bottom: 1px solid var(--border-color);
  }

  .setting-row:last-child {
    border-bottom: none;
  }

  .setting-info {
    flex: 1;
    padding-right: 1.5rem;
  }

  .setting-label {
    font-weight: 600;
    color: var(--text-primary);
    font-size: 0.95rem;
    display: block;
    margin-bottom: 0.25rem;
  }

  .form-input {
    width: 100%;
    max-width: 400px;
    padding: 0.5rem 0.75rem;
    background: var(--bg-app);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .form-input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  .input-group {
    display: flex;
    align-items: center;
    width: 160px;
  }

  .input-group .form-input {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
    text-align: right;
    max-width: 100px;
  }

  .input-suffix {
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--border-color);
    border-left: none;
    padding: 0.5rem 0.75rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
    border-top-right-radius: var(--radius-sm);
    border-bottom-right-radius: var(--radius-sm);
    white-space: nowrap;
    height: 38px;
    display: flex;
    align-items: center;
  }

  /* Toggle Switch */
  .toggle {
    position: relative;
    display: inline-block;
    width: 52px;
    height: 28px;
    flex-shrink: 0;
  }

  .toggle input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--bg-tertiary);
    transition: 0.3s;
    border-radius: 28px;
  }

  .slider:before {
    position: absolute;
    content: '';
    height: 20px;
    width: 20px;
    left: 4px;
    bottom: 4px;
    background-color: white;
    transition: 0.3s;
    border-radius: 50%;
  }

  input:checked + .slider {
    background-color: var(--color-primary);
  }

  input:checked + .slider:before {
    transform: translateX(24px);
  }

  .fade-in {
    animation: fadeIn 0.3s ease-out;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
