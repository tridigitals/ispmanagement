<script lang="ts">
    import { page } from "$app/stores";
    import { onDestroy, onMount } from "svelte";
    import { api, type Invoice, type BankAccount } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import { user } from "$lib/stores/auth";
    import Icon from "$lib/components/ui/Icon.svelte";
    import { toast } from "svelte-sonner";
    import { t } from "svelte-i18n";
    import { get } from "svelte/store";

    let invoiceId = $page.params.id as string;
    let invoice = $state<Invoice | null>(null);
    let bankAccounts = $state<BankAccount[]>([]);
    let loading = $state(true);
    let paymentMethod = $state<"online" | "manual">("online");
    let midtransEnabled = $state(false);
    let manualEnabled = $state(true);
    let snapToken = $state("");
    let snapReady = $state(false);
    let snapLoading = $state(false);
    let autoChecking = $state(false);
    let statusCheckTimer: ReturnType<typeof setInterval> | null = null;
    let statusCheckAttempts = 0;
    const STATUS_CHECK_INTERVAL_MS = 3000;
    const MAX_STATUS_CHECK_ATTEMPTS = 20;
    let manualInstructions = $state("");
    let publicSettings = $state<any>({});

    onMount(async () => {
        try {
            // Load Public Settings (contains payment config)
            publicSettings = await api.settings.getPublicSettings();

            midtransEnabled = !!publicSettings.payment_midtrans_enabled;
            manualEnabled = publicSettings.payment_manual_enabled ?? true; // Default true

            // Set default method
            if (midtransEnabled) paymentMethod = "online";
            else if (manualEnabled) paymentMethod = "manual";

            // Load Invoice
            invoice = await api.payment.getInvoice(invoiceId);

            // If invoice currency is not IDR, disable Midtrans (backend also enforces this)
            if (
                invoice?.currency_code &&
                String(invoice.currency_code).toUpperCase() !== "IDR"
            ) {
                midtransEnabled = false;
                if (paymentMethod === "online") paymentMethod = "manual";
            }

            // Load Manual Bank Accounts & Instructions
            if (manualEnabled) {
                bankAccounts = await api.payment.listBanks();
            }

            // Load Midtrans Snap JS if enabled
            if (midtransEnabled) {
                const clientKey = publicSettings.payment_midtrans_client_key;
                const isProd = !!publicSettings.payment_midtrans_is_production;
                if (clientKey) loadSnapScript(clientKey, isProd);
            }

            if (
                midtransEnabled &&
                invoice?.status === "pending" &&
                hasMidtransPending()
            ) {
                startStatusPolling();
            }
        } catch (e: any) {
            toast.error(
                e.message ||
                    get(t)("payment.checkout.errors.load_failed") ||
                    "Failed to load invoice",
            );
        } finally {
            loading = false;
        }
    });

    onDestroy(() => {
        stopStatusPolling();
    });

    function pendingKey() {
        return `midtrans:pending:${invoiceId}`;
    }

    function markMidtransPending() {
        if (typeof localStorage === "undefined") return;
        localStorage.setItem(pendingKey(), "1");
    }

    function clearMidtransPending() {
        if (typeof localStorage === "undefined") return;
        localStorage.removeItem(pendingKey());
    }

    function hasMidtransPending() {
        if (typeof localStorage === "undefined") return false;
        return localStorage.getItem(pendingKey()) === "1";
    }

    function loadSnapScript(clientKey: string, isProd: boolean) {
        if (typeof window !== "undefined" && (window as any).snap) {
            snapReady = true;
            return;
        }

        const existing = document.getElementById(
            "midtrans-snap-js",
        ) as HTMLScriptElement | null;
        if (existing) {
            existing.addEventListener("load", () => {
                snapReady = true;
                snapLoading = false;
            });
            return;
        }

        snapLoading = true;
        const script = document.createElement("script");
        script.id = "midtrans-snap-js";
        script.src = isProd
            ? "https://app.midtrans.com/snap/snap.js"
            : "https://app.sandbox.midtrans.com/snap/snap.js";
        script.setAttribute("data-client-key", clientKey);
        script.onload = () => {
            snapReady = true;
            snapLoading = false;
        };
        script.onerror = () => {
            snapLoading = false;
            toast.error(
                get(t)("payment.checkout.errors.load_failed") ||
                    "Failed to load Midtrans payment script",
            );
        };
        document.head.appendChild(script);
    }

    async function handlePayOnline() {
        if (!invoice) return;
        try {
            const token = await api.payment.payMidtrans(invoice.id);
            snapToken = token;

            const snap = (window as any).snap;
            if (!snap) {
                toast.error(
                    get(t)("payment.checkout.errors.load_failed") ||
                        "Midtrans is not ready yet. Please try again.",
                );
                return;
            }

            markMidtransPending();
            snap.pay(token, {
                onSuccess: function (result: any) {
                    toast.success(
                        get(t)("payment.checkout.toasts.payment_success") ||
                            "Payment successful!",
                    );
                    startStatusPolling();
                },
                onPending: function (result: any) {
                    toast.info(
                        get(t)("payment.checkout.toasts.waiting") ||
                            "Waiting for payment...",
                    );
                    startStatusPolling();
                },
                onError: function (result: any) {
                    toast.error(
                        get(t)("payment.checkout.toasts.payment_failed") ||
                            "Payment failed",
                    );
                    clearMidtransPending();
                    stopStatusPolling();
                },
                onClose: function () {
                    // closed
                },
            });
        } catch (e: any) {
            toast.error(
                (get(t)("payment.checkout.errors.initiate_failed") ||
                    "Failed to initiate payment: ") + e.message,
            );
        }
    }

    async function checkPaymentStatus(options?: {
        silent?: boolean;
        notifyOnChange?: boolean;
    }) {
        if (!invoice) return;
        try {
            const status = await api.payment.checkStatus(invoice.id);
            const notifyOnChange = options?.notifyOnChange ?? true;
            const silent = options?.silent ?? false;

            if (status !== invoice.status) {
                invoice = { ...invoice, status };
                if (status === "paid" || status === "failed") {
                    clearMidtransPending();
                    stopStatusPolling();
                    invoice = await api.payment.getInvoice(invoice.id);
                }
                if (notifyOnChange && !silent) {
                    toast.success(
                        (get(t)("payment.checkout.toasts.status_updated") ||
                            "Status updated: ") + status,
                    );
                } else if (notifyOnChange && status === "paid") {
                    toast.success(
                        get(t)("payment.checkout.toasts.payment_success") ||
                            "Payment successful!",
                    );
                } else if (notifyOnChange && status === "failed") {
                    toast.error(
                        get(t)("payment.checkout.toasts.payment_failed") ||
                            "Payment failed",
                    );
                }
            } else if (!silent) {
                toast.info(
                    (get(t)("payment.checkout.toasts.current_status") ||
                        "Current status: ") + status,
                );
            }

            return status;
        } catch (e: any) {
            if (!options?.silent) {
                toast.error(
                    (get(t)("payment.checkout.errors.check_status_failed") ||
                        "Failed to check status: ") + e.message,
                );
            }
        }
    }

    function startStatusPolling() {
        if (!invoice || invoice.status !== "pending") return;
        if (statusCheckTimer) return;
        autoChecking = true;
        statusCheckAttempts = 0;

        const poll = async () => {
            statusCheckAttempts += 1;
            const status = await checkPaymentStatus({
                silent: true,
                notifyOnChange: true,
            });
            if (status && status !== "pending") {
                stopStatusPolling();
                return;
            }
            if (statusCheckAttempts >= MAX_STATUS_CHECK_ATTEMPTS) {
                stopStatusPolling();
            }
        };

        void poll();
        statusCheckTimer = setInterval(poll, STATUS_CHECK_INTERVAL_MS);
    }

    function stopStatusPolling() {
        if (statusCheckTimer) {
            clearInterval(statusCheckTimer);
            statusCheckTimer = null;
        }
        autoChecking = false;
    }

    function formatCurrency(amount: number) {
        const locale = publicSettings?.default_locale || "id-ID";
        const currency =
            invoice?.currency_code || publicSettings?.currency_code || "IDR";

        return new Intl.NumberFormat(locale, {
            style: "currency",
            currency,
        }).format(amount);
    }
    let fileInput = $state<HTMLInputElement | null>(null);
    let uploading = $state(false);

    async function handleFileUpload(e: Event) {
        const target = e.target as HTMLInputElement;
        const file = target.files?.[0];
        if (!file) return;

        if (file.size > 5 * 1024 * 1024) {
            toast.error(
                get(t)("payment.checkout.errors.file_too_large") ||
                    "File size must be less than 5MB",
            );
            return;
        }

        uploading = true;
        try {
            // 1. Upload file to storage
            // Note: This requires the user to be logged in, which they should be for subscription payment.
            // If this is a public invoice link for non-logged users, we'd need a public upload endpoint.
            // Assuming logged in for now as per `submit_payment_proof` requirement.

            const uploadedFile = await api.storage.uploadFile(file);

            // 2. Submit proof path/url
            // We'll store the URL or ID. Let's store the URL for easy display.
            // Assuming `uploadedFile.url` or we construct it.
            // `FileRecord` has `url` usually? Let's check `client.ts` interface.
            // If not, we store `uploadedFile.id` and fetch via ID?
            // Actually `uploadFile` returns `FileRecord`.

            // Let's assume we can get a serving URL.
            // For local storage, it might be served via specific route.
            // For now, let's store the file ID or name.
            // Ideally, we store the full accessible URL.
            // Let's verify `FileRecord` interface in `client.ts`.

            // Temporary: Just store the ID or Name if URL isn't explicit in `FileRecord`
            // But `submit_payment_proof` takes string.

            await api.payment.submitPaymentProof(invoice!.id, uploadedFile.id); // Storing ID for security/lookup

            toast.success(
                get(t)("payment.checkout.toasts.proof_uploaded") ||
                    "Proof uploaded successfully!",
            );
            // Reload to show pending state
            location.reload();
        } catch (e: any) {
            toast.error(
                (get(t)("payment.checkout.errors.upload_failed") ||
                    "Upload failed: ") + e.message,
            );
        } finally {
            uploading = false;
        }
    }
