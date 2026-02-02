<script lang="ts">
    import { t } from "svelte-i18n";

    let { details } = $props<{
        details: string | null;
    }>();
</script>

<div class="details-block">
    <div class="details-title">
        {$t("superadmin.audit_logs.labels.details") || "Details"}
    </div>
    <div class="details-text">
        {details || "—"}
    </div>
</div>

<style>
    .details-block {
        margin-top: 1rem;
        padding: 0.75rem;
        background: rgba(0, 0, 0, 0.2);
        border-radius: 8px;
        border: 1px solid rgba(255, 255, 255, 0.05);
    }

    :global([data-theme="light"]) .details-block {
        background: rgba(0, 0, 0, 0.03);
        border-color: rgba(0, 0, 0, 0.05);
    }

    .details-title {
        font-size: 0.75rem;
        font-weight: 700;
        color: var(--text-secondary);
        margin-bottom: 0.25rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .details-text {
        font-family: var(--font-mono);
        font-size: 0.85rem;
        color: var(--text-primary);
        white-space: pre-wrap;
        word-break: break-all;
    }
</style>
