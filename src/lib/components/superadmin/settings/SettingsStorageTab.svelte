<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { t } from "svelte-i18n";

    export let storageDriver: string;
    export let storageS3Bucket: string;
    export let storageS3Region: string;
    export let storageS3Endpoint: string;
    export let storageS3AccessKey: string;
    export let storageS3SecretKey: string;
    export let storageS3PublicUrl: string;
    export let storageMaxFileSizeMb: number;
    export let storageAllowedExtensions: string;

    const dispatch = createEventDispatcher();

    function handleChange() {
        dispatch("change");
    }
</script>

<div class="card section fade-in">
    <div class="card-header">
        <h3>
            {$t("superadmin.settings.sections.storage") ||
                "Storage Configuration"}
        </h3>
    </div>
    <div class="card-body">
        <!-- Driver Selection -->
        <div class="setting-row">
            <div class="setting-info">
                <label class="setting-label" for="storage-driver">
                    {$t("superadmin.settings.storage.driver.label") ||
                        "Storage Driver"}
                </label>
                <p class="setting-description">
                    {$t("superadmin.settings.storage.driver.desc") ||
                        "Choose where files are stored. Local uses the server's disk."}
                </p>
            </div>
            <div class="input-group">
                <select
                    id="storage-driver"
                    bind:value={storageDriver}
                    on:change={handleChange}
                    class="form-input native-select"
                >
                    <option value="local">
                        {$t(
                            "superadmin.settings.storage.driver.options.local",
                        ) || "Local Disk"}
                    </option>
                    <option value="s3">
                        {$t("superadmin.settings.storage.driver.options.s3") ||
                            "AWS S3 / MinIO"}
                    </option>
                    <option value="r2">
                        {$t("superadmin.settings.storage.driver.options.r2") ||
                            "Cloudflare R2"}
                    </option>
                </select>
            </div>
        </div>

        {#if storageDriver !== "local"}
            <div class="sub-settings fade-in">
                <div class="setting-row">
                    <div class="setting-info">
                        <label class="setting-label" for="bucket-name">
                            {$t("superadmin.settings.storage.s3.bucket_name") ||
                                "Bucket Name"}
                        </label>
                    </div>
                    <input
                        type="text"
                        id="bucket-name"
                        bind:value={storageS3Bucket}
                        on:input={handleChange}
                        class="form-input"
                        placeholder={$t(
                            "superadmin.settings.placeholders.s3_bucket",
                        ) || "e.g. my-app-uploads"}
                    />
                </div>
                <div class="setting-row">
                    <div class="setting-info">
                        <label class="setting-label" for="bucket-region">
                            {$t("superadmin.settings.storage.s3.region") ||
                                "Region"}
                        </label>
                        <p class="setting-description">
                            {$t("superadmin.settings.storage.s3.region_hint") ||
                                "Use 'auto' for R2."}
                        </p>
                    </div>
                    <input
                        type="text"
                        id="bucket-region"
                        bind:value={storageS3Region}
                        on:input={handleChange}
                        class="form-input"
                        placeholder={$t(
                            "superadmin.settings.placeholders.s3_region",
                        ) || "us-east-1"}
                    />
                </div>
                <div class="setting-row">
                    <div class="setting-info">
                        <label class="setting-label" for="bucket-endpoint">
                            {$t(
                                "superadmin.settings.storage.s3.endpoint_url",
                            ) || "Endpoint URL"}
                        </label>
                        <p class="setting-description">
                            {$t(
                                "superadmin.settings.storage.s3.endpoint_hint",
                            ) ||
                                "Required for R2 (https://ID.r2.cloudflarestorage.com) or MinIO."}
                        </p>
                    </div>
                    <input
                        type="text"
                        id="bucket-endpoint"
                        bind:value={storageS3Endpoint}
                        on:input={handleChange}
                        class="form-input"
                        placeholder={$t(
                            "superadmin.settings.placeholders.url",
                        ) || "https://..."}
                    />
                </div>
                <div class="setting-row">
                    <div class="setting-info">
                        <label class="setting-label" for="access-key">
                            {$t(
                                "superadmin.settings.storage.s3.access_key_id",
                            ) || "Access Key ID"}
                        </label>
                    </div>
                    <input
                        type="text"
                        id="access-key"
                        bind:value={storageS3AccessKey}
                        on:input={handleChange}
                        class="form-input"
                    />
                </div>
                <div class="setting-row">
                    <div class="setting-info">
                        <label class="setting-label" for="access-secret-key">
                            {$t(
                                "superadmin.settings.storage.s3.secret_access_key",
                            ) || "Secret Access Key"}
                        </label>
                    </div>
                    <input
                        type="password"
                        id="access-secret-key"
                        bind:value={storageS3SecretKey}
                        on:input={handleChange}
                        class="form-input"
                    />
                </div>
                <div class="setting-row">
                    <div class="setting-info">
                        <label class="setting-label" for="public-access-url">
                            {$t(
                                "superadmin.settings.storage.s3.public_url_optional",
                            ) || "Public Access URL (Optional)"}
                        </label>
                        <p class="setting-description">
                            {$t(
                                "superadmin.settings.storage.s3.public_url_hint",
                            ) || "CDN URL if serving files publicly."}
                        </p>
                    </div>
                    <input
                        type="text"
                        id="public-access-url"
                        bind:value={storageS3PublicUrl}
                        on:input={handleChange}
                        class="form-input"
                        placeholder={$t(
                            "superadmin.settings.placeholders.public_url",
                        ) || "https://cdn.example.com"}
                    />
                </div>
            </div>
        {/if}

        <div class="setting-row">
            <div class="setting-info">
                <label class="setting-label" for="max-file-size">
                    {$t("superadmin.settings.storage.max_file_size_mb") ||
                        "Max File Size (MB)"}
                </label>
                <p class="setting-description">
                    {$t("superadmin.settings.storage.max_file_size_mb_desc") ||
                        "Maximum allowed size for a single file upload."}
                </p>
            </div>
            <div class="input-group">
                <input
                    type="number"
                    id="max-file-size"
                    bind:value={storageMaxFileSizeMb}
                    on:input={handleChange}
                    min="1"
                    class="form-input"
                />
                <span class="input-suffix">{$t("common.units.mb") || "MB"}</span
                >
            </div>
        </div>

        <div class="setting-row">
            <div class="setting-info full-width">
                <label class="setting-label" for="allowed-extensions">
                    {$t("superadmin.settings.storage.allowed_extensions") ||
                        "Allowed Extensions"}
                </label>
                <p class="setting-description">
                    {$t(
                        "superadmin.settings.storage.allowed_extensions_desc",
                    ) ||
                        "Comma-separated list of allowed file extensions (e.g., jpg, png, pdf). Use * for all."}
                </p>
                <input
                    type="text"
                    id="allowed-extensions"
                    bind:value={storageAllowedExtensions}
                    on:input={handleChange}
                    class="form-input"
                    placeholder="jpg, png, pdf, zip"
                />
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

    .native-select {
        cursor: pointer;
        padding-right: 2rem;
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

    .sub-settings {
        padding: 1.5rem;
        background: rgba(0, 0, 0, 0.15);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
        margin: 1rem 0;
    }

    .sub-settings .setting-row {
        gap: 2rem;
        padding: 1rem 0;
    }

    .sub-settings .setting-row:first-child {
        padding-top: 0;
    }

    .sub-settings .setting-row:last-child {
        padding-bottom: 0;
    }

    .sub-settings .form-input {
        max-width: 320px;
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

    @media (max-width: 640px) {
        .sub-settings .form-input {
            max-width: 100%;
        }
    }
</style>