</script>

<div class="checkout-page fade-in">
    <div class="checkout-card">
        {#if loading}
            <div class="loading">
                {$t("payment.checkout.loading") || "Loading invoice..."}
            </div>
        {:else if invoice}
            <div class="header">
                <button
                    class="back-link"
                    onclick={() =>
                        goto(
                            $user?.tenant_slug
                                ? `/${$user.tenant_slug}/admin/subscription`
                                : "/dashboard",
                        )}
                >
                    <Icon name="arrow-left" size={18} />
                    <span>{$t("common.back") || "Back"}</span>
                </button>
                <h1>{$t("payment.checkout.title") || "Checkout"}</h1>
                <span class="invoice-number">#{invoice.invoice_number}</span>
            </div>

            <div class="summary-section">
                <div class="item-row">
                    <span class="label">
                        {$t("payment.checkout.item") || "Item"}
                    </span>
                    <span class="value">{invoice.description}</span>
                </div>
                <div class="item-row total">
                    <span class="label">
                        {$t("payment.checkout.total") || "Total"}
                    </span>
                    <span class="value">{formatCurrency(invoice.amount)}</span>
                </div>
                <div class="status-row">
                    <span class="label">
                        {$t("payment.checkout.status") || "Status"}
                    </span>
                    <span class="status-pill {invoice.status}"
                        >{invoice.status}</span
                    >
                </div>
            </div>

            {#if invoice.status === "pending"}
                <div class="payment-tabs">
                    {#if midtransEnabled}
                        <button
                            class="tab {paymentMethod === 'online'
                                ? 'active'
                                : ''}"
                            onclick={() => (paymentMethod = "online")}
                        >
                            <Icon name="credit-card" size={18} />
                            {$t("payment.checkout.tabs.online") ||
                                "Online Payment"}
                        </button>
                    {/if}
                    {#if manualEnabled}
                        <button
                            class="tab {paymentMethod === 'manual'
                                ? 'active'
                                : ''}"
                            onclick={() => (paymentMethod = "manual")}
                        >
                            <Icon name="landmark" size={18} />
                            <!-- Bank icon -->
                            {$t("payment.checkout.tabs.manual") ||
                                "Bank Transfer"}
                        </button>
                    {/if}
                </div>

                <div class="payment-content">
                    {#if paymentMethod === "online" && midtransEnabled}
                        <div class="online-method">
                            <p>
                                {$t("payment.checkout.online.description") ||
                                    "Pay securely with Credit Card, GoPay, ShopeePay, or Virtual Account via Midtrans."}
                            </p>
                            <button
                                class="btn btn-primary btn-lg w-full"
                                onclick={handlePayOnline}
                            >
                                {$t("payment.checkout.online.pay_now") ||
                                    "Pay Now"}
                            </button>
                            <div style="margin-top: 1rem;">
                                <button
                                    class="btn btn-secondary w-full"
                                    onclick={() =>
                                        checkPaymentStatus({
                                            silent: false,
                                            notifyOnChange: true,
                                        })}
                                    disabled={autoChecking}
                                >
                                    {#if autoChecking}
                                        {$t(
                                            "payment.checkout.online.checking",
                                        ) || "Checking..."}
                                    {:else}
                                        {$t(
                                            "payment.checkout.online.check_status",
                                        ) || "Check Payment Status"}
                                    {/if}
                                </button>
                            </div>
                        </div>
                    {:else if paymentMethod === "manual" && manualEnabled}
                        <div class="manual-method">
                            <p class="instructions">
                                {$t(
                                    "payment.checkout.manual.instructions",
                                ) ||
                                    "Please transfer the exact amount to one of the following accounts:"}
                            </p>

                            <div class="bank-list">
                                {#each bankAccounts as bank}
                                    <div class="bank-item">
                                        <div class="bank-name">
                                            {bank.bank_name}
                                        </div>
                                        <div class="bank-details">
                                            <span class="number"
                                                >{bank.account_number}</span
                                            >
                                            <span class="holder"
                                                >{bank.account_holder}</span
                                            >
                                        </div>
                                    </div>
                                {/each}
                            </div>

                            <div class="upload-section">
                                <p>
                                    {$t(
                                        "payment.checkout.manual.upload_hint",
                                    ) ||
                                        "Already transferred? Upload your receipt."}
                                </p>
                                <input
                                    type="file"
                                    accept="image/*,application/pdf"
                                    onchange={handleFileUpload}
                                    style="display: none;"
                                    bind:this={fileInput}
                                />
                                <button
                                    class="btn btn-secondary w-full"
                                    onclick={() => fileInput?.click()}
                                    disabled={uploading}
                                >
                                    {#if uploading}
                                        {$t(
                                            "payment.checkout.manual.uploading",
                                        ) || "Uploading..."}
                                    {:else}
                                        <Icon name="upload" size={18} />
                                        {$t(
                                            "payment.checkout.manual.upload",
                                        ) || "Upload Proof of Payment"}
                                    {/if}
                                </button>
                            </div>
                        </div>
                    {/if}
                </div>
            {:else if invoice.status === "verification_pending"}
                <div class="pending-message">
                    <div class="icon-circle pending">
                        <Icon name="clock" size={32} />
                    </div>
                    <h3>
                        {$t("payment.checkout.pending.title") ||
                            "Payment Verification Pending"}
                    </h3>
                    <p>
                        {$t("payment.checkout.pending.message") ||
                            "We have received your payment proof. Our team is verifying it. We will notify you once approved."}
                    </p>
                    <button
                        class="btn btn-secondary"
                        onclick={() =>
                            goto(
                                $user?.tenant_slug
                                    ? `/${$user.tenant_slug}/admin/subscription`
                                    : "/dashboard",
                            )}
                    >
                        {$t("payment.checkout.pending.back") ||
                            "Return to Dashboard"}
                    </button>
                    <!-- Allow re-upload in case of mistake? Optional. -->
                </div>
            {:else if invoice.status === "paid"}
                <div class="success-message">
                    <div class="icon-circle success">
                        <Icon name="check" size={32} />
                    </div>
                    <h3>
                        {$t("payment.checkout.success.title") ||
                            "Payment Successful!"}
                    </h3>
                    <p>
                        {$t("payment.checkout.success.message") ||
                            "Thank you for your payment. Your subscription has been activated."}
                    </p>
                    <button
                        class="btn btn-primary"
                        onclick={() =>
                            goto(
                                $user?.tenant_slug
                                    ? `/${$user.tenant_slug}/admin/subscription`
                                    : "/dashboard",
                            )}
                    >
                        {$t("payment.checkout.success.cta") ||
                            "Go to Subscription"}
                    </button>
                </div>
            {/if}
        {:else}
            <div class="error">
                {$t("payment.checkout.not_found") || "Invoice not found"}
            </div>
        {/if}
    </div>
</div>

<style>
    .checkout-page {
        min-height: 100vh;
        background: var(--bg-app);
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 1rem;
    }

    .checkout-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 16px;
        width: 100%;
        max-width: 480px;
        box-shadow: var(--shadow-lg);
        overflow: hidden;
    }

    .header {
        padding: 1.5rem;
        border-bottom: 1px solid var(--border-color);
        background: var(--bg-tertiary);
        text-align: center;
        position: relative;
    }

    .back-link {
        position: absolute;
        left: 1.5rem;
        top: 50%;
        transform: translateY(-50%);
        display: flex;
        align-items: center;
        gap: 0.5rem;
        background: transparent;
        border: none;
        color: var(--text-secondary);
        font-weight: 500;
        cursor: pointer;
        padding: 0.5rem;
        border-radius: 6px;
        transition: all 0.2s;
    }

    .back-link:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .header h1 {
        margin: 0;
        font-size: 1.5rem;
    }
    .invoice-number {
        color: var(--text-secondary);
        font-size: 0.9rem;
        font-family: monospace;
    }

    .summary-section {
        padding: 1.5rem;
        background: var(--bg-surface);
    }

    .item-row {
        display: flex;
        justify-content: space-between;
        margin-bottom: 0.5rem;
        font-size: 0.95rem;
    }

    .item-row.total {
        margin-top: 1rem;
        padding-top: 1rem;
        border-top: 1px dashed var(--border-color);
        font-weight: 700;
        font-size: 1.2rem;
    }

    .status-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-top: 1rem;
    }

    .status-pill {
        padding: 0.2rem 0.6rem;
        border-radius: 12px;
        font-size: 0.75rem;
        font-weight: 700;
        text-transform: uppercase;
    }
    .status-pill.pending {
        background: #fef3c7;
        color: #d97706;
    }
    .status-pill.paid {
        background: #dcfce7;
        color: #16a34a;
    }
    .status-pill.failed {
        background: #fee2e2;
        color: #dc2626;
    }

    /* Tabs */
    .payment-tabs {
        display: flex;
        border-bottom: 1px solid var(--border-color);
    }

    .tab {
        flex: 1;
        padding: 1rem;
        background: transparent;
        border: none;
        border-bottom: 2px solid transparent;
        cursor: pointer;
        font-weight: 500;
        color: var(--text-secondary);
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 0.5rem;
        transition: all 0.2s;
    }

    .tab.active {
        color: var(--color-primary);
        border-bottom-color: var(--color-primary);
        background: var(--color-primary-subtle);
    }

    .payment-content {
        padding: 1.5rem;
    }

    .online-method p {
        color: var(--text-secondary);
        margin-bottom: 1.5rem;
        text-align: center;
        font-size: 0.9rem;
    }

    .bank-list {
        display: flex;
        flex-direction: column;
        gap: 1rem;
        margin-bottom: 1.5rem;
    }

    .bank-item {
        border: 1px solid var(--border-color);
        border-radius: 8px;
        padding: 1rem;
        background: var(--bg-app);
    }

    .bank-name {
        font-weight: 700;
        margin-bottom: 0.25rem;
    }
    .bank-details {
        display: flex;
        justify-content: space-between;
        font-size: 0.9rem;
        color: var(--text-secondary);
    }
    .number {
        font-family: monospace;
        font-size: 1rem;
        color: var(--text-primary);
    }

    .btn {
        padding: 0.75rem 1.5rem;
        border-radius: 8px;
        font-weight: 600;
        border: none;
        cursor: pointer;
        transition: all 0.2s;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 0.5rem;
    }

    .btn-primary {
        background: var(--color-primary);
        color: white;
    }
    .btn-secondary {
        background: var(--bg-tertiary);
        color: var(--text-primary);
        border: 1px solid var(--border-color);
    }
    .w-full {
        width: 100%;
    }
    .btn-lg {
        font-size: 1rem;
        padding: 1rem;
    }

    .loading,
    .error {
        text-align: center;
        padding: 2rem;
    }

    .success-message {
        display: flex;
        flex-direction: column;
        align-items: center;
        text-align: center;
        padding: 2rem;
    }
    .icon-circle.success {
        width: 64px;
        height: 64px;
        background: #dcfce7;
        color: #16a34a;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        margin: 0 auto 1rem;
    }

    .pending-message {
        display: flex;
        flex-direction: column;
        align-items: center;
        text-align: center;
        padding: 2rem;
    }

    .icon-circle.pending {
        width: 64px;
        height: 64px;
        background: #fef3c7;
        color: #d97706;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        margin: 0 auto 1rem;
    }

    .upload-section {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 1rem;
        text-align: center;
    }
</style>

