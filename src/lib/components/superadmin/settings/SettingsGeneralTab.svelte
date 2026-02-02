<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { t } from "svelte-i18n";
    import Icon from "$lib/components/ui/Icon.svelte";

    export let appPublicUrl: string;
    export let appMainDomain: string;
    export let currencyCode: string;
    export let currencyCodeOptions: string[];
    export let maintenanceMode: boolean;
    export let maintenanceMessage: string;

    const dispatch = createEventDispatcher();

    function handleChange() {
        dispatch("change");
    }
</script>

<div class="card section fade-in">
    <div class="card-header">
        <h3>
            {$t("superadmin.settings.sections.general") || "General Settings"}
        </h3>
    </div>
    <div class="card-body">
        <div class="setting-row">
            <div class="setting-info full-width">
                <label class="setting-label" for="public-url">
                    {$t("superadmin.settings.fields.public_url.label") ||
                        "Public Application URL"}
                </label>
                <p class="setting-description">
                    {$t("superadmin.settings.fields.public_url.desc") ||
                        "Base URL for redirects, emails, and payment callbacks (e.g. https://app.example.com)."}
                </p>
                <input
                    type="text"
                    id="public-url"
                    bind:value={appPublicUrl}
                    on:input={handleChange}
                    class="form-input"
                    placeholder={$t("superadmin.settings.placeholders.url") ||
                        "https://..."}
                />
            </div>
        </div>

        <div class="setting-row">
            <div class="setting-info full-width">
                <label class="setting-label" for="main-domain">
                    {$t("superadmin.settings.fields.main_domain.label") ||
                        "Main Domain"}
                </label>
                <p class="setting-description">
                    {$t("superadmin.settings.fields.main_domain.desc") ||
                        "The primary domain where the application is hosted (e.g. example.com). Used for domain validation and tenant routing."}
                </p>
                <input
                    type="text"
                    id="main-domain"
                    bind:value={appMainDomain}
                    on:input={handleChange}
                    class="form-input"
                    placeholder={$t(
                        "superadmin.settings.placeholders.domain",
                    ) || "example.com"}
                />
            </div>
        </div>

        <div class="setting-row">
            <div class="setting-info">
                <label class="setting-label" for="currency-code">
                    {$t("superadmin.settings.fields.currency.label") ||
                        "Default Currency"}
                </label>
                <p class="setting-description">
                    {$t("superadmin.settings.fields.currency.desc") ||
                        "Currency used for plan pricing and invoice display (ISO 4217, e.g. IDR, USD)."}
                </p>
            </div>
            <select
                id="currency-code"
                class="form-input"
                bind:value={currencyCode}
                on:change={handleChange}
            >
                {#each currencyCodeOptions as opt}
                    <option value={opt}>{opt}</option>
                {/each}
            </select>
        </div>

        <div class="setting-row">
            <div class="setting-info">
                <label class="setting-label" for="maintenance-mode">
                    {$t("superadmin.settings.fields.maintenance_mode.label") ||
                        "Enable Maintenance Mode"}
                </label>
                <p class="setting-description">
                    {$t("superadmin.settings.fields.maintenance_mode.desc") ||
                        "When enabled, all users except superadmins will see a maintenance page."}
                </p>
            </div>
            <label class="toggle">
                <input
                    type="checkbox"
                    id="maintenance-mode"
                    bind:checked={maintenanceMode}
                    on:change={handleChange}
                />
                <span class="slider"></span>
            </label>
        </div>

        <div class="setting-row">
            <div class="setting-info full-width">
                <label class="setting-label" for="maintenance-message">
                    {$t(
                        "superadmin.settings.fields.maintenance_message.label",
                    ) || "Maintenance Message"}
                </label>
                <p class="setting-description">
                    {$t(
                        "superadmin.settings.fields.maintenance_message.desc",
                    ) ||
                        "The message displayed to users when maintenance mode is active."}
                </p>
                <textarea
                    id="maintenance-message"
                    bind:value={maintenanceMessage}
                    on:input={handleChange}
                    class="form-input"
                    rows="3"
                ></textarea>
            </div>
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

    .setting-info.full-width {
        width: 100%;
        padding-right: 0;
    }

    .setting-label {
        font-weight: 600;
        color: var(--text-primary);
        font-size: 0.95rem;
        display: block;
        margin-bottom: 0.25rem;
    }

    .setting-description {
        color: var(--text-secondary);
        font-size: 0.85rem;
        margin: 0;
        line-height: 1.4;
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

    textarea.form-input {
        resize: vertical;
        min-height: 80px;
        margin-top: 0.75rem;
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
        content: "";
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
