<script lang="ts">
    import { page } from "$app/stores";
    import { onMount } from "svelte";
    import { api, type Invoice } from "$lib/api/client";
    import Icon from "$lib/components/Icon.svelte";
    import { toast } from "svelte-sonner";
    import { goto } from "$app/navigation";
    import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
    import { formatMoney } from "$lib/utils/money";
    import Lightbox from "$lib/components/Lightbox.svelte";

    let invoiceId = $state("");
    let invoice = $state<Invoice | null>(null);
    let loading = $state(true);
    let processing = $state(false);

    // For Lightbox
    let showLightbox = $state(false);
    let lightboxFiles = $state<any[]>([]);

    // For Confirmation
    let showConfirm = $state(false);
    let confirmConfig = $state({
        title: "",
        message: "",
        type: "info" as "danger" | "warning" | "info",
        confirmText: "Confirm",
        onConfirm: async () => {},
    });

    $effect(() => {
        invoiceId = $page.params.id ?? "";
    });

    onMount(() => {
        void loadInvoice();
    });

    async function loadInvoice() {
        if (!invoiceId) {
            invoice = null;
            loading = false;
            toast.error("Missing invoice id");
            return;
        }
        loading = true;
        try {
            invoice = await api.payment.getInvoice(invoiceId);
        } catch (e: any) {
            toast.error("Failed to load invoice: " + e.message);
        } finally {
            loading = false;
        }
    }

    function triggerVerify(status: "paid" | "failed") {
        confirmConfig = {
            title: status === "paid" ? "Approve Payment" : "Reject Payment",
            message:
                status === "paid"
                    ? "Are you sure you want to approve this payment? This will activate the subscription immediately."
                    : "Are you sure you want to reject this payment? The user will be notified.",
            type: status === "paid" ? "info" : "danger",
            confirmText: status === "paid" ? "Approve" : "Reject",
            onConfirm: async () => await handleVerify(status),
        };
        showConfirm = true;
    }

    async function handleVerify(status: "paid" | "failed") {
        if (!invoiceId) return;
        processing = true;
        try {
            await api.payment.verifyPayment(invoiceId, status);
            toast.success(`Invoice marked as ${status}`);
            void loadInvoice();
            showConfirm = false;
        } catch (e: any) {
            toast.error("Verification failed: " + e.message);
        } finally {
            processing = false;
        }
    }

    function formatCurrency(amount: number, currency?: string) {
        return formatMoney(amount, { currency });
    }

    function getProofUrl(fileId: string) {
        const API_BASE =
            import.meta.env.VITE_API_URL || "http://localhost:3000/api";
        return `${API_BASE}/storage/files/${fileId}/content`;
    }

    function openLightbox(fileId: string) {
        lightboxFiles = [
            {
                id: fileId,
                original_name: "Payment Proof",
                content_type: "image/jpeg",
                size: 0,
                created_at: new Date().toISOString(),
            },
        ];
        showLightbox = true;
    }
</script>

