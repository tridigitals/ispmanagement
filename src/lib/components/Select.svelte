<script lang="ts">
    import Icon from "./Icon.svelte";
    import { createEventDispatcher, onDestroy } from "svelte";
    import { fade, slide } from "svelte/transition";

    export let value: any = "";
    export let options: Array<{ label: string; value: any }> | Array<any> = [];
    export let placeholder = "Select option";
    export let label = "";
    export let disabled = false;
    export let width = "auto";
    export let placement: "top" | "bottom" = "bottom"; // Control opening direction
    export let onchange: ((e: any) => void) | undefined = undefined; // Svelte 5 compatibility

    let isOpen = false;
    let containerElement: HTMLElement;

    // Normalize options
    $: normalizedOptions = options.map((opt) => {
        if (typeof opt === "object" && opt !== null && "value" in opt) {
            return opt as { label: string; value: any };
        }
        return { label: String(opt), value: opt };
    });

    $: selectedLabel =
        normalizedOptions.find((opt) => opt.value === value)?.label ||
        placeholder;

    const dispatch = createEventDispatcher();

    function toggle() {
        if (!disabled) {
            isOpen = !isOpen;
        }
    }

    function selectOption(optionVal: any) {
        value = optionVal;
        dispatch("change", value);
        if (onchange) {
            onchange({ detail: value }); // Mock CustomEvent for compatibility
        }
        isOpen = false;
    }

    function handleClickOutside(event: MouseEvent) {
        if (
            isOpen &&
            containerElement &&
            !containerElement.contains(event.target as Node)
        ) {
            isOpen = false;
        }
    }
</script>

<svelte:window on:click={handleClickOutside} />

<div
    class="select-container"
    style="width: {width}"
    bind:this={containerElement}
>
    {#if label}
        <span class="label">{label}</span>
    {/if}

    <button
        class="select-trigger {disabled ? 'disabled' : ''} {isOpen
            ? 'open'
            : ''}"
        on:click={toggle}
        type="button"
        {disabled}
    >
        <span class="selected-text {value === '' ? 'placeholder' : ''}">
            {selectedLabel}
        </span>
        <div class="icon-wrapper {isOpen ? 'rotate' : ''}">
            <Icon name="chevron-down" size={16} />
        </div>
    </button>

    {#if isOpen}
        <div
            class="dropdown-menu {placement}"
            transition:slide|local={{ duration: 200 }}
        >
            {#each normalizedOptions as option}
                <button
                    class="dropdown-item {option.value === value
                        ? 'selected'
                        : ''}"
                    on:click={() => selectOption(option.value)}
                    type="button"
                >
                    {option.label}
                    {#if option.value === value}
                        <Icon name="check" size={14} class="check-icon" />
                    {/if}
                </button>
            {/each}
        </div>
    {/if}
</div>

<style>
    .select-container {
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
        position: relative;
    }

    .label {
        font-size: 0.85rem;
        font-weight: 500;
        color: var(--text-secondary);
        margin-left: 0.2rem;
    }

    .select-trigger {
        width: 100%;
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        padding: 0.6rem 2.5rem 0.6rem 1rem;
        border-radius: 8px;
        color: var(--text-primary);
        font-size: 0.9rem;
        display: flex;
        align-items: center;
        justify-content: space-between;
        cursor: pointer;
        transition: all 0.2s;
        text-align: left;
        position: relative;
    }

    .select-trigger:hover:not(.disabled) {
        border-color: var(--text-secondary);
    }

    .select-trigger.open {
        border-color: var(--color-primary);
        box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
    }

    .selected-text {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .selected-text.placeholder {
        color: var(--text-secondary);
    }

    .icon-wrapper {
        position: absolute;
        right: 0.8rem;
        top: 50%;
        transform: translateY(-50%);
        color: var(--text-secondary);
        display: flex;
        align-items: center;
        transition: transform 0.2s;
    }

    .icon-wrapper.rotate {
        transform: translateY(-50%) rotate(180deg);
    }

    .dropdown-menu {
        position: absolute;
        left: 0;
        right: 0;
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        box-shadow:
            0 10px 15px -3px rgba(0, 0, 0, 0.1),
            0 4px 6px -2px rgba(0, 0, 0, 0.05);
        z-index: 50;
        max-height: 250px;
        overflow-y: auto;
        padding: 0.25rem;
    }

    .dropdown-menu.bottom {
        top: calc(100% + 0.5rem);
    }

    .dropdown-menu.top {
        bottom: calc(100% + 0.5rem);
    }

    .dropdown-item {
        width: 100%;
        text-align: left;
        padding: 0.6rem 1rem;
        background: transparent;
        border: none;
        color: var(--text-primary);
        cursor: pointer;
        font-size: 0.9rem;
        border-radius: 6px;
        display: flex;
        align-items: center;
        justify-content: space-between;
    }

    .dropdown-item:hover {
        background: var(--bg-hover);
    }

    .dropdown-item.selected {
        background: rgba(99, 102, 241, 0.1);
        color: var(--color-primary);
        font-weight: 500;
    }

    .disabled {
        opacity: 0.6;
        cursor: not-allowed;
        background: var(--bg-app);
    }

    /* Scrollbar */
    .dropdown-menu::-webkit-scrollbar {
        width: 6px;
    }

    .dropdown-menu::-webkit-scrollbar-track {
        background: transparent;
    }

    .dropdown-menu::-webkit-scrollbar-thumb {
        background-color: var(--border-color);
        border-radius: 20px;
    }
</style>
