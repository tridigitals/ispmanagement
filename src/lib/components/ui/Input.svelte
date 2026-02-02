<script lang="ts">
    import Icon from "$lib/components/ui/Icon.svelte";

    interface Props {
        value?: string | number;
        type?: "text" | "password" | "email" | "number";
        label?: string;
        placeholder?: string;
        disabled?: boolean;
        readonly?: boolean;
        id?: string;
        error?: string;
        showPasswordToggle?: boolean;
        oninput?: (e: Event) => void;
        onchange?: (e: Event) => void;
        [key: string]: any;
    }

    let {
        value = $bindable(""),
        type = "text",
        label = "",
        placeholder = "",
        disabled = false,
        readonly = false,
        id = "",
        error = "",
        showPasswordToggle = false,
        oninput,
        onchange,
        ...restProps
    }: Props = $props();

    let isPasswordVisible = $state(false);

    let inputType = $derived(
        showPasswordToggle && isPasswordVisible ? "text" : type,
    );

    function handleInput(e: Event) {
        const target = e.target as HTMLInputElement;
        // The bind:value handles the update, but we might need to cast if it's a number type
        // However, standard HTML input binding in Svelte usually handles this.
        // For strict type parity with previous logic:
        if (type === "number") {
            value = Number(target.value);
        } else {
            value = target.value;
        }

        if (oninput) oninput(e);
    }

    function handleChange(e: Event) {
        if (onchange) onchange(e);
    }
</script>

<div class="input-group {error ? 'has-error' : ''}">
    {#if label}
        <label for={id} class="input-label">{label}</label>
    {/if}

    <div class="input-wrapper">
        <input
            {id}
            type={inputType}
            bind:value
            {placeholder}
            {disabled}
            {readonly}
            class="form-input"
            oninput={handleInput}
            onchange={handleChange}
            {...restProps}
        />

        {#if showPasswordToggle && type === "password"}
            <button
                type="button"
                class="password-toggle"
                onclick={() => (isPasswordVisible = !isPasswordVisible)}
                tabindex="-1"
            >
                <Icon name={isPasswordVisible ? "eye-off" : "eye"} size={18} />
            </button>
        {/if}
    </div>

    {#if error}
        <p class="error-text">{error}</p>
    {/if}
</div>

<style>
    .input-group {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        width: 100%;
    }

    .input-label {
        font-weight: 500;
        font-size: 0.9rem;
        color: var(--text-primary);
    }

    .input-wrapper {
        position: relative;
        width: 100%;
    }

    .form-input {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        padding: 0.75rem 1rem;
        border-radius: var(--radius-md);
        width: 100%;
        font-size: 0.95rem;
        transition: all 0.2s;
    }

    .form-input:focus {
        border-color: var(--color-primary);
        outline: none;
        box-shadow: 0 0 0 3px var(--color-primary-subtle);
    }

    .form-input:disabled {
        opacity: 0.6;
        cursor: not-allowed;
        background: var(--bg-tertiary);
    }

    .form-input:read-only {
        background: var(--bg-tertiary);
    }

    /* Password Toggle */
    .password-toggle {
        position: absolute;
        right: 12px;
        top: 50%;
        transform: translateY(-50%);
        background: none;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
        padding: 4px;
        display: flex;
        align-items: center;
        border-radius: 4px;
        transition: color 0.2s;
    }

    .password-toggle:hover {
        color: var(--text-primary);
        background: var(--bg-hover);
    }

    /* Error State */
    .has-error .form-input {
        border-color: var(--color-danger);
    }

    .has-error .form-input:focus {
        box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.1);
    }

    .error-text {
        color: var(--color-danger);
        font-size: 0.85rem;
    }

    /* Mobile Optimization */
    @media (max-width: 640px) {
        .form-input {
            padding: 0.875rem 1rem; /* Larger touch target */
            font-size: 1rem; /* Better readability */
        }
    }
</style>