<div class="page-container fade-in">
    <div class="page-header">
        <button class="back-btn" onclick={() => goto("/superadmin/invoices")}>
            <Icon name="arrow-left" size={20} />
            Back to Invoices
        </button>
        <h1>Invoice Details</h1>
    </div>

    {#if loading}
        <div class="loading">Loading details...</div>
    {:else if invoice}
        <div class="details-grid">
            <!-- Left: Info -->
            <div class="card info-card">
                <div class="card-header">
                    <h2>Invoice #{invoice.invoice_number}</h2>
                    <span class="status-pill {invoice.status}"
                        >{invoice.status}</span
                    >
                </div>

                <div class="info-rows">
                    <div class="row">
                        <span class="label">Tenant ID</span>
                        <span class="value">{invoice.tenant_id}</span>
                    </div>
                    <div class="row">
                        <span class="label">Description</span>
                        <span class="value">{invoice.description}</span>
                    </div>
                    <div class="row">
                        <span class="label">Amount</span>
                        <span class="value highlight"
                            >{formatCurrency(
                                invoice.amount,
                                invoice.currency_code,
                            )}</span
                        >
                    </div>
                    <div class="row">
                        <span class="label">Created At</span>
                        <span class="value"
                            >{invoice.created_at
                                ? new Date(invoice.created_at).toLocaleString()
                                : "-"}</span
                        >
                    </div>
                    <div class="row">
                        <span class="label">Updated At</span>
                        <span class="value"
                            >{invoice.updated_at
                                ? new Date(invoice.updated_at).toLocaleString()
                                : "-"}</span
                        >
                    </div>
                </div>

                <div class="actions">
                    {#if invoice.status === "verification_pending" || invoice.status === "pending"}
                        <h3 class="section-title">Manual Verification</h3>
                        <div class="btn-group">
                            <button
                                class="btn btn-success"
                                onclick={() => triggerVerify("paid")}
                                disabled={processing}
                            >
                                <Icon name="check" size={18} />
                                Approve Payment
                            </button>
                            <button
                                class="btn btn-danger"
                                onclick={() => triggerVerify("failed")}
                                disabled={processing}
                            >
                                <Icon name="x" size={18} />
                                Reject
                            </button>
                        </div>
                    {:else}
                        <p class="info-text">
                            This invoice is already {invoice.status}.
                        </p>
                    {/if}
                </div>
            </div>

            <!-- Right: Proof Attachment -->
            <div class="card proof-card">
                <h2>Payment Proof</h2>
                {#if invoice.proof_attachment}
                    <div class="proof-wrapper">
                        <!-- svelte-ignore a11y_click_events_have_key_events -->
                        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                        <img
                            src={getProofUrl(invoice.proof_attachment)}
                            alt="Payment Proof"
                            class="proof-img"
                            onclick={() =>
                                openLightbox(invoice!.proof_attachment!)}
                        />
                        <p class="hint">Click to enlarge</p>
                    </div>
                {:else}
                    <div class="no-proof">
                        <Icon name="image" size={48} />
                        <p>No proof uploaded yet.</p>
                    </div>
                {/if}
            </div>
        </div>
    {:else}
        <div class="error">Invoice not found</div>
    {/if}
</div>

<ConfirmDialog
    bind:show={showConfirm}
    title={confirmConfig.title}
    message={confirmConfig.message}
    type={confirmConfig.type}
    confirmText={confirmConfig.confirmText}
    onconfirm={confirmConfig.onConfirm}
    loading={processing}
/>

{#if showLightbox}
    <Lightbox files={lightboxFiles} onclose={() => (showLightbox = false)} />
{/if}

<style>
    .page-container {
        padding: 2rem;
        max-width: 1200px;
        margin: 0 auto;
    }
    .page-header {
        margin-bottom: 2rem;
    }
    .back-btn {
        background: none;
        border: none;
        display: flex;
        align-items: center;
        gap: 0.5rem;
        color: var(--text-secondary);
        cursor: pointer;
        font-weight: 500;
        margin-bottom: 0.5rem;
    }
    .back-btn:hover {
        color: var(--text-primary);
    }

    .details-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 2rem;
    }
    @media (max-width: 768px) {
        .details-grid {
            grid-template-columns: 1fr;
        }
    }

    .card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 1.5rem;
    }

    .card-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1.5rem;
        padding-bottom: 1rem;
        border-bottom: 1px solid var(--border-color);
    }

    .status-pill {
        padding: 0.25rem 0.75rem;
        border-radius: 20px;
        font-weight: 600;
        text-transform: uppercase;
        font-size: 0.85rem;
    }
    .status-pill.pending {
        background: #fef3c7;
        color: #d97706;
    }
    .status-pill.verification_pending {
        background: #fef3c7;
        color: #d97706;
        border: 1px solid #d97706;
    }
    .status-pill.paid {
        background: #dcfce7;
        color: #16a34a;
    }
    .status-pill.failed {
        background: #fee2e2;
        color: #dc2626;
    }

    .info-rows {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }
    .row {
        display: flex;
        justify-content: space-between;
        padding: 0.5rem 0;
        border-bottom: 1px solid var(--border-color-light);
    }
    .label {
        color: var(--text-secondary);
        font-weight: 500;
    }
    .value {
        font-weight: 600;
        color: var(--text-primary);
    }
    .value.highlight {
        font-size: 1.1em;
        color: var(--primary-color);
    }

    .actions {
        margin-top: 2rem;
        padding-top: 1rem;
        border-top: 1px solid var(--border-color);
    }
    .btn-group {
        display: flex;
        gap: 1rem;
    }
    .btn {
        flex: 1;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 0.5rem;
        padding: 0.75rem;
        border-radius: 8px;
        font-weight: 600;
        cursor: pointer;
        border: none;
    }
    .btn-success {
        background: #16a34a;
        color: white;
    }
    .btn-success:hover {
        background: #15803d;
    }
    .btn-danger {
        background: #dc2626;
        color: white;
    }
    .btn-danger:hover {
        background: #b91c1c;
    }

    .proof-card {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        min-height: 300px;
    }
    .proof-wrapper {
        width: 100%;
        display: flex;
        flex-direction: column;
        align-items: center;
    }
    .proof-img {
        max-width: 100%;
        max-height: 500px;
        border-radius: 8px;
        border: 1px solid var(--border-color);
        cursor: zoom-in;
        transition: transform 0.2s;
    }
    .proof-img:hover {
        transform: scale(1.02);
    }
    .hint {
        margin-top: 0.5rem;
        color: var(--text-secondary);
        font-size: 0.85rem;
    }

    .no-proof {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 1rem;
        color: var(--text-tertiary);
    }
</style>
